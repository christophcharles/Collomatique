//! Regression test for the rotation fast-track.
//!
//! Both rotations are always active: the option only chooses between a strict
//! constraint and an optimisation goal. But a subject taught by a single
//! teacher has nothing to rotate, and neither has a subject with a single slot.
//! Emitting rotation rows there is pure waste: the soft equalities are
//! identities (`Aₜ == A` and `Uₜ == U`, so `A·Uₜ − Aₜ·U == 0` reads `0 == 0`),
//! and the hard windows only restate the subject's periodicity bound. So the
//! two builders skip such subjects outright.
//!
//! The fixture is a minimal self-contained document with two interrogated
//! subjects, a single teacher teaching both, and a single slot per subject.
//! The global options make both rotations strict; the sole per-subject override
//! makes both soft for subject 1. That way the fast-track is exercised on the
//! hard path (subject 2) and on the soft path (subject 1) at once — before the
//! skip existed, subject 2 emitted hard `Balancing(Slot)Rotation` constraints
//! and subject 1 emitted objectified regularity rows under a
//! `Balancing(Slot)RotationPenalty` extra.

use collomatique_constraints_colloscopes::{
    ConstraintDesc, ConstraintSource, ExtraVarName, PreferenceConstraint, build_model,
};
use collomatique_storage::deserialize_data;

const FIXTURE: &str = include_str!("fixtures/single_teacher_single_slot.collomatique");

#[test]
fn single_teacher_single_slot_subjects_skip_rotation_entirely() {
    let (inner, _caveats) = deserialize_data(FIXTURE).expect("fixture should decode");
    let model = build_model(&inner.params);

    for (_constraint, source) in model.problem().get_constraints() {
        match source {
            // The hard path attributes its rows to the constraint description.
            ConstraintSource::User(desc) => {
                let is_rotation_row = matches!(
                    desc,
                    ConstraintDesc::Level4(
                        PreferenceConstraint::BalancingRotation { .. }
                            | PreferenceConstraint::BalancingRotationRegularity { .. }
                            | PreferenceConstraint::BalancingSlotRotation { .. }
                            | PreferenceConstraint::BalancingSlotRotationRegularity { .. }
                    )
                );
                assert!(
                    !is_rotation_row,
                    "a subject with a single teacher and a single slot must emit no rotation \
                     constraints, found {desc:?}"
                );
            }
            // The soft path objectifies its rows, so they surface as the
            // penalty variable's definition rather than as `User(..)`.
            ConstraintSource::DefiningExtra { extra, .. } => {
                let is_rotation_penalty = matches!(
                    extra,
                    ExtraVarName::BalancingRotationPenalty { .. }
                        | ExtraVarName::BalancingSlotRotationPenalty { .. }
                );
                assert!(
                    !is_rotation_penalty,
                    "a subject with a single teacher and a single slot must emit no rotation \
                     penalty rows, found {extra:?}"
                );
            }
        }
    }
}
