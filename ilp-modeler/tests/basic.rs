use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, SolverModel, collo_cbc::ColloCbcSolver};
use collomatique_ilp::{Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{
    BuildError, DescribeVar, DuplicateExtra, ExtraEntry, ExtraVar, HelperId, InternalVar, Modeler,
    ReconstructionError, Var,
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

#[test]
fn trivial_problem() {
    let mut m = fresh();
    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    m.add_constraint((&a + &b).leq(&LinExpr::constant(1.0)), "a+b<=1".into());
    m.add_objective(1.0, Objective::new(a + b, ObjectiveSense::Maximize));
    let model = m.build(&()).unwrap();
    let solver = ColloCbcSolver::new();
    let cfg = solver
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    let sum = cfg
        .get(InternalVar::<B, E>::Base("a".to_string()))
        .unwrap_or(0.0)
        + cfg
            .get(InternalVar::<B, E>::Base("b".to_string()))
            .unwrap_or(0.0);
    assert_eq!(sum, 1.0);
}

#[test]
fn referenced_extra_runs() {
    let ran = Arc::new(Mutex::new(false));
    let ran2 = Arc::clone(&ran);
    let mut m = fresh();
    // Extra `s` is defined as a + b (via constraint s = a + b).
    m.declare_extra("s".to_string(), Variable::integer(), move |_f, _ctx, e| {
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
    let model = m.build(&()).unwrap();
    assert!(*ran.lock().unwrap());
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("s".to_string()))
            .unwrap(),
        1.0
    );
}

#[test]
fn unreferenced_extra_does_not_run() {
    let ran = Arc::new(Mutex::new(false));
    let ran2 = Arc::clone(&ran);
    let mut m = fresh();
    m.declare_extra(
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
    let _ = m.build(&()).unwrap();
    assert!(!*ran.lock().unwrap());
}

#[test]
fn extra_chain() {
    let mut m = fresh();
    // c = b
    m.declare_extra("c".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
        ])
    })
    .unwrap();
    // bx = c (chains through c)
    m.declare_extra("bx".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("c"))),
        ])
    })
    .unwrap();
    // ax = bx (chains through bx)
    m.declare_extra("ax".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("bx"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("ax")).eq(&LinExpr::constant(1.0)),
        "ax=1".into(),
    );
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap(),
        1.0
    );
}

#[test]
fn undeclared_extra() {
    let mut m = fresh();
    m.add_constraint(
        LinExpr::var(xtra("ghost")).eq(&LinExpr::constant(0.0)),
        "ghost".into(),
    );
    let err = m.build(&()).unwrap_err();
    match err {
        BuildError::UndeclaredExtra(e) => assert_eq!(e, "ghost"),
        other => panic!("expected UndeclaredExtra, got {:?}", other),
    }
}

#[test]
fn undeclared_extra_slow_path() {
    let mut m = fresh();
    m.declare_extra("real".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Ok(vec![])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("ghost")).eq(&LinExpr::constant(0.0)),
        "ghost".into(),
    );
    let err = m.build(&()).unwrap_err();
    match err {
        BuildError::UndeclaredExtra(e) => assert_eq!(e, "ghost"),
        other => panic!("expected UndeclaredExtra, got {:?}", other),
    }
}

#[test]
fn extra_returns_error() {
    let mut m = fresh();
    m.declare_extra("bad".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Err("boom".to_string())
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("bad")).eq(&LinExpr::constant(0.0)),
        "use bad".into(),
    );
    let err = m.build(&()).unwrap_err();
    match err {
        BuildError::ExtraError(e, msg) => {
            assert_eq!(e, "bad");
            assert_eq!(msg, "boom");
        }
        other => panic!("expected ExtraError, got {:?}", other),
    }
}

