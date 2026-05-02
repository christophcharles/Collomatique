//! Tests for `ConstraintBundle` / `IntConstraintBundle` and
//! `Modeler::apply_bundle`.

use std::collections::HashMap;

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, coin_cbc::CbcSolver};
use collomatique_ilp::{IntConstraint, IntLinExpr, Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{
    BuildError, ConstraintBundle, DuplicateExtra, EagerObjectifyError, EagerReifyError, ExtraEntry,
    ExtraVar, IntConstraintBundle, InternalVar, Modeler, ReifyError, Var,
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
    let pb = m.build(&()).await.unwrap().into_problem();
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
    let pb = m.build(&()).await.unwrap().into_problem();
    assert_eq!(pb.get_constraints().len(), 2);
}

#[tokio::test]
async fn bundle_only_objectives() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::new()
        .with_objective(2.0, Objective::new(a.clone(), ObjectiveSense::Maximize))
        .with_objective(1.0, Objective::new(b.clone(), ObjectiveSense::Maximize));
    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    // Should maximize 2a + b → both 1 (binary).
    let pb = m.build(&()).await.unwrap().into_problem();
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
    let entry: ExtraEntry<B, E, (), String> =
        ExtraEntry::new(Variable::integer(), |_f, _ctx, e| {
            Box::pin(async move {
                let lhs = LinExpr::var(ExtraVar::Extra(e));
                let rhs = LinExpr::var(ExtraVar::Base("a".to_string()))
                    + LinExpr::var(ExtraVar::Base("b".to_string()));
                Ok(vec![lhs.eq(&rhs)])
            })
        });
    let bundle: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new()
        .with_extra("s".to_string(), entry)
        .unwrap();

    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    // Force expansion by referencing `s`.
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    let pb = m.build(&()).await.unwrap().into_problem();
    // Three vars: a, b, s.
    assert_eq!(pb.get_variables().len(), 3);
}

#[tokio::test]
async fn bundle_merge_concat() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let left = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        (&a + &b).leq(&LinExpr::constant(1.0)),
        "left".into(),
    )])
    .with_objective(1.0, Objective::new(a.clone(), ObjectiveSense::Maximize));

    let right = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        (&a - &b).eq(&LinExpr::constant(0.0)),
        "right".into(),
    )]);

    let merged = left.merge(right).unwrap();

    assert_eq!(merged.constraints().len(), 2);
    assert_eq!(merged.objectives().len(), 1);
    assert_eq!(merged.extras().len(), 0);
    // Check order: left's constraint first, right's second.
    assert_eq!(merged.constraints()[0].1, "left");
    assert_eq!(merged.constraints()[1].1, "right");
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
    assert_eq!(bundle.constraints().len(), 2);
    assert_eq!(bundle.constraints()[0].0, c1);
    assert_eq!(bundle.constraints()[1].0, c2);
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
    assert_eq!(bundle.constraints().len(), 2);
    assert_eq!(bundle.constraints()[0].0, c1);
    assert_eq!(bundle.constraints()[1].0, c2);
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
    assert_eq!(general.constraints().len(), 1);
    // The unwrapped constraint matches the int constraint's
    // underlying representation.
    assert_eq!(&general.constraints()[0].0, c1.as_constraint());
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
    assert_eq!(reified.constraints().len(), 0);
    assert_eq!(reified.objectives().len(), 0);
    assert_eq!(reified.extras().len(), 1);
    assert!(reified.extras().contains_key(&"ind".to_string()));
    assert_eq!(
        *reified.extras().get(&"ind".to_string()).unwrap().kind(),
        Variable::binary()
    );

    // Apply and build; the resulting problem should require ind=1.
    let mut m = fresh_reify();
    m.apply_bundle(reified.into_general()).unwrap();
    // Force expansion of `ind` by referencing it.
    m.add_constraint(
        LinExpr::var(xtra("ind")).leq(&LinExpr::constant(1.0)),
        "ref ind".into(),
    );
    let pb = m.build(&()).await.unwrap().into_problem();
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
    m.apply_bundle(reified.into_general()).unwrap();
    // Maximise is_one.
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("is_one")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
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
    m.apply_bundle(reified.into_general()).unwrap();
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
    let left = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        c1.clone(),
        "left".into(),
    )]);
    let right = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        c2.clone(),
        "right".into(),
    )]);
    let merged = left.merge(right).unwrap();
    assert_eq!(merged.constraints().len(), 2);
    assert_eq!(merged.constraints()[0].1, "left");
    assert_eq!(merged.constraints()[1].1, "right");
}

