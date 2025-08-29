#[test]
fn test_sa() {
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
    //
    // We add a cost function: putting y on the first course of the second week costs "1.0".
    // So, the prefered solution should be : ["x12", "y11", "y22", "x21"]

    let x11 = Expr::<String>::var("x11");
    let x12 = Expr::<String>::var("x12");
    let x21 = Expr::<String>::var("x21");
    let x22 = Expr::<String>::var("x22");

    let y11 = Expr::<String>::var("y11");
    let y12 = Expr::<String>::var("y12");
    let y21 = Expr::<String>::var("y21");
    let y22 = Expr::<String>::var("y22");

    let one = Expr::<String>::constant(1);

    let pb = ProblemBuilder::<String>::new()
        .add_variables(["x11", "x12", "x21", "x22"])
        .unwrap()
        .add_variables(["y11", "y12", "y21", "y22"])
        .unwrap()
        // Both class should not attend a course at the same time
        .add_constraint((&x11 + &y11).leq(&one))
        .unwrap()
        .add_constraint((&x12 + &y12).leq(&one))
        .unwrap()
        .add_constraint((&x21 + &y21).leq(&one))
        .unwrap()
        .add_constraint((&x22 + &y22).leq(&one))
        .unwrap()
        // Each class should not attend more than one course at a given time
        .add_constraint((&x11 + &x21).leq(&one))
        .unwrap()
        .add_constraint((&x12 + &x22).leq(&one))
        .unwrap()
        .add_constraint((&y11 + &y21).leq(&one))
        .unwrap()
        .add_constraint((&y12 + &y22).leq(&one))
        .unwrap()
        // Each class must complete each course exactly once
        .add_constraint((&x11 + &x12).eq(&one))
        .unwrap()
        .add_constraint((&x21 + &x22).eq(&one))
        .unwrap()
        .add_constraint((&y11 + &y12).eq(&one))
        .unwrap()
        .add_constraint((&y21 + &y22).eq(&one))
        .unwrap()
        // eval func
        .eval_fn(crate::debuggable!(|x| if x.get("y12").unwrap() {
            1000.0
        } else {
            0.0
        }))
        .build();

    let mut sa_optimizer = super::Optimizer::new(&pb);

    let config = pb.config_from(["x11", "y12", "y21"]).unwrap(); // We choose a starting closer to the "bad" (high cost) solution
    sa_optimizer.set_init_config(config);

    let mut random_gen = crate::ilp::random::DefaultRndGen::new();
    let solver = crate::ilp::solvers::a_star::Solver::new();
    let solution = sa_optimizer.iterate(solver, &mut random_gen).best_in(2);
    // There are only two solutions so only two iterations should even be enough to find the optimal one

    assert_eq!(
        solution.expect("Solution found").0.inner().clone(),
        pb.config_from(["x12", "y11", "y22", "x21"]).unwrap()
    );

    let config = pb.config_from(["x12", "y11", "y22"]).unwrap(); // We choose a starting closer to the "good" (low cost) solution
    sa_optimizer.set_init_config(config);

    let solver = crate::ilp::solvers::a_star::Solver::new();
    let solution = sa_optimizer.iterate(solver, &mut random_gen).best_in(2);

    assert_eq!(
        solution.expect("Solution found").0.inner().clone(),
        pb.config_from(["x12", "y11", "y22", "x21"]).unwrap()
    );
}
