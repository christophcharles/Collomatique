//! Slot pairings submodule
//!
//! This module defines the relevant types to describe pairing rules between slots
//! within the same subject

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{ContentOrd, Join, References};

use crate::Table;
use crate::ids::{NewId, PeriodId, SlotId, SlotPairingRuleId};
use crate::ops::AnnotatedSlotPairingOp;

/// Description of the slot pairing rules
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct SlotPairings {
    /// Map from slot pairing rule id to slot pairing rule
    pub slot_pairing_rule_map: Table<SlotPairingRuleId, SlotPairingRule>,
}

/// One part (antecedent or consequent) of a slot pairing rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
#[join(error = NewId)]
pub struct SlotRulePart {
    /// The slot this part refers to
    #[fk(name = slot)]
    pub slot_id: SlotId,
    /// Whether the slot should be used (true) or not used (false)
    pub should_have: bool,
}

/// A pairing rule between two slots of the same subject
///
/// Represents an implication: if the antecedent condition holds for a slot
/// in a given week, then the consequent condition must also hold.
///
/// For example, "Slot A used => Slot B used" means that if slot A has groups
/// registered in a given week, slot B must also have groups registered that week.
///
/// Both slots must belong to the same subject. Rules only apply on weeks where
/// both slots are active.
///
/// Sealed: the fields are private and every value is built through
/// [`SlotPairingRule::new`], which enforces the one value-internal invariant
/// (the antecedent and consequent must name different slots — an implication
/// from a slot to itself is meaningless). The cross-entity fact that both slots
/// belong to the same subject is state-dependent (it needs the slot→subject
/// map) and stays with the checker/validator. Serialized exactly like the raw
/// four-field struct via `RawSlotPairingRule`; deserializing a rule with both
/// parts on one slot is a hard error (the
/// [`crate::non_empty_range::NonEmptyRangeInclusive`] precedent).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
#[join(error = NewId)]
#[serde(try_from = "RawSlotPairingRule", into = "RawSlotPairingRule")]
pub struct SlotPairingRule {
    /// The antecedent of the implication
    #[fk]
    antecedent: SlotRulePart,
    /// The consequent of the implication
    #[fk]
    consequent: SlotRulePart,
    /// Periods where the rule does NOT apply
    #[fk]
    excluded_periods: BTreeSet<PeriodId>,
    /// Whether this is a soft constraint (best-effort) or hard (strict)
    soft: bool,
}

/// Private serde mirror of [`SlotPairingRule`]: the transparent four-field
/// struct. Deserialization funnels through [`SlotPairingRule::new`] (honest
/// decode); serialization is the plain field dump, so the wire format is
/// byte-identical to the pre-sealing struct.
#[derive(Serialize, Deserialize)]
struct RawSlotPairingRule {
    antecedent: SlotRulePart,
    consequent: SlotRulePart,
    excluded_periods: BTreeSet<PeriodId>,
    soft: bool,
}

impl From<SlotPairingRule> for RawSlotPairingRule {
    fn from(rule: SlotPairingRule) -> Self {
        RawSlotPairingRule {
            antecedent: rule.antecedent,
            consequent: rule.consequent,
            excluded_periods: rule.excluded_periods,
            soft: rule.soft,
        }
    }
}

impl TryFrom<RawSlotPairingRule> for SlotPairingRule {
    type Error = SlotPairingRuleBuildError;
    fn try_from(raw: RawSlotPairingRule) -> Result<Self, SlotPairingRuleBuildError> {
        SlotPairingRule::new(
            raw.antecedent,
            raw.consequent,
            raw.excluded_periods,
            raw.soft,
        )
    }
}

/// Value-internal build failure of [`SlotPairingRule::new`]: a
/// self-contradictory rule, independent of any state.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotPairingRuleBuildError {
    /// Antecedent and consequent name the same slot
    #[error("antecedent and consequent slots are the same ({0:?})")]
    SameSlotInBothParts(SlotId),
}

impl SlotPairingRule {
    /// Build a validated slot pairing rule.
    ///
    /// Fails if the antecedent and consequent name the same slot: an
    /// implication from a slot to itself is meaningless. This is the only
    /// value-internal invariant; the cross-entity fact that both slots belong
    /// to the same subject is state-dependent and checked elsewhere.
    pub fn new(
        antecedent: SlotRulePart,
        consequent: SlotRulePart,
        excluded_periods: BTreeSet<PeriodId>,
        soft: bool,
    ) -> Result<Self, SlotPairingRuleBuildError> {
        if antecedent.slot_id == consequent.slot_id {
            return Err(SlotPairingRuleBuildError::SameSlotInBothParts(
                antecedent.slot_id,
            ));
        }
        Ok(SlotPairingRule {
            antecedent,
            consequent,
            excluded_periods,
            soft,
        })
    }

    /// The antecedent of the implication
    pub fn antecedent(&self) -> &SlotRulePart {
        &self.antecedent
    }
    /// The consequent of the implication
    pub fn consequent(&self) -> &SlotRulePart {
        &self.consequent
    }
    /// Periods where the rule does NOT apply
    pub fn excluded_periods(&self) -> &BTreeSet<PeriodId> {
        &self.excluded_periods
    }
    /// Whether this is a soft constraint (best-effort) or hard (strict)
    pub fn soft(&self) -> bool {
        self.soft
    }

