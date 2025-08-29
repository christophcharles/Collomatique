#[test]
fn test_backtracking() {
    use crate::ilp::linexpr::Expr;
    use crate::ilp::ProblemBuilder;

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

    let x11 = Expr::<String>::var("x11");
    let x12 = Expr::<String>::var("x12");
    let x21 = Expr::<String>::var("x21");
    let x22 = Expr::<String>::var("x22");

    let y11 = Expr::<String>::var("y11");
    let y12 = Expr::<String>::var("y12");
    let y21 = Expr::<String>::var("y21");
    let y22 = Expr::<String>::var("y22");

    let one = Expr::<String>::constant(1);

    let pb = ProblemBuilder::new()
        .add_variables(["x11", "x12", "x21", "x22"])
        .unwrap()
        .add_variables(["y11", "y12", "y21", "y22"])
        .unwrap()
        // Both class should not attend a course at the same time
        .add_constraint((&x11 + &y11).leq(&one))
        .add_constraint((&x12 + &y12).leq(&one))
        .add_constraint((&x21 + &y21).leq(&one))
        .add_constraint((&x22 + &y22).leq(&one))
        // Each class should not attend more than one course at a given time
        .add_constraint((&x11 + &x21).leq(&one))
        .add_constraint((&x12 + &x22).leq(&one))
        .add_constraint((&y11 + &y21).leq(&one))
        .add_constraint((&y12 + &y22).leq(&one))
        // Each class must complete each course exactly once
        .add_constraint((&x11 + &x12).eq(&one))
        .add_constraint((&x21 + &x22).eq(&one))
        .add_constraint((&y11 + &y12).eq(&one))
        .add_constraint((&y21 + &y22).eq(&one))
        .build()
        .unwrap();
    let config = pb.default_config();

    let solver = super::Solver::new();

    use crate::ilp::solvers::FeasabilitySolver;

    let solution = solver.restore_feasability(&config);

    use std::collections::BTreeSet;
    let possible_solutions = BTreeSet::from([
        pb.config_from(["x11", "y12", "y21", "x22"]).unwrap(),
        pb.config_from(["x12", "y11", "y22", "x21"]).unwrap(),
    ]);

    assert!(possible_solutions.contains(&solution.expect("Solution should be found").into_inner()));
}
