use std::collections::HashSet;

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::{BuildError, Objective, ObjectiveSense, ProblemBuilder, Variable};

/// `Problem::filter` keeps exactly the predicate-selected constraints,
/// variables and objective terms — nothing is auto-pruned.
#[test]
fn problem_filter_keeps_exactly_what_is_asked() {
    let a = LinExpr::<String>::var("A");
    let b = LinExpr::<String>::var("B");
    let c = LinExpr::<String>::var("C");

    let c1 = (a.clone() + b.clone()).leq(&LinExpr::constant(1.0));
    let c2 = (b.clone() + c.clone()).leq(&LinExpr::constant(1.0));

    let problem = ProblemBuilder::<String, String>::new()
        .set_variable("A", Variable::binary())
        .set_variable("B", Variable::binary())
        .set_variable("C", Variable::binary())
        .add_constraint(c1, "c1".to_string())
        .add_constraint(c2, "c2".to_string())
        .set_objective(Objective::new(a + b + c, ObjectiveSense::Maximize))
        .build()
        .expect("valid problem");

    // Keep only c1; keep all three variables; keep only A in the objective.
    let filtered = problem
        .filter(|_c, desc| desc == "c1", |_v| true, |v| v == "A")
        .expect("consistent");

    // Only c1 survives.
    assert_eq!(filtered.get_constraints().len(), 1);
    assert_eq!(filtered.get_constraints()[0].1, "c1");

    // Objective keeps only A.
    let obj_vars: HashSet<&String> = filtered
        .get_objective()
        .get_function()
        .variable_refs()
        .collect();
    assert_eq!(obj_vars, HashSet::from([&"A".to_string()]));

    // Nothing auto-pruned: C is still declared even though no surviving
    // constraint or objective term references it.
    let vars: HashSet<&String> = filtered.get_variables().keys().collect();
    assert_eq!(
        vars,
        HashSet::from([&"A".to_string(), &"B".to_string(), &"C".to_string()])
    );
}

/// A variable may be dropped explicitly, and the result stays consistent as
/// long as nothing kept still references it.
#[test]
fn problem_filter_drops_unreferenced_variable() {
    let a = LinExpr::<String>::var("A");
    let b = LinExpr::<String>::var("B");
    let c = LinExpr::<String>::var("C");

    let c1 = (a.clone() + b.clone()).leq(&LinExpr::constant(1.0));
    let c2 = (b.clone() + c.clone()).leq(&LinExpr::constant(1.0));

    let problem = ProblemBuilder::<String, String>::new()
        .set_variable("A", Variable::binary())
        .set_variable("B", Variable::binary())
        .set_variable("C", Variable::binary())
        .add_constraint(c1, "c1".to_string())
        .add_constraint(c2, "c2".to_string())
        .set_objective(Objective::new(a + b, ObjectiveSense::Maximize))
        .build()
        .expect("valid problem");

    // Drop c2 (the only user of C) and drop C. Consistent.
    let filtered = problem
        .filter(|_c, desc| desc == "c1", |v| v != "C", |_v| true)
        .expect("consistent");
    let vars: HashSet<&String> = filtered.get_variables().keys().collect();
    assert_eq!(vars, HashSet::from([&"A".to_string(), &"B".to_string()]));
}

/// Dropping a variable still referenced by a kept constraint is reported as
/// an error rather than silently repaired.
#[test]
fn problem_filter_inconsistent_constraint_errors() {
    let a = LinExpr::<String>::var("A");
    let b = LinExpr::<String>::var("B");
    let c1 = (a.clone() + b.clone()).leq(&LinExpr::constant(1.0));

    let problem = ProblemBuilder::<String, String>::new()
        .set_variable("A", Variable::binary())
        .set_variable("B", Variable::binary())
        .add_constraint(c1, "c1".to_string())
        .set_objective(Objective::new(a, ObjectiveSense::Maximize))
        .build()
        .expect("valid problem");

    // Keep c1 (uses B) but drop B.
    let err = problem
        .filter(|_c, _desc| true, |v| v != "B", |_v| true)
        .unwrap_err();
    match err {
        BuildError::UndeclaredVariableInConstraint(v, _, _) => assert_eq!(v, "B"),
        other => panic!("expected UndeclaredVariableInConstraint, got {other:?}"),
    }
}

/// Dropping a variable still referenced by a kept objective term errors too.
#[test]
fn problem_filter_inconsistent_objective_errors() {
    let a = LinExpr::<String>::var("A");
    let b = LinExpr::<String>::var("B");

    let problem = ProblemBuilder::<String, String>::new()
        .set_variable("A", Variable::binary())
        .set_variable("B", Variable::binary())
        .add_constraint(a.clone().leq(&LinExpr::constant(1.0)), "c1".to_string())
        .set_objective(Objective::new(a + b, ObjectiveSense::Maximize))
        .build()
        .expect("valid problem");

    // Keep the whole objective (uses B) but drop B.
    let err = problem
        .filter(|_c, _desc| true, |v| v != "B", |_v| true)
        .unwrap_err();
    match err {
        BuildError::UndeclaredVariableInObjFunc(v, _) => assert_eq!(v, "B"),
        other => panic!("expected UndeclaredVariableInObjFunc, got {other:?}"),
    }
}

/// Keeping everything is an identity on constraints/variables.
#[test]
fn problem_filter_keep_all_is_identity() {
    let a = LinExpr::<String>::var("A");
    let b = LinExpr::<String>::var("B");
    let problem = ProblemBuilder::<String, String>::new()
        .set_variable("A", Variable::binary())
        .set_variable("B", Variable::binary())
        .add_constraint(
            (a.clone() + b.clone()).leq(&LinExpr::constant(1.0)),
            "c".to_string(),
        )
        .set_objective(Objective::new(a + b, ObjectiveSense::Maximize))
        .build()
        .expect("valid problem");

    let same = problem
        .filter(|_, _| true, |_| true, |_| true)
        .expect("consistent");
    assert_eq!(same.get_constraints().len(), 1);
    assert_eq!(same.get_variables().len(), 2);
}
