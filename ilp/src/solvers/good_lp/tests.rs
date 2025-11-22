#[test]
fn good_lp() {
    use crate::{ConfigData, LinExpr, Objective, ObjectiveSense, ProblemBuilder, Variable};

    // We test on a simple scheduling problem.
    //
    // We have two student groups x and y.
    // They must both attend exactly once two different courses (1 and 2)
    // on the span of two weeks.
    // But the courses happen simultaneously.
    //
    // This means we must fill a timtable of the following form:
    //
    // ------------------------------
    // |          | Week 1 | Week 2 |
    // ------------------------------
    // | Course 1 |        |        |
    // ------------------------------
    // | Course 2 |        |        |
    // ------------------------------
    //
    // by putting an x or a y in each cell.
    //
    // We have three broad conditions :
    // - we should not put an x and a y in the same cell. But a cell can possibly be empty
    // - we should not put two xs or two ys in the same column (but column could have zero)
    // - we must put exactly one x and one y on each line
    //
    // We represent this with 8 boolean variables.
    // The variable xij is 1 if X is written in the cell on the line i and column j, 0 otherwise.
    // The same pattern is used for yij.

    let x11 = LinExpr::<String>::var("x11"); // Group X has course 1 on week 1
    let x12 = LinExpr::<String>::var("x12"); // Group X has course 1 on week 2
    let x21 = LinExpr::<String>::var("x21"); // Group X has course 2 on week 1
    let x22 = LinExpr::<String>::var("x22"); // Group X has course 2 on week 2

    let y11 = LinExpr::<String>::var("y11"); // Group Y has course 1 on week 1
    let y12 = LinExpr::<String>::var("y12"); // Group Y has course 1 on week 2
    let y21 = LinExpr::<String>::var("y21"); // Group Y has course 2 on week 1
    let y22 = LinExpr::<String>::var("y22"); // Group Y has course 2 on week 2

    let one = LinExpr::<String>::constant(1.0); // Constant for easier writing of constraints

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
        // Both class should not attend a course at the same time
        .add_constraints([
            (
                (&x11 + &y11).leq(&one),
                "At most one group in course 1 on week 1",
            ),
            (
                (&x12 + &y12).leq(&one),
                "At most one group in course 1 on week 2",
            ),
            (
                (&x21 + &y21).leq(&one),
                "At most one group in course 2 on week 1",
            ),
            (
                (&x22 + &y22).leq(&one),
                "At most one group in course 2 on week 2",
            ),
        ])
        // Each class should not attend more than one course at a given time
        .add_constraints([
            (
                (&x11 + &x21).leq(&one),
                "At most one course for group X on week 1",
            ),
            (
                (&x12 + &x22).leq(&one),
                "At most one course for group X on week 2",
            ),
            (
                (&y11 + &y21).leq(&one),
                "At most one course for group Y on week 1",
            ),
            (
                (&y12 + &y22).leq(&one),
                "At most one course for group Y on week 2",
            ),
        ])
        // Each class must complete each course exactly once
        .add_constraints([
            (
                (&x11 + &x12).eq(&one),
                "Group X should have course 1 exactly once",
            ),
            (
                (&x21 + &x22).eq(&one),
                "Group X should have course 2 exactly once",
            ),
            (
                (&y11 + &y12).eq(&one),
                "Group Y should have course 1 exactly once",
            ),
            (
                (&y21 + &y22).eq(&one),
                "Group Y should have course 2 exactly once",
            ),
        ])
        // Objective function : prefer group X in course 1 on week 1
        .set_objective(Objective::new(x11.clone(), ObjectiveSense::Maximize))
        .build()
        .unwrap();

    let solver = super::GoodSolver::new();

    use crate::solvers::Solver;

    let solution = solver.solve(&problem).expect("Solution should be found");

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
fn good_lp_impossible() {
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

    let solver = super::GoodSolver::new();

    use crate::solvers::Solver;

    let solution = solver.solve(&problem);

    assert!(solution.is_none());
}
