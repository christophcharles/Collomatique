use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, coin_cbc::CbcSolver};
use collomatique_ilp::{Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{
    BuildError, DuplicateExtra, ExtraEntry, ExtraVar, HelperId, InternalVar, Modeler,
    ReconstructionError, SourceVar, Var,
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
    m.declare_extra_sync("s".to_string(), Variable::integer(), move |_f, _ctx, e| {
        *ran2.lock().unwrap() = true;
        let lhs = LinExpr::var(ExtraVar::Extra(e));
        let rhs = LinExpr::var(ebase("a")) + LinExpr::var(ebase("b"));
        Ok(vec![lhs.eq(&rhs)])
    })
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
        move |_f, _ctx, _e| {
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
    m.declare_extra_sync("c".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
        ])
    })
    .unwrap();
    // bx = c (chains through c)
    m.declare_extra_sync("bx".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("c"))),
        ])
    })
    .unwrap();
    // ax = bx (chains through bx)
    m.declare_extra_sync("ax".to_string(), Variable::integer(), |_f, _ctx, e| {
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
    m.declare_extra_sync("bad".to_string(), Variable::integer(), |_f, _ctx, _e| {
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
    m.declare_extra_sync("a1".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("a2"))),
        ])
    })
    .unwrap();
    m.declare_extra_sync("a2".to_string(), Variable::integer(), |_f, _ctx, e| {
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
        move |_f, _ctx, e| {
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
    m.declare_extra_sync("dup".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Ok(vec![])
    })
    .unwrap();
    let DuplicateExtra(name) = m
        .declare_extra_sync("dup".to_string(), Variable::integer(), |_f, _ctx, _e| {
            Ok(vec![])
        })
        .unwrap_err();
    assert_eq!(name, "dup");
}

// ----- add_fixer tests ---------------------------------------------------

#[tokio::test]
async fn fix_undeclared_variable() {
    // Constraint references undeclared "c"; fixer returns 1.0 for it.
    // a + c <= 1, with c fixed to 1 → a <= 0 → a = 0.
    let mut m = fresh();
    let a = LinExpr::var(base("a"));
    let c = LinExpr::var(base("c"));
    m.add_constraint((&a + &c).leq(&LinExpr::constant(1.0)), "a+c<=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(1.0) } else { None } })
    });
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap(),
        0.0
    );
}

#[tokio::test]
async fn fixer_chain_first_wins() {
    // Two fixers: first returns Some(1.0) for "c", second returns
    // Some(0.0). First should win.
    let mut m = fresh();
    let a = LinExpr::var(base("a"));
    let c = LinExpr::var(base("c"));
    m.add_constraint((&a + &c).leq(&LinExpr::constant(1.0)), "a+c<=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(1.0) } else { None } })
    });
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(0.0) } else { None } })
    });
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    // c fixed to 1.0 by first fixer → a <= 0 → a = 0
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap(),
        0.0
    );
}

#[tokio::test]
async fn fix_in_extra_closure() {
    // Extra's closure references undeclared "c"; fixer returns 1.0.
    // s = a + c, with c fixed to 1 → s = a + 1.
    let mut m = fresh();
    m.declare_extra_sync("s".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&(LinExpr::var(ExtraVar::Base("a".to_string()))
                + LinExpr::var(ExtraVar::Base("c".to_string())))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(2.0)),
        "s<=2".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("s")), ObjectiveSense::Maximize),
    );
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(1.0) } else { None } })
    });
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    // s = a + 1, s <= 2, maximize s → a = 1, s = 2.
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("s".to_string()))
            .unwrap(),
        2.0
    );
}

// ----- reconstruction_problem tests --------------------------------------

#[tokio::test]
async fn reconstruction_basic() {
    // Extra s = a + b. Solve main problem, then reconstruct
    // with base values and verify s matches.
    let mut m = fresh();
    m.declare_extra_sync("s".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e))
                .eq(&(LinExpr::var(ebase("a")) + LinExpr::var(ebase("b")))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("s")), ObjectiveSense::Maximize),
    );
    let model = m.build(&()).await.unwrap();

    // Solve the main problem.
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    let av = cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap();
    let bv = cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap();

    // Reconstruct from base values.
    let base_values = HashMap::from([("a".to_string(), av), ("b".to_string(), bv)]);
    let recon_pb = model.reconstruction_problem(&base_values).unwrap();
    let recon_cfg = CbcSolver::new().solve(&recon_pb).expect("solvable");
    let sv = recon_cfg
        .get(InternalVar::<B, E>::Extra("s".to_string()))
        .unwrap();
    assert_eq!(sv, av + bv);
}

