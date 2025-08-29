//! Rules submodule
//!
//! This module defines the relevant types to describes the rules for complex teacher schedule

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{PeriodId, RuleId, SlotId};

/// Description of the rules
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Rules {
    /// Rules
    ///
    /// Each item associates a rule id to an actual rule
    pub rule_map: BTreeMap<RuleId, Rule>,
}

/// Description of a single rule
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    /// name for the rule
    pub name: String,
    /// excluded periods
    ///
    /// The rule should be enforced only on the other periods
    pub excluded_periods: BTreeSet<PeriodId>,
    /// Rule description
    pub desc: LogicRule,
}

/// Logic rule enumeration
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicRule {
    And(Box<LogicRule>, Box<LogicRule>),
    Or(Box<LogicRule>, Box<LogicRule>),
    Not(Box<LogicRule>),
    Variable(SlotId),
}

impl Rule {
    /// Builds a single rule from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [RuleExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: RuleExternalData) -> Rule {
        Rule {
            name: external_data.name,
            excluded_periods: external_data
                .excluded_periods
                .into_iter()
                .map(|x| unsafe { PeriodId::new(x) })
                .collect(),
            desc: unsafe { LogicRule::from_external_data(external_data.desc) },
        }
    }
}

impl Rules {
    /// Builds rules from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [Rules::validate_all].
    pub(crate) unsafe fn from_external_data(external_data: RulesExternalData) -> Rules {
        Rules {
            rule_map: external_data
                .rule_map
                .into_iter()
                .map(|(id, rule)| {
                    (unsafe { RuleId::new(id) }, unsafe {
                        Rule::from_external_data(rule)
                    })
                })
                .collect(),
        }
    }
}

impl LogicRule {
    /// Builds a rule description from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [LogicRuleExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: LogicRuleExternalData) -> LogicRule {
        match external_data {
            LogicRuleExternalData::And(e1, e2) => {
                let l1 = unsafe { LogicRule::from_external_data(*e1) };
                let l2 = unsafe { LogicRule::from_external_data(*e2) };
                LogicRule::And(Box::new(l1), Box::new(l2))
            }
            LogicRuleExternalData::Or(e1, e2) => {
                let l1 = unsafe { LogicRule::from_external_data(*e1) };
                let l2 = unsafe { LogicRule::from_external_data(*e2) };
                LogicRule::Or(Box::new(l1), Box::new(l2))
            }
            LogicRuleExternalData::Not(e) => {
                let l = unsafe { LogicRule::from_external_data(*e) };
                LogicRule::Not(Box::new(l))
            }
            LogicRuleExternalData::Variable(id) => LogicRule::Variable(unsafe { SlotId::new(id) }),
        }
    }
}

/// Description of the rules but unchecked
///
/// This structure is an unchecked equivalent of [Rules].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RulesExternalData {
    /// Rules
    ///
    /// Each item associates a rule id to an actual rule
    pub rule_map: BTreeMap<u64, RuleExternalData>,
}

impl RulesExternalData {
    /// Checks the validity [RulesExternalData]
    pub fn validate_all(&self, period_ids: &BTreeSet<u64>, slot_ids: &BTreeSet<u64>) -> bool {
        self.rule_map
            .iter()
            .all(|(_rule_id, rule)| rule.validate(period_ids, slot_ids))
    }
}

/// Description of a single rule but unchecked
///
/// This structure is an unchecked equivalent of [Rule].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleExternalData {
    /// name for the rule
    pub name: String,
    /// excluded periods
    ///
    /// The rule should be enforced only on the other periods
    pub excluded_periods: BTreeSet<u64>,
    /// Rule description
    pub desc: LogicRuleExternalData,
}

impl RuleExternalData {
    /// Checks the validity of a [RuleExternalData]
    pub fn validate(&self, period_ids: &BTreeSet<u64>, slot_ids: &BTreeSet<u64>) -> bool {
        for period_id in &self.excluded_periods {
            if !period_ids.contains(period_id) {
                return false;
            }
        }
        self.desc.validate(slot_ids)
    }
}

/// Prefilled groups for a single group list but unchecked
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicRuleExternalData {
    And(Box<LogicRuleExternalData>, Box<LogicRuleExternalData>),
    Or(Box<LogicRuleExternalData>, Box<LogicRuleExternalData>),
    Not(Box<LogicRuleExternalData>),
    Variable(u64),
}

impl LogicRuleExternalData {
    /// Checks the validity of a [LogicRuleExternalData]
    pub fn validate(&self, slot_ids: &BTreeSet<u64>) -> bool {
        match self {
            LogicRuleExternalData::And(l1, l2) => l1.validate(slot_ids) && l2.validate(slot_ids),
            LogicRuleExternalData::Or(l1, l2) => l1.validate(slot_ids) && l2.validate(slot_ids),
            LogicRuleExternalData::Not(l) => l.validate(slot_ids),
            LogicRuleExternalData::Variable(id) => slot_ids.contains(id),
        }
    }
}

impl From<Rule> for RuleExternalData {
    fn from(value: Rule) -> Self {
        RuleExternalData {
            name: value.name,
            excluded_periods: value
                .excluded_periods
                .into_iter()
                .map(|x| x.inner())
                .collect(),
            desc: value.desc.into(),
        }
    }
}

impl From<LogicRule> for LogicRuleExternalData {
    fn from(value: LogicRule) -> Self {
        match value {
            LogicRule::And(l1, l2) => {
                let e1 = (*l1).into();
                let e2 = (*l2).into();

                LogicRuleExternalData::And(Box::new(e1), Box::new(e2))
            }
            LogicRule::Or(l1, l2) => {
                let e1 = (*l1).into();
                let e2 = (*l2).into();

                LogicRuleExternalData::Or(Box::new(e1), Box::new(e2))
            }
            LogicRule::Not(l) => {
                let e = (*l).into();
                LogicRuleExternalData::Not(Box::new(e))
            }
            LogicRule::Variable(id) => LogicRuleExternalData::Variable(id.inner()),
        }
    }
}
