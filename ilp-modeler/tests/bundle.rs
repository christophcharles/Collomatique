//! Tests for `ConstraintBundle` / `IntConstraintBundle` and
//! `Modeler::apply_bundle`.

use std::collections::HashMap;

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, coin_cbc::CbcSolver};
use collomatique_ilp::{IntConstraint, IntLinExpr, Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{
    ConstraintBundle, ExtraEntry, ExtraVar, IntConstraintBundle, InternalVar, Modeler, Var,
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
    m.apply_bundle(empty_bundle());
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
    m.apply_bundle(bundle);
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
    m.apply_bundle(bundle);
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
    m.apply_bundle(bundle);
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
