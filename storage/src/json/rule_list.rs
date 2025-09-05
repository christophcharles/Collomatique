//! rule submodule
//!
//! This module defines the rule list entry for the JSON description
//!
use super::*;

use collomatique_state_colloscopes::ids::Id;

use std::collections::{BTreeMap, BTreeSet};

/// JSON desc of rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between rule ids and corresponding rules
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub rule_map: BTreeMap<u64, Rule>,
}

/// JSON desc of a single rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub excluded_periods: BTreeSet<u64>,
    pub desc: LogicRule,
}

/// JSON desc of a single logic rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicRule {
    And(Box<LogicRule>, Box<LogicRule>),
    Or(Box<LogicRule>, Box<LogicRule>),
    Not(Box<LogicRule>),
    Variable(u64),
}

impl<SlotId: Id> From<&collomatique_state_colloscopes::rules::LogicRule<SlotId>> for LogicRule {
    fn from(value: &collomatique_state_colloscopes::rules::LogicRule<SlotId>) -> Self {
        use collomatique_state_colloscopes::rules::LogicRule as LR;
        match value {
            LR::And(l1, l2) => LogicRule::And(
                Box::new(LogicRule::from(l1.as_ref())),
                Box::new(LogicRule::from(l2.as_ref())),
            ),
            LR::Or(l1, l2) => LogicRule::Or(
                Box::new(LogicRule::from(l1.as_ref())),
                Box::new(LogicRule::from(l2.as_ref())),
            ),
            LR::Not(l) => LogicRule::Not(Box::new(LogicRule::from(l.as_ref()))),
            LR::Variable(id) => LogicRule::Variable(id.inner()),
        }
    }
}

impl From<LogicRule> for collomatique_state_colloscopes::rules::LogicRuleExternalData {
    fn from(value: LogicRule) -> Self {
        use collomatique_state_colloscopes::rules::LogicRuleExternalData as LRED;
        match value {
            LogicRule::And(l1, l2) => {
                LRED::And(Box::new(LRED::from(*l1)), Box::new(LRED::from(*l2)))
            }
            LogicRule::Or(l1, l2) => LRED::Or(Box::new(LRED::from(*l1)), Box::new(LRED::from(*l2))),
            LogicRule::Not(l) => LRED::Not(Box::new(LRED::from(*l))),
            LogicRule::Variable(id) => LRED::Variable(id),
        }
    }
}

impl<PeriodId: Id, SlotId: Id> From<&collomatique_state_colloscopes::rules::Rule<PeriodId, SlotId>>
    for Rule
{
    fn from(value: &collomatique_state_colloscopes::rules::Rule<PeriodId, SlotId>) -> Self {
        Rule {
            name: value.name.clone(),
            excluded_periods: value.excluded_periods.iter().map(|x| x.inner()).collect(),
            desc: (&value.desc).into(),
        }
    }
}

impl From<Rule> for collomatique_state_colloscopes::rules::RuleExternalData {
    fn from(value: Rule) -> Self {
        collomatique_state_colloscopes::rules::RuleExternalData {
            name: value.name,
            excluded_periods: value.excluded_periods,
            desc: value.desc.into(),
        }
    }
}