#[test]
fn helpers_namespaced_per_extra() {
    let mut m = fresh();
    // Two extras each mint their own helper.
    m.declare_extra("e1".to_string(), Variable::integer(), |f, _kinds, e| {
        let h = f.new_helper(Variable::binary());
        Ok(vec![LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(h))])
    })
    .unwrap();
    m.declare_extra("e2".to_string(), Variable::integer(), |f, _kinds, e| {
        let h = f.new_helper(Variable::binary());
        Ok(vec![LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(h))])
    })
    .unwrap();
    m.add_constraint(
        (LinExpr::var(xtra("e1")) + LinExpr::var(xtra("e2"))).eq(&LinExpr::constant(1.0)),
        "use both".into(),
    );
    let model = m.build(&()).unwrap();
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

#[test]
fn cyclic_extras() {
    let mut m = fresh();
    m.declare_extra("a1".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("a2"))),
        ])
    })
    .unwrap();
    m.declare_extra("a2".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("a1"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("a1")).eq(&LinExpr::constant(0.0)),
        "use a1".into(),
    );
    let err = m.build(&()).unwrap_err();
    match err {
        BuildError::CyclicExtra { cycle } => {
            assert!(cycle.contains(&"a1".to_string()));
            assert!(cycle.contains(&"a2".to_string()));
        }
        other => panic!("expected CyclicExtra, got {:?}", other),
    }
}

