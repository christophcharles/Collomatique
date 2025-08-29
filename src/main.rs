use collomatique::*;

fn main() {
    let a = LinExpr::from_var("a");
    let b = LinExpr::from_var("b");
    let c = LinExpr::from_var("c");

    let expr = 2*a - 3*b + 4*c;
    println!("{}", expr);
}
