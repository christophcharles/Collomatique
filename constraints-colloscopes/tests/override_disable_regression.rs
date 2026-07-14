//! Regression test for the settings/balancing "disable" bug.
//!
//! A per-student `Settings::students` override (like a per-subject
//! `Balancing::subjects` override) is meant to be **whole-entry**: if a student
//! has an override entry, it wins verbatim — `None` included — so a student can
//! *disable* a globally-enabled limit. The plain-`bool` balancing path already
//! does this (`effective_balancing_flag`). But the `Option<SoftParam<_>>` limits
//! helpers do per-field `get(id).and_then(extract).or(global)`, and because
//! `and_then` *flattens*, an override entry whose field is `None` silently falls
//! through to the global value — so the student **cannot** disable it.
//!
//! The fixture is a minimal self-contained document: one period with one
//! interrogation week, one duration-counting interrogated subject with a
//! null-week-pattern slot, and two students both enrolled. The global
//! `interrogations_per_week_max` is on, and exactly one student (the sole entry
//! in `settings.students`) carries an override whose weekly-max field is `None`.
//! That overridden student must have **no** `MaxInterrogationsPerWeek`
//! constraint while the other (non-overridden) student keeps theirs.
//!
//! FAILS today: the per-field `and_then(..).or(global)` fallback re-applies the
//! global weekly max to the overridden student, so the constraint is still
//! emitted.

use collomatique_constraints_colloscopes::{
    ConstraintDesc, ConstraintSource, PreferenceConstraint, build_model,
};
use collomatique_state_colloscopes::ids::{StudentId, SubjectId};
use collomatique_storage::deserialize_data;
use std::collections::BTreeSet;

const FIXTURE: &str = include_str!("fixtures/override_disable.collomatique");
const BALANCING_FIXTURE: &str = include_str!("fixtures/override_disable_balancing.collomatique");

#[test]
fn per_student_override_can_disable_global_weekly_max() {
    let (data, _caveats) = deserialize_data(FIXTURE).expect("fixture should decode");
    let params = &data.get_inner_data().params;

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

#[test]
fn per_subject_override_can_disable_global_teacher_rotation() {
    let (data, _caveats) = deserialize_data(BALANCING_FIXTURE).expect("fixture should decode");
    let params = &data.get_inner_data().params;

    // The fixture pins exactly one overridden subject (the disable target); the
    // other interrogated subject is the control that must stay constrained.
    let overridden: Vec<SubjectId> = params.balancing.subjects.keys().collect();
    assert_eq!(
        overridden.len(),
        1,
        "fixture invariant: exactly one per-subject balancing override entry"
    );
    let target = overridden[0];

    let model = build_model(params);
    let mut constrained = BTreeSet::new();
    for (_constraint, source) in model.problem().get_constraints() {
        if let ConstraintSource::User(ConstraintDesc::Level4(
            PreferenceConstraint::BalancingRotation { subject, .. },
        )) = source
        {
            constrained.insert(*subject);
        }
    }

    assert!(
        constrained.iter().any(|s| *s != target),
        "the global teacher rotation must still constrain the non-overridden subject"
    );
    assert!(
        !constrained.contains(&target),
        "a per-subject override entry with `teacher_rotation: None` must DISABLE the \
         global teacher rotation for {target:?}, but a BalancingRotation constraint is \
         still emitted (per-field `and_then(..).or(global)` fallback bug)"
    );
}