#[test]
fn helper_smuggling_detected() {
    // Smuggle a HelperId out of one closure into another via shared state.
    let stash: Arc<Mutex<Option<HelperId>>> = Arc::new(Mutex::new(None));
    let stash1 = Arc::clone(&stash);
    let stash2 = Arc::clone(&stash);
    let mut m = fresh();
    m.declare_extra(
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
    m.declare_extra(
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
    let err = m.build(&()).unwrap_err();
    match err {
        BuildError::HelperLeak { used_in, .. } => {
            assert_eq!(used_in, "thief");
        }
        other => panic!("expected HelperLeak, got {:?}", other),
    }
}

#[test]
fn duplicate_extra_fails() {
    let mut m = fresh();
    m.declare_extra("dup".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Ok(vec![])
    })
    .unwrap();
    let DuplicateExtra(name) = m
        .declare_extra("dup".to_string(), Variable::integer(), |_f, _ctx, _e| {
            Ok(vec![])
        })
        .unwrap_err();
    assert_eq!(name, "dup");
}

// ----- add_fixer tests ---------------------------------------------------

#[test]
fn fix_undeclared_variable() {
    // Constraint references undeclared "c"; fixer returns 1.0 for it.
    // a + c <= 1, with c fixed to 1 → a <= 0 → a = 0.
    let mut m = fresh();
    let a = LinExpr::var(base("a"));
    let c = LinExpr::var(base("c"));
    m.add_constraint((&a + &c).leq(&LinExpr::constant(1.0)), "a+c<=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));
    m.add_fixer(
        |b: &String, _env: &()| {
            if b == "c" { Some(1.0) } else { None }
        },
    );
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap(),
        0.0
    );
}

#[test]
fn fixer_chain_first_wins() {
    // Two fixers: first returns Some(1.0) for "c", second returns
    // Some(0.0). First should win.
    let mut m = fresh();
    let a = LinExpr::var(base("a"));
    let c = LinExpr::var(base("c"));
    m.add_constraint((&a + &c).leq(&LinExpr::constant(1.0)), "a+c<=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));
    m.add_fixer(
        |b: &String, _env: &()| {
            if b == "c" { Some(1.0) } else { None }
        },
    );
    m.add_fixer(
        |b: &String, _env: &()| {
            if b == "c" { Some(0.0) } else { None }
        },
    );
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    // c fixed to 1.0 by first fixer → a <= 0 → a = 0
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap(),
        0.0
    );
}

#[test]
fn fix_in_extra_closure() {
    // Extra's closure references undeclared "c"; fixer returns 1.0.
    // s = a + c, with c fixed to 1 → s = a + 1.
    let mut m = fresh();
    m.declare_extra("s".to_string(), Variable::integer(), |_f, _ctx, e| {
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
    m.add_fixer(
        |b: &String, _env: &()| {
            if b == "c" { Some(1.0) } else { None }
        },
    );
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    // s = a + 1, s <= 2, maximize s → a = 1, s = 2.
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("s".to_string()))
            .unwrap(),
        2.0
    );
}

// ----- reconstruction_problem tests --------------------------------------

#[test]
fn reconstruction_basic() {
    // Extra s = a + b. Solve main problem, then reconstruct
    // with base values and verify s matches.
    let mut m = fresh();
    m.declare_extra("s".to_string(), Variable::integer(), |_f, _ctx, e| {
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
    let model = m.build(&()).unwrap();

    // Solve the main problem.
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    let av = cfg.get(InternalVar::<B, E>::Base("a".to_string())).unwrap();
    let bv = cfg.get(InternalVar::<B, E>::Base("b".to_string())).unwrap();

    // Reconstruct from base values.
    let base_values = HashMap::from([("a".to_string(), av), ("b".to_string(), bv)]);
    let recon_pb = model.reconstruction_problem(&base_values).unwrap();
    let recon_cfg = ColloCbcSolver::new()
        .build_model(&recon_pb)
        .solve()
        .expect("solvable");
    let sv = recon_cfg
        .get(InternalVar::<B, E>::Extra("s".to_string()))
        .unwrap();
    assert_eq!(sv, av + bv);
}

#[test]
fn reconstruction_missing_var() {
    let mut m = fresh();
    m.declare_extra("s".to_string(), Variable::integer(), |_f, _ctx, e| {
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
    let model = m.build(&()).unwrap();

    // Only provide "a", missing "b".
    let base_values = HashMap::from([("a".to_string(), 1.0)]);
    let ReconstructionError(missing) = model.reconstruction_problem(&base_values).unwrap_err();
    assert_eq!(missing, "b");
}

#[test]
fn reconstruction_with_fixed_vars() {
    // Extra s = a + c, where c is fixed to 1.
    // After build, c is substituted out. Reconstruction only
    // needs base var "a".
    let mut m = fresh();
    m.declare_extra("s".to_string(), Variable::integer(), |_f, _ctx, e| {
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
    m.add_fixer(
        |b: &String, _env: &()| {
            if b == "c" { Some(1.0) } else { None }
        },
    );
    let model = m.build(&()).unwrap();

    // Reconstruct with a=1 only (c was fixed, not a base var).
    let base_values = HashMap::from([("a".to_string(), 1.0)]);
    let recon_pb = model.reconstruction_problem(&base_values).unwrap();
    let recon_cfg = ColloCbcSolver::new()
        .build_model(&recon_pb)
        .solve()
        .expect("solvable");
    let sv = recon_cfg
        .get(InternalVar::<B, E>::Extra("s".to_string()))
        .unwrap();
    // s = a + c = 1 + 1 = 2
    assert_eq!(sv, 2.0);
}

#[test]
fn reconstruction_no_extras() {
    // Model with no extras. Reconstruction problem is trivial.
    let mut m = fresh();
    m.add_constraint(
        LinExpr::var(base("a")).leq(&LinExpr::constant(1.0)),
        "a<=1".into(),
    );
    let model = m.build(&()).unwrap();

    // No base vars appear in reconstruction (no extras exist).
    let base_values: HashMap<B, f64> = HashMap::new();
    let recon_pb = model.reconstruction_problem(&base_values).unwrap();
    assert_eq!(recon_pb.get_constraints().len(), 0);
    assert_eq!(recon_pb.get_variables().len(), 0);
}

// ----- declare_extras tests ----------------------------------------------

#[test]
fn declare_extras_batch() {
    let mut m = fresh();
    m.declare_extras(vec![
        (
            "s1".to_string(),
            ExtraEntry::new(Variable::integer(), |_f, _ctx, e| {
                Ok(vec![
                    LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("a"))),
                ])
            }),
        ),
        (
            "s2".to_string(),
            ExtraEntry::new(Variable::integer(), |_f, _ctx, e| {
                Ok(vec![
                    LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
                ])
            }),
        ),
    ])
    .unwrap();
    m.add_constraint(
        (LinExpr::var(xtra("s1")) + LinExpr::var(xtra("s2"))).leq(&LinExpr::constant(1.0)),
        "s1+s2<=1".into(),
    );
    let model = m.build(&()).unwrap();
    // 2 base + 2 extras = 4 variables.
    assert_eq!(model.problem().get_variables().len(), 4);
}

#[test]
fn declare_extras_internal_duplicate_fails() {
    let mut m = fresh();
    let DuplicateExtra(name) = m
        .declare_extras(vec![
            (
                "dup".to_string(),
                ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| Ok(vec![])),
            ),
            (
                "dup".to_string(),
                ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| Ok(vec![])),
            ),
        ])
        .unwrap_err();
    assert_eq!(name, "dup");
}

#[test]
fn declare_extras_conflicts_with_existing_fails() {
    let mut m = fresh();
    m.declare_extra("exists".to_string(), Variable::integer(), |_f, _ctx, _e| {
        Ok(vec![])
    })
    .unwrap();
    let DuplicateExtra(name) = m
        .declare_extras(vec![(
            "exists".to_string(),
            ExtraEntry::new(Variable::integer(), |_f, _ctx, _e| Ok(vec![])),
        )])
        .unwrap_err();
    assert_eq!(name, "exists");
}

// ----- from_described tests (TestVar) ----------------------------------------

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum TestVar {
    X,
    Y,
    /// Undeclared variable used to test check_fix().
    Z,
}

impl DescribeVar for TestVar {
    type Env = ();
    fn enumerate(_env: &()) -> HashMap<Self, Variable> {
        HashMap::from([
            (TestVar::X, Variable::binary()),
            (TestVar::Y, Variable::binary()),
        ])
    }
    fn check_fix(&self, _env: &()) -> Option<f64> {
        match self {
            TestVar::Z => Some(1.0),
            _ => None,
        }
    }
}

#[test]
fn from_described_creates_modeler_testvar() {
    let mut m: Modeler<'_, TestVar, String, String, (), String> = Modeler::from_described(&());
    let x = LinExpr::var(Var::Base(TestVar::X));
    let y = LinExpr::var(Var::Base(TestVar::Y));
    m.add_constraint((&x + &y).leq(&LinExpr::constant(1.0)), "x+y<=1".into());
    m.add_objective(1.0, Objective::new(x + y, ObjectiveSense::Maximize));
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    let sum = cfg.get(InternalVar::Base(TestVar::X)).unwrap_or(0.0)
        + cfg.get(InternalVar::Base(TestVar::Y)).unwrap_or(0.0);
    assert_eq!(sum, 1.0);
}

#[test]
fn from_described_auto_fixes_via_check_fix() {
    // Constraint references Z (undeclared). from_described registers
    // DescribeVar::check_fix as a fixer, which returns Some(1.0) for Z.
    // x + Z <= 1, Z fixed to 1 → x <= 0 → x = 0.
    let mut m: Modeler<'_, TestVar, String, String, (), String> = Modeler::from_described(&());
    let x = LinExpr::var(Var::Base(TestVar::X));
    let z = LinExpr::var(Var::Base(TestVar::Z));
    m.add_constraint((&x + &z).leq(&LinExpr::constant(1.0)), "x+z<=1".into());
    m.add_objective(1.0, Objective::new(x, ObjectiveSense::Maximize));
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(cfg.get(InternalVar::Base(TestVar::X)).unwrap(), 0.0);
}

#[test]
fn from_described_additional_fixer_composes() {
    // from_described registers check_fix, then we add another fixer
    // for a different variable W (not in the enum). The additional
    // fixer should compose with the check_fix one.
    let mut m: Modeler<'_, TestVar, String, String, (), String> = Modeler::from_described(&());
    let x = LinExpr::var(Var::Base(TestVar::X));
    let z = LinExpr::var(Var::Base(TestVar::Z));
    // x + Z <= 1, Z fixed to 1 by check_fix → x = 0.
    m.add_constraint((&x + &z).leq(&LinExpr::constant(1.0)), "x+z<=1".into());
    m.add_objective(1.0, Objective::new(x, ObjectiveSense::Maximize));
    // Add a second fixer that doesn't handle Z (returns None for all).
    // Shouldn't break anything — the check_fix fixer handles Z.
    m.add_fixer(|_b: &TestVar, _env: &()| None);
    let model = m.build(&()).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(cfg.get(InternalVar::Base(TestVar::X)).unwrap(), 0.0);
}

// ----- DescribeVar + from_described tests ------------------------------------

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum DescVar {
    A,
    B,
    /// Undeclared variable, fixed to 1.0.
    C,
}

struct DescEnv {
    c_value: f64,
}

impl DescribeVar for DescVar {
    type Env = DescEnv;

    fn enumerate(_env: &DescEnv) -> HashMap<Self, Variable> {
        HashMap::from([
            (DescVar::A, Variable::binary()),
            (DescVar::B, Variable::binary()),
        ])
    }

    fn check_fix(&self, env: &DescEnv) -> Option<f64> {
        match self {
            DescVar::C => Some(env.c_value),
            _ => None,
        }
    }
}

#[test]
fn from_described_creates_modeler() {
    let env = DescEnv { c_value: 1.0 };
    let mut m: Modeler<'_, DescVar, String, String, DescEnv, String> =
        Modeler::from_described(&env);
    let a = LinExpr::var(Var::Base(DescVar::A));
    let b = LinExpr::var(Var::Base(DescVar::B));
    m.add_constraint((&a + &b).leq(&LinExpr::constant(1.0)), "a+b<=1".into());
    m.add_objective(1.0, Objective::new(a + b, ObjectiveSense::Maximize));
    let model = m.build(&env).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    let sum = cfg.get(InternalVar::Base(DescVar::A)).unwrap_or(0.0)
        + cfg.get(InternalVar::Base(DescVar::B)).unwrap_or(0.0);
    assert_eq!(sum, 1.0);
}

#[test]
fn from_described_fixes_via_env() {
    // a + C <= 1, C fixed to 1 via check_fix → a <= 0 → a = 0.
    let env = DescEnv { c_value: 1.0 };
    let mut m: Modeler<'_, DescVar, String, String, DescEnv, String> =
        Modeler::from_described(&env);
    let a = LinExpr::var(Var::Base(DescVar::A));
    let c = LinExpr::var(Var::Base(DescVar::C));
    m.add_constraint((&a + &c).leq(&LinExpr::constant(1.0)), "a+c<=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));
    let model = m.build(&env).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(cfg.get(InternalVar::Base(DescVar::A)).unwrap(), 0.0);
}

#[test]
fn describe_var_via_from_described() {
    // DescVar uses from_described directly.
    let env = DescEnv { c_value: 1.0 };
    let mut m: Modeler<'_, DescVar, String, String, DescEnv, String> =
        Modeler::from_described(&env);
    let a = LinExpr::var(Var::Base(DescVar::A));
    let c = LinExpr::var(Var::Base(DescVar::C));
    m.add_constraint((&a + &c).leq(&LinExpr::constant(1.0)), "a+c<=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));
    let model = m.build(&env).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    assert_eq!(cfg.get(InternalVar::Base(DescVar::A)).unwrap(), 0.0);
}

// ----- #[derive(DescribeVar)] tests -----------------------------------------

struct DeriveEnv {
    max_slot: i32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, DescribeVar)]
#[env(DeriveEnv)]
enum DeriveVar {
    Slot {
        #[range(0..env.max_slot)]
        slot: i32,
    },
    Flag {
        active: bool,
    },
}

#[test]
fn derive_enumerate() {
    let env = DeriveEnv { max_slot: 3 };
    let vars = DeriveVar::enumerate(&env);
    // 3 slots + 2 bools = 5 variables
    assert_eq!(vars.len(), 5);
    assert!(vars.contains_key(&DeriveVar::Slot { slot: 0 }));
    assert!(vars.contains_key(&DeriveVar::Slot { slot: 2 }));
    assert!(!vars.contains_key(&DeriveVar::Slot { slot: 3 }));
    assert!(vars.contains_key(&DeriveVar::Flag { active: true }));
    assert!(vars.contains_key(&DeriveVar::Flag { active: false }));
}

#[test]
fn derive_check_fix() {
    let env = DeriveEnv { max_slot: 3 };
    assert_eq!(DeriveVar::Slot { slot: 0 }.check_fix(&env), None);
    assert_eq!(DeriveVar::Slot { slot: 2 }.check_fix(&env), None);
    assert_eq!(DeriveVar::Slot { slot: 3 }.check_fix(&env), Some(0.0));
    assert_eq!(DeriveVar::Slot { slot: 100 }.check_fix(&env), Some(0.0));
    assert_eq!(DeriveVar::Flag { active: true }.check_fix(&env), None);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, DescribeVar)]
#[env(DeriveEnv)]
#[fix_with(5.0)]
enum DeriveVarCustomFix {
    #[var(Variable::integer())]
    Slot {
        #[range(0..env.max_slot)]
        slot: i32,
    },
}

#[test]
fn derive_custom_fix_with_and_var_type() {
    let env = DeriveEnv { max_slot: 2 };
    let vars = DeriveVarCustomFix::enumerate(&env);
    assert_eq!(vars.len(), 2);
    assert_eq!(
        *vars.get(&DeriveVarCustomFix::Slot { slot: 0 }).unwrap(),
        Variable::integer()
    );
    assert_eq!(
        DeriveVarCustomFix::Slot { slot: 99 }.check_fix(&env),
        Some(5.0)
    );
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, DescribeVar)]
#[env(DeriveEnv)]
enum DeriveVarDeferFix {
    #[defer_fix(if *slot >= 2 { Some(0.0) } else { None })]
    Slot {
        #[range(0..env.max_slot)]
        slot: i32,
    },
}

#[test]
fn derive_defer_fix() {
    let env = DeriveEnv { max_slot: 5 };
    let vars = DeriveVarDeferFix::enumerate(&env);
    // defer_fix filters: slots 0,1 are free; slots 2,3,4 are fixed
    assert_eq!(vars.len(), 2);
    assert!(vars.contains_key(&DeriveVarDeferFix::Slot { slot: 0 }));
    assert!(vars.contains_key(&DeriveVarDeferFix::Slot { slot: 1 }));
    assert!(!vars.contains_key(&DeriveVarDeferFix::Slot { slot: 2 }));
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, DescribeVar)]
#[env(DeriveEnv)]
enum DeriveVarOption {
    Slot {
        #[range(0..env.max_slot)]
        slot: Option<i32>,
    },
}

#[test]
fn derive_option_field() {
    let env = DeriveEnv { max_slot: 2 };
    let vars = DeriveVarOption::enumerate(&env);
    // None + Some(0) + Some(1) = 3 variables
    assert_eq!(vars.len(), 3);
    assert!(vars.contains_key(&DeriveVarOption::Slot { slot: None }));
    assert!(vars.contains_key(&DeriveVarOption::Slot { slot: Some(0) }));
    assert!(vars.contains_key(&DeriveVarOption::Slot { slot: Some(1) }));
}

#[test]
fn derive_integration_from_described() {
    let env = DeriveEnv { max_slot: 3 };
    let mut m: Modeler<'_, DeriveVar, String, String, DeriveEnv, String> =
        Modeler::from_described(&env);
    let s0 = LinExpr::var(Var::Base(DeriveVar::Slot { slot: 0 }));
    let s1 = LinExpr::var(Var::Base(DeriveVar::Slot { slot: 1 }));
    m.add_constraint((&s0 + &s1).leq(&LinExpr::constant(1.0)), "s0+s1<=1".into());
    m.add_objective(1.0, Objective::new(s0 + s1, ObjectiveSense::Maximize));
    let model = m.build(&env).unwrap();
    let cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");
    let sum = cfg
        .get(InternalVar::Base(DeriveVar::Slot { slot: 0 }))
        .unwrap_or(0.0)
        + cfg
            .get(InternalVar::Base(DeriveVar::Slot { slot: 1 }))
            .unwrap_or(0.0);
    assert_eq!(sum, 1.0);
}

// ----- checker problem tests ------------------------------------------------

#[test]
fn checker_excludes_objective_only_extras() {
    let mut m = fresh();
    m.declare_extra("s".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("a"))),
        ])
    })
    .unwrap();
    m.declare_extra("t".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("t")), ObjectiveSense::Maximize),
    );
    let model = m.build(&()).unwrap();

    // Full problem has both extras.
    assert!(
        model
            .problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("s".to_string()))
    );
    assert!(
        model
            .problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("t".to_string()))
    );

    // Checker problem has "s" but not "t".
    assert!(
        model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("s".to_string()))
    );
    assert!(
        !model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("t".to_string()))
    );

    // Checker problem still has all base variables.
    assert!(
        model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Base("a".to_string()))
    );
    assert!(
        model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Base("b".to_string()))
    );
}

