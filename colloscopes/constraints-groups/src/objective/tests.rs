use super::*;
use crate::frozen::FrozenPlacements;
use crate::specs::GenerationPlan;
use crate::specs::tests::student;
use crate::vars::tests::plan_with_uses;
use crate::vars::{GroupListIdx, Var, VarEnv};
use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
use collomatique_ilp::{ConfigData, f64_equals};
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::{PeriodId, SubjectId};
use std::collections::BTreeSet;

/// One name per spec, in plan order — the conversions only ever copy them.
fn names(plan: &GenerationPlan) -> Vec<String> {
    (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect()
}

/// The group lists of a hand-built placement: `config[list][group]` lists the
/// students of that group, at the model's own group indices. Every seat is
/// spelled out, so the placement goes in through the same conversion a solved
/// configuration does.
fn lists_of(
    plan: &GenerationPlan,
    config: &[&[&[u64]]],
) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
    let env = VarEnv::new(plan);
    let mut data = ConfigData::new();
    for (list, groups) in config.iter().enumerate() {
        let list = GroupListIdx(list);
        for (slot, students) in groups.iter().enumerate() {
            for &s in *students {
                for group in 0..env.group_count(list) {
                    data = data.set(
                        Var::StudentInGroup {
                            list,
                            student: student(s),
                            group,
                        },
                        if group == slot as u32 { 1.0 } else { 0.0 },
                    );
                }
            }
        }
    }
    crate::build_group_lists(plan, &names(plan), &data)
}

/// What one model scores a hand-built placement at — the objective read at a
/// configuration, which is all it takes to compare placements. No solver is
/// involved: these tests never search, they weigh placements written out by
/// hand, and the model's variables are valued exactly like a warm start's.
///
/// Each placement is checked feasible on the way through, so a comparison can
/// never be between a placement and something the model would have refused.
fn model_value<'a>(
    model: &'a crate::GroupListsModel,
    plan: &'a GenerationPlan,
) -> impl Fn(&[&[&[u64]]]) -> f64 + 'a {
    move |config| {
        let lists = lists_of(plan, config);
        let solution = model
            .solution_from_complete_data(crate::group_lists_to_warm_start(plan, &lists))
            .expect("the placement must value exactly the model's variables");
        assert!(
            solution.is_feasible(),
            "the placement breaks {} constraint(s), so nothing can be read off it",
            solution.blame().len(),
        );
        solution.eval()
    }
}

/// Every seat of one list of a hand-built placement, held fixed — what the
/// prefill hands the model in fast mode.
fn pin(list: usize, groups: &[&[u64]]) -> FrozenPlacements {
    FrozenPlacements::new(
        groups
            .iter()
            .enumerate()
            .flat_map(|(group, students)| {
                students
                    .iter()
                    .map(move |&s| ((GroupListIdx(list), student(s)), group as u32))
            })
            .collect(),
    )
}

/// CBC and the two summations reach the same rationals by different routes, so
/// every comparison of this module goes through the crate's tolerance rather
/// than `assert_eq!`.
fn assert_close(got: f64, expected: f64) {
    assert!(f64_equals(got, expected), "expected {expected}, got {got}");
}

