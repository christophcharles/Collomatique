use std::collections::BTreeMap;

use pyo3::prelude::*;

use crate::rpc::{
    cmd_msg::{ExtensionDesc, MsgStudentId, MsgWeekPatternId, OpenFileDialogMsg},
    error_msg::{
        AddNewStudentError, AddNewSubjectError, AddNewTeacherError, AssignAllError, AssignError,
        AssignmentsError, CutPeriodError, DeletePeriodError, DeleteStudentError,
        DeleteSubjectError, DeleteTeacherError, DeleteWeekPatternError,
        DuplicatePreviousPeriodError, GeneralPlanningError, MergeWithPreviousPeriodError,
        MoveDownError, MoveUpError, StudentsError, SubjectsError, TeachersError,
        UpdatePeriodStatusError, UpdatePeriodWeekCountError, UpdateStudentError,
        UpdateSubjectError, UpdateTeacherError, UpdateWeekPatternError, UpdateWeekStatusError,
        WeekPatternsError,
    },
    ErrorMsg, GuiAnswer, ResultMsg,
};

use pyo3::exceptions::PyValueError;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    m.add_class::<general_planning::Period>()?;
    m.add_class::<subjects::Subject>()?;
    m.add_class::<subjects::SubjectParameters>()?;
    m.add_class::<subjects::SubjectPeriodicity>()?;
    m.add_class::<teachers::Teacher>()?;
    m.add_class::<students::Student>()?;
    m.add_class::<time::NaiveMondayDate>()?;
    m.add_class::<time::NaiveDate>()?;

    m.add_function(wrap_pyfunction!(log, m)?)?;
    m.add_function(wrap_pyfunction!(current_session, m)?)?;
    m.add_function(wrap_pyfunction!(open_dialog, m)?)?;

    Ok(())
}

#[pyfunction]
pub fn log(msg: String) {
    use std::io::Write;
    eprint!("{}\r\n", msg);
    std::io::stderr().flush().expect("no error on flush");
}

#[pyfunction]
pub fn current_session() -> Session {
    Session { token: Token {} }
}

#[pyfunction]
pub fn open_dialog(title: String, list: Vec<(String, String)>) -> Option<std::path::PathBuf> {
    let token = Token {};

    let result = token.send_msg(crate::rpc::CmdMsg::GuiRequest(
        crate::rpc::cmd_msg::GuiMsg::OpenFileDialog(OpenFileDialogMsg {
            title,
            list: list
                .into_iter()
                .map(|ext| ExtensionDesc {
                    desc: ext.0,
                    extension: ext.1,
                })
                .collect(),
        }),
    ));

    match result {
        ResultMsg::AckGui(GuiAnswer::OpenFileDialog(answer)) => answer.file_path,
        _ => panic!("Unexpected result: {:?}", result),
    }
}

#[pyclass]
pub struct Session {
    token: Token,
}

mod common;
use common::PersonWithContact;
mod general_planning;
use general_planning::{Period, PeriodId};
mod subjects;
use subjects::{Subject, SubjectId};
mod teachers;
mod time;
use teachers::{Teacher, TeacherId};
mod students;
use students::{Student, StudentId};
mod week_patterns;
use week_patterns::{WeekPattern, WeekPatternId};

use crate::rpc::cmd_msg::{MsgPeriodId, MsgSubjectId, MsgTeacherId};

