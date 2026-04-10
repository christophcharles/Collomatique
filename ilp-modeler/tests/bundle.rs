//! Tests for `ConstraintBundle` / `IntConstraintBundle` and
//! `Modeler::apply_bundle`.

use std::collections::HashMap;

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, coin_cbc::CbcSolver};
use collomatique_ilp::{IntConstraint, IntLinExpr, Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{
    BuildError, ConstraintBundle, EagerReifyError, ExtraEntry, ExtraVar, IntConstraintBundle,
    InternalVar, Modeler, ReifyError, Var,
};

type B = String;
type E = String;
type C = String;

fn base(name: &str) -> Var<B, E> {
    Var::Base(name.to_string())
}
fn xtra(name: &str) -> Var<B, E> {
    Var::Extra(name.to_string())
}

fn fresh<'m>() -> Modeler<'m, B, E, C, (), String> {
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Variable::binary());
    vars.insert("b".to_string(), Variable::binary());
    Modeler::new(vars)
}

fn empty_bundle<'m>() -> ConstraintBundle<'m, B, E, C, (), String> {
    ConstraintBundle::new()
}

#[tokio::test]
async fn empty_bundle_apply_is_noop() {
    let mut m = fresh();
    m.apply_bundle(empty_bundle()).unwrap();
    let pb = m.build(&()).await.unwrap();
    assert_eq!(pb.get_constraints().len(), 0);
    // Two declared base variables, no extras, no helpers.
    assert_eq!(pb.get_variables().len(), 2);
}

#[tokio::test]
async fn bundle_only_constraints() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![
        ((&a + &b).leq(&LinExpr::constant(1.0)), "a+b<=1".into()),
        ((&a - &b).eq(&LinExpr::constant(0.0)), "a=b".into()),
    ]);
    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    let pb = m.build(&()).await.unwrap();
    assert_eq!(pb.get_constraints().len(), 2);
}

#[tokio::test]
async fn bundle_only_objectives() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let mut bundle = ConstraintBundle::<B, E, C, (), String>::new();
    bundle
        .objectives
        .push((2.0, Objective::new(a.clone(), ObjectiveSense::Maximize)));
    bundle
        .objectives
        .push((1.0, Objective::new(b.clone(), ObjectiveSense::Maximize)));
    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    // Should maximize 2a + b → both 1 (binary).
    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap(),
        1.0
    );
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap(),
        1.0
    );
}

#[tokio::test]
async fn bundle_only_extras() {
    // Bundle declares one extra `s = a + b` and nothing else.
    let entry: ExtraEntry<B, E, (), String> = ExtraEntry::new(
        "s".to_string(),
        Variable::integer(),
        |_db, _f, _kinds, e| {
            Box::pin(async move {
                let lhs = LinExpr::var(ExtraVar::Extra(e));
                let rhs = LinExpr::var(ExtraVar::Base("a".to_string()))
                    + LinExpr::var(ExtraVar::Base("b".to_string()));
                Ok(vec![lhs.eq(&rhs)])
            })
        },
    );
    let mut bundle: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new();
    bundle.extras.push(entry);

    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    // Force expansion by referencing `s`.
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    let pb = m.build(&()).await.unwrap();
    // Three vars: a, b, s.
    assert_eq!(pb.get_variables().len(), 3);
}

#[tokio::test]
async fn bundle_merge_concat() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let mut left = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        (&a + &b).leq(&LinExpr::constant(1.0)),
        "left".into(),
    )]);
    left.objectives
        .push((1.0, Objective::new(a.clone(), ObjectiveSense::Maximize)));

    let right = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        (&a - &b).eq(&LinExpr::constant(0.0)),
        "right".into(),
    )]);

    left.merge(right);

    assert_eq!(left.constraints.len(), 2);
    assert_eq!(left.objectives.len(), 1);
    assert_eq!(left.extras.len(), 0);
    // Check order: left's constraint first, right's second.
    assert_eq!(left.constraints[0].1, "left");
    assert_eq!(left.constraints[1].1, "right");
}

#[tokio::test]
async fn constraint_bundle_from_constraints_roundtrip() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let c1 = (&a + &b).leq(&LinExpr::constant(1.0));
    let c2 = (&a - &b).eq(&LinExpr::constant(0.0));
    let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![
        (c1.clone(), "c1".into()),
        (c2.clone(), "c2".into()),
    ]);
    assert_eq!(bundle.constraints.len(), 2);
    assert_eq!(bundle.constraints[0].0, c1);
    assert_eq!(bundle.constraints[1].0, c2);
}

