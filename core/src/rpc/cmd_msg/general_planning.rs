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

impl From<crate::ops::GeneralPlanningUpdateOp> for GeneralPlanningCmdMsg {
    fn from(value: crate::ops::GeneralPlanningUpdateOp) -> Self {
        use crate::ops::GeneralPlanningUpdateOp;
        match value {
            GeneralPlanningUpdateOp::DeleteFirstWeek => GeneralPlanningCmdMsg::DeleteFirstWeek,
            GeneralPlanningUpdateOp::UpdateFirstWeek(monday_date) => {
                GeneralPlanningCmdMsg::UpdateFirstWeek(monday_date.into_inner())
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                GeneralPlanningCmdMsg::AddNewPeriod(week_count)
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(id, week_count) => {
                GeneralPlanningCmdMsg::UpdatePeriodWeekCount(id.into(), week_count)
            }
            GeneralPlanningUpdateOp::DeletePeriod(id) => {
                GeneralPlanningCmdMsg::DeletePeriod(id.into())
            }
            GeneralPlanningUpdateOp::CutPeriod(id, remaining_weeks) => {
                GeneralPlanningCmdMsg::CutPeriod(id.into(), remaining_weeks)
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(id) => {
                GeneralPlanningCmdMsg::MergeWithPreviousPeriod(id.into())
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(id, week, new_status) => {
                GeneralPlanningCmdMsg::UpdateWeekStatus(id.into(), week, new_status)
            }
        }
    }
}
