use std::collections::{HashMap, HashSet};

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::solvers::{Solver, SolverModel, collo_cbc::ColloCbcSolver};
use collomatique_ilp::{Objective, ObjectiveSense, Variable};

use collomatique_ilp_modeler::{ConstraintSource, ExtraVar, InternalVar, Model, Modeler, Var};

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

fn bset<const N: usize>(names: [&str; N]) -> HashSet<B> {
    names.iter().map(|s| s.to_string()).collect()
}

fn modeler_with_bases<'m>(names: &[&str]) -> Modeler<'m, B, E, C, (), String> {
    let mut vars = HashMap::new();
    for n in names {
        vars.insert(n.to_string(), Variable::binary());
    }
    Modeler::new(vars)
}

/// Declare an extra `name` defined by `name = sum(refs)`.
fn declare_sum(m: &mut Modeler<'_, B, E, C, (), String>, name: &str, refs: &[ExtraVar<B, E>]) {
    let refs: Vec<ExtraVar<B, E>> = refs.to_vec();
    m.declare_extra(name.to_string(), Variable::integer(), move |_f, _ctx, e| {
        let mut rhs = LinExpr::constant(0.0);
        for r in &refs {
            rhs = rhs + LinExpr::var(r.clone());
        }
        Ok(vec![LinExpr::var(ExtraVar::Extra(e)).eq(&rhs)])
    })
    .unwrap();
}

/// The dependency graph reflects the transitive *base* footprint of every
/// expanded extra: chains, pure extras and diamonds; and extras never
/// expanded are absent.
#[test]
fn dependency_graph_transitive_footprints() {
    let mut m = modeler_with_bases(&["b1", "b2", "b3", "b4"]);

    // e2 = b2 + b3 ; e1 = b1 + e2   (chain)
    declare_sum(&mut m, "e2", &[ebase("b2"), ebase("b3")]);
    declare_sum(&mut m, "e1", &[ebase("b1"), eextra("e2")]);
    // pe = b4   (pure extra)
    declare_sum(&mut m, "pe", &[ebase("b4")]);
    // da = b1 ; db1 = da ; db2 = da ; dtop = db1 + db2   (diamond over b1)
    declare_sum(&mut m, "da", &[ebase("b1")]);
    declare_sum(&mut m, "db1", &[eextra("da")]);
    declare_sum(&mut m, "db2", &[eextra("da")]);
    declare_sum(&mut m, "dtop", &[eextra("db1"), eextra("db2")]);
    // dead = b1   (declared but never referenced → not expanded)
    declare_sum(&mut m, "dead", &[ebase("b1")]);

    // One user constraint pulls in e1, pe and dtop.
    let expr = LinExpr::var(xtra("e1")) + LinExpr::var(xtra("pe")) + LinExpr::var(xtra("dtop"));
    m.add_constraint(expr.leq(&LinExpr::constant(10.0)), "use".into());

    let model = m.build(&()).unwrap();
    let g = model.dependency_graph();

    assert_eq!(*g.base_footprint(&"e2".to_string()), bset(["b2", "b3"]));
    assert_eq!(
        *g.base_footprint(&"e1".to_string()),
        bset(["b1", "b2", "b3"])
    );
    assert_eq!(*g.base_footprint(&"pe".to_string()), bset(["b4"]));
    // Diamond: b1 counted once through both db1 and db2.
    assert_eq!(*g.base_footprint(&"dtop".to_string()), bset(["b1"]));
    assert_eq!(*g.base_footprint(&"da".to_string()), bset(["b1"]));

    // `dead` was never expanded, so it has no footprint recorded.
    assert!(g.base_footprint(&"dead".to_string()).is_empty());

    // var_footprint works on the user-facing Var view.
    assert_eq!(g.var_footprint(&base("b1")), bset(["b1"]));
    assert_eq!(g.var_footprint(&xtra("e1")), bset(["b1", "b2", "b3"]));
    // ... and on the flattened InternalVar view.
    assert_eq!(
        g.var_footprint(&InternalVar::<B, E>::Extra("e2".to_string())),
        bset(["b2", "b3"])
    );
}

/// `build_full` keeps declared-but-unreferenced extras (and their
/// footprint); a normal `build` drops them.
#[test]
fn build_full_retains_unreferenced_extra() {
    let make = || {
        let mut m = modeler_with_bases(&["b1", "b2"]);
        declare_sum(&mut m, "lonely", &[ebase("b1")]);
        // A user constraint that does not reference `lonely`.
        m.add_constraint(
            LinExpr::var(base("b2")).leq(&LinExpr::constant(1.0)),
            "keep-alive".into(),
        );
        m
    };

    // Normal build: `lonely` never expanded.
    let normal = make().build(&()).unwrap();
    assert!(
        normal
            .dependency_graph()
            .base_footprint(&"lonely".to_string())
            .is_empty()
    );
    assert!(
        !normal
            .problem()
            .get_variables()
            .keys()
            .any(|v| matches!(v, InternalVar::Extra(e) if e == "lonely"))
    );

    // build_full: `lonely` expanded and present.
    let full = make().build_full(&()).unwrap();
    assert_eq!(
        *full
            .dependency_graph()
            .base_footprint(&"lonely".to_string()),
        bset(["b1"])
    );
    assert!(
        full.problem()
            .get_variables()
            .keys()
            .any(|v| matches!(v, InternalVar::Extra(e) if e == "lonely"))
    );
}

