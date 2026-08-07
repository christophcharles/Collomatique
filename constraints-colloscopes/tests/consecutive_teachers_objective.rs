//! The "avoid the same teacher twice in a row" objective must be emitted
//! exactly for interrogated subjects with at least two distinct teachers whose
//! `avoid_twice_in_a_row` goal is on *and* soft. It must stay silent for
//! single-teacher subjects, for subjects carrying the goal as a strict
//! constraint (the hard rule subsumes it), and for subjects where the goal is
//! off — off means the goal is not pursued at all, neither as a constraint nor
//! as an objective term.
//!
//! The objective's rows are *objectified*: `objectify_weighted_sum` consumes
//! each constraint's [`ConstraintDesc`] to compute its weight and folds the row
//! into the penalty variable's definition, so the rows surface as
//! `ConstraintSource::DefiningExtra { extra: AvoidTwiceInARowPenalty { .. } }`,
//! not as `ConstraintSource::User(..)`. The penalty variable is keyed by
//! subject, which is the per-subject signal these tests scan for.

use collomatique_constraints_colloscopes::{
    ColloscopeModel, ConstraintDesc, ConstraintSource, ExtraVarName, PreferenceConstraint,
    build_model,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::ids::{SubjectId, TeacherId};
use collomatique_state_colloscopes::soft_param::SoftParam;
use collomatique_storage::deserialize_data;
use std::collections::BTreeSet;

/// Copy of `examples/hogwarts.collomatique` frozen at the time this test was
/// written: it pins the periodic subjects, their teachers and the one subject
/// carrying the hard rule. Own copy on purpose — the example is free to evolve,
/// this test is about the objective.
const WINDOW_FIXTURE: &str = include_str!("fixtures/consecutive_teachers_windows.collomatique");
const RECURSIVE_FIXTURE: &str =
    include_str!("fixtures/consecutive_teachers_recursive.collomatique");
/// The window fixture with the *global* `avoid_twice_in_a_row` turned off
/// (`null`), everything else untouched — so the multi-teacher subjects without
/// an override entry no longer pursue the goal at all, while the subject whose
/// override carries the hard rule keeps it.
const OFF_FIXTURE: &str = include_str!("fixtures/consecutive_teachers_off.collomatique");

/// Distinct teachers per interrogated subject, mirroring
/// `balancing::helpers::teachers_for_subject`, keeping only subjects with at
/// least two.
fn multi_teacher_subjects(params: &Parameters) -> BTreeSet<SubjectId> {
    let mut result = BTreeSet::new();
    for (subject_id, subject) in params.subjects.ordered_subject_list.iter() {
        if subject.parameters.interrogation_parameters.is_none() {
            continue;
        }
        let Some(subject_slots) = params.slots.slots_for_subject(subject_id) else {
            continue;
        };
        let teachers: BTreeSet<TeacherId> = subject_slots
            .map(|(_, slot_data)| slot_data.teacher_id)
            .collect();
        if teachers.len() >= 2 {
            result.insert(subject_id);
        }
    }
    result
}

fn is_strict(goal: &Option<SoftParam<()>>) -> bool {
    matches!(goal, Some(param) if !param.soft)
}

/// Subjects carrying at least one `AvoidTwiceInARowPenalty` variable.
fn subjects_with_penalty(model: &ColloscopeModel) -> BTreeSet<SubjectId> {
    let mut emitted = BTreeSet::new();
    for (_constraint, source) in model.problem().get_constraints() {
        if let ConstraintSource::DefiningExtra {
            extra: ExtraVarName::AvoidTwiceInARowPenalty { subject, .. },
            ..
        } = source
        {
            emitted.insert(*subject);
        }
    }
    emitted
}

/// Window form: every interrogated subject of the fixture is either
/// `ExactlyPeriodic` or `OnceForEveryBlockOfWeeks`. The fixture also carries a
/// multi-teacher subject whose *hard* `avoid_twice_in_a_row` flag is on — the
/// hard rule subsumes the objective, so that subject must get no penalty.
#[test]
fn window_objective_emitted_iff_subject_has_several_teachers() {
    let (inner, _caveats) = deserialize_data(WINDOW_FIXTURE).expect("fixture should decode");
    let params = &inner.params;

    let multi_teacher = multi_teacher_subjects(params);
    let hard: BTreeSet<SubjectId> = multi_teacher
        .iter()
        .copied()
        .filter(|s| is_strict(&params.balancing.options_for(*s).avoid_twice_in_a_row))
        .collect();
    assert!(
        !hard.is_empty(),
        "fixture invariant: a multi-teacher subject must carry the hard rule"
    );
    let expected: BTreeSet<SubjectId> = multi_teacher.difference(&hard).copied().collect();
    assert!(
        !expected.is_empty(),
        "fixture invariant: some multi-teacher subjects must be without the hard rule"
    );

    let model = build_model(params);
    let emitted = subjects_with_penalty(&model);

    assert_eq!(
        emitted, expected,
        "the soft avoid-twice objective must cover exactly the multi-teacher subjects that \
         do not already carry the hard rule"
    );
}

/// Recursive form: both subjects of the fixture are `AmountInYear`, so the
/// `IsLastTeacherSeen` chain is what produces the penalty here.
#[test]
fn recursive_objective_emitted_for_multi_teacher_amount_in_year_subject() {
    let (inner, _caveats) = deserialize_data(RECURSIVE_FIXTURE).expect("fixture should decode");
    let params = &inner.params;

    // Fixture invariant: subject "A" has two teachers, subject "B" one.
    let expected = multi_teacher_subjects(params);
    assert_eq!(
        expected.len(),
        1,
        "fixture invariant: exactly one multi-teacher subject"
    );

    let model = build_model(params);
    let emitted = subjects_with_penalty(&model);

    assert_eq!(
        emitted, expected,
        "the recursive soft objective must cover exactly the two-teacher subject"
    );
}

/// Subjects carrying at least one hard avoid-twice constraint, in either of its
/// two forms (window and recursive).
fn subjects_with_hard_rule(model: &ColloscopeModel) -> BTreeSet<SubjectId> {
    let mut emitted = BTreeSet::new();
    for (_constraint, source) in model.problem().get_constraints() {
        let ConstraintSource::User(ConstraintDesc::Level4(preference)) = source else {
            continue;
        };
        match preference {
            PreferenceConstraint::BalancingAvoidTwiceInARow { subject, .. }
            | PreferenceConstraint::BalancingAvoidTwiceInARowRecursive { subject, .. } => {
                emitted.insert(*subject);
            }
            _ => {}
        }
    }
    emitted
}

/// An off (`None`) goal builds *nothing*: no hard constraint, and — the point
/// of the three-state option — no objective term either. The subject carrying
/// the hard rule through its own override is the control.
#[test]
fn off_goal_builds_neither_constraints_nor_objective() {
    let (inner, _caveats) = deserialize_data(OFF_FIXTURE).expect("fixture should decode");
    let params = &inner.params;

    let multi_teacher = multi_teacher_subjects(params);
    let hard: BTreeSet<SubjectId> = multi_teacher
        .iter()
        .copied()
        .filter(|s| is_strict(&params.balancing.options_for(*s).avoid_twice_in_a_row))
        .collect();
    let off: BTreeSet<SubjectId> = multi_teacher
        .iter()
        .copied()
        .filter(|s| {
            params
                .balancing
                .options_for(*s)
                .avoid_twice_in_a_row
                .is_none()
        })
        .collect();
    assert!(
        !hard.is_empty(),
        "fixture invariant: a multi-teacher subject must carry the hard rule"
    );
    assert!(
        !off.is_empty(),
        "fixture invariant: some multi-teacher subjects must have the goal off"
    );

    let model = build_model(params);
    let penalised = subjects_with_penalty(&model);
    let hard_constrained = subjects_with_hard_rule(&model);

    assert_eq!(
        penalised,
        BTreeSet::new(),
        "an off avoid-twice goal must emit no objective term at all, yet penalties \
         showed up for {penalised:?}"
    );
    assert!(
        off.is_disjoint(&hard_constrained),
        "an off avoid-twice goal must emit no hard constraint either, yet some of \
         {off:?} are constrained"
    );
    assert_eq!(
        hard_constrained, hard,
        "the subject whose override keeps the strict rule must stay constrained"
    );
}