#[tokio::test]
async fn apply_bundle_duplicate_extra_fails() {
    let mut m = fresh();
    // Declare an extra directly on the modeler.
    m.declare_extra_sync("s".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Ok(vec![])
    })
    .unwrap();
    // Then try to apply a bundle that defines the same extra.
    let entry: ExtraEntry<B, E, (), String> =
        ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| {
            Box::pin(async move { Ok(vec![]) })
        });
    let bundle: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new()
        .with_extra("s".to_string(), entry)
        .unwrap();
    let err = m.apply_bundle(bundle).unwrap_err();
    assert_eq!(err.0, "s");
}

#[tokio::test]
async fn merged_bundles_duplicate_extra_fails() {
    // Two bundles each define extra "s"; merge them and apply.
    let entry1: ExtraEntry<B, E, (), String> =
        ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| {
            Box::pin(async move { Ok(vec![]) })
        });
    let entry2: ExtraEntry<B, E, (), String> =
        ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| {
            Box::pin(async move { Ok(vec![]) })
        });
    let left: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new()
        .with_extra("s".to_string(), entry1)
        .unwrap();
    let right: ConstraintBundle<B, E, C, (), String> = ConstraintBundle::new()
        .with_extra("s".to_string(), entry2)
        .unwrap();
    match left.merge(right) {
        Err(DuplicateExtra(name)) => assert_eq!(name, "s"),
        Ok(_) => panic!("expected DuplicateExtra, got Ok"),
    }
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
    let entry: ExtraEntry<B, E, (), TestErr> =
        ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| {
            Box::pin(async move { Ok(vec![]) })
        });
    let int_bundle: IntConstraintBundle<B, E, C, (), TestErr> = IntConstraintBundle::new()
        .with_extra("x".to_string(), entry)
        .unwrap();
    match int_bundle.reify("x".to_string()) {
        Err(EagerReifyError::DuplicateVariable(name)) => assert_eq!(name, "x"),
        Err(other) => panic!("expected DuplicateVariable, got {other:?}"),
        Ok(_) => panic!("expected DuplicateVariable, got Ok"),
    }
}

// ----- with_reified / and_reified tests ----------------------------------

#[tokio::test]
async fn with_reified_constructor() {
    let bundle =
        IntConstraintBundle::<B, E, C, (), TestErr>::with_reified("ind".to_string(), || {
            let a = IntLinExpr::var(base("a"));
            vec![a.leq(&IntLinExpr::constant(1))]
        })
        .unwrap();
    assert_eq!(bundle.constraints().len(), 0);
    assert_eq!(bundle.extras().len(), 1);
    assert!(bundle.extras().contains_key(&"ind".to_string()));
}

#[tokio::test]
async fn and_reified_accumulates() {
    let bundle = IntConstraintBundle::<B, E, C, (), TestErr>::new()
        .and_reified("x".to_string(), || {
            vec![IntLinExpr::var(base("a")).leq(&IntLinExpr::constant(1))]
        })
        .unwrap()
        .and_reified("y".to_string(), || {
            vec![IntLinExpr::var(base("b")).leq(&IntLinExpr::constant(1))]
        })
        .unwrap();
    assert_eq!(bundle.extras().len(), 2);
    assert!(bundle.extras().contains_key(&"x".to_string()));
    assert!(bundle.extras().contains_key(&"y".to_string()));
}

#[tokio::test]
async fn and_reified_duplicate_fails() {
    let result = IntConstraintBundle::<B, E, C, (), TestErr>::new()
        .and_reified("dup".to_string(), || vec![])
        .unwrap()
        .and_reified("dup".to_string(), || vec![]);
    match result {
        Err(EagerReifyError::DuplicateVariable(name)) => assert_eq!(name, "dup"),
        Err(other) => panic!("expected DuplicateVariable, got {other:?}"),
        Ok(_) => panic!("expected DuplicateVariable, got Ok"),
    }
}

#[tokio::test]
async fn and_reified_with_epsilon_validates() {
    for bad_eps in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        let result = IntConstraintBundle::<B, E, C, (), TestErr>::new().and_reified_with_epsilon(
            "ind".to_string(),
            || vec![],
            bad_eps,
        );
        match result {
            Err(EagerReifyError::InvalidEpsilon(_)) => {}
            Err(other) => panic!("expected InvalidEpsilon for {bad_eps}, got {other:?}"),
            Ok(_) => panic!("expected InvalidEpsilon for {bad_eps}, got Ok"),
        }
    }
}

