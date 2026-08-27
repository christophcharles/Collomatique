use super::*;
use crate::builder::MyModeler;
use crate::specs::GenerationPlan;
use crate::specs::tests::student;
use crate::vars::GroupListIdx;
use crate::vars::tests::plan_with_uses;
use collomatique_ilp::ConfigData;
use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
use collomatique_ilp_modeler::{InternalVar, Modeler};

/// Apply the extras to a fresh modeler, maximize each weighted term, build
/// (lazily — the terms are what force the expansion), solve, and return every
/// variable of the solution, extras included.
///
/// The "exactly one group per student" family comes along: the base binaries
/// only mean "the placement of the students" under it. The size constraints
/// stay out — this harness places students by hand.
fn solve_with_objective(
    plan: &GenerationPlan,
    terms: &[(f64, V)],
) -> ConfigData<InternalVar<Var, ExtraVarName>> {
    let env = VarEnv::new(plan);
    let pairs = PairData::new(plan, &env);
    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
    modeler
        .apply_bundle(build_extras(&pairs).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(crate::constraints::build_student_in_one_group(&env).into_general())
        .expect("no duplicate extras");
    for (weight, var) in terms {
        // The weight goes into the `LinExpr`, before the sense is applied.
        // `maximize`'s own `coef` scales the finished `Objective` instead, and
        // scaling an `Objective` by a negative number reverses its sense too
        // (`generic/ilp/src/objectives.rs:128`), so a negative weight there
        // would reward the term rather than penalize it.
        modeler.maximize(1.0, *weight * LinExpr::var(var.clone()));
    }
    let model = modeler.build(&env).expect("build should succeed");
    let solution = model
        .solve(&ColloCbcSolver::with_disable_logging(true))
        .expect("model should be solvable");
    solution.get_complete_data()
}

/// A weight-100 term placing `student` in `group` of `list` — the base binary
/// itself, at a weight far above the ±1 adversarial ones, so the placement
/// never bends.
fn place(list: usize, s: u64, group: u32) -> (f64, V) {
    (
        100.0,
        base_var(Var::StudentInGroup {
            list: GroupListIdx(list),
            student: student(s),
            group,
        }),
    )
}

fn together(a: u64, b: u64, list: usize, group: u32) -> V {
    extra_var(ExtraVarName::Together {
        a: student(a),
        b: student(b),
        list: GroupListIdx(list),
        group,
    })
}

fn coincide(a: u64, b: u64, first: (usize, u32), second: (usize, u32)) -> V {
    extra_var(ExtraVarName::Coincide {
        a: student(a),
        b: student(b),
        list1: GroupListIdx(first.0),
        target1: first.1,
        list2: GroupListIdx(second.0),
        target2: second.1,
    })
}

fn value(cfg: &ConfigData<InternalVar<Var, ExtraVarName>>, var: V) -> f64 {
    cfg.get(var.clone())
        .unwrap_or_else(|| panic!("{:?} should be part of the solved problem", var))
}

/// The declared variable set of a plan, forced open: `build_full` expands
/// *every* declared extra instead of only the referenced ones.
fn declared(plan: &GenerationPlan) -> Vec<InternalVar<Var, ExtraVarName>> {
    let env = VarEnv::new(plan);
    let pairs = PairData::new(plan, &env);
    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
    modeler
        .apply_bundle(build_extras(&pairs).into_general())
        .expect("no duplicate extras");
    let model = modeler
        .build_full(&env)
        .expect("every declared extra should expand");
    model
        .problem()
        .get_variables()
        .keys()
        .cloned()
        .collect::<Vec<_>>()
}

#[test]
fn declarations_expand_cleanly() {
    // Two overlapping lists: 1 and 2 are in both, 3 and 4 only in the first,
    // 5 and 6 only in the second. `build_full` force-expands every declared
    // extra, so a `Coincide` referencing a `Together` that was never declared
    // — the failure mode of two families enumerating the sites differently —
    // surfaces here, as do cycles and duplicate declarations.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 1), (&[1, 2, 5, 6], (2, 2), 1)],
        &[],
    );
    let vars = declared(&plan);

    // 1 and 2 are the only pair of both lists, so they are the only pair with
    // a product at all.
    assert!(vars.contains(&InternalVar::Extra(ExtraVarName::Coincide {
        a: student(1),
        b: student(2),
        list1: GroupListIdx(0),
        target1: 2,
        list2: GroupListIdx(1),
        target2: 2,
    })));
    // Both of its sites in list 0 (2 groups of 2) are declared, since the
    // product's defining row sums over the whole tier.
    for group in 0..2 {
        assert!(vars.contains(&InternalVar::Extra(ExtraVarName::Together {
            a: student(1),
            b: student(2),
            list: GroupListIdx(0),
            group,
        })));
    }
    // 3 and 5 never share a list, so the pair is not declared at all.
    assert!(!vars.iter().any(|v| matches!(
        v,
        InternalVar::Extra(ExtraVarName::Together { a, b, .. })
            if *a == student(3) && *b == student(5)
    )));
    // No helper column: helper ids are not externally addressable, and the
    // warm start has to name every variable of the model.
    assert!(!vars.iter().any(|v| matches!(v, InternalVar::Helper { .. })));
}

