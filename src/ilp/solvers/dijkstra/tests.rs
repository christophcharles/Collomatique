#[test]
fn test_dijkstra() {
    use crate::ilp::linexpr::Expr;
    use crate::ilp::{Config, ProblemBuilder};

    // We test on a simple schedule problem.
    //
    //

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
        //
        .add((&x11 + &y11).leq(&one))
        .add((&x12 + &y12).leq(&one))
        .add((&x21 + &y21).leq(&one))
        .add((&x22 + &y22).leq(&one))
        //
        .add((&x11 + &x21).leq(&one))
        .add((&x12 + &x22).leq(&one))
        .add((&y11 + &y21).leq(&one))
        .add((&y12 + &y22).leq(&one))
        //
        .add((&x11 + &x12).eq(&one))
        .add((&x21 + &x22).eq(&one))
        .add((&y11 + &y12).eq(&one))
        .add((&y21 + &y22).eq(&one))
        .build();
    let config = Config::from_iter(["x11", "y12", "y21"]);

    let dijkstra_solver = super::Solver::new(&pb);

    use crate::ilp::solvers::FeasabilitySolver;

    let solution = dijkstra_solver.restore_feasability(&config);

    assert_eq!(
        solution,
        Some(Config::from_iter(["x11", "y12", "y21", "x22",]))
    );
}