#[tokio::test]
async fn with_reified_matches_reify_behavior() {
    let a = IntLinExpr::var(base("a"));
    let b = IntLinExpr::var(base("b"));
    let c1 = (&a + &b).leq(&IntLinExpr::constant(1));
    let c2 = (&a + &b).geq(&IntLinExpr::constant(1));

    let bundle = IntConstraintBundle::<B, E, C, (), TestErr>::with_reified(
        "is_one".to_string(),
        move || vec![c1, c2],
    )
    .unwrap();

    let mut m = fresh_reify();
    m.apply_bundle(bundle.into_general()).unwrap();
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("is_one")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let is_one = cfg
        .get(InternalVar::<B, E>::Extra("is_one".to_string()))
        .unwrap();
    let av = cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap();
    let bv = cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap();
    assert_eq!(is_one, 1.0);
    assert_eq!(av + bv, 1.0);
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
    m.apply_bundle(reified.into_general()).unwrap();
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("eq_ind")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
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
    m.apply_bundle(reified.into_general()).unwrap();
    // Force x >= 4, so x == 3 can't be satisfied.
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(4.0)),
        "x>=4".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("eq_ind")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
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
    m.apply_bundle(reified.into_general()).unwrap();
    m.add_objective(
        10.0,
        Objective::new(LinExpr::var(xtra("le_ind")), ObjectiveSense::Maximize),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(base("x")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
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
    m.apply_bundle(reified.into_general()).unwrap();
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(3.0)),
        "x>=3".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("le_ind")), ObjectiveSense::Maximize),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let le_ind = cfg
        .get(InternalVar::<B, E>::Extra("le_ind".to_string()))
        .unwrap();
    assert_eq!(le_ind, 0.0);
}

// ----- Objectify tests ---------------------------------------------------

#[tokio::test]
async fn objectify_empty_bundle_errors() {
    let bundle = ConstraintBundle::<B, E, C, (), String>::new();
    match bundle.objectify("pen".to_string()) {
        Err(EagerObjectifyError::EmptyConstraints) => {}
        Err(other) => panic!("expected EmptyConstraints, got {other:?}"),
        Ok(_) => panic!("expected EmptyConstraints, got Ok"),
    }
}

#[tokio::test]
async fn objectify_duplicate_variable_errors() {
    let a = LinExpr::var(base("a"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        a.leq(&LinExpr::constant(0.0)),
        "c".into(),
    )])
    .with_extra(
        "pen".to_string(),
        ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| {
            Box::pin(async move { Ok(vec![]) })
        }),
    )
    .unwrap();
    match bundle.objectify("pen".to_string()) {
        Err(EagerObjectifyError::DuplicateVariable(name)) => assert_eq!(name, "pen"),
        Err(other) => panic!("expected DuplicateVariable, got {other:?}"),
        Ok(_) => panic!("expected DuplicateVariable, got Ok"),
    }
}

#[tokio::test]
async fn objectify_invalid_balance_errors() {
    let a = LinExpr::var(base("a"));
    for bad_alpha in [-0.1, 1.1, f64::NAN] {
        let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
            a.leq(&LinExpr::constant(0.0)),
            "c".into(),
        )]);
        match bundle.objectify_with_balance("pen".to_string(), bad_alpha) {
            Err(EagerObjectifyError::InvalidBalance(_)) => {}
            Err(other) => panic!("expected InvalidBalance for {bad_alpha}, got {other:?}"),
            Ok(_) => panic!("expected InvalidBalance for {bad_alpha}, got Ok"),
        }
    }
}

#[tokio::test]
async fn objectify_single_inequality() {
    // x <= 3 on x in [0,5], force x = 5. Penalty = 2.0.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::non_negative().max(5.0));
    let mut m: Modeler<'_, B, E, C, (), String> = Modeler::new(vars);

    let x = LinExpr::var(base("x"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        x.leq(&LinExpr::constant(3.0)),
        "x<=3".into(),
    )]);
    let objectified = bundle.objectify("pen".to_string()).unwrap();
    m.apply_bundle(objectified.into_general()).unwrap();
    // Force x = 5.
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(5.0)),
        "x>=5".into(),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let pen = cfg
        .get(InternalVar::<B, E>::Extra("pen".to_string()))
        .unwrap();
    assert_eq!(pen, 2.0);
}

