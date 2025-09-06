use crate::rpc::error_msg::week_patterns::WeekPatternsError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeekPatternsCmdMsg {
    AddNewWeekPattern(WeekPatternMsg),
    UpdateWeekPattern(MsgWeekPatternId, WeekPatternMsg),
    DeleteWeekPattern(MsgWeekPatternId),
}

impl WeekPatternsCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::WeekPatternsUpdateOp, WeekPatternsError> {
        use crate::ops::WeekPatternsUpdateOp;
        Ok(match self {
            WeekPatternsCmdMsg::AddNewWeekPattern(week_pattern_msg) => {
                let new_week_pattern = week_pattern_msg.into();

                WeekPatternsUpdateOp::AddNewWeekPattern(new_week_pattern)
            }
            WeekPatternsCmdMsg::UpdateWeekPattern(id, week_pattern_msg) => {
                let Some(week_pattern_id) = data
                    .get_inner_data()
                    .main_params
                    .validate_week_pattern_id(id.0)
                else {
                    return Err(error_msg::UpdateWeekPatternError::InvalidWeekPatternId(id).into());
                };
                let new_week_pattern = week_pattern_msg.into();
                WeekPatternsUpdateOp::UpdateWeekPattern(week_pattern_id, new_week_pattern)
            }
            WeekPatternsCmdMsg::DeleteWeekPattern(id) => {
                let Some(week_pattern_id) = data
                    .get_inner_data()
                    .main_params
                    .validate_week_pattern_id(id.0)
                else {
                    return Err(error_msg::DeleteWeekPatternError::InvalidWeekPatternId(id).into());
                };
                WeekPatternsUpdateOp::DeleteWeekPattern(week_pattern_id)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPatternMsg {
    pub name: String,
    pub weeks: Vec<bool>,
}

impl From<WeekPatternMsg> for collomatique_state_colloscopes::week_patterns::WeekPattern {
    fn from(value: WeekPatternMsg) -> Self {
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: value.name,
            weeks: value.weeks,
        }
    }
}
