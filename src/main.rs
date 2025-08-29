use collomatique::*;

fn main() {
    let expr1 = - (2 * LinExpr::from("a") - 3 * LinExpr::from("b") + 4 * LinExpr::from("c") + 2);
    println!("{}", expr1);

    let expr2 = -3 * LinExpr::from("c") + 42 * LinExpr::from("d") - 5;
    println!("{}", expr2);

    let constraint1 = expr1.leq(&expr2);
    println!("{}", constraint1);

    let constraint2 = expr1.geq(&expr2);
    println!("{}", constraint2);

    let constraint3 = expr1.eq(&expr2);
    println!("{}", constraint3);
}