#[tokio::test]
async fn objectify_single_equality() {
    // x == 3 on x in [0,5], force x = 5. Penalty = |5-3| = 2.0.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::non_negative().max(5.0));
    let mut m: Modeler<'_, B, E, C, (), String> = Modeler::new(vars);

    let x = LinExpr::var(base("x"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        x.eq(&LinExpr::constant(3.0)),
        "x==3".into(),
    )]);
    let objectified = bundle.objectify("pen".to_string()).unwrap();
    m.apply_bundle(objectified.into_general()).unwrap();
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(5.0)),
        "x>=5".into(),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let pen = cfg
        .get(InternalVar::<B, E>::Extra("pen".to_string()))
        .unwrap();
    assert_eq!(pen, 2.0);
}

/// Helper: build a two-constraint objectify problem with forced
/// violations of 2 and 3, then return the penalty value.
async fn objectify_two_constraints(alpha: f64) -> f64 {
    // x <= 2 (violation 2 when x=4) and y <= 2 (violation 3 when y=5).
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::non_negative().max(5.0));
    vars.insert("y".to_string(), Variable::non_negative().max(5.0));
    let mut m: Modeler<'_, B, E, C, (), String> = Modeler::new(vars);

    let x = LinExpr::var(base("x"));
    let y = LinExpr::var(base("y"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![
        (x.leq(&LinExpr::constant(2.0)), "x<=2".into()),
        (y.leq(&LinExpr::constant(2.0)), "y<=2".into()),
    ]);
    let objectified = bundle
        .objectify_with_balance("pen".to_string(), alpha)
        .unwrap();
    m.apply_bundle(objectified.into_general()).unwrap();
    // Force x = 4, y = 5.
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(4.0)),
        "x>=4".into(),
    );
    m.add_constraint(
        LinExpr::var(base("y")).geq(&LinExpr::constant(5.0)),
        "y>=5".into(),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    cfg.get(InternalVar::<B, E>::Extra("pen".to_string()))
        .unwrap()
}

#[tokio::test]
async fn objectify_alpha_0_sum() {
    // alpha=0: penalty = (1-0)/2 * (2+3) = 2.5
    let pen = objectify_two_constraints(0.0).await;
    assert_eq!(pen, 2.5);
}

#[tokio::test]
async fn objectify_alpha_1_minimax() {
    // alpha=1: penalty = 1*max(2,3) = 3.0
    let pen = objectify_two_constraints(1.0).await;
    assert_eq!(pen, 3.0);
}

#[tokio::test]
async fn objectify_alpha_half_balanced() {
    // alpha=0.5: penalty = 0.5*3 + 0.5/2*(2+3) = 1.5 + 1.25 = 2.75
    let pen = objectify_two_constraints(0.5).await;
    assert_eq!(pen, 2.75);
}

#[tokio::test]
async fn objectify_int_bundle_convenience() {
    // Verify IntConstraintBundle::objectify works.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("x".to_string(), Variable::non_negative().max(5.0));
    let mut m: Modeler<'_, B, E, C, (), String> = Modeler::new(vars);

    let x = IntLinExpr::var(base("x"));
    let int_bundle = IntConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
        x.leq(&IntLinExpr::constant(3)),
        "x<=3".into(),
    )]);
    let objectified = int_bundle.objectify("pen".to_string()).unwrap();
    m.apply_bundle(objectified.into_general()).unwrap();
    m.add_constraint(
        LinExpr::var(base("x")).geq(&LinExpr::constant(5.0)),
        "x>=5".into(),
    );

    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    let pen = cfg
        .get(InternalVar::<B, E>::Extra("pen".to_string()))
        .unwrap();
    assert_eq!(pen, 2.0);
}

