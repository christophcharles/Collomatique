use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, coin_cbc::CbcSolver};
use collomatique_ilp::{Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{
    BuildError, DuplicateExtra, ExtraVar, HelperId, InternalVar, Modeler, Var,
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
fn ebase(name: &str) -> ExtraVar<B, E> {
    ExtraVar::Base(name.to_string())
}
fn eextra(name: &str) -> ExtraVar<B, E> {
    ExtraVar::Extra(name.to_string())
}

fn fresh<'m>() -> Modeler<'m, B, E, C, (), String> {
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Variable::binary());
    vars.insert("b".to_string(), Variable::binary());
    Modeler::new(vars)
}

#[tokio::test]
async fn trivial_problem() {
    let mut m = fresh();
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    m.add_constraint((&a + &b).leq(&LinExpr::constant(1.0)), "a+b<=1".into());
    m.add_objective(1.0, Objective::new(a + b, ObjectiveSense::Maximize));
    let model = m.build(&()).await.unwrap();
    let solver = CbcSolver::new();
    let cfg = solver.solve(model.problem()).expect("solvable");
    let sum = cfg
        .get(InternalVar::<B, E>::Base("a".to_string()))
        .unwrap_or(0.0)
        + cfg
            .get(InternalVar::<B, E>::Base("b".to_string()))
            .unwrap_or(0.0);
    assert_eq!(sum, 1.0);
}

#[tokio::test]
async fn referenced_extra_runs() {
    let ran = Arc::new(Mutex::new(false));
    let ran2 = Arc::clone(&ran);
    let mut m = fresh();
    // Extra `s` is defined as a + b (via constraint s = a + b).
    m.declare_extra_sync(
        "s".to_string(),
        Variable::integer(),
        move |_f, _kinds, e| {
            *ran2.lock().unwrap() = true;
            let lhs = LinExpr::var(ExtraVar::Extra(e));
            let rhs = LinExpr::var(ebase("a")) + LinExpr::var(ebase("b"));
            Ok(vec![lhs.eq(&rhs)])
        },
    )
    .unwrap();
    // User constraint references s.
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("s")), ObjectiveSense::Maximize),
    );
    let model = m.build(&()).await.unwrap();
    assert!(*ran.lock().unwrap());
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("s".to_string()))
            .unwrap(),
        1.0
    );
}

#[tokio::test]
async fn unreferenced_extra_does_not_run() {
    let ran = Arc::new(Mutex::new(false));
    let ran2 = Arc::clone(&ran);
    let mut m = fresh();
    m.declare_extra_sync(
        "dead".to_string(),
        Variable::integer(),
        move |_f, _kinds, _e| {
            *ran2.lock().unwrap() = true;
            Ok(vec![])
        },
    )
    .unwrap();
    m.add_constraint(
        LinExpr::var(base("a")).leq(&LinExpr::constant(1.0)),
        "trivial".into(),
    );
    let _ = m.build(&()).await.unwrap();
    assert!(!*ran.lock().unwrap());
}

#[tokio::test]
async fn extra_chain() {
    let mut m = fresh();
    // c = b
    m.declare_extra_sync("c".to_string(), Variable::integer(), |_f, _kinds, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
        ])
    })
    .unwrap();
    // bx = c (chains through c)
    m.declare_extra_sync("bx".to_string(), Variable::integer(), |_f, _kinds, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("c"))),
        ])
    })
    .unwrap();
    // ax = bx (chains through bx)
    m.declare_extra_sync("ax".to_string(), Variable::integer(), |_f, _kinds, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("bx"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("ax")).eq(&LinExpr::constant(1.0)),
        "ax=1".into(),
    );
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap(),
        1.0
    );
}

#[tokio::test]
async fn undeclared_extra() {
    let mut m = fresh();
    m.add_constraint(
        LinExpr::var(xtra("ghost")).eq(&LinExpr::constant(0.0)),
        "ghost".into(),
    );
    let err = m.build(&()).await.unwrap_err();
    match err {
        BuildError::UndeclaredExtra(e) => assert_eq!(e, "ghost"),
        other => panic!("expected UndeclaredExtra, got {:?}", other),
    }
}

#[tokio::test]
async fn extra_returns_error() {
    let mut m = fresh();
    m.declare_extra_sync("bad".to_string(), Variable::integer(), |_f, _kinds, _e| {
        Err("boom".to_string())
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("bad")).eq(&LinExpr::constant(0.0)),
        "use bad".into(),
    );
    let err = m.build(&()).await.unwrap_err();
    match err {
        BuildError::ExtraError(e, msg) => {
            assert_eq!(e, "bad");
            assert_eq!(msg, "boom");
        }
        other => panic!("expected ExtraError, got {:?}", other),
    }
}

