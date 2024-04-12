#[test]
fn test_sa() {
    use crate::ilp::linexpr::Expr;
    use crate::ilp::{Config, ProblemBuilder};

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

    let x11 = Expr::var("x11");
    let x12 = Expr::var("x12");
    let x21 = Expr::var("x21");
    let x22 = Expr::var("x22");

    let y11 = Expr::var("y11");
    let y12 = Expr::var("y12");
    let y21 = Expr::var("y21");
    let y22 = Expr::var("y22");

    let one = Expr::constant(1);

    let pb = ProblemBuilder::new()
        // Both class should not attend a course at the same time
        .add((&x11 + &y11).leq(&one))
        .add((&x12 + &y12).leq(&one))
        .add((&x21 + &y21).leq(&one))
        .add((&x22 + &y22).leq(&one))
        // Each class should not attend more than one course at a given time
        .add((&x11 + &x21).leq(&one))
        .add((&x12 + &x22).leq(&one))
        .add((&y11 + &y21).leq(&one))
        .add((&y12 + &y22).leq(&one))
        // Each class must complete each course exactly once
        .add((&x11 + &x12).eq(&one))
        .add((&x21 + &x22).eq(&one))
        .add((&y11 + &y12).eq(&one))
        .add((&y21 + &y22).eq(&one))
        // eval func
        .eval_fn(crate::eval_fn!(|x| if x.get("y12") { 1.0 } else { 0.0 }))
        .build();

    let dijkstra_solver = crate::ilp::solvers::dijkstra::Solver::new(&pb);
    let mut sa_optimizer = super::Optimizer::new(&pb, dijkstra_solver);

    let config = Config::from_iter(["x11", "y12", "y21"]); // We choose a starting closer to the "bad" (high cost) solution
    sa_optimizer.set_init_config(config);
    sa_optimizer.set_max_iter(1); // There are only two solutions so only one iteration should even be enough to find the optimal one

    let mut random_gen = crate::ilp::random::DefaultRndGen::new();
    let solution = sa_optimizer.optimize(&mut random_gen);

    assert_eq!(
        Config::from(solution.expect("Solution found")),
        Config::from_iter(["x12", "y11", "y22", "x21"])
    );

    let config = Config::from_iter(["x12", "y11", "y22"]); // We choose a starting closer to the "good" (low cost) solution
    sa_optimizer.set_init_config(config);
    let solution = sa_optimizer.optimize(&mut random_gen);

    assert_eq!(
        Config::from(solution.expect("Solution found")),
        Config::from_iter(["x12", "y11", "y22", "x21"])
    );
}