#[tokio::test]
async fn reconstruction_missing_var() {
    let mut m = fresh();
    m.declare_extra_sync("s".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e))
                .eq(&(LinExpr::var(ebase("a")) + LinExpr::var(ebase("b")))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    let model = m.build(&()).await.unwrap();

    // Only provide "a", missing "b".
    let base_values = HashMap::from([("a".to_string(), 1.0)]);
    let ReconstructionError(missing) = model.reconstruction_problem(&base_values).unwrap_err();
    assert_eq!(missing, "b");
}

#[tokio::test]
async fn reconstruction_with_fixed_vars() {
    // Extra s = a + c, where c is fixed to 1.
    // After build, c is substituted out. Reconstruction only
    // needs base var "a".
    let mut m = fresh();
    m.declare_extra_sync("s".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&(LinExpr::var(ExtraVar::Base("a".to_string()))
                + LinExpr::var(ExtraVar::Base("c".to_string())))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(2.0)),
        "s<=2".into(),
    );
    m.add_fixer(|b: &String, _db: &()| {
        let b = b.clone();
        Box::pin(async move { if b == "c" { Some(1.0) } else { None } })
    });
    let model = m.build(&()).await.unwrap();

    // Reconstruct with a=1 only (c was fixed, not a base var).
    let base_values = HashMap::from([("a".to_string(), 1.0)]);
    let recon_pb = model.reconstruction_problem(&base_values).unwrap();
    let recon_cfg = CbcSolver::new().solve(&recon_pb).expect("solvable");
    let sv = recon_cfg
        .get(InternalVar::<B, E>::Extra("s".to_string()))
        .unwrap();
    // s = a + c = 1 + 1 = 2
    assert_eq!(sv, 2.0);
}

#[tokio::test]
async fn reconstruction_no_extras() {
    // Model with no extras. Reconstruction problem is trivial.
    let mut m = fresh();
    m.add_constraint(
        LinExpr::var(base("a")).leq(&LinExpr::constant(1.0)),
        "a<=1".into(),
    );
    let model = m.build(&()).await.unwrap();

    // No base vars appear in reconstruction (no extras exist).
    let base_values: HashMap<B, f64> = HashMap::new();
    let recon_pb = model.reconstruction_problem(&base_values).unwrap();
    assert_eq!(recon_pb.get_constraints().len(), 0);
    assert_eq!(recon_pb.get_variables().len(), 0);
}

// ----- declare_extras tests ----------------------------------------------

#[tokio::test]
async fn declare_extras_batch() {
    let mut m = fresh();
    m.declare_extras(vec![
        ExtraEntry::new("s1".to_string(), Variable::integer(), |_f, _ctx, e| {
            Box::pin(async move {
                Ok(vec![
                    LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("a"))),
                ])
            })
        }),
        ExtraEntry::new("s2".to_string(), Variable::integer(), |_f, _ctx, e| {
            Box::pin(async move {
                Ok(vec![
                    LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
                ])
            })
        }),
    ])
    .unwrap();
    m.add_constraint(
        (LinExpr::var(xtra("s1")) + LinExpr::var(xtra("s2"))).leq(&LinExpr::constant(1.0)),
        "s1+s2<=1".into(),
    );
    let model = m.build(&()).await.unwrap();
    // 2 base + 2 extras = 4 variables.
    assert_eq!(model.problem().get_variables().len(), 4);
}

#[tokio::test]
async fn declare_extras_internal_duplicate_fails() {
    let mut m = fresh();
    let DuplicateExtra(name) = m
        .declare_extras(vec![
            ExtraEntry::new("dup".to_string(), Variable::integer(), |_f, _ctx, _e| {
                Box::pin(async move { Ok(vec![]) })
            }),
            ExtraEntry::new("dup".to_string(), Variable::integer(), |_f, _ctx, _e| {
                Box::pin(async move { Ok(vec![]) })
            }),
        ])
        .unwrap_err();
    assert_eq!(name, "dup");
}