    /// Decompose into the owned parts, for callers that need to rebuild.
    pub fn into_parts(self) -> (SlotRulePart, SlotRulePart, BTreeSet<PeriodId>, bool) {
        (
            self.antecedent,
            self.consequent,
            self.excluded_periods,
            self.soft,
        )
    }
}

// The rule's half of the dense renumbering walk (see [crate::compact]). The
// slot of each part is a distinct occurrence, as in the reference walk.
impl SlotPairingRule {
    pub(crate) fn collect_ids(&self, ids: &mut BTreeSet<u64>) {
        use crate::ids::Id as _;
        ids.insert(self.antecedent.slot_id.inner());
        ids.insert(self.consequent.slot_id.inner());
        for period_id in &self.excluded_periods {
            ids.insert(period_id.inner());
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        let SlotPairingRule {
            antecedent,
            consequent,
            excluded_periods,
            soft,
        } = self;
        SlotPairingRule::new(
            SlotRulePart {
                slot_id: remap(map, antecedent.slot_id),
                should_have: antecedent.should_have,
            },
            SlotRulePart {
                slot_id: remap(map, consequent.slot_id),
                should_have: consequent.should_have,
            },
            excluded_periods
                .into_iter()
                .map(|period_id| remap(map, period_id))
                .collect(),
            soft,
        )
        .expect("An injective remap keeps the two slots distinct")
    }
}

// The container's half of the dense renumbering walk (see [crate::compact]).
impl SlotPairings {
    pub(crate) fn collect_ids(&self, ids: &mut BTreeSet<u64>) {
        use crate::ids::Id as _;
        for (rule_id, rule) in self.slot_pairing_rule_map.iter() {
            ids.insert(rule_id.inner());
            rule.collect_ids(ids);
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        SlotPairings {
            slot_pairing_rule_map: self
                .slot_pairing_rule_map
                .into_iter()
                .map(|(rule_id, rule)| (remap(map, rule_id), rule.remap_ids(map)))
                .collect(),
        }
    }
}

// The `Join` derive gives [`JoinedSlotPairingRule`] the same field visibility
// as [`SlotPairingRule`], so sealing the base made the joined view's fields
// private too. The joined view is a transient read-only borrow (it cannot be
// turned back into a `SlotPairingRule`), so exposing it does not weaken the
// seal — these accessors keep the view usable outside the module.
impl<'a> JoinedSlotPairingRule<'a> {
    /// The joined antecedent view.
    pub fn antecedent(&self) -> &JoinedSlotRulePart<'a> {
        &self.antecedent
    }
    /// The joined consequent view.
    pub fn consequent(&self) -> &JoinedSlotRulePart<'a> {
        &self.consequent
    }
}

/// Precondition errors of the forced slot-pairing ops — the carve-out subset
/// (step-3 survey Table 2). Only no-clobber and op-target existence survive;
/// `validate_slot_pairing_rule` is stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotPairingPrecheckError {
    #[error("invalid slot pairing rule id ({0:?})")]
    InvalidSlotPairingRuleId(SlotPairingRuleId),
    #[error("slot pairing rule id ({0:?}) already exists")]
    SlotPairingRuleIdAlreadyExists(SlotPairingRuleId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a slot pairing op: carve-out guards kept (returned
    /// as [SlotPairingPrecheckError]), invariant guards stripped (step-3 survey
    /// Table 1). May leave the state invalid; the caller owns checking and
    /// rollback.
    pub(crate) fn force_apply_slot_pairing(
        &mut self,
        slot_pairing_op: &AnnotatedSlotPairingOp,
    ) -> Result<AnnotatedSlotPairingOp, SlotPairingPrecheckError> {
        let backward = match slot_pairing_op {
            AnnotatedSlotPairingOp::Add(new_id, rule) => {
                if self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .contains(new_id)
                {
                    return Err(SlotPairingPrecheckError::SlotPairingRuleIdAlreadyExists(
                        *new_id,
                    ));
                }

                // stripped: validate_slot_pairing_rule

                self.inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .insert(*new_id, rule.clone());

                AnnotatedSlotPairingOp::Remove(*new_id)
            }
            AnnotatedSlotPairingOp::Remove(id) => {
                let Some(old_rule) = self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .remove(id)
                else {
                    return Err(SlotPairingPrecheckError::InvalidSlotPairingRuleId(*id));
                };

                AnnotatedSlotPairingOp::Add(*id, old_rule)
            }
            AnnotatedSlotPairingOp::Update(id, new_rule) => {
                // stripped: validate_slot_pairing_rule

                let Some(rule) = self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .get_mut(id)
                else {
                    return Err(SlotPairingPrecheckError::InvalidSlotPairingRuleId(*id));
                };

                let old_rule = std::mem::replace(rule, new_rule.clone());

                AnnotatedSlotPairingOp::Update(*id, old_rule)
            }
        };
        Ok(backward)
    }
}