#[pymethods]
impl Session {
    fn periods_add(self_: PyRef<'_, Self>, week_count: usize) -> PeriodId {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::AddNewPeriod(week_count),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::PeriodId(id))) => id.into(),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_update(self_: PyRef<'_, Self>, id: PeriodId, new_week_count: usize) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::UpdatePeriodWeekCount(
                    id.into(),
                    new_week_count,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GeneralPlanning(
                GeneralPlanningError::UpdatePeriodWeekCount(e),
            )) => match e {
                UpdatePeriodWeekCountError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                UpdatePeriodWeekCountError::SubjectImpliesMinimumWeekCount(id, wc) => {
                    Err(PyValueError::new_err(format!(
                        "Minimum week count of {} required by subject {:?}",
                        wc, id
                    )))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_delete(self_: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::DeletePeriod(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GeneralPlanning(GeneralPlanningError::DeletePeriod(e))) => {
                match e {
                    DeletePeriodError::InvalidPeriodId(id) => {
                        Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_cut(
        self_: PyRef<'_, Self>,
        id: PeriodId,
        remaining_weeks: usize,
    ) -> PyResult<PeriodId> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::CutPeriod(id.into(), remaining_weeks),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::PeriodId(new_id))) => Ok(new_id.into()),
            ResultMsg::Error(ErrorMsg::GeneralPlanning(GeneralPlanningError::CutPeriod(e))) => {
                match e {
                    CutPeriodError::InvalidPeriodId(id) => {
                        Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                    }
                    CutPeriodError::RemainingWeekCountTooBig(w, t) => Err(PyValueError::new_err(
                        format!("Remaining weeks too big ({} > {})", w, t),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_merge_with_previous(self_: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::MergeWithPreviousPeriod(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GeneralPlanning(
                GeneralPlanningError::MergeWithPreviousPeriod(e),
            )) => match e {
                MergeWithPreviousPeriodError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith => {
                    Err(PyValueError::new_err(format!("Cannot merge first period")))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_update_week_status(
        self_: PyRef<'_, Self>,
        id: PeriodId,
        week: usize,
        new_status: bool,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::UpdateWeekStatus(
                    id.into(),
                    week,
                    new_status,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GeneralPlanning(
                GeneralPlanningError::UpdateWeekStatus(e),
            )) => match e {
                UpdateWeekStatusError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                UpdateWeekStatusError::InvalidWeekNumber(w, t) => Err(PyValueError::new_err(
                    format!("Week number too big ({} >= {}", w, t),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_set_first_week(self_: PyRef<'_, Self>, first_week: Option<time::NaiveMondayDate>) {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(match first_week {
                Some(week) => crate::rpc::cmd_msg::GeneralPlanningCmdMsg::UpdateFirstWeek(
                    collomatique_time::NaiveMondayDate::from(week).into_inner(),
                ),
                None => crate::rpc::cmd_msg::GeneralPlanningCmdMsg::DeleteFirstWeek,
            }),
        ));

        if result != ResultMsg::Ack(None) {
            panic!("Unexpected result: {:?}", result)
        }
    }

    fn periods_get_first_week(self_: PyRef<'_, Self>) -> Option<time::NaiveMondayDate> {
        self_
            .token
            .get_data()
            .get_periods()
            .first_week
            .as_ref()
            .map(|x| x.clone().into())
    }

    fn periods_get_list(self_: PyRef<'_, Self>) -> Vec<Period> {
        self_
            .token
            .get_data()
            .get_periods()
            .ordered_period_list
            .iter()
            .map(|(id, data)| Period {
                id: MsgPeriodId::from(*id).into(),
                weeks_status: data.clone(),
            })
            .collect()
    }

    fn subjects_add(
        self_: PyRef<'_, Self>,
        subject_params: subjects::SubjectParameters,
    ) -> PyResult<SubjectId> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Subjects(
                    crate::rpc::cmd_msg::SubjectsCmdMsg::AddNewSubject(subject_params.into()),
                )));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::SubjectId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::Subjects(SubjectsError::AddNewSubject(e))) => match e {
                AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty => Err(
                    PyValueError::new_err("groups per interrogation range cannot be empty"),
                ),
                AddNewSubjectError::StudentsPerGroupRangeIsEmpty => Err(PyValueError::new_err(
                    "students per group range cannot be empty",
                )),
                AddNewSubjectError::InterrogationCountRangeIsEmpty => Err(PyValueError::new_err(
                    "interrogation count range cannot be empty",
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_update(
        self_: PyRef<'_, Self>,
        id: SubjectId,
        new_subject_params: subjects::SubjectParameters,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Subjects(
                    crate::rpc::cmd_msg::SubjectsCmdMsg::UpdateSubject(
                        id.into(),
                        new_subject_params.into(),
                    ),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Subjects(SubjectsError::UpdateSubject(e))) => match e {
                UpdateSubjectError::GroupsPerInterrogationRangeIsEmpty => Err(
                    PyValueError::new_err("groups per interrogation range cannot be empty"),
                ),
                UpdateSubjectError::StudentsPerGroupRangeIsEmpty => Err(PyValueError::new_err(
                    "students per group range cannot be empty",
                )),
                UpdateSubjectError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                UpdateSubjectError::InterrogationCountRangeIsEmpty => Err(PyValueError::new_err(
                    "interrogation count range cannot be empty",
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_delete(self_: PyRef<'_, Self>, id: SubjectId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Subjects(
                    crate::rpc::cmd_msg::SubjectsCmdMsg::DeleteSubject(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Subjects(SubjectsError::DeleteSubject(e))) => match e {
                DeleteSubjectError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_move_up(self_: PyRef<'_, Self>, id: SubjectId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Subjects(
                    crate::rpc::cmd_msg::SubjectsCmdMsg::MoveUp(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Subjects(SubjectsError::MoveUp(e))) => match e {
                MoveUpError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                MoveUpError::NoUpperPosition => {
                    Err(PyValueError::new_err("The subject is already the first"))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_move_down(self_: PyRef<'_, Self>, id: SubjectId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Subjects(
                    crate::rpc::cmd_msg::SubjectsCmdMsg::MoveDown(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Subjects(SubjectsError::MoveDown(e))) => match e {
                MoveDownError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                MoveDownError::NoLowerPosition => {
                    Err(PyValueError::new_err("The subject is already the last"))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_update_period_status(
        self_: PyRef<'_, Self>,
        subject_id: SubjectId,
        period_id: PeriodId,
        new_status: bool,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Subjects(
                    crate::rpc::cmd_msg::SubjectsCmdMsg::UpdatePeriodStatus(
                        subject_id.into(),
                        period_id.into(),
                        new_status,
                    ),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Subjects(SubjectsError::UpdatePeriodStatus(e))) => match e {
                UpdatePeriodStatusError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                    format!("Invalid subject id {:?}", id),
                )),
                UpdatePeriodStatusError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_get_period_status(
        self_: PyRef<'_, Self>,
        subject_id: SubjectId,
        period_id: PeriodId,
    ) -> PyResult<bool> {
        let data = self_.token.get_data();

        let Some(validated_subject_id) =
            data.validate_subject_id(MsgSubjectId::from(subject_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid subject id {:?}",
                subject_id
            )));
        };

        let Some(subject) = data.get_subjects().find_subject(validated_subject_id) else {
            panic!("subject id should be valid at this point");
        };

        let Some(validated_period_id) =
            data.validate_period_id(MsgPeriodId::from(period_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid period id {:?}",
                period_id
            )));
        };

        Ok(!subject.excluded_periods.contains(&validated_period_id))
    }

    fn subjects_get_list(self_: PyRef<'_, Self>) -> Vec<subjects::Subject> {
        self_
            .token
            .get_data()
            .get_subjects()
            .ordered_subject_list
            .iter()
            .map(|(id, data)| Subject {
                id: MsgSubjectId::from(*id).into(),
                parameters: data.parameters.clone().into(),
            })
            .collect()
    }

    fn teachers_add(self_: PyRef<'_, Self>, teacher: teachers::Teacher) -> PyResult<TeacherId> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Teachers(
                    crate::rpc::cmd_msg::TeachersCmdMsg::AddNewTeacher(teacher.into()),
                )));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::TeacherId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::Teachers(TeachersError::AddNewTeacher(e))) => match e {
                AddNewTeacherError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn teachers_update(
        self_: PyRef<'_, Self>,
        id: TeacherId,
        new_teacher: teachers::Teacher,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Teachers(
                    crate::rpc::cmd_msg::TeachersCmdMsg::UpdateTeacher(
                        id.into(),
                        new_teacher.into(),
                    ),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Teachers(TeachersError::UpdateTeacher(e))) => {
                match e {
                    UpdateTeacherError::InvalidTeacherId(id) => Err(PyValueError::new_err(
                        format!("Invalid teacher id {:?}", id),
                    )),
                    UpdateTeacherError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn teachers_delete(self_: PyRef<'_, Self>, id: TeacherId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Teachers(
                    crate::rpc::cmd_msg::TeachersCmdMsg::DeleteTeacher(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Teachers(TeachersError::DeleteTeacher(e))) => match e {
                DeleteTeacherError::InvalidTeacherId(id) => Err(PyValueError::new_err(format!(
                    "Invalid teacher id {:?}",
                    id
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn teachers_get_list(self_: PyRef<'_, Self>) -> BTreeMap<TeacherId, Teacher> {
        self_
            .token
            .get_data()
            .get_teachers()
            .teacher_map
            .iter()
            .map(|(id, data)| (MsgTeacherId::from(*id).into(), data.clone().into()))
            .collect()
    }

    fn students_add(self_: PyRef<'_, Self>, student: students::Student) -> PyResult<StudentId> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Students(
                    crate::rpc::cmd_msg::StudentsCmdMsg::AddNewStudent(student.into()),
                )));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::StudentId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::Students(StudentsError::AddNewStudent(e))) => match e {
                AddNewStudentError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn students_update(
        self_: PyRef<'_, Self>,
        id: StudentId,
        new_student: students::Student,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Students(
                    crate::rpc::cmd_msg::StudentsCmdMsg::UpdateStudent(
                        id.into(),
                        new_student.into(),
                    ),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Students(StudentsError::UpdateStudent(e))) => match e {
                UpdateStudentError::InvalidStudentId(id) => Err(PyValueError::new_err(format!(
                    "Invalid student id {:?}",
                    id
                ))),
                UpdateStudentError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn students_delete(self_: PyRef<'_, Self>, id: StudentId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Students(
                    crate::rpc::cmd_msg::StudentsCmdMsg::DeleteStudent(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Students(StudentsError::DeleteStudent(e))) => match e {
                DeleteStudentError::InvalidStudentId(id) => Err(PyValueError::new_err(format!(
                    "Invalid student id {:?}",
                    id
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn students_get_list(self_: PyRef<'_, Self>) -> BTreeMap<StudentId, Student> {
        self_
            .token
            .get_data()
            .get_students()
            .student_map
            .iter()
            .map(|(id, data)| (MsgStudentId::from(*id).into(), data.clone().into()))
            .collect()
    }

    fn assignments_set(
        self_: PyRef<'_, Self>,
        period_id: PeriodId,
        student_id: StudentId,
        subject_id: SubjectId,
        status: bool,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::Assignments(crate::rpc::cmd_msg::AssignmentsCmdMsg::Assign(
                period_id.into(),
                student_id.into(),
                subject_id.into(),
                status,
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Assignments(AssignmentsError::Assign(e))) => match e {
                AssignError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                AssignError::InvalidStudentId(id) => Err(PyValueError::new_err(format!(
                    "Invalid student id {:?}",
                    id
                ))),
                AssignError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                AssignError::SubjectDoesNotRunOnPeriod(subject_id, period_id) => {
                    Err(PyValueError::new_err(format!(
                        "Subject {:?} does not run on period {:?}",
                        subject_id, period_id
                    )))
                }
                AssignError::StudentIsNotPresentOnPeriod(student_id, period_id) => {
                    Err(PyValueError::new_err(format!(
                        "Student {:?} is not present on period {:?}",
                        student_id, period_id
                    )))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn assignments_get(
        self_: PyRef<'_, Self>,
        period_id: PeriodId,
        student_id: StudentId,
        subject_id: SubjectId,
    ) -> PyResult<bool> {
        let current_data = self_.token.get_data();

        let Some(period_id) =
            current_data.validate_period_id(MsgPeriodId::from(period_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid period id {:?}",
                period_id
            )));
        };
        let Some(student_id) =
            current_data.validate_student_id(MsgStudentId::from(student_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid student id {:?}",
                student_id
            )));
        };
        let Some(subject_id) =
            current_data.validate_subject_id(MsgSubjectId::from(subject_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid subject id {:?}",
                subject_id
            )));
        };

        let student = current_data
            .get_students()
            .student_map
            .get(&student_id)
            .expect("Student id should be valid at this point");

        if student.excluded_periods.contains(&period_id) {
            return Err(PyValueError::new_err(format!(
                "Student {:?} is not present on period {:?}",
                student_id, period_id
            )));
        }

        let Some(assigned_students) = current_data
            .get_assignments()
            .period_map
            .get(&period_id)
            .expect("Period id should be valid at this point")
            .subject_map
            .get(&subject_id)
        else {
            return Err(PyValueError::new_err(format!(
                "Subject {:?} does not run on period {:?}",
                subject_id, period_id
            )));
        };

        let value = assigned_students.contains(&student_id);

        Ok(value)
    }

    fn assignments_duplicate_previous_period(
        self_: PyRef<'_, Self>,
        period_id: PeriodId,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::Assignments(
                crate::rpc::cmd_msg::AssignmentsCmdMsg::DuplicatePreviousPeriod(period_id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Assignments(AssignmentsError::DuplicatePreviousPeriod(
                e,
            ))) => match e {
                DuplicatePreviousPeriodError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(id) => Err(
                    PyValueError::new_err(format!("Period id {:?} is the first period", id)),
                ),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn assignments_set_all(
        self_: PyRef<'_, Self>,
        period_id: PeriodId,
        subject_id: SubjectId,
        status: bool,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::Assignments(crate::rpc::cmd_msg::AssignmentsCmdMsg::AssignAll(
                period_id.into(),
                subject_id.into(),
                status,
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Assignments(AssignmentsError::AssignAll(e))) => match e {
                AssignAllError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                AssignAllError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                AssignAllError::SubjectDoesNotRunOnPeriod(subject_id, period_id) => {
                    Err(PyValueError::new_err(format!(
                        "Subject {:?} does not run on period {:?}",
                        subject_id, period_id
                    )))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_add(
        self_: PyRef<'_, Self>,
        week_pattern: week_patterns::WeekPattern,
    ) -> WeekPatternId {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::WeekPatterns(
                crate::rpc::cmd_msg::WeekPatternsCmdMsg::AddNewWeekPattern(week_pattern.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::WeekPatternId(id))) => id.into(),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_update(
        self_: PyRef<'_, Self>,
        id: WeekPatternId,
        new_week_pattern: week_patterns::WeekPattern,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::WeekPatterns(
                crate::rpc::cmd_msg::WeekPatternsCmdMsg::UpdateWeekPattern(
                    id.into(),
                    new_week_pattern.into(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::WeekPatterns(WeekPatternsError::UpdateWeekPattern(e))) => {
                match e {
                    UpdateWeekPatternError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                        format!("Invalid week pattern id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_delete(self_: PyRef<'_, Self>, id: WeekPatternId) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::WeekPatterns(
                crate::rpc::cmd_msg::WeekPatternsCmdMsg::DeleteWeekPattern(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::WeekPatterns(WeekPatternsError::DeleteWeekPattern(e))) => {
                match e {
                    DeleteWeekPatternError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                        format!("Invalid week pattern id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_get_list(self_: PyRef<'_, Self>) -> BTreeMap<WeekPatternId, WeekPattern> {
        self_
            .token
            .get_data()
            .get_week_patterns()
            .week_pattern_map
            .iter()
            .map(|(id, data)| (MsgWeekPatternId::from(*id).into(), data.clone().into()))
            .collect()
    }
}

#[derive(Clone, Debug)]
struct Token {}

impl Token {
    fn get_data(&self) -> collomatique_state_colloscopes::Data {
        use crate::rpc::ResultMsg;

        let result =
            crate::rpc::send_rpc(crate::rpc::CmdMsg::GetData).expect("No error for getting data");
        let ResultMsg::Data(serialized_data) = result else {
            panic!("Unexpected response to GetData");
        };
        collomatique_state_colloscopes::Data::from(serialized_data)
    }

    fn send_msg(&self, msg: crate::rpc::CmdMsg) -> crate::rpc::ResultMsg {
        crate::rpc::send_rpc(msg).expect("Valid result message")
    }
}
