use super::*;

#[pyclass]
pub struct Database {
    sender: Sender<Job>,
}

#[pymethods]
impl Database {
    fn test(self_: PyRef<'_, Self>, num: u32) -> u32 {
        match SessionConnection::send_command(self_.py(), &self_.sender, Command::Test(num)) {
            Answer::Test(val) => val,
        }
    }
}

use std::sync::mpsc::{self, Receiver, Sender};

use crate::frontend::state;

#[derive(Debug, Clone)]
pub enum Command {
    Test(u32),
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
    Test(u32),
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
        while let Ok(job) = queue_receiver.recv() {
            if let Command::Exit = &job.command {
                return;
            }

            let answer_data = Self::execute_job(&job.command, manager);
            job.answer.send(answer_data).unwrap();
        }
    }

    fn execute_job<T: state::Manager>(command: &Command, _manager: &mut T) -> Answer {
        match command {
            Command::Test(val) => Answer::Test(val + 1),
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
