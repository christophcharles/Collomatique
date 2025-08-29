use pyo3::exceptions::{PyException, PyValueError};
use std::collections::BTreeMap;

use super::*;

mod classes;
use classes::*;

#[pyclass]
pub struct Database {
    sender: Sender<Job>,
}

#[pymethods]
impl Database {
    fn general_data_get(self_: PyRef<'_, Self>) -> PyResult<GeneralData> {
        let Answer::GeneralDataGet(val) =
            SessionConnection::send_command(self_.py(), &self_.sender, Command::GeneralDataGet)
        else {
            panic!("Bad answer type");
        };

        val
    }

    fn general_data_set(self_: PyRef<'_, Self>, general_data: GeneralData) -> PyResult<()> {
        let Answer::GeneralDataSet(val) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GeneralDataSet(general_data),
        ) else {
            panic!("Bad answer type");
        };

        val
    }

    fn week_patterns_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<WeekPatternHandle, WeekPattern>> {
        let Answer::WeekPatternsGetAll(val) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatternsGetAll,
        ) else {
            panic!("Bad answer type");
        };

        val
    }

    fn week_patterns_get(self_: PyRef<'_, Self>, handle: WeekPatternHandle) -> PyResult<WeekPattern> {
        let Answer::WeekPatternsGet(val) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatternsGet(handle),
        ) else {
            panic!("Bad answer type");
        };

        val
    }
}

use std::sync::mpsc::{self, Receiver, Sender};

use crate::backend::{self, IdError};
use crate::frontend::state::{self, Operation, UpdateError};

#[derive(Debug, Clone)]
pub enum Command {
    GeneralDataGet,
    GeneralDataSet(GeneralData),
    WeekPatternsGetAll,
    WeekPatternsGet(WeekPatternHandle),
    Exit,
}

#[derive(Debug)]
struct PythonError {
    int_err: Box<dyn std::error::Error + Send>,
}

impl std::fmt::Display for PythonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &*self.int_err)
    }
}

impl std::error::Error for PythonError {}

#[derive(Debug)]
pub enum Answer {
    GeneralDataGet(PyResult<GeneralData>),
    GeneralDataSet(PyResult<()>),
    WeekPatternsGetAll(PyResult<BTreeMap<WeekPatternHandle, WeekPattern>>),
    WeekPatternsGet(PyResult<WeekPattern>),
}

#[derive(Debug)]
pub struct Job {
    command: Command,
    answer: Sender<Answer>,
}

#[derive(Debug)]
pub struct SessionConnection<'scope> {
    queue_sender: Sender<Job>,
    thread: Option<std::thread::ScopedJoinHandle<'scope, ()>>,
}

impl<'scope> Drop for SessionConnection<'scope> {
    fn drop(&mut self) {
        if self.thread.is_some() {
            drop(Self::send_command_internal(
                &self.queue_sender,
                Command::Exit,
            ));
        }
    }
}

impl<'scope> SessionConnection<'scope> {
    pub fn new<T: state::Manager>(
        scope: &'scope std::thread::Scope<'scope, '_>,
        manager: &'scope mut T,
    ) -> SessionConnection<'scope> {
        let (queue_sender, queue_receiver) = mpsc::channel();

        let thread = Some(scope.spawn(move || {
            SessionConnection::thread_func(queue_receiver, manager);
        }));

        SessionConnection {
            queue_sender,
            thread,
        }
    }

    pub fn python_database(&self) -> Database {
        Database {
            sender: self.queue_sender.clone(),
        }
    }

    pub fn join(mut self) {
        drop(Self::send_command_internal(
            &self.queue_sender,
            Command::Exit,
        ));
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }

    fn thread_func<T: state::Manager>(queue_receiver: Receiver<Job>, manager: &'scope mut T) {
        use tokio::runtime::Runtime;
        let rt = Runtime::new().unwrap();

        while let Ok(job) = queue_receiver.recv() {
            if let Command::Exit = &job.command {
                return;
            }

            let answer_data = rt.block_on(Self::execute_job(&job.command, manager));
            job.answer.send(answer_data).unwrap();
        }
    }

    async fn execute_job<T: state::Manager>(command: &Command, manager: &mut T) -> Answer {
        match command {
            Command::GeneralDataGet => {
                let general_data = manager
                    .general_data_get()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))
                    .map(GeneralData::from);

                Answer::GeneralDataGet(general_data)
            }
            Command::GeneralDataSet(general_data) => {
                let result = manager
                    .apply(Operation::GeneralData(general_data.into()))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::InterrogationsPerWeekRangeIsEmpty => {
                            PyValueError::new_err("Interrogations per week range is empty")
                        }
                        UpdateError::WeekPatternsNeedTruncating(_week_patterns) => {
                            PyValueError::new_err("Some wwek patterns need truncating")
                        }
                        _ => panic!("Unexpected error!"),
                    });

                Answer::GeneralDataSet(result)
            }
            Command::WeekPatternsGetAll => {
                let result = manager
                    .week_patterns_get_all()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))
                    .map(|map| {
                        map.into_iter().map(|(handle, pattern)| (
                            WeekPatternHandle { handle },
                            WeekPattern::from(pattern)
                        ))
                        .collect::<BTreeMap<_,_>>()
                    });

                Answer::WeekPatternsGetAll(result)
            }
            Command::WeekPatternsGet(handle) => {
                let result = manager.week_patterns_get(handle.handle)
                    .await
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })
                    .map(WeekPattern::from);

                Answer::WeekPatternsGet(result)
            }
            Command::Exit => panic!("Exit command should be treated on level above"),
        }
    }

    fn send_command_internal(sender: &Sender<Job>, command: Command) -> Receiver<Answer> {
        let (answer_sender, answer_receiver) = mpsc::channel();

        let job = Job {
            command,
            answer: answer_sender,
        };

        sender
            .send(job)
            .expect("Python code should have finished before worker thread.");

        answer_receiver
    }

    fn send_command(py: Python, sender: &Sender<Job>, command: Command) -> Answer {
        let receiver = Self::send_command_internal(sender, command);

        py.allow_threads(move || receiver.recv().unwrap())
    }
}
