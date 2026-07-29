//! Pairings submodule
//!
//! This module defines the relevant types to describe pairing rules between subjects

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{ContentOrd, Join, References};

use crate::Table;
use crate::ids::{NewId, PairingRuleId, PeriodId, SubjectId};
use crate::ops::AnnotatedPairingOp;

/// Description of the pairing rules
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Pairings {
    /// Map from pairing rule id to pairing rule
    pub pairing_rule_map: Table<PairingRuleId, PairingRule>,
}

/// One part (antecedent or consequent) of a pairing rule
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
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
///
/// Sealed: the fields are private and every value is built through
/// [`PairingRule::new`], which enforces the one value-internal invariant (the
/// antecedent and consequent must name different subjects — an implication from
/// a subject to itself is meaningless). State-dependent facts (subject and
/// period existence) stay with the checker/walker as dangling FKs. Serialized
/// exactly like the raw four-field struct via [`RawPairingRule`]; deserializing
/// a rule with both parts on one subject is a hard error (the
/// [`crate::non_empty_range::NonEmptyRangeInclusive`] precedent).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
#[join(error = NewId)]
#[serde(try_from = "RawPairingRule", into = "RawPairingRule")]
pub struct PairingRule {
    /// The antecedent of the implication
    #[fk]
    antecedent: RulePart,
    /// The consequent of the implication
    #[fk]
    consequent: RulePart,
    /// Periods where the rule does NOT apply
    #[fk]
    excluded_periods: BTreeSet<PeriodId>,
    /// Whether this is a soft constraint (best-effort) or hard (strict)
    soft: bool,
}

/// Private serde mirror of [`PairingRule`]: the transparent four-field struct.
/// Deserialization funnels through [`PairingRule::new`] (honest decode);
/// serialization is the plain field dump, so the wire format is byte-identical
/// to the pre-sealing struct.
#[derive(Serialize, Deserialize)]
struct RawPairingRule {
    antecedent: RulePart,
    consequent: RulePart,
    excluded_periods: BTreeSet<PeriodId>,
    soft: bool,
}

impl From<PairingRule> for RawPairingRule {
    fn from(rule: PairingRule) -> Self {
        RawPairingRule {
            antecedent: rule.antecedent,
            consequent: rule.consequent,
            excluded_periods: rule.excluded_periods,
            soft: rule.soft,
        }
    }
}

impl TryFrom<RawPairingRule> for PairingRule {
    type Error = PairingRuleBuildError;
    fn try_from(raw: RawPairingRule) -> Result<Self, PairingRuleBuildError> {
        PairingRule::new(
            raw.antecedent,
            raw.consequent,
            raw.excluded_periods,
            raw.soft,
        )
    }
}

/// Value-internal build failure of [`PairingRule::new`]: a self-contradictory
/// rule, independent of any state.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PairingRuleBuildError {
    /// Antecedent and consequent name the same subject
    #[error("antecedent and consequent subjects are the same ({0:?})")]
    SameSubjectInBothParts(SubjectId),
}

impl PairingRule {
    /// Build a validated pairing rule.
    ///
    /// Fails if the antecedent and consequent name the same subject: an
    /// implication from a subject to itself is meaningless. This is the only
    /// value-internal invariant; subject/period existence is state-dependent
    /// and checked elsewhere.
    pub fn new(
        antecedent: RulePart,
        consequent: RulePart,
        excluded_periods: BTreeSet<PeriodId>,
        soft: bool,
    ) -> Result<Self, PairingRuleBuildError> {
        if antecedent.subject_id == consequent.subject_id {
            return Err(PairingRuleBuildError::SameSubjectInBothParts(
                antecedent.subject_id,
            ));
        }
        Ok(PairingRule {
            antecedent,
            consequent,
            excluded_periods,
            soft,
        })
    }

    /// The antecedent of the implication
    pub fn antecedent(&self) -> &RulePart {
        &self.antecedent
    }
    /// The consequent of the implication
    pub fn consequent(&self) -> &RulePart {
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
    pub fn into_parts(self) -> (RulePart, RulePart, BTreeSet<PeriodId>, bool) {
        (
            self.antecedent,
            self.consequent,
            self.excluded_periods,
            self.soft,
        )
    }
}

// The `Join` derive gives [`JoinedPairingRule`] the same field visibility as
// [`PairingRule`], so sealing the base made the joined view's fields private
// too. The joined view is a transient read-only borrow (it cannot be turned
// back into a `PairingRule`), so exposing it does not weaken the seal — these
// accessors keep the view usable outside the module.
impl<'a> JoinedPairingRule<'a> {
    /// The joined antecedent view.
    pub fn antecedent(&self) -> &JoinedRulePart<'a> {
        &self.antecedent
    }
    /// The joined consequent view.
    pub fn consequent(&self) -> &JoinedRulePart<'a> {
        &self.consequent
    }
}

/// Precondition errors of the forced pairing ops — the carve-out subset
/// (step-3 survey Table 2). Only no-clobber and op-target existence survive;
/// `validate_pairing_rule` is stripped.
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
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a pairing op: carve-out guards kept (returned as
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