#[tokio::test]
async fn objectify_with_coef_scales_penalty() {
    // x <= 3 on x in [0,5], force x = 5.
    // With coef=1, optimal x would be 3 (penalty=0). Force x=5, penalty=2.
    // With coef=2, the objective contribution is 2*penalty instead of 1*penalty.
    // Verify by giving x a maximize incentive of 1.5 and comparing outcomes.
    //
    // coef=1: minimize(pen) + maximize(1.5*x) → pen costs 1 per unit,
    //         x gains 1.5 per unit, so x=5 is optimal (pen=2, obj=2 - 7.5 = -5.5).
    // coef=2: minimize(2*pen) + maximize(1.5*x) → pen costs 2 per unit,
    //         x gains 1.5 per unit, so x=3 is optimal (pen=0, obj=0 - 4.5 = -4.5).
    for (coef, expected_x) in [(1.0, 5.0), (2.0, 3.0)] {
        let mut vars: HashMap<B, Variable> = HashMap::new();
        vars.insert("x".to_string(), Variable::non_negative().max(5.0));
        let mut m: Modeler<'_, B, E, C, (), String> = Modeler::new(vars);

        let x = LinExpr::var(base("x"));
        let bundle = ConstraintBundle::<B, E, C, (), String>::from_constraints(vec![(
            x.leq(&LinExpr::constant(3.0)),
            "x<=3".into(),
        )]);
        let objectified = bundle.objectify_with_coef("pen".to_string(), coef).unwrap();
        m.apply_bundle(objectified.into_general()).unwrap();
        m.maximize(1.5, LinExpr::var(base("x")));

        let pb = m.build(&()).await.unwrap().into_problem();
        let cfg = CbcSolver::new().solve(&pb).expect("solvable");
        let x_val = cfg.get(InternalVar::<B, E>::Base("x".to_string())).unwrap();
        assert_eq!(x_val, expected_x, "coef={coef}: expected x={expected_x}");
    }
}

// ----- fixer + reify/objectify tests -------------------------------------

#[tokio::test]
async fn fix_in_reify_closure() {
    // Reify {a + c <= 1} where c is fixed to 0.
    // With c=0, constraint becomes a <= 1, which is always true
    // for binary a. The indicator should be 1.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("a".to_string(), Variable::binary());
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let a = IntLinExpr::var(base("a"));
    let c = IntLinExpr::var(Var::Base("c".to_string()));
    let bundle = IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(
        (&a + &c).leq(&IntLinExpr::constant(1)),
        "a+c<=1".into(),
    )]);
    let reified = bundle.reify("ind".to_string()).unwrap();
    m.apply_bundle(reified.into_general()).unwrap();
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("ind")), ObjectiveSense::Maximize),
    );
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(0.0) } else { None } })
    });
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("ind".to_string()))
            .unwrap(),
        1.0
    );
}

#[tokio::test]
async fn fix_non_integer_in_reify_fails() {
    // Reify a constraint where a fixed variable has value 0.5.
    // Should fail with NonIntegerFixValue.
    let mut vars: HashMap<B, Variable> = HashMap::new();
    vars.insert("a".to_string(), Variable::binary());
    let mut m: Modeler<'_, B, E, C, (), TestErr> = Modeler::new(vars);

    let a = IntLinExpr::var(base("a"));
    let c = IntLinExpr::var(Var::Base("c".to_string()));
    let bundle = IntConstraintBundle::<B, E, C, (), TestErr>::from_constraints(vec![(
        (&a + &c).leq(&IntLinExpr::constant(1)),
        "a+c<=1".into(),
    )]);
    let reified = bundle.reify("ind".to_string()).unwrap();
    m.apply_bundle(reified.into_general()).unwrap();
    m.add_constraint(
        LinExpr::var(xtra("ind")).leq(&LinExpr::constant(1.0)),
        "ref ind".into(),
    );
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(0.5) } else { None } })
    });
    let err = m.build(&()).await.unwrap_err();
    match err {
        BuildError::ExtraError(name, TestErr::Reify(ReifyError::NonIntegerFixValue { .. })) => {
            assert_eq!(name, "ind");
        }
        other => panic!(
            "expected ExtraError(_, NonIntegerFixValue), got {:?}",
            other
        ),
    }
}

// ----- objective helper tests --------------------------------------------

#[tokio::test]
async fn bundle_with_maximize() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::new()
        .with_maximize(2.0, a)
        .with_maximize(1.0, b);
    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    // Maximize 2a + b → both 1 (binary).
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
async fn bundle_with_minimize() {
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let bundle = ConstraintBundle::<B, E, C, (), String>::new()
        .with_minimize(1.0, a)
        .with_minimize(1.0, b);
    let mut m = fresh();
    m.apply_bundle(bundle).unwrap();
    let pb = m.build(&()).await.unwrap().into_problem();
    let cfg = CbcSolver::new().solve(&pb).expect("solvable");
    // Minimize a + b → both 0 (binary).
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap(),
        0.0
    );
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap(),
        0.0
    );
}
