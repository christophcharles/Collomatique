use crate::rpc::error_msg::general_planning::GeneralPlanningError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralPlanningCmdMsg {
    DeleteFirstWeek,
    UpdateFirstWeek(chrono::NaiveDate),
    AddNewPeriod(usize),
    UpdatePeriodWeekCount(MsgPeriodId, usize),
    DeletePeriod(MsgPeriodId),
    CutPeriod(MsgPeriodId, usize),
    MergeWithPreviousPeriod(MsgPeriodId),
    UpdateWeekStatus(MsgPeriodId, usize, bool),
}

impl GeneralPlanningCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::GeneralPlanningUpdateOp, GeneralPlanningError> {
        use crate::ops::GeneralPlanningUpdateOp;
        Ok(match self {
            GeneralPlanningCmdMsg::DeleteFirstWeek => GeneralPlanningUpdateOp::DeleteFirstWeek,
            GeneralPlanningCmdMsg::UpdateFirstWeek(monday_date) => {
                let Some(date) = collomatique_time::NaiveMondayDate::new(monday_date) else {
                    return Err(error_msg::UpdateFirstWeekError::DateIsNotAMonday.into());
                };
                GeneralPlanningUpdateOp::UpdateFirstWeek(date)
            }
            GeneralPlanningCmdMsg::AddNewPeriod(week_count) => {
                GeneralPlanningUpdateOp::AddNewPeriod(week_count)
            }
            GeneralPlanningCmdMsg::UpdatePeriodWeekCount(id, week_count) => {
                let Some(period_id) = data.get_inner_data().main_params.validate_period_id(id.0)
                else {
                    return Err(error_msg::UpdatePeriodWeekCountError::InvalidPeriodId(id).into());
                };
                GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count)
            }
            GeneralPlanningCmdMsg::DeletePeriod(id) => {
                let Some(period_id) = data.get_inner_data().main_params.validate_period_id(id.0)
                else {
                    return Err(error_msg::UpdatePeriodWeekCountError::InvalidPeriodId(id).into());
                };
                GeneralPlanningUpdateOp::DeletePeriod(period_id)
            }
            GeneralPlanningCmdMsg::CutPeriod(id, remaining_weeks) => {
                let Some(period_id) = data.get_inner_data().main_params.validate_period_id(id.0)
                else {
                    return Err(error_msg::UpdatePeriodWeekCountError::InvalidPeriodId(id).into());
                };
                GeneralPlanningUpdateOp::CutPeriod(period_id, remaining_weeks)
            }
            GeneralPlanningCmdMsg::MergeWithPreviousPeriod(id) => {
                let Some(period_id) = data.get_inner_data().main_params.validate_period_id(id.0)
                else {
                    return Err(error_msg::UpdatePeriodWeekCountError::InvalidPeriodId(id).into());
                };
                GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id)
            }
            GeneralPlanningCmdMsg::UpdateWeekStatus(id, week, new_status) => {
                let Some(period_id) = data.get_inner_data().main_params.validate_period_id(id.0)
                else {
                    return Err(error_msg::UpdatePeriodWeekCountError::InvalidPeriodId(id).into());
                };
                GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week, new_status)
            }
        })
    }
}