#[tokio::test]
async fn int_bundle_from_constraints_roundtrip() {
    let a = IntLinExpr::var(base("a"));
    let b = IntLinExpr::var(base("b"));
    let c1: IntConstraint<Var<B, E>> = (&a + &b).leq(&IntLinExpr::constant(1));
    let c2: IntConstraint<Var<B, E>> = (&a - &b).eq(&IntLinExpr::constant(0));
    let bundle = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![
        (c1.clone(), "c1".into()),
        (c2.clone(), "c2".into()),
    ]);
    assert_eq!(bundle.constraints.len(), 2);
    assert_eq!(bundle.constraints[0].0, c1);
    assert_eq!(bundle.constraints[1].0, c2);
}

#[tokio::test]
async fn int_bundle_into_general_unwraps() {
    let a = IntLinExpr::var(base("a"));
    let b = IntLinExpr::var(base("b"));
    let c1: IntConstraint<Var<B, E>> = (&a + &b).leq(&IntLinExpr::constant(1));
    let int_bundle = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        c1.clone(),
        "c1".into(),
    )]);
    let general = int_bundle.into_general();
    assert_eq!(general.constraints.len(), 1);
    // The unwrapped constraint matches the int constraint's
    // underlying representation.
    assert_eq!(&general.constraints[0].0, c1.as_constraint());
}

// ----- Reify tests ---------------------------------------------------

/// Wrapper error type implementing `From<ReifyError<B, E>>`,
/// required by `IntConstraintBundle::reify`.
#[derive(Debug)]
enum TestErr {
    Reify(ReifyError<B, E>),
}

impl From<ReifyError<B, E>> for TestErr {
    fn from(e: ReifyError<B, E>) -> Self {
        TestErr::Reify(e)
    }
}

fn fresh_reify<'m>() -> Modeler<'m, B, E, C, (), TestErr> {
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Variable::binary());
    vars.insert("b".to_string(), Variable::binary());
    Modeler::new(vars)
}

#[tokio::test]
async fn reify_empty_bundle_pins_indicator_to_one() {
    // An empty IntConstraintBundle, when reified, produces a
    // bundle whose only contribution is a single extra (the
    // indicator) constrained to 1.
    let int_bundle: IntConstraintBundle<B, E, C, (), TestErr> = IntConstraintBundle::new();
    let reified = int_bundle.reify("ind".to_string()).unwrap();
    assert_eq!(reified.constraints.len(), 0);
    assert_eq!(reified.objectives.len(), 0);
    assert_eq!(reified.extras.len(), 1);
    assert_eq!(reified.extras[0].name, "ind");
    assert_eq!(reified.extras[0].kind, Variable::binary());

    // Apply and build; the resulting problem should require ind=1.
    let mut m = fresh_reify();
    m.apply_bundle(reified).unwrap();
    // Force expansion of `ind` by referencing it.
    m.add_constraint(
        LinExpr::var(xtra("ind")).leq(&LinExpr::constant(1.0)),
        "ref ind".into(),
    );
    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("ind".to_string()))
            .unwrap(),
        1.0
    );
}

#[tokio::test]
async fn reify_and_with_solver() {
    // Reify {a + b <= 1, a + b >= 1} into `is_one`. The
    // indicator is 1 iff a + b == 1. For binary a, b, that
    // means is_one == 1 iff exactly one of {a, b} is 1.
    //
    // We then maximise is_one and verify the solver picks an
    // assignment with a + b = 1, and is_one = 1.
    let a = IntLinExpr::var(base("a"));
    let b = IntLinExpr::var(base("b"));
    let c1 = (&a + &b).leq(&IntLinExpr::constant(1));
    let c2 = (&a + &b).geq(&IntLinExpr::constant(1));
    let int_bundle = IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![
        (c1, "a+b<=1".into()),
        (c2, "a+b>=1".into()),
    ]);
    let reified = int_bundle.reify("is_one".to_string()).unwrap();

    let mut m = fresh_reify();
    m.apply_bundle(reified).unwrap();
    // Maximise is_one.
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("is_one")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let is_one = cfg
        .get(InternalVar::<B, E>::Extra("is_one".to_string()))
        .unwrap();
    let av = cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap();
    let bv = cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap();
    assert_eq!(is_one, 1.0);
    assert_eq!(av + bv, 1.0);
}

