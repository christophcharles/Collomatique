use collomatique::eval_fn;
use collomatique::ilp::linexpr::Expr;
use collomatique::ilp::ProblemBuilder;

fn main() {
    let a = Expr::var("a");
    let b = Expr::var("b");

    let one = Expr::constant(1);

    let pb = ProblemBuilder::new()
        .add((&a + &b).leq(&one))
        .eval_fn(eval_fn!(|x| if x.get("a") { 1.0 } else { -1.0 }))
        .build();

    println!("{}", pb);
}
