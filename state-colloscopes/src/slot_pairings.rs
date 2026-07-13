//! Slot pairings submodule
//!
//! This module defines the relevant types to describe pairing rules between slots
//! within the same subject

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{PeriodId, SlotId, SlotPairingRuleId};

/// Description of the slot pairing rules
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotPairings {
    /// Map from slot pairing rule id to slot pairing rule
    pub slot_pairing_rule_map: BTreeMap<SlotPairingRuleId, SlotPairingRule>,
}

/// One part (antecedent or consequent) of a slot pairing rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotRulePart {
    /// The slot this part refers to
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotPairingRule {
    /// The antecedent of the implication
    pub antecedent: SlotRulePart,
    /// The consequent of the implication
    pub consequent: SlotRulePart,
    /// Periods where the rule does NOT apply
    pub excluded_periods: BTreeSet<PeriodId>,
    /// Whether this is a soft constraint (best-effort) or hard (strict)
    pub soft: bool,
}

/// Errors for slot pairing rule operations
///
/// These errors can be returned when trying to modify [crate::Data] with a slot pairing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotPairingError {
    #[error("invalid slot pairing rule id ({0:?})")]
    InvalidSlotPairingRuleId(SlotPairingRuleId),
    #[error("slot pairing rule id ({0:?}) already exists")]
    SlotPairingRuleIdAlreadyExists(SlotPairingRuleId),
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),
    #[error("same slot in both parts ({0:?})")]
    SameSlotInBothParts(SlotId),
    #[error("slots {0:?} and {1:?} do not belong to the same subject")]
    SlotsNotInSameSubject(SlotId, SlotId),
}
