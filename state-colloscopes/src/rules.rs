//! Rules submodule
//!
//! This module defines the relevant types to describes the rules for complex teacher schedule

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{PeriodId, RuleId, SlotId};

/// Description of the rules
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// Rules
    ///
    /// Each item associates a rule id to an actual rule
    pub rule_map: BTreeMap<RuleId, Rule>,
}

/// Description of a single rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicRule {
    And(Box<LogicRule>, Box<LogicRule>),
    Or(Box<LogicRule>, Box<LogicRule>),
    Not(Box<LogicRule>),
    Variable(SlotId),
}

impl LogicRule {
    pub fn references_slot(&self, slot_id: SlotId) -> bool {
        match self {
            LogicRule::And(l1, l2) => l1.references_slot(slot_id) || l2.references_slot(slot_id),
            LogicRule::Or(l1, l2) => l1.references_slot(slot_id) || l2.references_slot(slot_id),
            LogicRule::Not(l) => l.references_slot(slot_id),
            LogicRule::Variable(id) => *id == slot_id,
        }
    }
}
