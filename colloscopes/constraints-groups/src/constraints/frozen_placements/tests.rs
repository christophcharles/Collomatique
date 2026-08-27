use crate::constraints::tests::{assert_close, place, solve_with_objective_pinned, value};
use crate::extras::base_var;
use crate::frozen::FrozenPlacements;
use crate::specs::tests::student;
use crate::vars::tests::plan_of;
use crate::vars::{GroupListIdx, Var};
use std::collections::BTreeMap;

#[test]
fn a_pin_beats_the_objective() {
    // 4 students, sizes 1..=2 → 2 groups. Weight 100 sends student 1 to
    // group 0 — the same push the neighbouring families make lose against a
    // constraint. Here the pin is the constraint, so group 1 is where the
    // student must end up, and the push must buy nothing.
    let plan = plan_of(&[(&[1, 2, 3, 4], (1, 2))]);
    let list = GroupListIdx(0);
    let in_group = |s: u64, group: u32| {
        base_var(Var::StudentInGroup {
            list,
            student: student(s),
            group,
        })
    };

    let frozen = FrozenPlacements::new(BTreeMap::from([((list, student(1)), 1)]));
    let cfg = solve_with_objective_pinned(&plan, &[place(0, 1, 0)], &frozen);

    assert_close(value(&cfg, in_group(1, 1)), 1.0);
    // And the other group of the pinned student is 0, which is
    // `student_in_one_group`'s doing: the pin says nothing about it, which is
    // exactly why one row per seat is enough.
    assert_close(value(&cfg, in_group(1, 0)), 0.0);
}

#[test]
fn the_warm_start_still_solves_the_pinned_model() {
    // `convert::tests::the_warm_start_is_a_tight_solution_of_the_model`, with
    // the greedy's own frozen seats pinned. This is the feasibility argument
    // of the feature made executable: prefill placements are never revised,
    // so the greedy's final answer — which is what the warm start is read
    // off — already satisfies every pin.
    let plan = plan_of(&[
        (&[1, 2, 3, 4, 5, 6], (2, 3)),
        (&[1, 2, 3, 4], (2, 2)),
        (&[1, 2, 3, 4, 5, 6], (2, 2)),
    ]);
    let names: Vec<String> = (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect();
    let outcome = crate::greedy_group_lists(&plan, &names);
    assert!(
        !outcome.frozen.is_empty(),
        "this plan must have something to pin, or the test proves nothing",
    );

    let model = crate::build_model(&plan, crate::ObjectiveWeights::default(), &outcome.frozen);
    let warm = crate::group_lists_to_warm_start(&plan, &outcome.lists);

    let solution = model
        .solution_from_complete_data(warm)
        .expect("the warm start must value exactly the model's variables");
    assert!(
        solution.is_feasible(),
        "the warm start breaks {} constraint(s) of the pinned model",
        solution.blame().len(),
    );
}

#[test]
fn every_pinned_seat_is_one_row() {
    // The rows reach the model, one per seat and no more — the count the
    // build log reports.
    use collomatique_ilp_modeler::ConstraintSource;

    let plan = plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[5, 6, 7], (1, 2))]);
    let frozen = FrozenPlacements::new(BTreeMap::from([
        ((GroupListIdx(0), student(1)), 0),
        ((GroupListIdx(0), student(2)), 1),
        ((GroupListIdx(1), student(5)), 0),
    ]));
    let model = crate::build_model(&plan, crate::ObjectiveWeights::default(), &frozen);

    let pins: Vec<_> = model
        .problem()
        .get_constraints()
        .iter()
        .filter_map(|(_, source)| match source {
            ConstraintSource::User(crate::ConstraintDesc::FrozenPlacement {
                list,
                student,
                group,
            }) => Some((*list, *student, *group)),
            _ => None,
        })
        .collect();

    assert_eq!(
        pins,
        vec![
            (GroupListIdx(0), student(1), 0),
            (GroupListIdx(0), student(2), 1),
            (GroupListIdx(1), student(5), 0),
        ],
    );
}

#[test]
#[should_panic(expected = "is not in this plan")]
fn a_seat_outside_the_plan_is_refused() {
    // The seats are computed against the plan the naming dialog built, and
    // the loading dialog rebuilds its own: this is the backstop if the two
    // ever stop agreeing.
    let plan = plan_of(&[(&[1, 2, 3, 4], (2, 3))]);
    let frozen = FrozenPlacements::new(BTreeMap::from([((GroupListIdx(0), student(9)), 0)]));
    let _ = crate::build_model(&plan, crate::ObjectiveWeights::default(), &frozen);
}
