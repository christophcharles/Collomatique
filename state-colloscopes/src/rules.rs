//! Rules submodule
//!
//! This module defines the relevant types to describes the rules for complex teacher schedule

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{ColloscopePeriodId, ColloscopeRuleId, ColloscopeSlotId, Id};

/// Description of the rules
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules<RuleId: Id, PeriodId: Id, SlotId: Id> {
    /// Rules
    ///
    /// Each item associates a rule id to an actual rule
    pub rule_map: BTreeMap<RuleId, Rule<PeriodId, SlotId>>,
}

impl<RuleId: Id, PeriodId: Id, SlotId: Id> Default for Rules<RuleId, PeriodId, SlotId> {
    fn default() -> Self {
        Rules {
            rule_map: BTreeMap::new(),
        }
    }
}

/// Description of a single rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule<PeriodId: Id, SlotId: Id> {
    /// name for the rule
    pub name: String,
    /// excluded periods
    ///
    /// The rule should be enforced only on the other periods
    pub excluded_periods: BTreeSet<PeriodId>,
    /// Rule description
    pub desc: LogicRule<SlotId>,
}

/// Logic rule enumeration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicRule<SlotId: Id> {
    And(Box<LogicRule<SlotId>>, Box<LogicRule<SlotId>>),
    Or(Box<LogicRule<SlotId>>, Box<LogicRule<SlotId>>),
    Not(Box<LogicRule<SlotId>>),
    Variable(SlotId),
}

impl<SlotId: Id> LogicRule<SlotId> {
    pub fn references_slot(&self, slot_id: SlotId) -> bool {
        match self {
            LogicRule::And(l1, l2) => l1.references_slot(slot_id) || l2.references_slot(slot_id),
            LogicRule::Or(l1, l2) => l1.references_slot(slot_id) || l2.references_slot(slot_id),
            LogicRule::Not(l) => l.references_slot(slot_id),
            LogicRule::Variable(id) => *id == slot_id,
        }
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        slots_map: &BTreeMap<SlotId, ColloscopeSlotId>,
    ) -> Option<LogicRule<ColloscopeSlotId>> {
        Some(match self {
            LogicRule::And(l1, l2) => {
                let new_l1 = l1.duplicate_with_id_maps(slots_map)?;
                let new_l2 = l2.duplicate_with_id_maps(slots_map)?;
                LogicRule::And(Box::new(new_l1), Box::new(new_l2))
            }
            LogicRule::Or(l1, l2) => {
                let new_l1 = l1.duplicate_with_id_maps(slots_map)?;
                let new_l2 = l2.duplicate_with_id_maps(slots_map)?;
                LogicRule::Or(Box::new(new_l1), Box::new(new_l2))
            }
            LogicRule::Not(l) => {
                let new_l = l.duplicate_with_id_maps(slots_map)?;
                LogicRule::Not(Box::new(new_l))
            }
            LogicRule::Variable(slot_id) => {
                let new_id = slots_map.get(slot_id)?;
                LogicRule::Variable(*new_id)
            }
        })
    }
}

impl<PeriodId: Id, SlotId: Id> Rule<PeriodId, SlotId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
        slots_map: &BTreeMap<SlotId, ColloscopeSlotId>,
    ) -> Option<Rule<ColloscopePeriodId, ColloscopeSlotId>> {
        let mut excluded_periods = BTreeSet::new();

        for period_id in &self.excluded_periods {
            let new_id = periods_map.get(period_id)?;
            excluded_periods.insert(*new_id);
        }

        Some(Rule {
            name: self.name.clone(),
            excluded_periods,
            desc: self.desc.duplicate_with_id_maps(slots_map)?,
        })
    }
}

impl<RuleId: Id, PeriodId: Id, SlotId: Id> Rules<RuleId, PeriodId, SlotId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        rules_map: &BTreeMap<RuleId, ColloscopeRuleId>,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
        slots_map: &BTreeMap<SlotId, ColloscopeSlotId>,
    ) -> Option<Rules<ColloscopeRuleId, ColloscopePeriodId, ColloscopeSlotId>> {
        let mut rule_map = BTreeMap::new();

        for (rule_id, rule) in &self.rule_map {
            let new_id = rules_map.get(rule_id)?;
            let new_rule = rule.duplicate_with_id_maps(periods_map, slots_map)?;
            rule_map.insert(*new_id, new_rule);
        }

        Some(Rules { rule_map })
    }
}
