use std::collections::BTreeMap;

use pyo3::prelude::*;

use crate::rpc::{
    error_msg::{
        AddNewSubjectError, CutPeriodError, DeletePeriodError, DeleteSubjectError,
        GeneralPlanningError, MergeWithPreviousPeriodError, MoveDownError, MoveUpError,
        SubjectsError, UpdatePeriodStatusError, UpdatePeriodWeekCountError, UpdateSubjectError,
        UpdateWeekStatusError,
    },
    ErrorMsg, ResultMsg,
};

use pyo3::exceptions::PyValueError;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    m.add_class::<general_planning::Period>()?;
    m.add_class::<subjects::Subject>()?;
    m.add_class::<subjects::SubjectParameters>()?;
    m.add_class::<subjects::SubjectPeriodicity>()?;
    m.add_class::<time::NaiveMondayDate>()?;
    m.add_class::<time::NaiveDate>()?;

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
