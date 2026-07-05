#[test]
fn collo_cbc() {
    use crate::{ConfigData, LinExpr, Objective, ObjectiveSense, ProblemBuilder, Variable};

    let x11 = LinExpr::<String>::var("x11");
    let x12 = LinExpr::<String>::var("x12");
    let x21 = LinExpr::<String>::var("x21");
    let x22 = LinExpr::<String>::var("x22");

    let y11 = LinExpr::<String>::var("y11");
    let y12 = LinExpr::<String>::var("y12");
    let y21 = LinExpr::<String>::var("y21");
    let y22 = LinExpr::<String>::var("y22");

    let one = LinExpr::<String>::constant(1.0);

    let problem = ProblemBuilder::<String, String>::new()
        .set_variables([
            ("x11", Variable::binary()),
            ("x12", Variable::binary()),
            ("x21", Variable::binary()),
            ("x22", Variable::binary()),
        ])
        .set_variables([
            ("y11", Variable::binary()),
            ("y12", Variable::binary()),
            ("y21", Variable::binary()),
            ("y22", Variable::binary()),
        ])
        .add_constraints([
            ((&x11 + &y11).leq(&one), ""),
            ((&x12 + &y12).leq(&one), ""),
            ((&x21 + &y21).leq(&one), ""),
            ((&x22 + &y22).leq(&one), ""),
        ])
        .add_constraints([
            ((&x11 + &x21).leq(&one), ""),
            ((&x12 + &x22).leq(&one), ""),
            ((&y11 + &y21).leq(&one), ""),
            ((&y12 + &y22).leq(&one), ""),
        ])
        .add_constraints([
            ((&x11 + &x12).eq(&one), ""),
            ((&x21 + &x22).eq(&one), ""),
            ((&y11 + &y12).eq(&one), ""),
            ((&y21 + &y22).eq(&one), ""),
        ])
        .set_objective(Objective::new(x11.clone(), ObjectiveSense::Maximize))
        .build()
        .unwrap();

    let solver = super::ColloCbcSolver::new();

    use crate::solvers::{Solver, SolverModel};

    let solution = solver
        .build_model(&problem)
        .solve()
        .expect("Solution should be found");

    let expected_solution_data = ConfigData::new().set_iter([
        ("x11", 1.0),
        ("x12", 0.0),
        ("x21", 0.0),
        ("x22", 1.0),
        ("y11", 0.0),
        ("y12", 1.0),
        ("y21", 1.0),
        ("y22", 0.0),
    ]);

    let expected_solution = problem
        .build_config(expected_solution_data)
        .expect("No variables should be missing");

    assert!(solution.into_inner() == expected_solution);
}

#[test]
fn collo_cbc_impossible() {
    use crate::{LinExpr, ProblemBuilder, Variable};

    let x11 = LinExpr::<String>::var("x11");
    let x12 = LinExpr::<String>::var("x12");
    let x21 = LinExpr::<String>::var("x21");
    let x22 = LinExpr::<String>::var("x22");

    let one = LinExpr::<String>::constant(1.0);

    let problem = ProblemBuilder::<String, String>::new()
        .set_variables([
            ("x11", Variable::binary()),
            ("x12", Variable::binary()),
            ("x21", Variable::binary()),
            ("x22", Variable::binary()),
        ])
        .add_constraints([
            ((&x11 + &x12).eq(&one), ""),
            ((&x21 + &x22).eq(&one), ""),
            ((&x11 + &x21).eq(&one), ""),
            ((&x12 + &x22).eq(&one), ""),
            ((&x11 + &x22).eq(&one), ""),
        ])
        .build()
        .unwrap();

    let solver = super::ColloCbcSolver::new();

    use crate::solvers::{Solver, SolverModel};

    let solution = solver.build_model(&problem).solve();

    assert!(solution.is_none());
}

#[test]
fn collo_cbc_warm_start() {
    use crate::{ConfigData, LinExpr, Objective, ObjectiveSense, ProblemBuilder, Variable};

    let x11 = LinExpr::<String>::var("x11");
    let x12 = LinExpr::<String>::var("x12");
    let x21 = LinExpr::<String>::var("x21");
    let x22 = LinExpr::<String>::var("x22");

    let y11 = LinExpr::<String>::var("y11");
    let y12 = LinExpr::<String>::var("y12");
    let y21 = LinExpr::<String>::var("y21");
    let y22 = LinExpr::<String>::var("y22");

    let one = LinExpr::<String>::constant(1.0);

    let problem = ProblemBuilder::<String, String>::new()
        .set_variables([
            ("x11", Variable::binary()),
            ("x12", Variable::binary()),
            ("x21", Variable::binary()),
            ("x22", Variable::binary()),
        ])
        .set_variables([
            ("y11", Variable::binary()),
            ("y12", Variable::binary()),
            ("y21", Variable::binary()),
            ("y22", Variable::binary()),
        ])
        .add_constraints([
            ((&x11 + &y11).leq(&one), ""),
            ((&x12 + &y12).leq(&one), ""),
            ((&x21 + &y21).leq(&one), ""),
            ((&x22 + &y22).leq(&one), ""),
        ])
        .add_constraints([
            ((&x11 + &x21).leq(&one), ""),
            ((&x12 + &x22).leq(&one), ""),
            ((&y11 + &y21).leq(&one), ""),
            ((&y12 + &y22).leq(&one), ""),
        ])
        .add_constraints([
            ((&x11 + &x12).eq(&one), ""),
            ((&x21 + &x22).eq(&one), ""),
            ((&y11 + &y12).eq(&one), ""),
            ((&y21 + &y22).eq(&one), ""),
        ])
        .set_objective(Objective::new(x11.clone(), ObjectiveSense::Maximize))
        .build()
        .unwrap();

    let hint = ConfigData::new().set_iter([
        ("x11", 1.0),
        ("x12", 0.0),
        ("x21", 0.0),
        ("x22", 1.0),
        ("y11", 0.0),
        ("y12", 1.0),
        ("y21", 1.0),
        ("y22", 0.0),
    ]);

    let solver = super::ColloCbcSolver::new();

    use crate::solvers::{Solver, SolverModel, WarmSolver};

    let cold_solution = solver
        .build_model(&problem)
        .solve()
        .expect("Cold solve should find a solution");

    let warm_solution = solver
        .build_warm_model(&problem, &hint)
        .solve()
        .expect("Warm solve should find a solution");

    assert!(cold_solution.into_inner() == warm_solution.into_inner());
}

