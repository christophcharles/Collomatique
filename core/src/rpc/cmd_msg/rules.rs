use collomatique_state_colloscopes::colloscope_params::PromoteLogicRuleError;

use crate::rpc::error_msg::rules::RulesError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RulesCmdMsg {
    AddNewRule(String, LogicRuleMsg),
    UpdateRule(MsgRuleId, String, LogicRuleMsg),
    DeleteRule(MsgRuleId),
    UpdatePeriodStatusForRule(MsgRuleId, MsgPeriodId, bool),
}

impl RulesCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::RulesUpdateOp, RulesError> {
        use crate::ops::RulesUpdateOp;
        Ok(match self {
            RulesCmdMsg::AddNewRule(name, rule) => {
                let new_rule = match data
                    .get_inner_data()
                    .main_params
                    .promote_logic_rule(rule.into())
                {
                    Ok(l) => l,
                    Err(PromoteLogicRuleError::InvalidSlotId(id)) => {
                        return Err(error_msg::AddNewRuleError::InvalidSlotId(MsgSlotId(id)).into())
                    }
                };
                RulesUpdateOp::AddNewRule(name, new_rule)
            }
            RulesCmdMsg::UpdateRule(id, name, rule) => {
                let Some(rule_id) = data.get_inner_data().main_params.validate_rule_id(id.0) else {
                    return Err(error_msg::UpdateRuleError::InvalidRuleId(id).into());
                };
                let new_rule = match data
                    .get_inner_data()
                    .main_params
                    .promote_logic_rule(rule.into())
                {
                    Ok(l) => l,
                    Err(PromoteLogicRuleError::InvalidSlotId(id)) => {
                        return Err(error_msg::UpdateRuleError::InvalidSlotId(MsgSlotId(id)).into())
                    }
                };
                RulesUpdateOp::UpdateRule(rule_id, name, new_rule)
            }
            RulesCmdMsg::DeleteRule(id) => {
                let Some(rule_id) = data.get_inner_data().main_params.validate_rule_id(id.0) else {
                    return Err(error_msg::DeleteRuleError::InvalidRuleId(id).into());
                };
                RulesUpdateOp::DeleteRule(rule_id)
            }
            RulesCmdMsg::UpdatePeriodStatusForRule(rule_id, period_id, status) => {
                let Some(rule_id) = data
                    .get_inner_data()
                    .main_params
                    .validate_rule_id(rule_id.0)
                else {
                    return Err(
                        error_msg::UpdatePeriodStatusForRuleError::InvalidRuleId(rule_id).into(),
                    );
                };
                let Some(period_id) = data
                    .get_inner_data()
                    .main_params
                    .validate_period_id(period_id.0)
                else {
                    return Err(error_msg::UpdatePeriodStatusForRuleError::InvalidPeriodId(
                        period_id,
                    )
                    .into());
                };
                RulesUpdateOp::UpdatePeriodStatusForRule(rule_id, period_id, status)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicRuleMsg {
    And(Box<LogicRuleMsg>, Box<LogicRuleMsg>),
    Or(Box<LogicRuleMsg>, Box<LogicRuleMsg>),
    Not(Box<LogicRuleMsg>),
    Variable(MsgSlotId),
}

impl From<LogicRuleMsg> for collomatique_state_colloscopes::rules::LogicRuleExternalData {
    fn from(value: LogicRuleMsg) -> Self {
        use collomatique_state_colloscopes::rules::LogicRuleExternalData;
        match value {
            LogicRuleMsg::And(l1, l2) => {
                LogicRuleExternalData::And(Box::new((*l1).into()), Box::new((*l2).into()))
            }
            LogicRuleMsg::Or(l1, l2) => {
                LogicRuleExternalData::Or(Box::new((*l1).into()), Box::new((*l2).into()))
            }
            LogicRuleMsg::Not(l) => LogicRuleExternalData::Not(Box::new((*l).into())),
            LogicRuleMsg::Variable(v) => LogicRuleExternalData::Variable(v.0),
        }
    }
}
