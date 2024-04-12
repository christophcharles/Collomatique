use collomatique::*;
use collomatique::ilp::linexpr::Expr;

fn main() {
    let pb = ilp::ProblemBuilder::new()
        .add((2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c") - 3).leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))))
        .add((- Expr::var("a") + Expr::var("b") + 3 * Expr::var("c") + 3).leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))))
        .add((2 * Expr::var("c") - 3 * Expr::var("d") + 4 * Expr::var("e") + 2).eq(&(-1 * Expr::var("e") + Expr::var("c"))))
        .build();

    println!("{}", pb);

    let solver = ilp::solvers::sa::Solver::new(&pb);

    println!("{:?}", solver);
}