/// The battery of `objective_matches_the_greedy_ground_truth`, each entry a
/// plan and what it is there to cover.
fn battery() -> Vec<(&'static str, GenerationPlan)> {
    vec![
        (
            "one list, one tier",
            plan_with_uses(&[(&[1, 2, 3, 4], (2, 2), 1)], &[]),
        ),
        (
            // targets 3 / 2 / 2: two tiers of one list, whose product is
            // identically zero and must not be counted.
            "two tiers in one list",
            plan_with_uses(&[(&[1, 2, 3, 4, 5, 6, 7], (2, 3), 1)], &[]),
        ),
        (
            // The §2.4 license case, scaled down: a tutorial in fours and two
            // colles in pairs, so a pair's sites carry *different* masses —
            // the case the retired level-binary scheme could not express.
            "the license case",
            plan_with_uses(
                &[
                    (&[1, 2, 3, 4, 5, 6, 7, 8], (4, 4), 1),
                    (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
                    (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
                ],
                &[],
            ),
        ),
        (
            // Partly overlapping student sets and unequal multiplicities: the
            // masses differ per list *and* per student.
            "overlapping lists of unequal weight",
            plan_with_uses(
                &[
                    (&[1, 2, 3, 4, 5, 6], (2, 3), 3),
                    (&[1, 2, 3, 4], (2, 2), 1),
                    (&[3, 4, 5, 6, 7, 8], (2, 2), 2),
                ],
                &[],
            ),
        ),
        (
            // Kept lists: one weighing (and unbalanced, so its masses come
            // from the actual group sizes), one inert, and students 9 and 10
            // known through the kept lists alone — the pair that lives in the
            // constant term and nowhere else.
            "kept lists",
            plan_with_uses(
                &[(&[1, 2, 3, 4, 5, 6], (2, 2), 2)],
                &[(&[&[1, 2, 3], &[4, 5]], 2), (&[&[6, 9], &[10]], 0)],
            ),
        ),
        (
            // A multiplicity-0 spec next to a real one: its students are
            // placed like anybody else and score nothing.
            "a spec that covers nothing",
            plan_with_uses(
                &[(&[1, 2, 3, 4], (2, 2), 0), (&[1, 2, 3, 4], (2, 2), 1)],
                &[],
            ),
        ),
        (
            // Groups of one everywhere: no site survives the filter, so the
            // model declares no extra at all and the objective is a bare
            // constant.
            "nothing but lone seats",
            plan_with_uses(&[(&[1, 2, 3], (1, 1), 2)], &[(&[&[1, 2]], 1)]),
        ),
    ]
}

#[test]
fn objective_matches_the_greedy_ground_truth() {
    // The anti-drift net of the whole redesign: the model's objective, read at
    // the greedy's own placement, must equal the score the greedy computed for
    // it — not merely rank placements the same way. The two are written
    // independently (a running expansion of squares here, a direct sum of
    // squared partner distributions there) and share only the mass formula, so
    // this equality is what catches either drifting.
    for (label, plan) in battery() {
        let lists = crate::greedy_group_lists(&plan, &names(&plan)).lists;
        let expected = crate::placement_objective(&plan, &lists);

        let model = crate::build_model(
            &plan,
            ObjectiveWeights::default(),
            &crate::FrozenPlacements::default(),
        );
        let warm = crate::group_lists_to_warm_start(&plan, &lists);
        let solution = model.solution_from_complete_data(warm).unwrap_or_else(|| {
            panic!("{label}: the warm start must value exactly the model's variables")
        });

        assert!(
            solution.is_feasible(),
            "{label}: the greedy placement breaks {} constraint(s)",
            solution.blame().len(),
        );
        assert!(
            f64_equals(solution.eval(), expected),
            "{label}: the model scores {}, the greedy scores {expected}",
            solution.eval(),
        );
    }
}

#[test]
fn kept_lists_enter_as_constants() {
    // Two students the plan knows only through a kept list. They have no site
    // and no variable anywhere, so the model must still score them — which is
    // why the constant term is carried in the objective's `LinExpr` rather
    // than dropped as an offset an argmax would not notice.
    //
    // 4 uses each way over a group of two: `P` is 1 in both directions, so the
    // pair alone is worth 2.
    let plan = plan_with_uses(&[(&[1, 2], (2, 2), 1)], &[(&[&[5, 6]], 4)]);
    let env = VarEnv::new(&plan);
    let pairs = PairData::new(&plan, &env);
    assert_close(pairs.constant_term(), 2.0);

    let lists = crate::greedy_group_lists(&plan, &names(&plan)).lists;
    let model = crate::build_model(
        &plan,
        ObjectiveWeights::default(),
        &crate::FrozenPlacements::default(),
    );
    let solution = model
        .solution_from_complete_data(crate::group_lists_to_warm_start(&plan, &lists))
        .expect("the warm start must value exactly the model's variables");

    // The only spec has one group of two, so 1 and 2 also meet, each way at
    // mass 1: the whole score is 4, of which the kept pair is half.
    assert_close(solution.eval(), 4.0);
    assert_close(solution.eval(), crate::placement_objective(&plan, &lists));
}

#[test]
fn reconstruction_recovers_tight_values() {
    // The load-bearing assumption behind the one-sided rows (see the `extras`
    // module doc). Under a stripped objective every extra may float *down*, so
    // this solves the *checker* problem — no objective at all — and then hands
    // its base values back through `Model::solution_from_data`, the
    // reconstruction path every strategy uses to report an objective.
    //
    // The checker returns any feasible placement, so nothing is hardcoded: the
    // reported value is checked against the greedy's score of the very
    // placement the checker produced.
    let plan = plan_with_uses(
        &[
            (&[1, 2, 3, 4, 5, 6], (2, 3), 2),
            (&[1, 2, 3, 4], (2, 2), 1),
            (&[3, 4, 5, 6], (2, 2), 1),
        ],
        &[(&[&[1, 5]], 1)],
    );
    let model = crate::build_model(
        &plan,
        ObjectiveWeights::default(),
        &crate::FrozenPlacements::default(),
    );
    let solver = ColloCbcSolver::with_disable_logging(true);

    let base = model
        .solve_checker(&solver)
        .expect("the checker problem must be feasible")
        .get_data();
    let solution = model
        .solution_from_data(&base, &solver)
        .expect("reconstruction should succeed");

    // The placement the checker settled on, read back out as group lists — the
    // conversion the greedy's own output goes through too, so the ground truth
    // is measured on exactly what the model was scored at.
    let lists = crate::build_group_lists(&plan, &names(&plan), &base);
    assert_close(solution.eval(), crate::placement_objective(&plan, &lists));
}

#[test]
fn every_declared_variable_is_paid_for() {
    // What keeps the one-sided rows tight under the maximize, and what keeps
    // declaration and expansion in lockstep: a coefficient of 0 would leave
    // its variable free to float, and a *missing* term would leave it out of
    // the built model while the warm start still named it.
    let plan = plan_with_uses(
        &[
            (&[1, 2, 3, 4, 5, 6, 7], (2, 3), 2),
            (&[1, 2, 3, 4], (2, 2), 1),
        ],
        &[(&[&[1, 2]], 1)],
    );
    let env = VarEnv::new(&plan);
    let pairs = PairData::new(&plan, &env);

    let mut sites = 0;
    let mut products = 0;
    for ((a, b), table) in pairs.pairs() {
        for tier in table {
            let coef = pairs.together_coefficient(a, b, tier.list, tier.target);
            assert!(coef > 0.0, "site coefficient of {a:?}/{b:?} is {coef}");
            sites += tier.groups.len();
        }
        for (first, second) in cross_tiers(table) {
            let coef = pairs.coincide_coefficient(a, b, first, second);
            assert!(coef > 0.0, "product coefficient of {a:?}/{b:?} is {coef}");
            products += 1;
        }
    }

    // And the built model holds exactly those columns — no more (a declared
    // extra nothing references is not expanded) and no fewer.
    let model = crate::build_model(
        &plan,
        ObjectiveWeights::default(),
        &crate::FrozenPlacements::default(),
    );
    let mut built_sites = 0;
    let mut built_products = 0;
    for var in model.problem().get_variables().keys() {
        match var {
            collomatique_ilp_modeler::InternalVar::Extra(ExtraVarName::Together { .. }) => {
                built_sites += 1
            }
            collomatique_ilp_modeler::InternalVar::Extra(ExtraVarName::Coincide { .. }) => {
                built_products += 1
            }
            _ => {}
        }
    }
    assert_eq!(built_sites, sites);
    assert_eq!(built_products, products);
    // The instance is not vacuous: list 0 has two tiers and list 1 one, so
    // every pair of the four students shared by both lists has two products.
    assert!(products > 0);
}

#[test]
fn the_optimum_is_reached_at_a_tight_configuration() {
    // The maximize really does pull the one-sided rows tight, on an instance
    // small enough to solve outright: the optimal value must be the collision
    // score of the placement the solver returns, not an inflated bound.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 1), (&[1, 2, 3, 4], (2, 2), 1)],
        &[],
    );
    let model = crate::build_model(
        &plan,
        ObjectiveWeights::default(),
        &crate::FrozenPlacements::default(),
    );
    let solution = model
        .solve(&ColloCbcSolver::with_disable_logging(true))
        .expect("the model must be feasible")
        .into_solution();

    let base = solution.get_data();
    let lists = crate::build_group_lists(&plan, &names(&plan), &base);
    assert_close(solution.eval(), crate::placement_objective(&plan, &lists));

    // Two lists of pairs over four students, one use each, so N = 2 and a
    // partner weighs 1/2. Repeating a partner concentrates all of a student's
    // mass on them (`P` = 1, scoring 1); splitting spreads it over two
    // partners at 1/2 each (scoring 1/2). Four students either way, so the
    // optimum is 4 — reached only by the two lists agreeing.
    assert_close(solution.eval(), 4.0);
}

#[test]
fn repeat_partners_beat_spread_partners() {
    // Superadditivity read straight off the objective — the `9² + 1² > 5² + 5²`
    // of §2.4.
    //
    // Four students, a heavy colle list of multiplicity 3 and a light one of
    // multiplicity 1, plus a kept list of weight 2 pairing them the *other*
    // way: N = 6 for everybody, so a seat in list 0 weighs 3/6, one in list 1
    // weighs 1/6, and the kept partner is worth 2/6 for free.
    //
    // List 0 is pinned to {1, 2} / {3, 4} — the fast mode's model, F4 — so
    // list 1 is the only choice left, and fixed N gives every student a partner
    // mass of exactly 1 whichever way it goes: the choice is purely how that
    // mass is *spread*. Repeating list 0's pairs gives student 1 the
    // distribution (2/3, 1/3) over (2, 3); handing the seat to the kept partner
    // gives (1/2, 1/2) — the same total, flatter. Squaring takes the
    // concentrated one, 4/9 + 1/9 > 1/4 + 1/4, where an objective linear in
    // meetings would be indifferent.
    //
    // Four students in two pairs is three placements and no more, so the model
    // is read at every one of them: this is the optimum of list 1 by
    // enumeration, no search involved.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 3), (&[1, 2, 3, 4], (2, 2), 1)],
        &[(&[&[1, 3], &[2, 4]], 2)],
    );
    let pinned: &[&[u64]] = &[&[1, 2], &[3, 4]];
    let model = crate::build_model(&plan, ObjectiveWeights::default(), &pin(0, pinned));
    let value = model_value(&model, &plan);

    let repeated = value(&[pinned, &[&[1, 2], &[3, 4]]]);
    let kept_partner = value(&[pinned, &[&[1, 3], &[2, 4]]]);
    let fresh = value(&[pinned, &[&[1, 4], &[2, 3]]]);

    // 4 · 5/9, 4 · 1/2 and 4 · 7/18 — every one of them a placement the pinned
    // model accepts (`model_value` checks it), so what separates them is the
    // objective and nothing else.
    assert_close(repeated, 20.0 / 9.0);
    assert_close(kept_partner, 2.0);
    assert_close(fresh, 14.0 / 9.0);
    assert!(
        repeated > kept_partner && kept_partner > fresh,
        "repeating scores {repeated}, the kept partner {kept_partner}, two fresh ones {fresh}",
    );
}

