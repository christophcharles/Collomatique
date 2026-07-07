use std::collections::HashSet;

use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp::{Objective, ObjectiveSense, ProblemBuilder, Variable};

/// `Problem::filter` keeps predicate-selected constraints, drops objective
/// terms by variable, and prunes variables that become unreferenced.
#[test]
fn problem_filter_keeps_prunes_and_drops_objective() {
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

    // Keep only c1; keep only A in the objective.
    let filtered = problem.filter(|_c, desc| desc == "c1", |v| v == "A");

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

    // C is pruned: it appeared only in the dropped c2 and the dropped
    // objective term. A and B remain (referenced by c1).
    let vars: HashSet<&String> = filtered.get_variables().keys().collect();
    assert_eq!(vars, HashSet::from([&"A".to_string(), &"B".to_string()]));
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

    let same = problem.filter(|_, _| true, |_| true);
    assert_eq!(same.get_constraints().len(), 1);
    assert_eq!(same.get_variables().len(), 2);
}
