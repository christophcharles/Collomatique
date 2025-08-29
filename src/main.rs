use collomatique::*;
use collomatique::ilp::linexpr::Expr;

fn main() {
    let pb = ilp::ProblemBuilder::new()
        .add((2 * Expr::from("a") - 3 * Expr::from("b") + 4 * Expr::from("c") - 3).leq(&(2 * Expr::from("a") - 5 * Expr::from("d"))))
        .add((- Expr::from("a") + Expr::from("b") + 3 * Expr::from("c") + 3).leq(&(2 * Expr::from("a") - 5 * Expr::from("d"))))
        .add((2 * Expr::from("c") - 3 * Expr::from("d") + 4 * Expr::from("e") + 2).eq(&(-1 * Expr::from("e") + Expr::from("c"))))
        .build();

    println!("{}", pb);

    let solver = ilp::solvers::sa::Solver::new(&pb);

    println!("{:?}", solver);
}