#[tokio::test]
async fn reify_continuous_var_errors() {
    // Reify a constraint over a *continuous* base variable. The
    // discreteness check inside reify_and_inner should fire and
    // surface as BuildError::ExtraError.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::default()); // continuous
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let x = IntLinExpr::var(base("x"));
    let c = x.leq(&IntLinExpr::constant(0));
    let int_bundle =
        IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(c, "x<=0".into())]);
    let reified = int_bundle.reify("ind".to_string()).unwrap();
    m.apply_bundle(reified).unwrap();
    // Force expansion.
    m.add_constraint(
        LinExpr::var(xtra("ind")).leq(&LinExpr::constant(1.0)),
        "ref ind".into(),
    );

    let err = m.build(&()).await.unwrap_err();
    match err {
        BuildError::ExtraError(name, TestErr::Reify(ReifyError::NonDiscreteVariable(_))) => {
            assert_eq!(name, "ind");
        }
        other => panic!(
            "expected ExtraError(_, NonDiscreteVariable), got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn int_bundle_merge_concat() {
    let a = IntLinExpr::var(base("a"));
    let b = IntLinExpr::var(base("b"));
    let c1: IntConstraint<Var<B, E>> = (&a + &b).leq(&IntLinExpr::constant(1));
    let c2: IntConstraint<Var<B, E>> = (&a - &b).eq(&IntLinExpr::constant(0));
    let mut left = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        c1.clone(),
        "left".into(),
    )]);
    let right = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        c2.clone(),
        "right".into(),
    )]);
    left.merge(right);
    assert_eq!(left.constraints.len(), 2);
    assert_eq!(left.constraints[0].1, "left");
    assert_eq!(left.constraints[1].1, "right");
}

#[tokio::test]
async fn apply_bundle_duplicate_extra_fails() {
    let mut m = fresh();
    // Declare an extra directly on the modeler.
    m.declare_extra_sync("s".to_string(), Variable::integer(), |_f, _kinds, _e| {
        Ok(vec![])
    })
    .unwrap();
    // Then try to apply a bundle that defines the same extra.
    let entry: ExtraEntry<B, E, (), String> = ExtraEntry::new(
        "s".to_string(),
        Variable::integer(),
        |_db, _f, _kinds, _e| Box::pin(async move { Ok(vec![]) }),
    );
    let mut bundle: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new();
    bundle.extras.push(entry);
    let err = m.apply_bundle(bundle).unwrap_err();
    assert_eq!(err.0, "s");
}

#[tokio::test]
async fn merged_bundles_duplicate_extra_fails() {
    // Two bundles each define extra "s"; merge them and apply.
    let entry1: ExtraEntry<B, E, (), String> = ExtraEntry::new(
        "s".to_string(),
        Variable::integer(),
        |_db, _f, _kinds, _e| Box::pin(async move { Ok(vec![]) }),
    );
    let entry2: ExtraEntry<B, E, (), String> = ExtraEntry::new(
        "s".to_string(),
        Variable::integer(),
        |_db, _f, _kinds, _e| Box::pin(async move { Ok(vec![]) }),
    );
    let mut left: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new();
    left.extras.push(entry1);
    let mut right: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new();
    right.extras.push(entry2);
    left.merge(right);

    let mut m = fresh();
    let err = m.apply_bundle(left).unwrap_err();
    assert_eq!(err.0, "s");
}

#[tokio::test]
async fn reify_invalid_epsilon_fails() {
    let a = IntLinExpr::var(base("a"));
    let c = a.leq(&IntLinExpr::constant(1));
    for bad_eps in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        let bundle = IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(
            c.clone(),
            "c".into(),
        )]);
        match bundle.reify_with_epsilon("ind".to_string(), bad_eps) {
            Err(EagerReifyError::InvalidEpsilon(_)) => {}
            Err(other) => panic!("expected InvalidEpsilon for {bad_eps}, got {other:?}"),
            Ok(_) => panic!("expected InvalidEpsilon for {bad_eps}, got Ok"),
        }
    }
}

