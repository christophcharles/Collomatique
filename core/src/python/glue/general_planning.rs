use crate::rpc::ResultMsg;

use super::*;

#[pyclass]
pub struct SessionPeriods {}

#[pymethods]
impl SessionPeriods {
    fn add(_self: PyRef<'_, Self>, week_count: usize) {
        let result = crate::rpc::send_rpc(crate::rpc::CmdMsg::GeneralPlanning(
            crate::rpc::cmd_msg::GeneralPlanningCmdMsg::AddNewPeriod(week_count),
        ))
        .expect("No error for adding period");

        assert!(result == ResultMsg::Ack);
    }
}