#[test]
fn massless_lists_and_tiers_declare_nothing() {
    // A spec covering no (period, subject) pair puts mass 0 on every pair of
    // it, so it is filtered out of the enumeration (F1) and declares no site.
    let unused = plan_with_uses(&[(&[1, 2, 3, 4], (2, 2), 0)], &[]);
    assert!(
        !declared(&unused)
            .iter()
            .any(|v| matches!(v, InternalVar::Extra(_)))
    );

    // 3 students in groups of 1 to 2 → targets 2 / 1. Nobody meets anybody in
    // the lone seat, so only group 0 is a site.
    let lone_seat = plan_with_uses(&[(&[1, 2, 3], (1, 2), 1)], &[]);
    let vars = declared(&lone_seat);
    assert!(vars.contains(&InternalVar::Extra(ExtraVarName::Together {
        a: student(1),
        b: student(2),
        list: GroupListIdx(0),
        group: 0,
    })));
    assert!(!vars.contains(&InternalVar::Extra(ExtraVarName::Together {
        a: student(1),
        b: student(2),
        list: GroupListIdx(0),
        group: 1,
    })));
}

#[test]
fn a_product_is_never_declared_inside_one_list() {
    // 7 students in groups of 2 to 3 → targets 3 / 2 / 2, hence two tiers of
    // the same list. A student sits in one group per list, so the two tiers
    // are mutually exclusive and their product is identically zero: declaring
    // it would put a permanently-0 column in the model.
    let plan = plan_with_uses(&[(&[1, 2, 3, 4, 5, 6, 7], (2, 3), 1)], &[]);
    assert!(
        !declared(&plan)
            .iter()
            .any(|v| matches!(v, InternalVar::Extra(ExtraVarName::Coincide { .. })))
    );
}

#[test]
fn a_together_cannot_exceed_the_placement() {
    // Two lists of 4 students in pairs. The rows are one-sided, so only the ≤
    // direction can be tested by an adversary: every site is pushed *up*, and
    // whichever comes back at 0 was held down by its defining rows alone.
    // That the value climbs to 1 when the pair does share is a property of the
    // maximizing objective, pinned by `objective/tests.rs` instead.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 1), (&[1, 2, 5, 6], (2, 2), 1)],
        &[],
    );

    let cfg = solve_with_objective(
        &plan,
        &[
            // List 0: {1, 2} and {3, 4}. List 1: {1, 5} and {2, 6}.
            place(0, 1, 0),
            place(0, 2, 0),
            place(0, 3, 1),
            place(0, 4, 1),
            place(1, 1, 0),
            place(1, 5, 0),
            place(1, 2, 1),
            place(1, 6, 1),
            // Adversarial: every site is rewarded, including the ones the
            // placement empties.
            (1.0, together(1, 2, 0, 0)),
            (1.0, together(1, 2, 0, 1)),
            (1.0, together(1, 5, 1, 0)),
            (1.0, together(1, 6, 1, 0)),
        ],
    );

    // 1 and 2 do share group 0 of list 0, so nothing holds that one down.
    assert_eq!(value(&cfg, together(1, 2, 0, 0)), 1.0);
    // But not group 1 of the same list, nor is 1 with 6 anywhere: both
    // students must be there, which is what tells the two rows apart.
    assert_eq!(value(&cfg, together(1, 2, 0, 1)), 0.0);
    assert_eq!(value(&cfg, together(1, 5, 1, 0)), 1.0);
    assert_eq!(value(&cfg, together(1, 6, 1, 0)), 0.0);
}

#[test]
fn a_coincide_needs_both_lists() {
    // Three lists of pairs over the same four students, so every couple of
    // lists is a product. The placement groups 1 with 2 in lists 0 and 1, and
    // separates them in list 2 — so exactly one of the three products is
    // reachable, and the two touching list 2 are held at 0 however hard the
    // adversary pushes.
    let plan = plan_with_uses(
        &[
            (&[1, 2, 3, 4], (2, 2), 1),
            (&[1, 2, 3, 4], (2, 2), 1),
            (&[1, 2, 3, 4], (2, 2), 1),
        ],
        &[],
    );

    let cfg = solve_with_objective(
        &plan,
        &[
            place(0, 1, 0),
            place(0, 2, 0),
            place(1, 1, 0),
            place(1, 2, 0),
            place(2, 1, 0),
            // Student 2 is placed *away* explicitly rather than left to the
            // group targets, which this harness leaves out: nothing else would
            // stop the solver from piling everybody into group 0 of list 2 to
            // collect the two rewards below.
            place(2, 2, 1),
            (1.0, coincide(1, 2, (0, 2), (1, 2))),
            (1.0, coincide(1, 2, (0, 2), (2, 2))),
            (1.0, coincide(1, 2, (1, 2), (2, 2))),
        ],
    );

    assert_eq!(value(&cfg, coincide(1, 2, (0, 2), (1, 2))), 1.0);
    assert_eq!(value(&cfg, coincide(1, 2, (0, 2), (2, 2))), 0.0);
    assert_eq!(value(&cfg, coincide(1, 2, (1, 2), (2, 2))), 0.0);
}