#[tokio::test]
async fn helpers_namespaced_per_extra() {
    let mut m = fresh();
    // Two extras each mint their own helper.
    m.declare_extra_sync("e1".to_string(), Variable::integer(), |f, _kinds, e| {
        let h = f.new_helper(Variable::binary());
        Ok(vec![LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(h))])
    })
    .unwrap();
    m.declare_extra_sync("e2".to_string(), Variable::integer(), |f, _kinds, e| {
        let h = f.new_helper(Variable::binary());
        Ok(vec![LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(h))])
    })
    .unwrap();
    m.add_constraint(
        (LinExpr::var(xtra("e1")) + LinExpr::var(xtra("e2"))).eq(&LinExpr::constant(1.0)),
        "use both".into(),
    );
    let model = m.build(&()).await.unwrap();
    let helper_count = model
        .problem()
        .get_variables()
        .keys()
        .filter(|v| matches!(v, InternalVar::Helper { .. }))
        .count();
    assert_eq!(helper_count, 2);
    // Verify the two helpers have distinct owners.
    let mut owners: Vec<_> = model
        .problem()
        .get_variables()
        .keys()
        .filter_map(|v| match v {
            InternalVar::Helper { owner, .. } => Some(owner.clone()),
            _ => None,
        })
        .collect();
    owners.sort();
    assert_eq!(owners, vec!["e1".to_string(), "e2".to_string()]);
}

#[tokio::test]
async fn cyclic_extras() {
    let mut m = fresh();
    m.declare_extra_sync("a1".to_string(), Variable::integer(), |_f, _kinds, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("a2"))),
        ])
    })
    .unwrap();
    m.declare_extra_sync("a2".to_string(), Variable::integer(), |_f, _kinds, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("a1"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("a1")).eq(&LinExpr::constant(0.0)),
        "use a1".into(),
    );
    let err = m.build(&()).await.unwrap_err();
    match err {
        BuildError::CyclicExtra { cycle } => {
            assert!(cycle.contains(&"a1".to_string()));
            assert!(cycle.contains(&"a2".to_string()));
        }
        other => panic!("expected CyclicExtra, got {:?}", other),
    }
}

#[tokio::test]
async fn helper_smuggling_detected() {
    // Smuggle a HelperId out of one closure into another via shared state.
    let stash: Arc<Mutex<Option<HelperId>>> = Arc::new(Mutex::new(None));
    let stash1 = Arc::clone(&stash);
    let stash2 = Arc::clone(&stash);
    let mut m = fresh();
    m.declare_extra_sync(
        "donor".to_string(),
        Variable::integer(),
        move |f, _kinds, e| {
            let h = f.new_helper(Variable::binary());
            if let ExtraVar::Helper(hid) = &h {
                *stash1.lock().unwrap() = Some(hid.clone());
            }
            Ok(vec![LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(h))])
        },
    )
    .unwrap();
    m.declare_extra_sync(
        "thief".to_string(),
        Variable::integer(),
        move |_f, _kinds, e| {
            let stolen = stash2
                .lock()
                .unwrap()
                .clone()
                .expect("donor must run first");
            Ok(vec![
                LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ExtraVar::Helper(stolen))),
            ])
        },
    )
    .unwrap();
    // Reference donor first so its closure runs and stashes the id,
    // then reference thief. Using two separate constraints makes the
    // root-discovery order deterministic (it follows insertion order
    // of `self.constraints`, which is a Vec).
    m.add_constraint(
        LinExpr::var(xtra("donor")).eq(&LinExpr::constant(0.0)),
        "use donor".into(),
    );
    m.add_constraint(
        LinExpr::var(xtra("thief")).eq(&LinExpr::constant(0.0)),
        "use thief".into(),
    );
    let err = m.build(&()).await.unwrap_err();
    match err {
        BuildError::HelperLeak { used_in, .. } => {
            assert_eq!(used_in, "thief");
        }
        other => panic!("expected HelperLeak, got {:?}", other),
    }
}

#[tokio::test]
async fn duplicate_extra_fails() {
    let mut m = fresh();
    m.declare_extra_sync("dup".to_string(), Variable::integer(), |_f, _kinds, _e| {
        Ok(vec![])
    })
    .unwrap();
    let DuplicateExtra(name) = m
        .declare_extra_sync("dup".to_string(), Variable::integer(), |_f, _kinds, _e| {
            Ok(vec![])
        })
        .unwrap_err();
    assert_eq!(name, "dup");
}