#[test]
fn collo_cbc_warm_start_partial_hint() {
    use crate::{ConfigData, LinExpr, Objective, ObjectiveSense, ProblemBuilder, Variable};

    let x = LinExpr::<String>::var("x");
    let y = LinExpr::<String>::var("y");
    let one = LinExpr::<String>::constant(1.0);

    let problem = ProblemBuilder::<String, String>::new()
        .set_variables([("x", Variable::binary()), ("y", Variable::binary())])
        .add_constraint((&x + &y).leq(&one), "")
        .set_objective(Objective::new(&x + &y, ObjectiveSense::Maximize))
        .build()
        .unwrap();

    let partial_hint = ConfigData::new().set("x", 1.0);

    let solver = super::ColloCbcSolver::new();

    use crate::solvers::{SolverModel, WarmSolver};

    let solution = solver
        .build_warm_model(&problem, &partial_hint)
        .solve()
        .expect("Warm solve with partial hint should find a solution");

    assert_eq!(solution.eval(), 1.0);
}

#[test]
fn collo_cbc_callback_not_stopped() {
    use crate::{LinExpr, Objective, ObjectiveSense, ProblemBuilder, Variable};

    let x = LinExpr::<String>::var("x");
    let y = LinExpr::<String>::var("y");
    let one = LinExpr::<String>::constant(1.0);

    let problem = ProblemBuilder::<String, String>::new()
        .set_variables([("x", Variable::binary()), ("y", Variable::binary())])
        .add_constraint((&x + &y).leq(&one), "")
        .set_objective(Objective::new(&x + &y, ObjectiveSense::Maximize))
        .build()
        .unwrap();

    let solver = super::ColloCbcSolver::new();

    use crate::solvers::{CallbackSolverModel, Solver};

    let result = solver.build_model(&problem).solve_with_callback(|_| true);

    assert!(!result.stopped_by_callback);
    assert!(result.config.is_some());
    assert_eq!(result.config.unwrap().eval(), 1.0);
}

#[test]
fn collo_cbc_callback_incumbent_data_is_feasible() {
    use crate::solvers::{CallbackSolverModel, ProgressIncumbentData, Solver};
    use crate::{ConfigData, LinExpr, Objective, ObjectiveSense, ProblemBuilder, Variable};

    // Small knapsack: maximize value subject to a weight cap, so CBC has to
    // report at least one integer incumbent during the solve.
    let a = LinExpr::<String>::var("a");
    let b = LinExpr::<String>::var("b");
    let c = LinExpr::<String>::var("c");
    let d = LinExpr::<String>::var("d");
    let e = LinExpr::<String>::var("e");

    let weight = 2.0 * &a + 3.0 * &b + 4.0 * &c + 5.0 * &d + 6.0 * &e;
    let value = 3.0 * &a + 4.0 * &b + 5.0 * &c + 6.0 * &d + 7.0 * &e;
    let cap = LinExpr::<String>::constant(10.0);

    let problem = ProblemBuilder::<String, String>::new()
        .set_variables([
            ("a", Variable::binary()),
            ("b", Variable::binary()),
            ("c", Variable::binary()),
            ("d", Variable::binary()),
            ("e", Variable::binary()),
        ])
        .add_constraint(weight.leq(&cap), "capacity")
        .set_objective(Objective::new(value, ObjectiveSense::Maximize))
        .build()
        .unwrap();

    let solver = super::ColloCbcSolver::new();

    let mut last_incumbent: Option<ConfigData<String>> = None;
    let result = solver
        .build_model(&problem)
        .solve_with_callback(|progress| {
            if let Some(config) = progress.incumbent_data() {
                last_incumbent = Some(config.clone());
            }
            true
        });

    assert!(!result.stopped_by_callback);
    assert!(result.config.is_some());

    let incumbent = last_incumbent.expect("an incumbent ConfigData should be reported");

    // The reported incumbent must build into the problem (i.e. cover exactly the
    // problem's variables) and satisfy every hard constraint.
    let config = problem
        .build_config(incumbent)
        .expect("incumbent should build into the problem");
    assert!(config.is_feasible(), "reported incumbent must be feasible");
}
