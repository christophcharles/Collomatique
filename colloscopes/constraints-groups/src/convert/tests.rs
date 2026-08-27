use super::*;
use crate::ObjectiveWeights;
use crate::specs::tests::{range, student};
use crate::vars::tests::plan_of;
use collomatique_ilp::f64_equals;
use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;

/// One name per spec, in plan order — the conversions only ever copy them.
fn names(plan: &GenerationPlan) -> Vec<String> {
    (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect()
}

fn greedy(plan: &GenerationPlan) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
    crate::greedy_group_lists(plan, &names(plan)).lists
}

/// The prefilled groups of one list, as student sets.
fn groups_of(list: &GroupList) -> Vec<BTreeSet<StudentId>> {
    match list.filling() {
        GroupListFilling::Prefilled { groups } => {
            groups.iter().map(|group| group.students.clone()).collect()
        }
        GroupListFilling::Automatic { .. } => panic!("both conversions only emit prefilled lists"),
    }
}

fn value(
    config: &ConfigData<InternalVar<Var, ExtraVarName>>,
    var: InternalVar<Var, ExtraVarName>,
) -> f64 {
    config
        .get(var.clone())
        .unwrap_or_else(|| panic!("{var:?} should be part of the warm start"))
}

#[test]
fn compaction_remaps_group_indices() {
    // 6 students in groups of 1 to 2 → 3 groups. The configuration is
    // in-domain but leaves the middle group empty, which the conversion
    // must compact away rather than emit.
    let plan = plan_of(&[(&[1, 2, 3, 4, 5, 6], (1, 2))]);
    let list = GroupListIdx(0);

    let mut config = ConfigData::new();
    for (s, slot) in [(1, 0), (2, 0), (3, 2), (4, 2), (5, 2), (6, 2)] {
        for group in 0..3 {
            config = config.set(
                Var::StudentInGroup {
                    list,
                    student: student(s),
                    group,
                },
                if group == slot { 1.0 } else { 0.0 },
            );
        }
    }

    let lists = build_group_lists(&plan, &[String::from("Liste")], &config);
    assert_eq!(lists.len(), 1);
    let (group_list, _covered) = &lists[0];

    assert_eq!(group_list.params().group_names.len(), 2);
    assert_eq!(group_list.filling().find_student_group(student(1)), Some(0));
    // Slot 2 was compacted down to group 1.
    assert_eq!(group_list.filling().find_student_group(student(3)), Some(1));
}

#[test]
fn the_warm_start_carries_the_placement_back_unchanged() {
    // Three overlapping lists of two size classes, so the round trip covers
    // several group counts at once: ceil(6 / 3) = 2 groups, ceil(4 / 2) = 2
    // and ceil(6 / 2) = 3.
    let plan = plan_of(&[
        (&[1, 2, 3, 4, 5, 6], (2, 3)),
        (&[1, 2, 3, 4], (2, 2)),
        (&[1, 2, 3, 4, 5, 6], (2, 2)),
    ]);
    let lists = greedy(&plan);

    let warm = group_lists_to_warm_start(&plan, &lists);
    // What the solver path does with a solved configuration: keep the base
    // variables and drop everything else (`Model::base_data_from_complete_data`).
    let base = warm.filter_transmute(|var| match var {
        InternalVar::Base(v) => Some(v.clone()),
        _ => None,
    });
    let back = build_group_lists(&plan, &names(&plan), &base);

    assert_eq!(back.len(), lists.len());
    for (i, ((got, got_covered), (want, want_covered))) in back.iter().zip(lists.iter()).enumerate()
    {
        assert_eq!(
            groups_of(got),
            groups_of(want),
            "list {i} came back changed"
        );
        assert_eq!(got_covered, want_covered);
    }
}

#[test]
fn the_warm_start_is_a_tight_solution_of_the_model() {
    // The strong one: the configuration is handed to the *real* model, whose
    // variable set it must match exactly — `solution_from_complete_data`
    // refuses a configuration that misses a variable or carries one the model
    // does not have, which is also what would catch an extra family growing a
    // helper column behind the crate's back.
    //
    // Two size classes over nested student sets, with a template: every extra
    // family of the model is populated.
    let plan = plan_of(&[
        (&[1, 2, 3, 4, 5, 6], (2, 3)),
        (&[1, 2, 3, 4], (2, 2)),
        (&[1, 2, 3, 4, 5, 6], (2, 2)),
    ]);
    let weights = ObjectiveWeights::default();
    let model = crate::build_model(&plan, weights, &crate::FrozenPlacements::default());
    let warm = group_lists_to_warm_start(&plan, &greedy(&plan));

    let solution = model
        .solution_from_complete_data(warm.clone())
        .expect("the warm start must value exactly the model's variables");
    assert!(
        solution.is_feasible(),
        "the warm start breaks {} constraint(s)",
        solution.blame().len(),
    );

    // Feasible is not enough: every extra of this model is one-sided, so a
    // configuration setting them all to 1 would be feasible too and would
    // report a wildly inflated objective as the incumbent. The tight values
    // are the ones a minimizing solve settles on with the placement fixed,
    // which is exactly what the reconstruction problem computes.
    let solver = ColloCbcSolver::with_disable_logging(true);
    let base = model.base_data_from_complete_data(&warm);
    let reconstructed = model
        .solution_from_data(&base, &solver)
        .expect("reconstruction should succeed");
    assert!(
        f64_equals(solution.eval(), reconstructed.eval()),
        "warm start evaluates to {}, the true cost of that placement is {}",
        solution.eval(),
        reconstructed.eval(),
    );
}

#[test]
fn a_pinned_pair_is_valued_by_the_pin() {
    // A pair pinned by a kept list has no defining row — its variable is
    // fixed to 1 — so reading it off the placement alone would value it 0
    // whenever the greedy separates the pair, and the model would refuse the
    // whole warm start.
    let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
    plan.pinned_pairs = [(
        range(2, 2),
        [(student(1), student(3))].into_iter().collect(),
    )]
    .into_iter()
    .collect();

    let lists = greedy(&plan);
    let seat = |s: u64| lists[0].0.filling().find_student_group(student(s));
    assert_ne!(
        seat(1),
        seat(3),
        "the pin only means something on a pair the greedy separates",
    );

    let warm = group_lists_to_warm_start(&plan, &lists);
    let shared = |a: u64, b: u64| {
        InternalVar::Extra(ExtraVarName::SharedPair {
            a: student(a),
            b: student(b),
            class: crate::vars::SizeClassIdx(0),
        })
    };
    assert_eq!(value(&warm, shared(1, 3)), 1.0);
    // A pair of the same class that the placement did not group either stays
    // at 0: the pin is a property of the pair, not of the family.
    let free = if seat(1) == seat(2) { (1, 4) } else { (1, 2) };
    assert_eq!(value(&warm, shared(free.0, free.1)), 0.0);

    let model = crate::build_model(
        &plan,
        ObjectiveWeights::default(),
        &crate::FrozenPlacements::default(),
    );
    let solution = model
        .solution_from_complete_data(warm)
        .expect("the warm start must value exactly the model's variables");
    assert!(
        solution.is_feasible(),
        "the warm start breaks {} constraint(s)",
        solution.blame().len(),
    );
}
