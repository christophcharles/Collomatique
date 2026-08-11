//! Regression tests for the whole-entry override rule.
//!
//! A per-student `Settings::students` override (like a per-subject
//! `Balancing::subjects` override) is meant to be **whole-entry**: if a student
//! has an override entry, it wins verbatim — `None` included — so a student can
//! *disable* a globally-enabled limit.
//!
//! The limits helpers used to get this wrong. They fell back per field, with
//! `get(id).and_then(extract).or(global)`, and `and_then` *flattens*: "this
//! student has no override entry" and "the entry's field is `None`" collapse
//! into the same `None`, so `.or(global)` re-applied the global limit in both
//! cases and the student could never disable anything. The three helpers now go
//! through the whole-entry `Settings::limits_for` accessor instead, which is
//! what this test guards against regressing.
//!
//! The limits fixture is a minimal self-contained document: one period with one
//! interrogation week, one duration-counting interrogated subject with a
//! null-week-pattern slot, and two students both enrolled. The global
//! `interrogations_per_week_max` is on, and exactly one student (the sole entry
//! in `settings.students`) carries an override whose weekly-max field is `None`.
//! That overridden student must have **no** `MaxInterrogationsPerWeek`
//! constraint while the other (non-overridden) student keeps theirs.
//!
//! The balancing half pins the same whole-entry rule on the rotation side.
//! `teacher_rotation` is an `Option<SoftParam<()>>`, so an override entry can
//! both *soften* the goal (`Some { soft: true }` against a globally-hard one)
//! and *disable* it outright (`None`). Two fixtures, identical but for that one
//! field, pin the two cases. Each has two interrogated subjects taught by two
//! teachers each, globally-hard rotation, and a single per-subject override:
//!
//! - `override_disable_balancing.collomatique` softens the target subject: it
//!   must emit the soft penalty rows and no hard `BalancingRotation`
//!   constraint.
//! - `override_disable_balancing_off.collomatique` turns it off: the target
//!   subject must emit *neither* — no hard constraint and no penalty row, since
//!   an off goal is not pursued at all.
//!
//! In both, the control subject keeps its hard constraints.

use collomatique_constraints_colloscopes::{
    ConstraintDesc, ConstraintSource, ExtraVarName, PreferenceConstraint, build_model,
};
use collomatique_state_colloscopes::ids::{StudentId, SubjectId};
use collomatique_storage::deserialize_data;
use std::collections::BTreeSet;

const FIXTURE: &str = include_str!("fixtures/override_disable.collomatique");
const BALANCING_FIXTURE: &str = include_str!("fixtures/override_disable_balancing.collomatique");
const BALANCING_OFF_FIXTURE: &str =
    include_str!("fixtures/override_disable_balancing_off.collomatique");

#[test]
fn per_student_override_can_disable_global_weekly_max() {
    let (inner, _caveats) = deserialize_data(FIXTURE).expect("fixture should decode");
    let params = &inner.params;

    // The fixture pins exactly one overridden student (the disable target); the
    // other enrolled student is the control that must stay constrained.
    let overridden: Vec<StudentId> = params.settings.students.keys().collect();
    assert_eq!(
        overridden.len(),
        1,
        "fixture invariant: exactly one per-student override entry"
    );
    let target = overridden[0];

    let model = build_model(params);
    let mut constrained = BTreeSet::new();
    for (_constraint, source) in model.problem().get_constraints() {
        if let ConstraintSource::User(ConstraintDesc::Level4(
            PreferenceConstraint::MaxInterrogationsPerWeek { student, .. },
        )) = source
        {
            constrained.insert(*student);
        }
    }

    assert!(
        constrained.iter().any(|s| *s != target),
        "the global weekly max must still constrain the non-overridden student"
    );
    assert!(
        !constrained.contains(&target),
        "a per-student override entry with `interrogations_per_week_max: None` must \
         DISABLE the global weekly max for {target:?}, but a MaxInterrogationsPerWeek \
         constraint is still emitted (per-field `and_then(..).or(global)` fallback bug)"
    );
}

/// The teacher-rotation footprint of a balancing fixture: the overridden
/// subject, the subjects carrying hard rotation constraints, and the subjects
/// carrying soft rotation penalty rows.
fn rotation_footprint(fixture: &str) -> (SubjectId, BTreeSet<SubjectId>, BTreeSet<SubjectId>) {
    let (inner, _caveats) = deserialize_data(fixture).expect("fixture should decode");
    let params = &inner.params;

    // The fixture pins exactly one overridden subject (the target); the other
    // interrogated subject is the control that must stay hard-constrained.
    let overridden: Vec<SubjectId> = params.balancing.subjects.keys().collect();
    assert_eq!(
        overridden.len(),
        1,
        "fixture invariant: exactly one per-subject balancing override entry"
    );

    let model = build_model(params);
    let mut hard_constrained = BTreeSet::new();
    let mut softened = BTreeSet::new();
    for (_constraint, source) in model.problem().get_constraints() {
        match source {
            ConstraintSource::User(ConstraintDesc::Level4(
                PreferenceConstraint::BalancingRotation { subject, .. },
            )) => {
                hard_constrained.insert(*subject);
            }
            // The soft path objectifies its rows, so they surface as the
            // penalty variable's definition rather than as `User(..)`.
            ConstraintSource::DefiningExtra {
                extra: ExtraVarName::BalancingRotationPenalty { subject, .. },
                ..
            } => {
                softened.insert(*subject);
            }
            _ => {}
        }
    }

    (overridden[0], hard_constrained, softened)
}

#[test]
fn per_subject_override_can_soften_global_teacher_rotation() {
    let (target, hard_constrained, softened) = rotation_footprint(BALANCING_FIXTURE);

    assert!(
        hard_constrained.iter().any(|s| *s != target),
        "the globally-hard teacher rotation must still constrain the non-overridden subject"
    );
    assert!(
        !hard_constrained.contains(&target),
        "a per-subject override entry with a soft `teacher_rotation` must SOFTEN the \
         globally-hard teacher rotation for {target:?}, but a hard BalancingRotation \
         constraint is still emitted"
    );
    assert!(
        softened.contains(&target),
        "the softened rotation must show up as BalancingRotationPenalty rows for \
         {target:?} — otherwise the override dropped it instead of softening it"
    );
}

#[test]
fn per_subject_override_can_disable_global_teacher_rotation() {
    let (target, hard_constrained, softened) = rotation_footprint(BALANCING_OFF_FIXTURE);

    assert!(
        hard_constrained.iter().any(|s| *s != target),
        "the globally-hard teacher rotation must still constrain the non-overridden subject"
    );
    assert!(
        !hard_constrained.contains(&target),
        "a per-subject override entry with `teacher_rotation: null` must DISABLE the \
         globally-hard teacher rotation for {target:?}, but a hard BalancingRotation \
         constraint is still emitted"
    );
    assert!(
        !softened.contains(&target),
        "an off rotation must not be pursued at all, yet BalancingRotationPenalty rows \
         are emitted for {target:?} — the override softened it instead of dropping it"
    );
}