#[test]
fn the_license_case_ranks_as_designed() {
    // `greedy::tests::license_case` on the model's side: the §2.4 scenario,
    // scored by the ILP objective instead of the greedy's summation. Eight
    // students, a tutorial in two groups of four and two colles in pairs, one
    // use each: N = 3, so a colle partner weighs 1/3 and a tutorial mate 1/9.
    //
    // The tutorial is pinned (F4, the fast mode's model), which leaves the two
    // colles. Three of a student's nine ninths go to tutorial mates whatever
    // happens, so the most concentrated distribution reachable is
    // 1/9 + 1/3 + 1/3 = 7/9 on a single partner — a tutorial mate taken as
    // *both* colle partners — with 1/9 on each of the other two. That is
    // scenario (a), 8 · (49 + 1 + 1)/81 = 408/81, and no placement of the
    // colles can beat it. Nothing here searches for it: the three scenarios are
    // spelled out and the model is read at each.
    let plan = plan_with_uses(
        &[
            (&[1, 2, 3, 4, 5, 6, 7, 8], (4, 4), 1),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
        ],
        &[],
    );
    let tutorial: &[&[u64]] = &[&[1, 2, 3, 4], &[5, 6, 7, 8]];
    let pairs_inside: &[&[u64]] = &[&[1, 2], &[3, 4], &[5, 6], &[7, 8]];
    let pairs_across: &[&[u64]] = &[&[1, 5], &[2, 6], &[3, 7], &[4, 8]];
    let scattered: &[&[u64]] = &[&[1, 3], &[2, 4], &[5, 7], &[6, 8]];

    let model = crate::build_model(&plan, ObjectiveWeights::default(), &pin(0, tutorial));
    let value = model_value(&model, &plan);

    // (a) stable colle partners who are also tutorial mates, (b) stable colle
    // partners in the other tutorial group, (c) colle partners scattered among
    // one's own tutorial mates — the same three the greedy test ranks, at
    // 8 · 51/81, 8 · 39/81 and 8 · 33/81.
    let a = value(&[tutorial, pairs_inside, pairs_inside]);
    let b = value(&[tutorial, pairs_across, pairs_across]);
    let c = value(&[tutorial, pairs_inside, scattered]);

    assert_close(a, 408.0 / 81.0);
    assert_close(b, 312.0 / 81.0);
    assert_close(c, 264.0 / 81.0);
    assert!(a > b, "stable colle partners belong in your tutorial group");
    assert!(
        b > c,
        "a big tutorial is no license to scatter colle partners: {b} vs {c}",
    );
}
