//! Rules submodule
//!
//! This module defines the relevant types to describes the rules for complex teacher schedule

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{ColloscopePeriodId, ColloscopeRuleId, ColloscopeSlotId, Id};

/// Description of the rules
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Builds a single rule from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [RuleExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: RuleExternalData) -> Self {
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
    /// Builds rules from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [Rules::validate_all].
    pub(crate) unsafe fn from_external_data(external_data: RulesExternalData) -> Self {
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

impl<SlotId: Id> LogicRule<SlotId> {
    /// Builds a rule description from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [LogicRuleExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: LogicRuleExternalData) -> Self {
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

impl<PeriodId: Id, SlotId: Id> From<Rule<PeriodId, SlotId>> for RuleExternalData {
    fn from(value: Rule<PeriodId, SlotId>) -> Self {
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

impl<SlotId: Id> From<LogicRule<SlotId>> for LogicRuleExternalData {
    fn from(value: LogicRule<SlotId>) -> Self {
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
