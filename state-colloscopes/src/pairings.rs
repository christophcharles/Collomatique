//! Pairings submodule
//!
//! This module defines the relevant types to describe pairing rules between subjects

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{Join, References};

use crate::Table;
use crate::ids::{NewId, PairingRuleId, PeriodId, SubjectId};
use crate::ops::AnnotatedPairingOp;

/// Description of the pairing rules
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pairings {
    /// Map from pairing rule id to pairing rule
    pub pairing_rule_map: Table<PairingRuleId, PairingRule>,
}

/// One part (antecedent or consequent) of a pairing rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join)]
#[join(error = NewId)]
pub struct RulePart {
    /// The subject this part refers to
    #[fk(name = subject)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join)]
#[join(error = NewId)]
pub struct PairingRule {
    /// The antecedent of the implication
    #[fk]
    pub antecedent: RulePart,
    /// The consequent of the implication
    #[fk]
    pub consequent: RulePart,
    /// Periods where the rule does NOT apply
    #[fk]
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

/// Precondition errors of the forced pairing ops — the carve-out subset
/// (step-3 survey Table 2). Only no-clobber and op-target existence survive;
/// `validate_pairing_rule` is stripped. Variants copied verbatim from
/// [PairingError].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PairingPrecheckError {
    /// A pairing rule id is invalid
    #[error("invalid pairing rule id ({0:?})")]
    InvalidPairingRuleId(PairingRuleId),

    /// The pairing rule id already exists
    #[error("pairing rule id ({0:?}) already exists")]
    PairingRuleIdAlreadyExists(PairingRuleId),
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
                    .contains(new_id)
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

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_pairing]: carve-out guards kept (returned as
    /// [PairingPrecheckError]), invariant guards stripped (step-3 survey Table 1).
    /// May leave the state invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_pairing(
        &mut self,
        pairing_op: &AnnotatedPairingOp,
    ) -> std::result::Result<AnnotatedPairingOp, PairingPrecheckError> {
        match pairing_op {
            AnnotatedPairingOp::Add(new_id, rule) => {
                if self
                    .inner_data
                    .params
                    .pairings
                    .pairing_rule_map
                    .contains(new_id)
                {
                    return Err(PairingPrecheckError::PairingRuleIdAlreadyExists(*new_id));
                }
                // stripped: validate_pairing_rule

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
                    return Err(PairingPrecheckError::InvalidPairingRuleId(*id));
                };

                Ok(AnnotatedPairingOp::Add(*id, old_rule))
            }
            AnnotatedPairingOp::Update(id, new_rule) => {
                // stripped: validate_pairing_rule

                let Some(rule) = self.inner_data.params.pairings.pairing_rule_map.get_mut(id)
                else {
                    return Err(PairingPrecheckError::InvalidPairingRuleId(*id));
                };

                let old_rule = std::mem::replace(rule, new_rule.clone());

                Ok(AnnotatedPairingOp::Update(*id, old_rule))
            }
        }
    }
}
