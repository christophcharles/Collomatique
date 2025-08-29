use crate::rpc::{
    error_msg::{
        CutPeriodError, DeletePeriodError, GeneralPlanningError, MergeWithPreviousPeriodError,
        UpdatePeriodWeekCountError, UpdateWeekStatusError,
    },
    ErrorMsg, ResultMsg,
};

use super::*;
use pyo3::{exceptions::PyValueError, types::PyString};

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeriodId {
    id: crate::rpc::cmd_msg::MsgPeriodId,
}

#[pymethods]
impl PeriodId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgPeriodId> for PeriodId {
    fn from(value: &crate::rpc::cmd_msg::MsgPeriodId) -> Self {
        PeriodId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgPeriodId> for PeriodId {
    fn from(value: crate::rpc::cmd_msg::MsgPeriodId) -> Self {
        PeriodId::from(&value)
    }
}

impl From<&PeriodId> for crate::rpc::cmd_msg::MsgPeriodId {
    fn from(value: &PeriodId) -> Self {
        value.id.clone()
    }
}

impl From<PeriodId> for crate::rpc::cmd_msg::MsgPeriodId {
    fn from(value: PeriodId) -> Self {
        crate::rpc::cmd_msg::MsgPeriodId::from(&value)
    }
}

#[pyclass]
pub struct SessionPeriods {
    pub(super) token: super::Token,
}

#[pymethods]
impl SessionPeriods {
    fn set_first_week(self_: PyRef<'_, Self>, first_week: Option<time::NaiveMondayDate>) {
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

    fn add(self_: PyRef<'_, Self>, week_count: usize) -> PeriodId {
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

    fn update(self_: PyRef<'_, Self>, id: PeriodId, new_week_count: usize) -> PyResult<()> {
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
            },
            _ => panic!("Unexpected result: {:?}", result),
        }
    }

    fn delete(self_: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
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

    fn cut(self_: PyRef<'_, Self>, id: PeriodId, remaining_weeks: usize) -> PyResult<PeriodId> {
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

    fn merge_with_previous(self_: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
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

    fn update_week_status(
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
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Period {
    #[pyo3(set, get)]
    pub id: PeriodId,
    #[pyo3(set, get)]
    pub weeks_status: Vec<bool>,
}

#[pymethods]
impl Period {
    #[new]
    fn new(id: PeriodId, weeks_status: Vec<bool>) -> Self {
        Period { id, weeks_status }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}