#[tokio::test]
async fn reify_duplicate_variable_fails() {
    // Bundle already has an extra named "x"; reify("x") should fail.
    let entry: ExtraEntry<B, E, (), TestErr> = ExtraEntry::new(
        "x".to_string(),
        Variable::integer(),
        |_db, _f, _kinds, _e| Box::pin(async move { Ok(vec![]) }),
    );
    let mut int_bundle: IntConstraintBundle<B, E, C, (), TestErr> = IntConstraintBundle::new();
    int_bundle.extras.push(entry);
    match int_bundle.reify("x".to_string()) {
        Err(EagerReifyError::DuplicateVariable(name)) => assert_eq!(name, "x"),
        Err(other) => panic!("expected DuplicateVariable, got {other:?}"),
        Ok(_) => panic!("expected DuplicateVariable, got Ok"),
    }
}

// ----- Reification coverage tests ----------------------------------------

#[tokio::test]
async fn reify_equality_constraint() {
    // Reify { x == 3 } into indicator `eq_ind`.
    // Maximize eq_ind: solver should set x = 3, eq_ind = 1.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::integer().min(0.0).max(5.0));
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let x = IntLinExpr::var(base("x"));
    let c = x.eq(&IntLinExpr::constant(3));
    let int_bundle =
        IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(c, "x==3".into())]);
    let reified = int_bundle.reify("eq_ind".to_string()).unwrap();
    m.apply_bundle(reified).unwrap();
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("eq_ind")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let eq_ind = cfg
        .get(InternalVar::<B, E>::Extra("eq_ind".to_string()))
        .unwrap();
    let xv = cfg.get(InternalVar::<B, E>::Base("x".to_string())).unwrap();
    assert_eq!(eq_ind, 1.0);
    assert_eq!(xv, 3.0);
}

#[tokio::test]
async fn reify_equality_constraint_forced_false() {
    // Reify { x == 3 } but force x >= 4. eq_ind must be 0.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::integer().min(0.0).max(5.0));
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let x = IntLinExpr::var(base("x"));
    let c = x.eq(&IntLinExpr::constant(3));
    let int_bundle =
        IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(c, "x==3".into())]);
    let reified = int_bundle.reify("eq_ind".to_string()).unwrap();
    m.apply_bundle(reified).unwrap();
    // Force x >= 4, so x == 3 can't be satisfied.
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(4.0)),
        "x>=4".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("eq_ind")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let eq_ind = cfg
        .get(InternalVar::<B, E>::Extra("eq_ind".to_string()))
        .unwrap();
    assert_eq!(eq_ind, 0.0);
}

#[tokio::test]
async fn reify_non_binary_integer_variable() {
    // Reify { x <= 2 } into indicator `le_ind`.
    // Maximize 10 * le_ind + x: solver should pick x = 2, le_ind = 1
    // (score 12) over x = 5, le_ind = 0 (score 5).
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::integer().min(0.0).max(5.0));
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let x = IntLinExpr::var(base("x"));
    let c = x.leq(&IntLinExpr::constant(2));
    let int_bundle =
        IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(c, "x<=2".into())]);
    let reified = int_bundle.reify("le_ind".to_string()).unwrap();
    m.apply_bundle(reified).unwrap();
    m.add_objective(
        10.0,
        Objective::new(LinExpr::var(xtra("le_ind")), ObjectiveSense::Maximize),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(base("x")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let le_ind = cfg
        .get(InternalVar::<B, E>::Extra("le_ind".to_string()))
        .unwrap();
    let xv = cfg.get(InternalVar::<B, E>::Base("x".to_string())).unwrap();
    assert_eq!(le_ind, 1.0);
    assert_eq!(xv, 2.0);
}

#[tokio::test]
async fn reify_non_binary_integer_variable_forced_false() {
    // Reify { x <= 2 } but force x >= 3. le_ind must be 0.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::integer().min(0.0).max(5.0));
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let x = IntLinExpr::var(base("x"));
    let c = x.leq(&IntLinExpr::constant(2));
    let int_bundle =
        IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(c, "x<=2".into())]);
    let reified = int_bundle.reify("le_ind".to_string()).unwrap();
    m.apply_bundle(reified).unwrap();
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(3.0)),
        "x>=3".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("le_ind")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let le_ind = cfg
        .get(InternalVar::<B, E>::Extra("le_ind".to_string()))
        .unwrap();
    assert_eq!(le_ind, 0.0);
}
