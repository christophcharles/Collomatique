//! Pairings submodule
//!
//! This module defines the relevant types to describe pairing rules between subjects

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ids::{PairingRuleId, PeriodId, SubjectId};

/// Description of the pairing rules
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pairings {
    /// Map from pairing rule id to pairing rule
    pub pairing_rule_map: BTreeMap<PairingRuleId, PairingRule>,
}

/// One part (antecedent or consequent) of a pairing rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePart {
    /// The subject this part refers to
    pub subject_id: SubjectId,
    /// Whether the student should have an interrogation in the subject (true)
    /// or should not have one (false)
    pub should_have: bool,
}

/// A pairing rule between two subjects
///
/// Represents an implication: if the antecedent condition holds for a student
/// in a given week, then the consequent condition must also hold.
///
/// For example, "Having Math => Having Chemistry" means that if a student
/// has a math interrogation in a given week, they must also have a chemistry
/// interrogation that week.
///
/// Rules only apply to students enrolled in both subjects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRule {
    /// The antecedent of the implication
    pub antecedent: RulePart,
    /// The consequent of the implication
    pub consequent: RulePart,
    /// Periods where the rule does NOT apply
    pub excluded_periods: BTreeSet<PeriodId>,
    /// Whether this is a soft constraint (best-effort) or hard (strict)
    pub soft: bool,
}