/// A model set up for the `Model::filter` tests: base b1/b2, extras
/// s = b1 and t = b2, user constraints over each, and an objective over
/// all of them.
fn filter_fixture<'m>() -> Model<B, E, C> {
    let mut m = modeler_with_bases(&["b1", "b2"]);
    declare_sum(&mut m, "s", &[ebase("b1")]);
    declare_sum(&mut m, "t", &[ebase("b2")]);

    m.add_constraint(
        LinExpr::var(base("b1")).leq(&LinExpr::constant(1.0)),
        "u1".into(),
    );
    m.add_constraint(
        LinExpr::var(base("b2")).leq(&LinExpr::constant(1.0)),
        "u2".into(),
    );
    m.add_constraint(
        LinExpr::var(xtra("s")).leq(&LinExpr::constant(1.0)),
        "u3".into(),
    );
    m.add_constraint(
        LinExpr::var(xtra("t")).leq(&LinExpr::constant(1.0)),
        "u4".into(),
    );
    m.add_objective(
        1.0,
        Objective::new(
            LinExpr::var(base("b1")) + LinExpr::var(base("b2")) + LinExpr::var(xtra("s")),
            ObjectiveSense::Maximize,
        ),
    );
    m.build(&()).unwrap()
}

/// `Model::filter` keeps user constraints/objective terms by their base
/// footprint (callbacks work at the `Var` level), keeps every
/// extra-defining constraint, and prunes now-unreferenced base variables.
#[test]
fn model_filter_by_footprint() {
    let model = filter_fixture();
    let graph = model.dependency_graph();
    let blessed = bset(["b1"]);

    let filtered = model.filter(
        // `c` is a Constraint<Var<B, E>> — proven by calling the Var-level
        // constraint_footprint on it.
        |c: &_, _desc| graph.constraint_footprint(c).is_subset(&blessed),
        |v: &Var<B, E>| graph.var_footprint(v).is_subset(&blessed),
    );

    // User constraints kept: u1 ({b1}) and u3 (s -> {b1}). Dropped: u2
    // ({b2}) and u4 (t -> {b2}).
    let kept_user: HashSet<&String> = filtered
        .get_constraints()
        .iter()
        .filter_map(|(_, src)| match src {
            ConstraintSource::User(desc) => Some(desc),
            _ => None,
        })
        .collect();
    assert_eq!(
        kept_user,
        HashSet::from([&"u1".to_string(), &"u3".to_string()])
    );

    // Every extra-defining constraint is still there (s and t both).
    let defining_extras: HashSet<&String> = filtered
        .get_constraints()
        .iter()
        .filter_map(|(_, src)| match src {
            ConstraintSource::DefiningExtra { extra, .. } => Some(extra),
            _ => None,
        })
        .collect();
    assert_eq!(
        defining_extras,
        HashSet::from([&"s".to_string(), &"t".to_string()])
    );

    // Objective keeps b1 and s, drops b2.
    let obj_vars: HashSet<&InternalVar<B, E>> = filtered
        .get_objective()
        .get_function()
        .variable_refs()
        .collect();
    assert!(obj_vars.contains(&InternalVar::Base("b1".to_string())));
    assert!(obj_vars.contains(&InternalVar::Extra("s".to_string())));
    assert!(!obj_vars.contains(&InternalVar::Base("b2".to_string())));
}

/// Round-trip: filter → from_model_problem → build sheds extras whose only
/// user references were filtered out, and the minimal sub-problem solves.
#[test]
fn filter_roundtrip_drops_dead_extras() {
    let model = filter_fixture();
    let graph = model.dependency_graph();
    let blessed = bset(["b1"]);

    let filtered = model.filter(
        |c: &_, _desc| graph.constraint_footprint(c).is_subset(&blessed),
        |v: &Var<B, E>| graph.var_footprint(v).is_subset(&blessed),
    );

    // Re-model from the filtered problem and rebuild. `t` is no longer
    // referenced by any user constraint or the objective, so it is dropped.
    let remodeled: Modeler<'_, B, E, C, (), String> = Modeler::from_model_problem(&filtered);
    let rebuilt = remodeled.build(&()).unwrap();

    let extras: HashSet<&String> = rebuilt
        .problem()
        .get_variables()
        .keys()
        .filter_map(|v| match v {
            InternalVar::Extra(e) => Some(e),
            _ => None,
        })
        .collect();
    assert!(extras.contains(&"s".to_string()), "s should survive");
    assert!(!extras.contains(&"t".to_string()), "t should be dropped");

    // The minimal sub-problem still solves.
    let cfg = ColloCbcSolver::new()
        .build_model(rebuilt.problem())
        .solve()
        .expect("solvable");
    assert_eq!(
        cfg.get(InternalVar::<B, E>::Extra("s".to_string()))
            .unwrap(),
        1.0
    );
}