#[test]
fn checker_includes_shared_extra() {
    let mut m = fresh();
    m.declare_extra("shared".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("a"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("shared")).leq(&LinExpr::constant(1.0)),
        "shared<=1".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("shared")), ObjectiveSense::Maximize),
    );
    let model = m.build(&()).unwrap();

    assert!(
        model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("shared".to_string()))
    );
}

#[test]
fn checker_includes_transitive_deps() {
    let mut m = fresh();
    m.declare_extra("leaf".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
        ])
    })
    .unwrap();
    m.declare_extra("mid".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(eextra("leaf"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("mid")).leq(&LinExpr::constant(1.0)),
        "mid<=1".into(),
    );
    let model = m.build(&()).unwrap();

    assert!(
        model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("mid".to_string()))
    );
    assert!(
        model
            .checker_problem()
            .get_variables()
            .contains_key(&InternalVar::<B, E>::Extra("leaf".to_string()))
    );
}

#[test]
fn checker_reconstruction_basic() {
    let mut m = fresh();
    m.declare_extra("s".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e))
                .eq(&(LinExpr::var(ebase("a")) + LinExpr::var(ebase("b")))),
        ])
    })
    .unwrap();
    m.declare_extra("t".to_string(), Variable::integer(), |_f, _ctx, e| {
        Ok(vec![
            LinExpr::var(ExtraVar::Extra(e)).eq(&LinExpr::var(ebase("b"))),
        ])
    })
    .unwrap();
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "s<=1".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(LinExpr::var(xtra("t")), ObjectiveSense::Maximize),
    );
    let model = m.build(&()).unwrap();

    let base_values = HashMap::from([("a".to_string(), 0.0), ("b".to_string(), 1.0)]);
    let checker_recon = model.checker_reconstruction_problem(&base_values).unwrap();
    let recon_cfg = ColloCbcSolver::new()
        .build_model(&checker_recon)
        .solve()
        .expect("solvable");

    // s should be reconstructed.
    assert_eq!(
        recon_cfg
            .get(InternalVar::<B, E>::Extra("s".to_string()))
            .unwrap(),
        1.0
    );
    // t should NOT be in the checker reconstruction.
    assert!(
        recon_cfg
            .get(InternalVar::<B, E>::Extra("t".to_string()))
            .is_none()
    );
}

