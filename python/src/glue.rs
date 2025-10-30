use std::collections::BTreeMap;

use colloscopes::ColloscopeId;
use pyo3::prelude::*;

use collomatique_rpc::{
    cmd_msg::{ExtensionDesc, OpenFileDialogMsg},
    GuiAnswer, ResultMsg,
};

use collomatique_ops::{
    AddNewGroupListError, AddNewIncompatError, AddNewRuleError, AddNewSlotError,
    AddNewStudentError, AddNewSubjectError, AddNewTeacherError, AssignAllError, AssignError,
    AssignGroupListToSubjectError, AssignmentsUpdateError, CutPeriodError, DeleteGroupListError,
    DeleteIncompatError, DeletePeriodError, DeleteRuleError, DeleteSlotError, DeleteStudentError,
    DeleteSubjectError, DeleteTeacherError, DeleteWeekPatternError, DuplicatePreviousPeriodError,
    GeneralPlanningUpdateError, GroupListsUpdateError, IncompatibilitiesUpdateError,
    MergeWithPreviousPeriodError, MoveSlotDownError, MoveSlotUpError, MoveSubjectDownError,
    MoveSubjectUpError, PrefillGroupListError, RemoveStudentLimitsError, RulesUpdateError,
    SettingsUpdateError, SlotsUpdateError, StudentsUpdateError, SubjectsUpdateError,
    TeachersUpdateError, UpdateGroupListError, UpdateIncompatError, UpdatePeriodStatusError,
    UpdatePeriodStatusForRuleError, UpdatePeriodWeekCountError, UpdateRuleError, UpdateSlotError,
    UpdateStudentError, UpdateStudentLimitsError, UpdateSubjectError, UpdateTeacherError,
    UpdateWeekAnnotationError, UpdateWeekPatternError, UpdateWeekStatusError,
    WeekPatternsUpdateError,
};
use collomatique_ops::{DuplicatePreviousPeriodAssociationsError, UpdateError};

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
    m.add_class::<settings::Limits>()?;
    m.add_class::<settings::SoftU32>()?;
    m.add_class::<settings::SoftNonZeroU32>()?;

    m.add_class::<general_planning::ColloscopePeriod>()?;
    m.add_class::<subjects::ColloscopeSubject>()?;
    m.add_class::<teachers::ColloscopeTeacher>()?;
    m.add_class::<students::ColloscopeStudent>()?;
    m.add_class::<slots::ColloscopeSlot>()?;
    m.add_class::<slots::ColloscopeSlotParameters>()?;
    m.add_class::<incompatibilities::ColloscopeIncompat>()?;
    m.add_class::<group_lists::ColloscopeGroupListParameters>()?;
    m.add_class::<group_lists::ColloscopePrefilledGroup>()?;
    m.add_class::<rules::ColloscopeLogicRule>()?;
    m.add_class::<rules::ColloscopeRule>()?;

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
    Session {}
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct Session {}

impl Session {
    fn send_msg(&self, msg: collomatique_rpc::CmdMsg) -> collomatique_rpc::ResultMsg {
        collomatique_rpc::send_rpc(msg).expect("Valid result message")
    }
}

