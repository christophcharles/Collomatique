//! Pairings submodule
//!
//! This module defines the relevant types to describe pairing rules between subjects

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Table;
use crate::ids::{PairingRuleId, PeriodId, SubjectId};
use crate::ops::AnnotatedPairingOp;

/// Description of the pairing rules
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pairings {
    /// Map from pairing rule id to pairing rule
    pub pairing_rule_map: Table<PairingRuleId, PairingRule>,
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

/// Errors for pairing rule operations
///
/// These errors can be returned when trying to modify [crate::Data] with a pairing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PairingError {
    /// A pairing rule id is invalid
    #[error("invalid pairing rule id ({0:?})")]
    InvalidPairingRuleId(PairingRuleId),

    /// The pairing rule id already exists
    #[error("pairing rule id ({0:?}) already exists")]
    PairingRuleIdAlreadyExists(PairingRuleId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Antecedent and consequent subjects are the same
    #[error("antecedent and consequent subjects are the same ({0:?})")]
    SameSubjectInBothParts(SubjectId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply pairing rule operations
    pub(crate) fn apply_pairing(
        &mut self,
        pairing_op: &AnnotatedPairingOp,
    ) -> std::result::Result<AnnotatedPairingOp, PairingError> {
        match pairing_op {
            AnnotatedPairingOp::Add(new_id, rule) => {
                if self
                    .inner_data
                    .params
                    .pairings
                    .pairing_rule_map
                    .contains_key(new_id)
                {
                    return Err(PairingError::PairingRuleIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_pairing_rule(rule)?;

                self.inner_data
                    .params
                    .pairings
                    .pairing_rule_map
                    .insert(*new_id, rule.clone());

                Ok(AnnotatedPairingOp::Remove(*new_id))
            }
            AnnotatedPairingOp::Remove(id) => {
                let Some(old_rule) = self.inner_data.params.pairings.pairing_rule_map.remove(id)
                else {
                    return Err(PairingError::InvalidPairingRuleId(*id));
                };

                Ok(AnnotatedPairingOp::Add(*id, old_rule))
            }
            AnnotatedPairingOp::Update(id, new_rule) => {
                self.inner_data.params.validate_pairing_rule(new_rule)?;

                let Some(rule) = self.inner_data.params.pairings.pairing_rule_map.get_mut(id)
                else {
                    return Err(PairingError::InvalidPairingRuleId(*id));
                };

                let old_rule = std::mem::replace(rule, new_rule.clone());

                Ok(AnnotatedPairingOp::Update(*id, old_rule))
            }
        }
    }
}