#[test]
fn checker_no_extras() {
    let mut m = fresh();
    m.add_constraint(
        LinExpr::var(base("a")).leq(&LinExpr::constant(1.0)),
        "a<=1".into(),
    );
    let model = m.build(&()).unwrap();

    // Checker problem has the user constraint + base variables.
    assert_eq!(model.checker_problem().get_variables().len(), 2);
    assert_eq!(model.checker_problem().get_constraints().len(), 1);

    // Checker reconstruction is trivial.
    let base_values: HashMap<B, f64> = HashMap::new();
    let checker_recon = model.checker_reconstruction_problem(&base_values).unwrap();
    assert_eq!(checker_recon.get_constraints().len(), 0);
    assert_eq!(checker_recon.get_variables().len(), 0);
}

// ----- ModelDesc round-trip test -------------------------------------------

#[test]
fn model_desc_round_trip_preserves_solution() {
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Variable::binary());
    vars.insert("b".to_string(), Variable::binary());
    vars.insert("c".to_string(), Variable::binary());
    vars.insert("d".to_string(), Variable::binary());
    let mut m: Modeler<'_, B, E, C, (), String> = Modeler::new(vars);

    let a = LinExpr::var(base("a"));
    let b = LinExpr::var(base("b"));
    let c = LinExpr::var(base("c"));
    let d = LinExpr::var(base("d"));

    m.add_constraint((&a + &b).eq(&LinExpr::constant(1.0)), "a+b=1".into());
    m.add_constraint((&c + &d).eq(&LinExpr::constant(1.0)), "c+d=1".into());
    m.add_constraint((&a + &c).eq(&LinExpr::constant(1.0)), "a+c=1".into());
    m.add_constraint((&b + &d).eq(&LinExpr::constant(1.0)), "b+d=1".into());
    m.add_objective(1.0, Objective::new(a, ObjectiveSense::Maximize));

    let model = m.build(&()).unwrap();

    // Solve the original model.
    let original_cfg = ColloCbcSolver::new()
        .build_model(model.problem())
        .solve()
        .expect("solvable");

    // Simulate the IPC round-trip:
    // Parent side: extract var_order then build ModelDesc.
    let (_, parent_var_order) = model.problem().get_desc();
    let model_desc = model.to_desc();

    // Subprocess side: extract canonical var_order from var_descs, then rebuild.
    let subprocess_var_order: Vec<InternalVar<usize, usize>> = model_desc
        .main
        .var_descs
        .iter()
        .map(|d| d.to_internal_var())
        .collect();
    let rebuilt_model = model_desc.to_model();

    // Subprocess solves and encodes solution as Vec<f64> using var_descs order.
    let rebuilt_cfg = ColloCbcSolver::new()
        .build_model(rebuilt_model.problem())
        .solve()
        .expect("solvable");
    let solution_vec: Vec<f64> = subprocess_var_order
        .iter()
        .map(|iv| rebuilt_cfg.get(iv.clone()).unwrap_or(0.0))
        .collect();

    // Parent side: decode Vec<f64> back to ConfigData using its var_order.
    let round_tripped = collomatique_ilp::solution_to_config_data(&solution_vec, &parent_var_order);

    // Compare: both should give (a=1, b=0, c=0, d=1).
    for name in ["a", "b", "c", "d"] {
        let iv = InternalVar::<B, E>::Base(name.to_string());
        assert_eq!(
            original_cfg.get(iv.clone()).unwrap_or(0.0),
            round_tripped.get(iv).unwrap_or(0.0),
            "mismatch for variable {name}"
        );
    }
}