#[tokio::test]
async fn declare_extras_conflicts_with_existing_fails() {
    let mut m = fresh();
    m.declare_extra_sync("exists".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Ok(vec![])
    })
    .unwrap();
    let DuplicateExtra(name) = m
        .declare_extras(vec![ExtraEntry::new(
            "exists".to_string(),
            Variable::integer(),
            |_f, _ctx, _e| Box::pin(async move { Ok(vec![]) }),
        )])
        .unwrap_err();
    assert_eq!(name, "exists");
}

// ----- from_source tests ----------------------------------------------------

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum TestVar {
    X,
    Y,
    /// Undeclared variable used to test fix().
    Z,
}

impl SourceVar<()> for TestVar {
    async fn vars(_db: &()) -> HashMap<Self, Variable> {
        // Z is intentionally excluded — it is fixed, not a decision variable.
        HashMap::from([
            (TestVar::X, Variable::binary()),
            (TestVar::Y, Variable::binary()),
        ])
    }

    async fn fix(&self, _db: &()) -> Option<f64> {
        match self {
            TestVar::Z => Some(1.0),
            _ => None,
        }
    }
}

#[tokio::test]
async fn from_source_creates_modeler() {
    let m: Modeler<'_, TestVar, String, String, (), String> = Modeler::from_source(&()).await;
    let mut m = m;
    let x = LinExpr::var(Var::Base(TestVar::X));
    let y = LinExpr::var(Var::Base(TestVar::Y));
    m.add_constraint((&x + &y).leq(&LinExpr::constant(1.0)), "x+y<=1".into());
    m.add_objective(1.0, Objective::new(x + y, ObjectiveSense::Maximize));
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    let sum = cfg.get(InternalVar::Base(TestVar::X)).unwrap_or(0.0)
        + cfg.get(InternalVar::Base(TestVar::Y)).unwrap_or(0.0);
    assert_eq!(sum, 1.0);
}

#[tokio::test]
async fn from_source_auto_fixes_via_source_var() {
    // Constraint references Z (undeclared). from_source registers
    // SourceVar::fix as a fixer, which returns Some(1.0) for Z.
    // x + Z <= 1, Z fixed to 1 → x <= 0 → x = 0.
    let mut m: Modeler<'_, TestVar, String, String, (), String> = Modeler::from_source(&()).await;
    let x = LinExpr::var(Var::Base(TestVar::X));
    let z = LinExpr::var(Var::Base(TestVar::Z));
    m.add_constraint((&x + &z).leq(&LinExpr::constant(1.0)), "x+z<=1".into());
    m.add_objective(1.0, Objective::new(x, ObjectiveSense::Maximize));
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    assert_eq!(cfg.get(InternalVar::Base(TestVar::X)).unwrap(), 0.0);
}

#[tokio::test]
async fn from_source_additional_fixer_composes() {
    // from_source registers SourceVar::fix, then we add another fixer
    // for a different variable W (not in the enum). The additional
    // fixer should compose with the SourceVar one.
    let mut m: Modeler<'_, TestVar, String, String, (), String> = Modeler::from_source(&()).await;
    let x = LinExpr::var(Var::Base(TestVar::X));
    let z = LinExpr::var(Var::Base(TestVar::Z));
    // x + Z <= 1, Z fixed to 1 by SourceVar::fix → x = 0.
    m.add_constraint((&x + &z).leq(&LinExpr::constant(1.0)), "x+z<=1".into());
    m.add_objective(1.0, Objective::new(x, ObjectiveSense::Maximize));
    // Add a second fixer that doesn't handle Z (returns None for all).
    // Shouldn't break anything — the SourceVar fixer handles Z.
    m.add_fixer(|_b: &TestVar, _db: &()| Box::pin(async move { None }));
    let model = m.build(&()).await.unwrap();
    let cfg = CbcSolver::new().solve(model.problem()).expect("solvable");
    assert_eq!(cfg.get(InternalVar::Base(TestVar::X)).unwrap(), 0.0);
}
