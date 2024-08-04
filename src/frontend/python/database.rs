use pyo3::exceptions::{PyException, PyValueError};
use std::collections::BTreeMap;

use super::*;

mod classes;
use classes::*;

mod utils;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    use utils::*;

    m.add_class::<WeekPattern>()?;
    m.add_class::<Teacher>()?;
    m.add_class::<Student>()?;
    m.add_class::<SubjectGroup>()?;
    m.add_class::<Weekday>()?;
    m.add_class::<Time>()?;
    m.add_class::<SlotStart>()?;
    m.add_class::<IncompatSlot>()?;
    m.add_class::<Incompat>()?;
    m.add_class::<Group>()?;
    m.add_class::<GroupList>()?;
    m.add_class::<Subject>()?;

    m.add_function(wrap_pyfunction!(extract_name_parts, m)?)?;

    Ok(())
}

#[pyclass]
pub struct Database {
    sender: Sender<Job>,
}

#[pymethods]
impl Database {
    fn undo(self_: PyRef<'_, Self>) -> PyResult<()> {
        let Answer::Undo =
            SessionConnection::send_command(self_.py(), &self_.sender, Command::Undo)?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn redo(self_: PyRef<'_, Self>) -> PyResult<()> {
        let Answer::Redo =
            SessionConnection::send_command(self_.py(), &self_.sender, Command::Redo)?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn general_data_get(self_: PyRef<'_, Self>) -> PyResult<GeneralData> {
        let Answer::GeneralData(GeneralDataAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GeneralData(GeneralDataCommand::Get),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn general_data_set(self_: PyRef<'_, Self>, general_data: GeneralData) -> PyResult<()> {
        let Answer::GeneralData(GeneralDataAnswer::Set) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GeneralData(GeneralDataCommand::Set(general_data)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn week_patterns_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<WeekPatternHandle, WeekPattern>> {
        let Answer::WeekPatterns(WeekPatternsAnswer::GetAll(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::WeekPatterns(WeekPatternsCommand::GetAll),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn week_patterns_get(
        self_: PyRef<'_, Self>,
        handle: WeekPatternHandle,
    ) -> PyResult<WeekPattern> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatterns(WeekPatternsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn week_patterns_create(
        self_: PyRef<'_, Self>,
        pattern: WeekPattern,
    ) -> PyResult<WeekPatternHandle> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Create(handle)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::WeekPatterns(WeekPatternsCommand::Create(pattern)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn week_patterns_update(
        self_: PyRef<'_, Self>,
        handle: WeekPatternHandle,
        pattern: WeekPattern,
    ) -> PyResult<()> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatterns(WeekPatternsCommand::Update(handle, pattern)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn week_patterns_remove(self_: PyRef<'_, Self>, handle: WeekPatternHandle) -> PyResult<()> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatterns(WeekPatternsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn teachers_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<TeacherHandle, Teacher>> {
        let Answer::Teachers(TeachersAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn teachers_get(self_: PyRef<'_, Self>, handle: TeacherHandle) -> PyResult<Teacher> {
        let Answer::Teachers(TeachersAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn teachers_create(self_: PyRef<'_, Self>, teacher: Teacher) -> PyResult<TeacherHandle> {
        let Answer::Teachers(TeachersAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Create(teacher)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn teachers_update(
        self_: PyRef<'_, Self>,
        handle: TeacherHandle,
        teacher: Teacher,
    ) -> PyResult<()> {
        let Answer::Teachers(TeachersAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Update(handle, teacher)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn teachers_remove(self_: PyRef<'_, Self>, handle: TeacherHandle) -> PyResult<()> {
        let Answer::Teachers(TeachersAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn students_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<StudentHandle, Student>> {
        let Answer::Students(StudentsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn students_get(self_: PyRef<'_, Self>, handle: StudentHandle) -> PyResult<Student> {
        let Answer::Students(StudentsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn students_create(self_: PyRef<'_, Self>, student: Student) -> PyResult<StudentHandle> {
        let Answer::Students(StudentsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Create(student)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn students_update(
        self_: PyRef<'_, Self>,
        handle: StudentHandle,
        student: Student,
    ) -> PyResult<()> {
        let Answer::Students(StudentsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Update(handle, student)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn students_remove(self_: PyRef<'_, Self>, handle: StudentHandle) -> PyResult<()> {
        let Answer::Students(StudentsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subject_groups_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<SubjectGroupHandle, SubjectGroup>> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::GetAll(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SubjectGroups(SubjectGroupsCommand::GetAll),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn subject_groups_get(
        self_: PyRef<'_, Self>,
        handle: SubjectGroupHandle,
    ) -> PyResult<SubjectGroup> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SubjectGroups(SubjectGroupsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn subject_groups_create(
        self_: PyRef<'_, Self>,
        subject_group: SubjectGroup,
    ) -> PyResult<SubjectGroupHandle> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Create(handle)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SubjectGroups(SubjectGroupsCommand::Create(subject_group)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn subject_groups_update(
        self_: PyRef<'_, Self>,
        handle: SubjectGroupHandle,
        subject_group: SubjectGroup,
    ) -> PyResult<()> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SubjectGroups(SubjectGroupsCommand::Update(handle, subject_group)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subject_groups_remove(self_: PyRef<'_, Self>, handle: SubjectGroupHandle) -> PyResult<()> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SubjectGroups(SubjectGroupsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }
}

use std::sync::mpsc::{self, Receiver, Sender};

use crate::backend::{self, IdError};
use crate::frontend::state::update::ReturnHandle;
use crate::frontend::state::{self, Operation, RedoError, UndoError, UpdateError};

#[derive(Debug, Clone)]
pub enum Command {
    GeneralData(GeneralDataCommand),
    WeekPatterns(WeekPatternsCommand),
    Teachers(TeachersCommand),
    Students(StudentsCommand),
    SubjectGroups(SubjectGroupsCommand),
    Undo,
    Redo,
    Exit,
}

#[derive(Debug, Clone)]
pub enum GeneralDataCommand {
    Get,
    Set(GeneralData),
}

#[derive(Debug, Clone)]
pub enum WeekPatternsCommand {
    GetAll,
    Get(WeekPatternHandle),
    Create(WeekPattern),
    Update(WeekPatternHandle, WeekPattern),
    Remove(WeekPatternHandle),
}

#[derive(Debug, Clone)]
pub enum TeachersCommand {
    GetAll,
    Get(TeacherHandle),
    Create(Teacher),
    Update(TeacherHandle, Teacher),
    Remove(TeacherHandle),
}

#[derive(Debug, Clone)]
pub enum StudentsCommand {
    GetAll,
    Get(StudentHandle),
    Create(Student),
    Update(StudentHandle, Student),
    Remove(StudentHandle),
}

#[derive(Debug, Clone)]
pub enum SubjectGroupsCommand {
    GetAll,
    Get(SubjectGroupHandle),
    Create(SubjectGroup),
    Update(SubjectGroupHandle, SubjectGroup),
    Remove(SubjectGroupHandle),
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
    GeneralData(GeneralDataAnswer),
    WeekPatterns(WeekPatternsAnswer),
    Teachers(TeachersAnswer),
    Students(StudentsAnswer),
    SubjectGroups(SubjectGroupsAnswer),
    Undo,
    Redo,
}

#[derive(Debug)]
pub enum GeneralDataAnswer {
    Get(GeneralData),
    Set,
}

#[derive(Debug)]
pub enum WeekPatternsAnswer {
    GetAll(BTreeMap<WeekPatternHandle, WeekPattern>),
    Get(WeekPattern),
    Create(WeekPatternHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum TeachersAnswer {
    GetAll(BTreeMap<TeacherHandle, Teacher>),
    Get(Teacher),
    Create(TeacherHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum StudentsAnswer {
    GetAll(BTreeMap<StudentHandle, Student>),
    Get(Student),
    Create(StudentHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum SubjectGroupsAnswer {
    GetAll(BTreeMap<SubjectGroupHandle, SubjectGroup>),
    Get(SubjectGroup),
    Create(SubjectGroupHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub struct Job {
    command: Command,
    answer: Sender<PyResult<Answer>>,
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

    async fn execute_general_data_job<T: state::Manager>(
        general_data_command: &GeneralDataCommand,
        manager: &mut T,
    ) -> PyResult<GeneralDataAnswer> {
        match general_data_command {
            GeneralDataCommand::Get => {
                let general_data = manager
                    .general_data_get()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))?;

                Ok(GeneralDataAnswer::Get(general_data.into()))
            }
            GeneralDataCommand::Set(general_data) => {
                manager
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
                    })?;

                Ok(GeneralDataAnswer::Set)
            }
        }
    }

    async fn execute_week_patterns_job<T: state::Manager>(
        week_patterns_command: &WeekPatternsCommand,
        manager: &mut T,
    ) -> PyResult<WeekPatternsAnswer> {
        match week_patterns_command {
            WeekPatternsCommand::GetAll => {
                let result = manager
                    .week_patterns_get_all()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, pattern)| (handle.into(), WeekPattern::from(pattern)))
                    .collect::<BTreeMap<_, _>>();

                Ok(WeekPatternsAnswer::GetAll(result))
            }
            WeekPatternsCommand::Get(handle) => {
                let result =
                    manager
                        .week_patterns_get(handle.handle)
                        .await
                        .map_err(|e| match e {
                            IdError::InternalError(int_err) => {
                                PyException::new_err(int_err.to_string())
                            }
                            IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                        })?;

                Ok(WeekPatternsAnswer::Get(result.into()))
            }
            WeekPatternsCommand::Create(pattern) => {
                let output = manager
                    .apply(Operation::WeekPatterns(
                        state::WeekPatternsOperation::Create(pattern.into()),
                    ))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::WeekNumberTooBig(_) => {
                            PyValueError::new_err("Week number larger than week_count")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::WeekPattern(handle) = output else {
                    panic!("No week pattern handle returned on WeekPatternsOperation::Create");
                };

                Ok(WeekPatternsAnswer::Create(handle.into()))
            }
            WeekPatternsCommand::Update(handle, pattern) => {
                manager
                    .apply(Operation::WeekPatterns(
                        state::WeekPatternsOperation::Update(handle.handle, pattern.into()),
                    ))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::WeekNumberTooBig(_) => {
                            PyValueError::new_err("Week number larger than week_count")
                        }
                        UpdateError::WeekPatternRemoved(_) => {
                            PyValueError::new_err("Week pattern was previsouly removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(WeekPatternsAnswer::Update)
            }
            WeekPatternsCommand::Remove(handle) => {
                manager
                    .apply(Operation::WeekPatterns(
                        state::WeekPatternsOperation::Remove(handle.handle),
                    ))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::WeekPatternRemoved(_) => {
                            PyValueError::new_err("Week pattern was previsouly removed")
                        }
                        UpdateError::WeekPatternDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this week pattern",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(WeekPatternsAnswer::Remove)
            }
        }
    }

    async fn execute_teachers_job<T: state::Manager>(
        teachers_command: &TeachersCommand,
        manager: &mut T,
    ) -> PyResult<TeachersAnswer> {
        match teachers_command {
            TeachersCommand::GetAll => {
                let result = manager
                    .teachers_get_all()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, teacher)| (handle.into(), Teacher::from(teacher)))
                    .collect::<BTreeMap<_, _>>();

                Ok(TeachersAnswer::GetAll(result))
            }
            TeachersCommand::Get(handle) => {
                let result = manager
                    .teachers_get(handle.handle)
                    .await
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })?;

                Ok(TeachersAnswer::Get(result.into()))
            }
            TeachersCommand::Create(teacher) => {
                let output = manager
                    .apply(Operation::Teachers(state::TeachersOperation::Create(
                        teacher.into(),
                    )))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Teacher(handle) = output else {
                    panic!("No teacher handle returned on TeachersOperation::Create");
                };

                Ok(TeachersAnswer::Create(handle.into()))
            }
            TeachersCommand::Update(handle, teacher) => {
                manager
                    .apply(Operation::Teachers(state::TeachersOperation::Update(
                        handle.handle,
                        teacher.into(),
                    )))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TeacherRemoved(_) => {
                            PyValueError::new_err("Teacher was previsouly removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(TeachersAnswer::Update)
            }
            TeachersCommand::Remove(handle) => {
                manager
                    .apply(Operation::Teachers(state::TeachersOperation::Remove(
                        handle.handle,
                    )))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TeacherRemoved(_) => {
                            PyValueError::new_err("Teacher was previsouly removed")
                        }
                        UpdateError::TeacherDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this teacher",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(TeachersAnswer::Remove)
            }
        }
    }

    async fn execute_students_job<T: state::Manager>(
        students_command: &StudentsCommand,
        manager: &mut T,
    ) -> PyResult<StudentsAnswer> {
        match students_command {
            StudentsCommand::GetAll => {
                let result = manager
                    .students_get_all()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, student)| (handle.into(), Student::from(student)))
                    .collect::<BTreeMap<_, _>>();

                Ok(StudentsAnswer::GetAll(result))
            }
            StudentsCommand::Get(handle) => {
                let result = manager
                    .students_get(handle.handle)
                    .await
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })?;

                Ok(StudentsAnswer::Get(result.into()))
            }
            StudentsCommand::Create(student) => {
                let output = manager
                    .apply(Operation::Students(state::StudentsOperation::Create(
                        student.into(),
                    )))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Student(handle) = output else {
                    panic!("No student handle returned on StudentsOperation::Create");
                };

                Ok(StudentsAnswer::Create(handle.into()))
            }
            StudentsCommand::Update(handle, student) => {
                manager
                    .apply(Operation::Students(state::StudentsOperation::Update(
                        handle.handle,
                        student.into(),
                    )))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::StudentRemoved(_) => {
                            PyValueError::new_err("Student was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(StudentsAnswer::Update)
            }
            StudentsCommand::Remove(handle) => {
                manager
                    .apply(Operation::Students(state::StudentsOperation::Remove(
                        handle.handle,
                    )))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::StudentRemoved(_) => {
                            PyValueError::new_err("Student was previously removed")
                        }
                        UpdateError::StudentDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this student",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(StudentsAnswer::Remove)
            }
        }
    }

    async fn execute_subject_groups_job<T: state::Manager>(
        subject_groups_command: &SubjectGroupsCommand,
        manager: &mut T,
    ) -> PyResult<SubjectGroupsAnswer> {
        match subject_groups_command {
            SubjectGroupsCommand::GetAll => {
                let result = manager
                    .subject_groups_get_all()
                    .await
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, subject_group)| {
                        (handle.into(), SubjectGroup::from(subject_group))
                    })
                    .collect::<BTreeMap<_, _>>();

                Ok(SubjectGroupsAnswer::GetAll(result))
            }
            SubjectGroupsCommand::Get(handle) => {
                let result =
                    manager
                        .subject_groups_get(handle.handle)
                        .await
                        .map_err(|e| match e {
                            IdError::InternalError(int_err) => {
                                PyException::new_err(int_err.to_string())
                            }
                            IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                        })?;

                Ok(SubjectGroupsAnswer::Get(result.into()))
            }
            SubjectGroupsCommand::Create(subject_group) => {
                let output = manager
                    .apply(Operation::SubjectGroups(
                        state::SubjectGroupsOperation::Create(subject_group.into()),
                    ))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::SubjectGroup(handle) = output else {
                    panic!("No subject group handle returned on SubjectGroupsCommand::Create");
                };

                Ok(SubjectGroupsAnswer::Create(handle.into()))
            }
            SubjectGroupsCommand::Update(handle, subject_group) => {
                manager
                    .apply(Operation::SubjectGroups(
                        state::SubjectGroupsOperation::Update(handle.handle, subject_group.into()),
                    ))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectGroupRemoved(_) => {
                            PyValueError::new_err("Subject group was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SubjectGroupsAnswer::Update)
            }
            SubjectGroupsCommand::Remove(handle) => {
                manager
                    .apply(Operation::SubjectGroups(
                        state::SubjectGroupsOperation::Remove(handle.handle),
                    ))
                    .await
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectGroupRemoved(_) => {
                            PyValueError::new_err("Subject group was previously removed")
                        }
                        UpdateError::SubjectGroupDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this subject group",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SubjectGroupsAnswer::Remove)
            }
        }
    }

    async fn execute_job<T: state::Manager>(
        command: &Command,
        manager: &mut T,
    ) -> PyResult<Answer> {
        match command {
            Command::GeneralData(general_data_command) => {
                let answer = Self::execute_general_data_job(general_data_command, manager).await?;
                Ok(Answer::GeneralData(answer))
            }
            Command::WeekPatterns(week_patterns_command) => {
                let answer =
                    Self::execute_week_patterns_job(week_patterns_command, manager).await?;
                Ok(Answer::WeekPatterns(answer))
            }
            Command::Teachers(teachers_command) => {
                let answer = Self::execute_teachers_job(teachers_command, manager).await?;
                Ok(Answer::Teachers(answer))
            }
            Command::Students(students_command) => {
                let answer = Self::execute_students_job(students_command, manager).await?;
                Ok(Answer::Students(answer))
            }
            Command::SubjectGroups(subject_groups_command) => {
                let answer =
                    Self::execute_subject_groups_job(subject_groups_command, manager).await?;
                Ok(Answer::SubjectGroups(answer))
            }
            Command::Undo => {
                manager.undo().await.map_err(|e| match e {
                    UndoError::HistoryDepleted => PyException::new_err("History depleted"),
                    UndoError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                })?;

                Ok(Answer::Undo)
            }
            Command::Redo => {
                manager.redo().await.map_err(|e| match e {
                    RedoError::HistoryFullyRewounded => {
                        PyException::new_err("History fully rewounded")
                    }
                    RedoError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                })?;

                Ok(Answer::Redo)
            }
            Command::Exit => panic!("Exit command should be treated on level above"),
        }
    }

    fn send_command_internal(sender: &Sender<Job>, command: Command) -> Receiver<PyResult<Answer>> {
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

    fn send_command(py: Python, sender: &Sender<Job>, command: Command) -> PyResult<Answer> {
        let receiver = Self::send_command_internal(sender, command);

        py.allow_threads(move || receiver.recv().unwrap())
    }
}
