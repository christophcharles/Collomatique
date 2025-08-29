use crate::rpc::{cmd_msg::MsgPeriodId, ResultMsg};

use super::*;
use pyo3::types::PyString;

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

    fn add(_self: PyRef<'_, Self>, week_count: usize) {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::Update(
            crate::rpc::UpdateMsg::GeneralPlanning(
                crate::rpc::cmd_msg::GeneralPlanningCmdMsg::AddNewPeriod(week_count),
            ),
        ))
        .expect("No error for adding period");

        assert!(result == ResultMsg::Ack);
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
