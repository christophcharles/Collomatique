use super::*;

#[test]
fn expr_display() {
    let expr = 2 * Expr::from("a") - 3 * Expr::from("b") + 4 * Expr::from("c");
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c");

    let expr = 2 * Expr::from("a") - 3 * Expr::from("b") + 4 * Expr::from("c") + 1;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + 1");

    let expr = 2 * Expr::from("a") - 3 * Expr::from("b") + 4 * Expr::from("c") - 2;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + (-2)");

    let expr = Expr::from(3);
    assert_eq!(format!("{}", expr), "3");

    let expr = Expr::from(-42);
    assert_eq!(format!("{}", expr), "(-42)");
}

#[test]
fn expr_add() {
    let expr1 = -2* Expr::from("a") + 3 *  Expr::from("b") + 2;
    let expr2 = -4 *  Expr::from("b") + 5 * Expr::from("c") + 3;

    let expr3 = -2 * Expr::from("a") -  Expr::from("b") + 5 * Expr::from("c") + 5;

    assert_eq!(expr1 + expr2, expr3);
}

#[test]
fn expr_sub() {
    let expr1 = -2* Expr::from("a") + 3 *  Expr::from("b") + 2;
    let expr2 = -4 *  Expr::from("b") + 5 * Expr::from("c") + 3;

    let expr3 = -2 * Expr::from("a") + 7* Expr::from("b") - 5 * Expr::from("c") - 1;

    assert_eq!(expr1 - expr2, expr3);
}

#[test]
fn expr_mul() {
    let expr1 = -2* Expr::from("a") + 3 *  Expr::from("b") + 2;
    let expr2 = -4 * Expr::from("a") + 6* Expr::from("b") + 4;
    assert_eq!(2*expr1, expr2);

    let expr1 = -2* Expr::from("a") + 3 *  Expr::from("b") + 2;
    let expr2 = 6 * Expr::from("a") - 9 * Expr::from("b") - 6;
    assert_eq!((-3)*expr1, expr2);

    let expr1 = -2* Expr::from("a") + 3 *  Expr::from("b") + 2;
    let expr2 = Expr::from(0);
    assert_eq!(0*expr1, expr2);
}
