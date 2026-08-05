//! Piece-1 verification (roadmap §7): the solver machinery accepts a model
//! with no constraints and an empty objective, and its solution converts
//! back into structurally valid prefilled group lists.

use collomatique_constraints_groups::{
    GenerationPlan, GroupListSpec, build_group_lists, build_model,
};
use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
use collomatique_state_colloscopes::ids::Id;
use collomatique_state_colloscopes::{NonEmptyRangeInclusive, StudentId};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

fn student(n: u64) -> StudentId {
    unsafe { StudentId::new(n) }
}

fn range(min: u32, max: u32) -> NonEmptyRangeInclusive<NonZeroU32> {
    NonEmptyRangeInclusive::new(
        NonZeroU32::new(min).expect("non-zero")..=NonZeroU32::new(max).expect("non-zero"),
    )
    .expect("non-empty")
}

#[test]
fn constraint_free_model_solves_and_converts() {
    // A hand-built plan: no document needed, the plan type is the model's
    // whole input. (An empty covered set is artificial but legal here.)
    let spec = GroupListSpec {
        students: (1..=6).map(student).collect(),
        students_per_group: range(2, 3),
    };
    let plan = GenerationPlan {
        specs: vec![(spec, BTreeSet::new())],
        skipped: BTreeSet::new(),
        pinned_pairs: BTreeSet::new(),
    };

    let model = build_model(&plan);

    let solver = ColloCbcSolver::with_disable_logging(true);
    let solution = model
        .solve(&solver)
        .expect("a constraint-free model must be feasible");

    // 6 students, min size 2 → 3 slots. Any in-domain assignment is
    // acceptable; the conversion must be structurally valid.
    let config = solution.get_data();
    let lists = build_group_lists(&plan, &[String::from("Test")], &config);
    assert_eq!(lists.len(), 1);
    let (list, covered) = &lists[0];
    assert!(covered.is_empty());
    assert!(list.is_prefilled());
    assert_eq!(list.filling().iter_students().count(), 6);
    assert!(list.params().group_names.len() <= 3);
}
