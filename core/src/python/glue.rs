use std::collections::BTreeMap;

use pyo3::prelude::*;

use crate::rpc::{
    cmd_msg::{
        ExtensionDesc, MsgGroupListId, MsgIncompatId, MsgRuleId, MsgSlotId, MsgStudentId,
        MsgWeekPatternId, OpenFileDialogMsg,
    },
    error_msg::{
        AddNewGroupListError, AddNewIncompatError, AddNewRuleError, AddNewSlotError,
        AddNewStudentError, AddNewSubjectError, AddNewTeacherError, AssignAllError, AssignError,
        AssignGroupListToSubjectError, AssignmentsError, CutPeriodError, DeleteGroupListError,
        DeleteIncompatError, DeletePeriodError, DeleteRuleError, DeleteSlotError,
        DeleteStudentError, DeleteSubjectError, DeleteTeacherError, DeleteWeekPatternError,
        DuplicatePreviousPeriodError, GeneralPlanningError, GroupListsError,
        IncompatibilitiesError, MergeWithPreviousPeriodError, MoveDownError, MoveSlotDownError,
        MoveSlotUpError, MoveUpError, PrefillGroupListError, RulesError, SlotsError, StudentsError,
        SubjectsError, TeachersError, UpdateGroupListError, UpdateIncompatError,
        UpdatePeriodStatusError, UpdatePeriodStatusForRuleError, UpdatePeriodWeekCountError,
        UpdateRuleError, UpdateSlotError, UpdateStudentError, UpdateSubjectError,
        UpdateTeacherError, UpdateWeekPatternError, UpdateWeekStatusError, WeekPatternsError,
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
    m.add_class::<subjects::SubjectInterrogationParameters>()?;
    m.add_class::<subjects::SubjectPeriodicity>()?;
    m.add_class::<teachers::Teacher>()?;
    m.add_class::<students::Student>()?;
    m.add_class::<time::NaiveMondayDate>()?;
    m.add_class::<time::NaiveDate>()?;
    m.add_class::<time::Time>()?;
    m.add_class::<time::SlotStart>()?;
    m.add_class::<time::Weekday>()?;
    m.add_class::<time::SlotWithDuration>()?;
    m.add_class::<slots::Slot>()?;
    m.add_class::<slots::SlotParameters>()?;
    m.add_class::<week_patterns::WeekPattern>()?;
    m.add_class::<incompatibilities::Incompat>()?;
    m.add_class::<group_lists::GroupListParameters>()?;
    m.add_class::<group_lists::PrefilledGroup>()?;
    m.add_class::<rules::LogicRule>()?;
    m.add_class::<rules::Rule>()?;
    m.add_class::<common::PersonWithContact>()?;
    m.add_class::<common::RangeInclusiveU32>()?;
    m.add_class::<settings::StrictLimits>()?;

    m.add_function(wrap_pyfunction!(log, m)?)?;
    m.add_function(wrap_pyfunction!(current_session, m)?)?;

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
mod group_lists;
mod incompatibilities;
mod rules;
use rules::RuleId;
mod settings;
mod slots;

use crate::rpc::cmd_msg::{MsgPeriodId, MsgSubjectId, MsgTeacherId};

#[pymethods]
impl Session {
    fn dialog_open_file(
        self_: PyRef<'_, Self>,
        title: String,
        list: Vec<(String, String)>,
    ) -> Option<std::path::PathBuf> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::GuiRequest(
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

    fn dialog_info_message(self_: PyRef<'_, Self>, text: String) {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::GuiRequest(
            crate::rpc::cmd_msg::GuiMsg::OkDialog(text),
        ));

        match result {
            ResultMsg::AckGui(GuiAnswer::OkDialogClosed) => {}
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn dialog_confirm_action(self_: PyRef<'_, Self>, text: String) -> bool {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::GuiRequest(
            crate::rpc::cmd_msg::GuiMsg::ConfirmDialog(text),
        ));

        match result {
            ResultMsg::AckGui(GuiAnswer::ConfirmDialog(value)) => value,
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    #[pyo3(signature = (info_text, placeholder_text=String::new()))]
    fn dialog_input(
        self_: PyRef<'_, Self>,
        info_text: String,
        placeholder_text: String,
    ) -> Option<String> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::GuiRequest(
            crate::rpc::cmd_msg::GuiMsg::InputDialog(info_text, placeholder_text),
        ));

        match result {
            ResultMsg::AckGui(GuiAnswer::InputDialog(value)) => value,
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

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
            .get_inner_data()
            .main_params
            .periods
            .first_week
            .as_ref()
            .map(|x| x.clone().into())
    }

    fn periods_get_list(self_: PyRef<'_, Self>) -> Vec<Period> {
        self_
            .token
            .get_data()
            .get_inner_data()
            .main_params
            .periods
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

        let Some(validated_subject_id) = data
            .get_inner_data()
            .main_params
            .validate_subject_id(MsgSubjectId::from(subject_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid subject id {:?}",
                subject_id
            )));
        };

        let Some(subject) = data
            .get_inner_data()
            .main_params
            .subjects
            .find_subject(validated_subject_id)
        else {
            panic!("subject id should be valid at this point");
        };

        let Some(validated_period_id) = data
            .get_inner_data()
            .main_params
            .validate_period_id(MsgPeriodId::from(period_id.clone()).0)
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
            .get_inner_data()
            .main_params
            .subjects
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
            .get_inner_data()
            .main_params
            .teachers
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
            .get_inner_data()
            .main_params
            .students
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

        let Some(period_id) = current_data
            .get_inner_data()
            .main_params
            .validate_period_id(MsgPeriodId::from(period_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid period id {:?}",
                period_id
            )));
        };
        let Some(student_id) = current_data
            .get_inner_data()
            .main_params
            .validate_student_id(MsgStudentId::from(student_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid student id {:?}",
                student_id
            )));
        };
        let Some(subject_id) = current_data
            .get_inner_data()
            .main_params
            .validate_subject_id(MsgSubjectId::from(subject_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid subject id {:?}",
                subject_id
            )));
        };

        let student = current_data
            .get_inner_data()
            .main_params
            .students
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
            .get_inner_data()
            .main_params
            .assignments
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
            .get_inner_data()
            .main_params
            .week_patterns
            .week_pattern_map
            .iter()
            .map(|(id, data)| (MsgWeekPatternId::from(*id).into(), data.clone().into()))
            .collect()
    }

    fn slots_add(
        self_: PyRef<'_, Self>,
        subject_id: subjects::SubjectId,
        slot: slots::SlotParameters,
    ) -> PyResult<slots::SlotId> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Slots(
                    crate::rpc::cmd_msg::SlotsCmdMsg::AddNewSlot(subject_id.into(), slot.into()),
                )));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::SlotId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::Slots(SlotsError::AddNewSlot(e))) => match e {
                AddNewSlotError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                AddNewSlotError::InvalidTeacherId(id) => Err(PyValueError::new_err(format!(
                    "Invalid teacher id {:?}",
                    id
                ))),
                AddNewSlotError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(format!(
                    "Invalid week pattern id {:?}",
                    id
                ))),
                AddNewSlotError::SubjectHasNoInterrogation(id) => Err(PyValueError::new_err(
                    format!("Subject id {:?} does not have interrogations", id),
                )),
                AddNewSlotError::TeacherDoesNotTeachInSubject(tid, sid) => {
                    Err(PyValueError::new_err(format!(
                        "Teacher id {:?} does not match subject id {:?}",
                        tid, sid,
                    )))
                }
                AddNewSlotError::SlotOverlapsWithNextDay => Err(PyValueError::new_err(format!(
                    "Slot overlaps with next day"
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_update(
        self_: PyRef<'_, Self>,
        id: slots::SlotId,
        new_slot: slots::SlotParameters,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Slots(
                    crate::rpc::cmd_msg::SlotsCmdMsg::UpdateSlot(id.into(), new_slot.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Slots(SlotsError::UpdateSlot(e))) => match e {
                UpdateSlotError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
                UpdateSlotError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                UpdateSlotError::InvalidTeacherId(id) => Err(PyValueError::new_err(format!(
                    "Invalid teacher id {:?}",
                    id
                ))),
                UpdateSlotError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(format!(
                    "Invalid week pattern id {:?}",
                    id
                ))),
                UpdateSlotError::SubjectHasNoInterrogation(id) => Err(PyValueError::new_err(
                    format!("Subject id {:?} does not have interrogations", id),
                )),
                UpdateSlotError::TeacherDoesNotTeachInSubject(tid, sid) => {
                    Err(PyValueError::new_err(format!(
                        "Teacher id {:?} does not match subject id {:?}",
                        tid, sid,
                    )))
                }
                UpdateSlotError::SlotOverlapsWithNextDay => Err(PyValueError::new_err(format!(
                    "Slot overlaps with next day"
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_delete(self_: PyRef<'_, Self>, id: slots::SlotId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Slots(
                    crate::rpc::cmd_msg::SlotsCmdMsg::DeleteSlot(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Slots(SlotsError::DeleteSlot(e))) => match e {
                DeleteSlotError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_move_up(self_: PyRef<'_, Self>, id: slots::SlotId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Slots(
                    crate::rpc::cmd_msg::SlotsCmdMsg::MoveSlotUp(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Slots(SlotsError::MoveSlotUp(e))) => match e {
                MoveSlotUpError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
                MoveSlotUpError::NoUpperPosition => {
                    Err(PyValueError::new_err("The slot is already the first"))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_move_down(self_: PyRef<'_, Self>, id: slots::SlotId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Slots(
                    crate::rpc::cmd_msg::SlotsCmdMsg::MoveSlotDown(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Slots(SlotsError::MoveSlotDown(e))) => match e {
                MoveSlotDownError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
                MoveSlotDownError::NoLowerPosition => {
                    Err(PyValueError::new_err("The slot is already the last"))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_get_list(self_: PyRef<'_, Self>) -> BTreeMap<subjects::SubjectId, Vec<slots::Slot>> {
        self_
            .token
            .get_data()
            .get_inner_data()
            .main_params
            .slots
            .subject_map
            .iter()
            .map(|(subject_id, subject_slots)| {
                (
                    MsgSubjectId::from(*subject_id).into(),
                    subject_slots
                        .ordered_slots
                        .iter()
                        .map(|(id, data)| slots::Slot {
                            id: MsgSlotId::from(*id).into(),
                            parameters: data.clone().into(),
                        })
                        .collect(),
                )
            })
            .collect()
    }

    fn incompats_add(
        self_: PyRef<'_, Self>,
        incompat: incompatibilities::Incompat,
    ) -> PyResult<incompatibilities::IncompatId> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::Incompats(
                crate::rpc::cmd_msg::IncompatibilitiesCmdMsg::AddNewIncompat(incompat.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::IncompatId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::Incompats(IncompatibilitiesError::AddNewIncompat(e))) => {
                match e {
                    AddNewIncompatError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                    AddNewIncompatError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                        format!("Invalid week pattern id {:?}", id),
                    )),
                    AddNewIncompatError::SlotOverlapsWithNextDay => Err(PyValueError::new_err(
                        format!("Schedule incompatibility slot overlaps with next day",),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn incompats_update(
        self_: PyRef<'_, Self>,
        id: incompatibilities::IncompatId,
        new_incompat: incompatibilities::Incompat,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::Incompats(
                crate::rpc::cmd_msg::IncompatibilitiesCmdMsg::UpdateIncompat(
                    id.into(),
                    new_incompat.into(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Incompats(IncompatibilitiesError::UpdateIncompat(e))) => {
                match e {
                    UpdateIncompatError::InvalidIncompatId(id) => Err(PyValueError::new_err(
                        format!("Invalid incompat id {:?}", id),
                    )),
                    UpdateIncompatError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                    UpdateIncompatError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                        format!("Invalid week pattern id {:?}", id),
                    )),
                    UpdateIncompatError::SlotOverlapsWithNextDay => Err(PyValueError::new_err(
                        format!("Schedule incompatibility slot overlaps with next day",),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn incompats_delete(self_: PyRef<'_, Self>, id: incompatibilities::IncompatId) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::Incompats(
                crate::rpc::cmd_msg::IncompatibilitiesCmdMsg::DeleteIncompat(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Incompats(IncompatibilitiesError::DeleteIncompat(e))) => {
                match e {
                    DeleteIncompatError::InvalidIncompatId(id) => Err(PyValueError::new_err(
                        format!("Invalid incompat id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn incompats_get_list(
        self_: PyRef<'_, Self>,
    ) -> BTreeMap<incompatibilities::IncompatId, incompatibilities::Incompat> {
        self_
            .token
            .get_data()
            .get_inner_data()
            .main_params
            .incompats
            .incompat_map
            .iter()
            .map(|(incompat_id, incompat)| {
                (
                    MsgIncompatId::from(*incompat_id).into(),
                    incompat.clone().into(),
                )
            })
            .collect()
    }

    fn group_lists_add(
        self_: PyRef<'_, Self>,
        params: group_lists::GroupListParameters,
    ) -> PyResult<group_lists::GroupListId> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GroupLists(
                crate::rpc::cmd_msg::GroupListsCmdMsg::AddNewGroupList(params.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::GroupListId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::GroupLists(GroupListsError::AddNewGroupList(e))) => {
                match e {
                    AddNewGroupListError::InvalidStudentId(id) => Err(PyValueError::new_err(
                        format!("Invalid student id {:?}", id),
                    )),
                    AddNewGroupListError::StudentsPerGroupRangeIsEmpty => {
                        Err(PyValueError::new_err("Empty students per group range"))
                    }
                    AddNewGroupListError::GroupCountRangeIsEmpty => {
                        Err(PyValueError::new_err("Empty group count range"))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_update(
        self_: PyRef<'_, Self>,
        id: group_lists::GroupListId,
        new_params: group_lists::GroupListParameters,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GroupLists(
                crate::rpc::cmd_msg::GroupListsCmdMsg::UpdateGroupList(
                    id.into(),
                    new_params.into(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GroupLists(GroupListsError::UpdateGroupList(e))) => {
                match e {
                    UpdateGroupListError::InvalidGroupListId(id) => Err(PyValueError::new_err(
                        format!("Invalid group list id {:?}", id),
                    )),
                    UpdateGroupListError::InvalidStudentId(id) => Err(PyValueError::new_err(
                        format!("Invalid student id {:?}", id),
                    )),
                    UpdateGroupListError::StudentsPerGroupRangeIsEmpty => {
                        Err(PyValueError::new_err("Empty students per group range"))
                    }
                    UpdateGroupListError::GroupCountRangeIsEmpty => {
                        Err(PyValueError::new_err("Empty group count range"))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_delete(self_: PyRef<'_, Self>, id: group_lists::GroupListId) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GroupLists(
                crate::rpc::cmd_msg::GroupListsCmdMsg::DeleteGroupList(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GroupLists(GroupListsError::DeleteGroupList(e))) => {
                match e {
                    DeleteGroupListError::InvalidGroupListId(id) => Err(PyValueError::new_err(
                        format!("Invalid group list id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_prefill(
        self_: PyRef<'_, Self>,
        id: group_lists::GroupListId,
        prefilled_groups: Vec<group_lists::PrefilledGroup>,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GroupLists(
                crate::rpc::cmd_msg::GroupListsCmdMsg::PrefillGroupList(
                    id.into(),
                    crate::rpc::cmd_msg::GroupListPrefilledGroupsMsg {
                        groups: prefilled_groups.into_iter().map(|x| x.into()).collect(),
                    },
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GroupLists(GroupListsError::PrefillGroupList(e))) => {
                match e {
                    PrefillGroupListError::InvalidGroupListId(id) => Err(PyValueError::new_err(
                        format!("Invalid group list id {:?}", id),
                    )),
                    PrefillGroupListError::InvalidStudentId(id) => Err(PyValueError::new_err(
                        format!("Invalid student id {:?}", id),
                    )),
                    PrefillGroupListError::StudentIsExcluded(group_list_id, student_id) => {
                        Err(PyValueError::new_err(format!(
                            "Student id {:?} is excluded from group list {:?}",
                            student_id, group_list_id
                        )))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_get_list(
        self_: PyRef<'_, Self>,
    ) -> BTreeMap<group_lists::GroupListId, group_lists::GroupList> {
        self_
            .token
            .get_data()
            .get_inner_data()
            .main_params
            .group_lists
            .group_list_map
            .iter()
            .map(|(group_list_id, group_list)| {
                (
                    MsgGroupListId::from(*group_list_id).into(),
                    group_list.clone().into(),
                )
            })
            .collect()
    }

    fn group_lists_set_association(
        self_: PyRef<'_, Self>,
        period_id: general_planning::PeriodId,
        subject_id: subjects::SubjectId,
        group_list_id: Option<group_lists::GroupListId>,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GroupLists(
                crate::rpc::cmd_msg::GroupListsCmdMsg::AssignGroupListToSubject(
                    period_id.into(),
                    subject_id.into(),
                    group_list_id.map(|x| x.into()),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::GroupLists(GroupListsError::AssignGroupListToSubject(
                e,
            ))) => match e {
                AssignGroupListToSubjectError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                    format!("Invalid subject id {:?}", id),
                )),
                AssignGroupListToSubjectError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                AssignGroupListToSubjectError::InvalidGroupListId(id) => Err(
                    PyValueError::new_err(format!("Invalid group list id {:?}", id)),
                ),
                AssignGroupListToSubjectError::SubjectDoesNotRunOnPeriod(subject_id, period_id) => {
                    Err(PyValueError::new_err(format!(
                        "Subject {:?} does not run on period {:?}",
                        subject_id, period_id
                    )))
                }
                AssignGroupListToSubjectError::SubjectHasNoInterrogation(subject_id) => {
                    Err(PyValueError::new_err(format!(
                        "Subject id {:?} does not have interrogations",
                        subject_id
                    )))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_get_association(
        self_: PyRef<'_, Self>,
        period_id: general_planning::PeriodId,
        subject_id: subjects::SubjectId,
    ) -> PyResult<Option<group_lists::GroupListId>> {
        let current_data = self_.token.get_data();

        let Some(period_id) = current_data
            .get_inner_data()
            .main_params
            .validate_period_id(MsgPeriodId::from(period_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid period id {:?}",
                period_id
            )));
        };

        let Some(subject_id) = current_data
            .get_inner_data()
            .main_params
            .validate_subject_id(MsgSubjectId::from(subject_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid subject id {:?}",
                subject_id
            )));
        };

        let subject_map = current_data
            .get_inner_data()
            .main_params
            .group_lists
            .subjects_associations
            .get(&period_id)
            .expect("Period id should be valid at this point");

        let group_list_id = subject_map.get(&subject_id);

        Ok(group_list_id.map(|x| MsgGroupListId::from(*x).into()))
    }

    fn rules_add(self_: PyRef<'_, Self>, rule: rules::Rule) -> PyResult<rules::RuleId> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Rules(
                    crate::rpc::cmd_msg::RulesCmdMsg::AddNewRule(rule.name, rule.logic_rule.into()),
                )));

        match result {
            ResultMsg::Ack(Some(crate::rpc::NewId::RuleId(id))) => Ok(id.into()),
            ResultMsg::Error(ErrorMsg::Rules(RulesError::AddNewRule(e))) => match e {
                AddNewRuleError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_update(self_: PyRef<'_, Self>, id: rules::RuleId, rule: rules::Rule) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Rules(
                    crate::rpc::cmd_msg::RulesCmdMsg::UpdateRule(
                        id.into(),
                        rule.name,
                        rule.logic_rule.into(),
                    ),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Rules(RulesError::UpdateRule(e))) => match e {
                UpdateRuleError::InvalidRuleId(id) => {
                    Err(PyValueError::new_err(format!("Invalid rule id {:?}", id)))
                }
                UpdateRuleError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_delete(self_: PyRef<'_, Self>, id: rules::RuleId) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Rules(
                    crate::rpc::cmd_msg::RulesCmdMsg::DeleteRule(id.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Rules(RulesError::DeleteRule(e))) => match e {
                DeleteRuleError::InvalidRuleId(id) => {
                    Err(PyValueError::new_err(format!("Invalid rule id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_get_list(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<rules::RuleId, rules::Rule>> {
        let mut new_map = BTreeMap::new();

        for (rule_id, rule) in &self_
            .token
            .get_data()
            .get_inner_data()
            .main_params
            .rules
            .rule_map
        {
            new_map.insert(
                MsgRuleId::from(*rule_id).into(),
                rules::Rule {
                    name: rule.name.clone(),
                    logic_rule: rule.desc.clone().try_into()?,
                },
            );
        }

        Ok(new_map)
    }

    fn rules_update_period_status(
        self_: PyRef<'_, Self>,
        rule_id: RuleId,
        period_id: PeriodId,
        new_status: bool,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Rules(
                    crate::rpc::cmd_msg::RulesCmdMsg::UpdatePeriodStatusForRule(
                        rule_id.into(),
                        period_id.into(),
                        new_status,
                    ),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(ErrorMsg::Rules(RulesError::UpdatePeriodStatusForRule(e))) => {
                match e {
                    UpdatePeriodStatusForRuleError::InvalidRuleId(id) => {
                        Err(PyValueError::new_err(format!("Invalid rule id {:?}", id)))
                    }
                    UpdatePeriodStatusForRuleError::InvalidPeriodId(id) => {
                        Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_get_period_status(
        self_: PyRef<'_, Self>,
        rule_id: RuleId,
        period_id: PeriodId,
    ) -> PyResult<bool> {
        let data = self_.token.get_data();

        let Some(validated_rule_id) = data
            .get_inner_data()
            .main_params
            .validate_rule_id(MsgRuleId::from(rule_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid rule id {:?}",
                rule_id
            )));
        };

        let Some(rule) = data
            .get_inner_data()
            .main_params
            .rules
            .rule_map
            .get(&validated_rule_id)
        else {
            panic!("rule id should be valid at this point");
        };

        let Some(validated_period_id) = data
            .get_inner_data()
            .main_params
            .validate_period_id(MsgPeriodId::from(period_id.clone()).0)
        else {
            return Err(PyValueError::new_err(format!(
                "Invalid period id {:?}",
                period_id
            )));
        };

        Ok(!rule.excluded_periods.contains(&validated_period_id))
    }

    fn settings_update_strict_limits(
        self_: PyRef<'_, Self>,
        strict_limits: settings::StrictLimits,
    ) -> PyResult<()> {
        let result =
            self_
                .token
                .send_msg(crate::rpc::CmdMsg::Update(crate::rpc::UpdateMsg::Settings(
                    crate::rpc::cmd_msg::SettingsCmdMsg::UpdateStrictLimits(strict_limits.into()),
                )));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn settings_get(self_: PyRef<'_, Self>) -> settings::GeneralSettings {
        self_
            .token
            .get_data()
            .get_inner_data()
            .main_params
            .settings
            .clone()
            .into()
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
