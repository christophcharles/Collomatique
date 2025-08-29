use crate::rpc::{
    cmd_msg::MsgPeriodId,
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
pub struct SessionPeriods {}

#[pymethods]
impl SessionPeriods {
    fn get(_self: PyRef<'_, Self>) -> Periods {
        let result =
            crate::rpc::send_rpc(crate::rpc::CmdMsg::GetData).expect("No error for getting data");
        let ResultMsg::Data(serialized_data) = result else {
            panic!("Unexpected response to GetData");
        };
        let data = collomatique_state_colloscopes::Data::from(serialized_data);
        data.get_periods().into()
    }

    fn set_first_week(_self: PyRef<'_, Self>, first_week: Option<time::NaiveMondayDate>) {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(match first_week {
                Some(week) => crate::rpc::cmd_msg::GeneralPlanningCmdMsg::UpdateFirstWeek(
                    collomatique_time::NaiveMondayDate::from(week).into_inner(),
                ),
                None => crate::rpc::cmd_msg::GeneralPlanningCmdMsg::DeleteFirstWeek,
            }),
        ))
        .expect("Valid result message");

        if result != ResultMsg::Ack {
            panic!("Unexpected result: {:?}", result)
        }
    }

    fn add(_self: PyRef<'_, Self>, week_count: usize) {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::AddNewPeriod(week_count),
            ),
        ))
        .expect("Valid result message");

        if result != ResultMsg::Ack {
            panic!("Unexpected result: {:?}", result)
        }
    }

    fn update(_self: PyRef<'_, Self>, id: PeriodId, new_week_count: usize) -> PyResult<()> {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::UpdatePeriodWeekCount(
                    id.into(),
                    new_week_count,
                ),
            ),
        ))
        .expect("Valid result message");

        match result {
            ResultMsg::Ack => Ok(()),
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

    fn delete(_self: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::DeletePeriod(id.into()),
            ),
        ))
        .expect("Valid result message");

        match result {
            ResultMsg::Ack => Ok(()),
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

    fn cut(_self: PyRef<'_, Self>, id: PeriodId, remaining_weeks: usize) -> PyResult<()> {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::CutPeriod(id.into(), remaining_weeks),
            ),
        ))
        .expect("Valid result message");

        match result {
            ResultMsg::Ack => Ok(()),
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

    fn merge_with_previous(_self: PyRef<'_, Self>, id: PeriodId) -> PyResult<()> {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::MergeWithPreviousPeriod(id.into()),
            ),
        ))
        .expect("Valid result message");

        match result {
            ResultMsg::Ack => Ok(()),
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
        _self: PyRef<'_, Self>,
        id: PeriodId,
        week: usize,
        new_status: bool,
    ) -> PyResult<()> {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::UpdateWeekStatus(
                    id.into(),
                    week,
                    new_status,
                ),
            ),
        ))
        .expect("Valid result message");

        match result {
            ResultMsg::Ack => Ok(()),
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
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Periods {
    #[pyo3(get)]
    first_week: Option<time::NaiveMondayDate>,
    #[pyo3(get)]
    ordered_period_list: Vec<(PeriodId, Vec<bool>)>,
}

#[pymethods]
impl Periods {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::periods::Periods> for Periods {
    fn from(value: &collomatique_state_colloscopes::periods::Periods) -> Self {
        Periods {
            first_week: value.first_week.clone().map(|x| x.into()),
            ordered_period_list: value
                .ordered_period_list
                .iter()
                .map(|(id, week_status)| (MsgPeriodId::from(*id).into(), week_status.clone()))
                .collect(),
        }
    }
}