#[pymethods]
impl Session {
    fn dialog_open_file(
        self_: PyRef<'_, Self>,
        title: String,
        list: Vec<(String, String)>,
    ) -> Option<std::path::PathBuf> {
        let result = self_.send_msg(collomatique_rpc::CmdMsg::GuiRequest(
            collomatique_rpc::cmd_msg::GuiMsg::OpenFileDialog(OpenFileDialogMsg {
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
        let result = self_.send_msg(collomatique_rpc::CmdMsg::GuiRequest(
            collomatique_rpc::cmd_msg::GuiMsg::OkDialog(text),
        ));

        match result {
            ResultMsg::AckGui(GuiAnswer::OkDialogClosed) => {}
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn dialog_confirm_action(self_: PyRef<'_, Self>, text: String) -> bool {
        let result = self_.send_msg(collomatique_rpc::CmdMsg::GuiRequest(
            collomatique_rpc::cmd_msg::GuiMsg::ConfirmDialog(text),
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
        let result = self_.send_msg(collomatique_rpc::CmdMsg::GuiRequest(
            collomatique_rpc::cmd_msg::GuiMsg::InputDialog(info_text, placeholder_text),
        ));

        match result {
            ResultMsg::AckGui(GuiAnswer::InputDialog(value)) => value,
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn get_current_collomatique_file(self_: PyRef<'_, Self>) -> CollomatiqueFile {
        CollomatiqueFile {
            token: Token {
                file: InternalFile::Session(self_.clone()),
            },
        }
    }
}

mod common;
use common::PersonWithContact;
mod general_planning;
use general_planning::{ColloscopePeriod, ColloscopePeriodId, Period, PeriodId};
mod subjects;
use subjects::{ColloscopeSubject, ColloscopeSubjectId, Subject, SubjectId};
mod teachers;
mod time;
use teachers::{ColloscopeTeacher, ColloscopeTeacherId, Teacher, TeacherId};
mod students;
use students::{ColloscopeStudent, ColloscopeStudentId, Student, StudentId};
mod week_patterns;
use week_patterns::{ColloscopeWeekPatternId, WeekPattern, WeekPatternId};
mod group_lists;
mod incompatibilities;
mod rules;
use rules::{ColloscopeRuleId, RuleId};
mod colloscopes;
mod params;
mod settings;
mod slots;

#[pyclass]
pub struct CollomatiqueFile {
    token: Token,
}

#[pymethods]
impl CollomatiqueFile {
    fn periods_add(self_: PyRef<'_, Self>, week_count: usize) -> PeriodId {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::AddNewPeriod(week_count),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::PeriodId(id))) => id.into(),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_update(self_: PyRef<'_, Self>, id: PeriodId, new_week_count: usize) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::UpdatePeriodWeekCount(
                    id.into(),
                    new_week_count,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GeneralPlanning(
                GeneralPlanningUpdateError::UpdatePeriodWeekCount(e),
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::DeletePeriod(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateError::DeletePeriod(e),
            )) => match e {
                DeletePeriodError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_cut(
        self_: PyRef<'_, Self>,
        id: PeriodId,
        remaining_weeks: usize,
    ) -> PyResult<PeriodId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::CutPeriod(id.into(), remaining_weeks),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::PeriodId(new_id))) => {
                Ok(new_id.into())
            }
            ResultMsg::Error(UpdateError::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateError::CutPeriod(e),
            )) => match e {
                CutPeriodError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                CutPeriodError::RemainingWeekCountTooBig(w, t) => Err(PyValueError::new_err(
                    format!("Remaining weeks too big ({} > {})", w, t),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_merge_with_previous(self_: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::MergeWithPreviousPeriod(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateError::MergeWithPreviousPeriod(e),
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::UpdateWeekStatus(
                    id.into(),
                    week,
                    new_status,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateError::UpdateWeekStatus(e),
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

    fn periods_update_week_annotation(
        self_: PyRef<'_, Self>,
        id: PeriodId,
        week: usize,
        annotation: String,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateOp::UpdateWeekAnnotation(
                    id.into(),
                    week,
                    non_empty_string::NonEmptyString::new(annotation).ok(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GeneralPlanning(
                collomatique_ops::GeneralPlanningUpdateError::UpdateWeekAnnotation(e),
            )) => match e {
                UpdateWeekAnnotationError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                UpdateWeekAnnotationError::InvalidWeekNumber(w, t) => Err(PyValueError::new_err(
                    format!("Week number too big ({} >= {}", w, t),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn periods_set_first_week(self_: PyRef<'_, Self>, first_week: Option<time::NaiveMondayDate>) {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GeneralPlanning(match first_week {
                Some(week) => collomatique_ops::GeneralPlanningUpdateOp::UpdateFirstWeek(
                    collomatique_time::NaiveMondayDate::from(week),
                ),
                None => collomatique_ops::GeneralPlanningUpdateOp::DeleteFirstWeek,
            }),
        ));

        if result != ResultMsg::Ack(None) {
            panic!("Unexpected result: {:?}", result)
        }
    }

    fn subjects_add(
        self_: PyRef<'_, Self>,
        subject_params: subjects::SubjectParameters,
    ) -> PyResult<SubjectId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::AddNewSubject(subject_params.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::SubjectId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::Subjects(SubjectsUpdateError::AddNewSubject(e))) => {
                match e {
                    AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty => Err(
                        PyValueError::new_err("groups per interrogation range cannot be empty"),
                    ),
                    AddNewSubjectError::StudentsPerGroupRangeIsEmpty => Err(PyValueError::new_err(
                        "students per group range cannot be empty",
                    )),
                    AddNewSubjectError::InterrogationCountRangeIsEmpty => Err(
                        PyValueError::new_err("interrogation count range cannot be empty"),
                    ),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_update(
        self_: PyRef<'_, Self>,
        id: SubjectId,
        new_subject_params: subjects::SubjectParameters,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::UpdateSubject(
                    id.into(),
                    new_subject_params.into(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Subjects(SubjectsUpdateError::UpdateSubject(e))) => {
                match e {
                    UpdateSubjectError::GroupsPerInterrogationRangeIsEmpty => Err(
                        PyValueError::new_err("groups per interrogation range cannot be empty"),
                    ),
                    UpdateSubjectError::StudentsPerGroupRangeIsEmpty => Err(PyValueError::new_err(
                        "students per group range cannot be empty",
                    )),
                    UpdateSubjectError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                    UpdateSubjectError::InterrogationCountRangeIsEmpty => Err(
                        PyValueError::new_err("interrogation count range cannot be empty"),
                    ),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_delete(self_: PyRef<'_, Self>, id: SubjectId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::DeleteSubject(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Subjects(SubjectsUpdateError::DeleteSubject(e))) => {
                match e {
                    DeleteSubjectError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_move_up(self_: PyRef<'_, Self>, id: SubjectId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::MoveSubjectUp(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Subjects(SubjectsUpdateError::MoveSubjectUp(e))) => {
                match e {
                    MoveSubjectUpError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                    MoveSubjectUpError::NoUpperPosition => {
                        Err(PyValueError::new_err("The subject is already the first"))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_move_down(self_: PyRef<'_, Self>, id: SubjectId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::MoveSubjectDown(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Subjects(SubjectsUpdateError::MoveSubjectDown(e))) => {
                match e {
                    MoveSubjectDownError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                    MoveSubjectDownError::NoLowerPosition => {
                        Err(PyValueError::new_err("The subject is already the last"))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn subjects_update_period_status(
        self_: PyRef<'_, Self>,
        subject_id: SubjectId,
        period_id: PeriodId,
        new_status: bool,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Subjects(
                collomatique_ops::SubjectsUpdateOp::UpdatePeriodStatus(
                    subject_id.into(),
                    period_id.into(),
                    new_status,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Subjects(SubjectsUpdateError::UpdatePeriodStatus(e))) => {
                match e {
                    UpdatePeriodStatusError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                    UpdatePeriodStatusError::InvalidPeriodId(id) => {
                        Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn teachers_add(self_: PyRef<'_, Self>, teacher: teachers::Teacher) -> PyResult<TeacherId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Teachers(
                collomatique_ops::TeachersUpdateOp::AddNewTeacher(teacher.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::TeacherId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::Teachers(TeachersUpdateError::AddNewTeacher(e))) => {
                match e {
                    AddNewTeacherError::InvalidSubjectId(id) => Err(PyValueError::new_err(
                        format!("Invalid subject id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn teachers_update(
        self_: PyRef<'_, Self>,
        id: TeacherId,
        new_teacher: teachers::Teacher,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Teachers(
                collomatique_ops::TeachersUpdateOp::UpdateTeacher(id.into(), new_teacher.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Teachers(TeachersUpdateError::UpdateTeacher(e))) => {
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Teachers(
                collomatique_ops::TeachersUpdateOp::DeleteTeacher(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Teachers(TeachersUpdateError::DeleteTeacher(e))) => {
                match e {
                    DeleteTeacherError::InvalidTeacherId(id) => Err(PyValueError::new_err(
                        format!("Invalid teacher id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn students_add(self_: PyRef<'_, Self>, student: students::Student) -> PyResult<StudentId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Students(
                collomatique_ops::StudentsUpdateOp::AddNewStudent(student.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::StudentId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::Students(StudentsUpdateError::AddNewStudent(e))) => {
                match e {
                    AddNewStudentError::InvalidPeriodId(id) => {
                        Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn students_update(
        self_: PyRef<'_, Self>,
        id: StudentId,
        new_student: students::Student,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Students(
                collomatique_ops::StudentsUpdateOp::UpdateStudent(id.into(), new_student.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Students(StudentsUpdateError::UpdateStudent(e))) => {
                match e {
                    UpdateStudentError::InvalidStudentId(id) => Err(PyValueError::new_err(
                        format!("Invalid student id {:?}", id),
                    )),
                    UpdateStudentError::InvalidPeriodId(id) => {
                        Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                    }
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn students_delete(self_: PyRef<'_, Self>, id: StudentId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Students(
                collomatique_ops::StudentsUpdateOp::DeleteStudent(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Students(StudentsUpdateError::DeleteStudent(e))) => {
                match e {
                    DeleteStudentError::InvalidStudentId(id) => Err(PyValueError::new_err(
                        format!("Invalid student id {:?}", id),
                    )),
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn assignments_set(
        self_: PyRef<'_, Self>,
        period_id: PeriodId,
        student_id: StudentId,
        subject_id: SubjectId,
        status: bool,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Assignments(collomatique_ops::AssignmentsUpdateOp::Assign(
                period_id.into(),
                student_id.into(),
                subject_id.into(),
                status,
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Assignments(AssignmentsUpdateError::Assign(e))) => {
                match e {
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
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn assignments_duplicate_previous_period(
        self_: PyRef<'_, Self>,
        period_id: PeriodId,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Assignments(
                collomatique_ops::AssignmentsUpdateOp::DuplicatePreviousPeriod(period_id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Assignments(
                AssignmentsUpdateError::DuplicatePreviousPeriod(e),
            )) => match e {
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Assignments(
                collomatique_ops::AssignmentsUpdateOp::AssignAll(
                    period_id.into(),
                    subject_id.into(),
                    status,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Assignments(AssignmentsUpdateError::AssignAll(e))) => {
                match e {
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
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_add(
        self_: PyRef<'_, Self>,
        week_pattern: week_patterns::WeekPattern,
    ) -> WeekPatternId {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::WeekPatterns(
                collomatique_ops::WeekPatternsUpdateOp::AddNewWeekPattern(week_pattern.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::WeekPatternId(id))) => {
                id.into()
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_update(
        self_: PyRef<'_, Self>,
        id: WeekPatternId,
        new_week_pattern: week_patterns::WeekPattern,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::WeekPatterns(
                collomatique_ops::WeekPatternsUpdateOp::UpdateWeekPattern(
                    id.into(),
                    new_week_pattern.into(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::WeekPatterns(
                WeekPatternsUpdateError::UpdateWeekPattern(e),
            )) => match e {
                UpdateWeekPatternError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                    format!("Invalid week pattern id {:?}", id),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn week_patterns_delete(self_: PyRef<'_, Self>, id: WeekPatternId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::WeekPatterns(
                collomatique_ops::WeekPatternsUpdateOp::DeleteWeekPattern(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::WeekPatterns(
                WeekPatternsUpdateError::DeleteWeekPattern(e),
            )) => match e {
                DeleteWeekPatternError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                    format!("Invalid week pattern id {:?}", id),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_add(
        self_: PyRef<'_, Self>,
        subject_id: subjects::SubjectId,
        slot: slots::SlotParameters,
    ) -> PyResult<slots::SlotId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Slots(collomatique_ops::SlotsUpdateOp::AddNewSlot(
                subject_id.into(),
                slot.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::SlotId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::Slots(SlotsUpdateError::AddNewSlot(e))) => match e {
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Slots(collomatique_ops::SlotsUpdateOp::UpdateSlot(
                id.into(),
                new_slot.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Slots(SlotsUpdateError::UpdateSlot(e))) => match e {
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Slots(collomatique_ops::SlotsUpdateOp::DeleteSlot(
                id.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Slots(SlotsUpdateError::DeleteSlot(e))) => match e {
                DeleteSlotError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn slots_move_up(self_: PyRef<'_, Self>, id: slots::SlotId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Slots(collomatique_ops::SlotsUpdateOp::MoveSlotUp(
                id.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Slots(SlotsUpdateError::MoveSlotUp(e))) => match e {
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Slots(collomatique_ops::SlotsUpdateOp::MoveSlotDown(
                id.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Slots(SlotsUpdateError::MoveSlotDown(e))) => match e {
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

    fn incompats_add(
        self_: PyRef<'_, Self>,
        incompat: incompatibilities::Incompat,
    ) -> PyResult<incompatibilities::IncompatId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Incompatibilities(
                collomatique_ops::IncompatibilitiesUpdateOp::AddNewIncompat(
                    incompat.try_into().map_err(|e| match e {
                        time::SlotWithDurationError::SlotOverlapsWithNextDay => {
                            PyValueError::new_err("Slot overlaps with next day")
                        }
                    })?,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::IncompatId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::Incompatibilities(
                IncompatibilitiesUpdateError::AddNewIncompat(e),
            )) => match e {
                AddNewIncompatError::InvalidSubjectId(id) => Err(PyValueError::new_err(format!(
                    "Invalid subject id {:?}",
                    id
                ))),
                AddNewIncompatError::InvalidWeekPatternId(id) => Err(PyValueError::new_err(
                    format!("Invalid week pattern id {:?}", id),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn incompats_update(
        self_: PyRef<'_, Self>,
        id: incompatibilities::IncompatId,
        new_incompat: incompatibilities::Incompat,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Incompatibilities(
                collomatique_ops::IncompatibilitiesUpdateOp::UpdateIncompat(
                    id.into(),
                    new_incompat.try_into().map_err(|e| match e {
                        time::SlotWithDurationError::SlotOverlapsWithNextDay => {
                            PyValueError::new_err("Slot overlaps with next day")
                        }
                    })?,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Incompatibilities(
                IncompatibilitiesUpdateError::UpdateIncompat(e),
            )) => {
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
                }
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn incompats_delete(self_: PyRef<'_, Self>, id: incompatibilities::IncompatId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Incompatibilities(
                collomatique_ops::IncompatibilitiesUpdateOp::DeleteIncompat(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Incompatibilities(
                IncompatibilitiesUpdateError::DeleteIncompat(e),
            )) => match e {
                DeleteIncompatError::InvalidIncompatId(id) => Err(PyValueError::new_err(format!(
                    "Invalid incompat id {:?}",
                    id
                ))),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_add(
        self_: PyRef<'_, Self>,
        params: group_lists::GroupListParameters,
    ) -> PyResult<group_lists::GroupListId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GroupLists(
                collomatique_ops::GroupListsUpdateOp::AddNewGroupList(params.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::GroupListId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::GroupLists(GroupListsUpdateError::AddNewGroupList(
                e,
            ))) => match e {
                AddNewGroupListError::InvalidStudentId(id) => Err(PyValueError::new_err(format!(
                    "Invalid student id {:?}",
                    id
                ))),
                AddNewGroupListError::StudentsPerGroupRangeIsEmpty => {
                    Err(PyValueError::new_err("Empty students per group range"))
                }
                AddNewGroupListError::GroupCountRangeIsEmpty => {
                    Err(PyValueError::new_err("Empty group count range"))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_update(
        self_: PyRef<'_, Self>,
        id: group_lists::GroupListId,
        new_params: group_lists::GroupListParameters,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GroupLists(
                collomatique_ops::GroupListsUpdateOp::UpdateGroupList(id.into(), new_params.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GroupLists(GroupListsUpdateError::UpdateGroupList(
                e,
            ))) => match e {
                UpdateGroupListError::InvalidGroupListId(id) => Err(PyValueError::new_err(
                    format!("Invalid group list id {:?}", id),
                )),
                UpdateGroupListError::InvalidStudentId(id) => Err(PyValueError::new_err(format!(
                    "Invalid student id {:?}",
                    id
                ))),
                UpdateGroupListError::StudentsPerGroupRangeIsEmpty => {
                    Err(PyValueError::new_err("Empty students per group range"))
                }
                UpdateGroupListError::GroupCountRangeIsEmpty => {
                    Err(PyValueError::new_err("Empty group count range"))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_delete(self_: PyRef<'_, Self>, id: group_lists::GroupListId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GroupLists(
                collomatique_ops::GroupListsUpdateOp::DeleteGroupList(id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GroupLists(GroupListsUpdateError::DeleteGroupList(
                e,
            ))) => match e {
                DeleteGroupListError::InvalidGroupListId(id) => Err(PyValueError::new_err(
                    format!("Invalid group list id {:?}", id),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_prefill(
        self_: PyRef<'_, Self>,
        id: group_lists::GroupListId,
        prefilled_groups: Vec<group_lists::PrefilledGroup>,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GroupLists(
                collomatique_ops::GroupListsUpdateOp::PrefillGroupList(
                    id.into(),
                    collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups {
                        groups: prefilled_groups.into_iter().map(|x| x.into()).collect(),
                    },
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GroupLists(GroupListsUpdateError::PrefillGroupList(
                e,
            ))) => match e {
                PrefillGroupListError::InvalidGroupListId(id) => Err(PyValueError::new_err(
                    format!("Invalid group list id {:?}", id),
                )),
                PrefillGroupListError::InvalidStudentId(id) => Err(PyValueError::new_err(format!(
                    "Invalid student id {:?}",
                    id
                ))),
                PrefillGroupListError::StudentIsExcluded(group_list_id, student_id) => {
                    Err(PyValueError::new_err(format!(
                        "Student id {:?} is excluded from group list {:?}",
                        student_id, group_list_id
                    )))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn group_lists_set_association(
        self_: PyRef<'_, Self>,
        period_id: general_planning::PeriodId,
        subject_id: subjects::SubjectId,
        group_list_id: Option<group_lists::GroupListId>,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GroupLists(
                collomatique_ops::GroupListsUpdateOp::AssignGroupListToSubject(
                    period_id.into(),
                    subject_id.into(),
                    group_list_id.map(|x| x.into()),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GroupLists(
                GroupListsUpdateError::AssignGroupListToSubject(e),
            )) => match e {
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

    fn group_lists_duplicate_previous_period_associations(
        self_: PyRef<'_, Self>,
        period_id: general_planning::PeriodId,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::GroupLists(
                collomatique_ops::GroupListsUpdateOp::DuplicatePreviousPeriod(period_id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::GroupLists(
                GroupListsUpdateError::DuplicatePreviousPeriod(e),
            )) => match e {
                DuplicatePreviousPeriodAssociationsError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
                DuplicatePreviousPeriodAssociationsError::FirstPeriodHasNoPreviousPeriod(id) => {
                    Err(PyValueError::new_err(format!(
                        "Period id {:?} is the first period",
                        id
                    )))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_add(self_: PyRef<'_, Self>, rule: rules::Rule) -> PyResult<rules::RuleId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Rules(collomatique_ops::RulesUpdateOp::AddNewRule(
                rule.name,
                rule.logic_rule.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::RuleId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(UpdateError::Rules(RulesUpdateError::AddNewRule(e))) => match e {
                AddNewRuleError::InvalidSlotId(id) => {
                    Err(PyValueError::new_err(format!("Invalid slot id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_update(self_: PyRef<'_, Self>, id: rules::RuleId, rule: rules::Rule) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Rules(collomatique_ops::RulesUpdateOp::UpdateRule(
                id.into(),
                rule.name,
                rule.logic_rule.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Rules(RulesUpdateError::UpdateRule(e))) => match e {
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
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Rules(collomatique_ops::RulesUpdateOp::DeleteRule(
                id.into(),
            )),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Rules(RulesUpdateError::DeleteRule(e))) => match e {
                DeleteRuleError::InvalidRuleId(id) => {
                    Err(PyValueError::new_err(format!("Invalid rule id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn rules_update_period_status(
        self_: PyRef<'_, Self>,
        rule_id: RuleId,
        period_id: PeriodId,
        new_status: bool,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Rules(
                collomatique_ops::RulesUpdateOp::UpdatePeriodStatusForRule(
                    rule_id.into(),
                    period_id.into(),
                    new_status,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Rules(RulesUpdateError::UpdatePeriodStatusForRule(
                e,
            ))) => match e {
                UpdatePeriodStatusForRuleError::InvalidRuleId(id) => {
                    Err(PyValueError::new_err(format!("Invalid rule id {:?}", id)))
                }
                UpdatePeriodStatusForRuleError::InvalidPeriodId(id) => {
                    Err(PyValueError::new_err(format!("Invalid period id {:?}", id)))
                }
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn settings_update_global_limits(
        self_: PyRef<'_, Self>,
        limits: settings::Limits,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Settings(
                collomatique_ops::SettingsUpdateOp::UpdateGlobalLimits(limits.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn settings_update_student_limits(
        self_: PyRef<'_, Self>,
        student_id: StudentId,
        limits: settings::Limits,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Settings(
                collomatique_ops::SettingsUpdateOp::UpdateStudentLimits(
                    student_id.into(),
                    limits.into(),
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Settings(SettingsUpdateError::UpdateStudentLimits(
                ue,
            ))) => match ue {
                UpdateStudentLimitsError::InvalidStudentId(id) => Err(PyValueError::new_err(
                    format!("Invalid student id {:?}", id),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn settings_remove_student_limits(
        self_: PyRef<'_, Self>,
        student_id: StudentId,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Settings(
                collomatique_ops::SettingsUpdateOp::RemoveStudentLimits(student_id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(UpdateError::Settings(SettingsUpdateError::RemoveStudentLimits(
                re,
            ))) => match re {
                RemoveStudentLimitsError::InvalidStudentId(id) => Err(PyValueError::new_err(
                    format!("Invalid student id {:?}", id),
                )),
                RemoveStudentLimitsError::NoLimitsForStudent(id) => Err(PyValueError::new_err(
                    format!("Student id {:?} has no associated limits", id),
                )),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn colloscopes_add_empty(
        self_: PyRef<'_, Self>,
        name: String,
    ) -> PyResult<colloscopes::ColloscopeId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Colloscopes(
                collomatique_ops::ColloscopesUpdateOp::AddEmptyColloscope(name),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::ColloscopeId(id))) => {
                Ok(id.into())
            }
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn colloscopes_copy(
        self_: PyRef<'_, Self>,
        colloscope_id: ColloscopeId,
        name: String,
    ) -> PyResult<colloscopes::ColloscopeId> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Colloscopes(
                collomatique_ops::ColloscopesUpdateOp::CopyColloscope(colloscope_id.into(), name),
            ),
        ));

        match result {
            ResultMsg::Ack(Some(collomatique_state_colloscopes::NewId::ColloscopeId(id))) => {
                Ok(id.into())
            }
            ResultMsg::Error(collomatique_ops::UpdateError::Colloscopes(
                collomatique_ops::ColloscopesUpdateError::CopyColloscope(ce),
            )) => match ce {
                collomatique_ops::CopyColloscopeError::InvalidColloscopeId(id) => Err(
                    PyValueError::new_err(format!("Invalid colloscope id {:?}", id)),
                ),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn colloscopes_update(
        self_: PyRef<'_, Self>,
        colloscope_id: ColloscopeId,
        colloscope: colloscopes::Colloscope,
    ) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Colloscopes(
                collomatique_ops::ColloscopesUpdateOp::UpdateColloscope(
                    colloscope_id.into(),
                    colloscope.try_into()?,
                ),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(collomatique_ops::UpdateError::Colloscopes(
                collomatique_ops::ColloscopesUpdateError::UpdateColloscope(ue),
            )) => Err(PyValueError::new_err(format!("{:?}", ue))),
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn colloscopes_delete(self_: PyRef<'_, Self>, colloscope_id: ColloscopeId) -> PyResult<()> {
        let result = self_.token.send_msg(collomatique_rpc::CmdMsg::Update(
            collomatique_ops::UpdateOp::Colloscopes(
                collomatique_ops::ColloscopesUpdateOp::DeleteColloscope(colloscope_id.into()),
            ),
        ));

        match result {
            ResultMsg::Ack(None) => Ok(()),
            ResultMsg::Error(collomatique_ops::UpdateError::Colloscopes(
                collomatique_ops::ColloscopesUpdateError::DeleteColloscope(de),
            )) => match de {
                collomatique_ops::DeleteColloscopeError::InvalidColloscopeId(id) => Err(
                    PyValueError::new_err(format!("Invalid colloscope id {:?}", id)),
                ),
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn get_main_params(self_: PyRef<'_, Self>) -> PyResult<params::GeneralParameters> {
        self_.token.get_data().params.clone().try_into()
    }

    fn get_colloscopes(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<ColloscopeId, colloscopes::Colloscope>> {
        let colloscopes = &self_.token.get_data().colloscopes;

        colloscopes
            .colloscope_map
            .iter()
            .map(|(collo_id, collo)| Ok((collo_id.into(), collo.clone().try_into()?)))
            .collect::<PyResult<_>>()
    }
}

#[derive(Clone, Debug)]
enum InternalFile {
    Session(Session),
}

#[derive(Clone, Debug)]
struct Token {
    file: InternalFile,
}

impl Token {
    fn get_data(&self) -> collomatique_state_colloscopes::InnerData {
        match &self.file {
            InternalFile::Session(session) => {
                use collomatique_rpc::ResultMsg;

                let result = session.send_msg(collomatique_rpc::CmdMsg::GetData);
                let ResultMsg::Data(serialized_data) = result else {
                    panic!("Unexpected response to GetData");
                };
                collomatique_state_colloscopes::InnerData::from(serialized_data)
            }
        }
    }

    fn send_msg(&self, msg: collomatique_rpc::CmdMsg) -> collomatique_rpc::ResultMsg {
        match &self.file {
            InternalFile::Session(session) => session.send_msg(msg),
        }
    }
}
