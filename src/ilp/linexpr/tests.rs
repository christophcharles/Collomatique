use super::*;

use std::collections::BTreeMap;

#[test]
fn expr_display() {
    let expr = 2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c");
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c");

    let expr = 2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c") + 1;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + 1");

    let expr = 2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c") - 2;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + (-2)");

    let expr = Expr::constant(3);
    assert_eq!(format!("{}", expr), "3");

    let expr = Expr::constant(-42);
    assert_eq!(format!("{}", expr), "(-42)");
}

#[test]
fn expr_add() {
    let expr1 = Expr {
        coefs: BTreeMap::from([
            ("a".into(), -2),
            ("b".into(), 3),
        ]),
        constant: 2,
    };
    let expr2 = Expr {
        coefs: BTreeMap::from([
            ("b".into(), -4),
            ("c".into(), 5),
        ]),
        constant: 3,
    };

    let expr3 = Expr {
        coefs: BTreeMap::from([
            ("a".into(), -2),
            ("b".into(), -1),
            ("c".into(), 5),
        ]),
        constant: 5,
    };

    assert_eq!(expr1 + expr2, expr3);
}

#[test]
fn expr_sub() {
    let expr1 = Expr {
        coefs: BTreeMap::from([
            ("a".into(), -2),
            ("b".into(), 3),
        ]),
        constant: 2,
    };
    let expr2 = Expr {
        coefs: BTreeMap::from([
            ("b".into(), -4),
            ("c".into(), 5),
        ]),
        constant: 3,
    };

    let expr3 = Expr {
        coefs: BTreeMap::from([
            ("a".into(), -2),
            ("b".into(), 7),
            ("c".into(), -5),
        ]),
        constant: -1,
    };

    assert_eq!(expr1 - expr2, expr3);
}

#[test]
fn expr_mul() {
    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") + 2;
    let expr2 = -4 * Expr::var("a") + 6* Expr::var("b") + 4;
    assert_eq!(2*expr1, expr2);

    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") + 2;
    let expr2 = 6 * Expr::var("a") - 9 * Expr::var("b") - 6;
    assert_eq!((-3)*expr1, expr2);

    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") + 2;
    let expr2 = Expr::constant(0);
    assert_eq!(0*expr1, expr2);
}

#[test]
fn expr_reduce() {
    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") - 4 * Expr::var("c") + 1;

    let config = Config {
        values: BTreeMap::from([
            ("a".into(), true),
            ("c".into(), false),
        ]),
    };

    let expr2 = 3 * Expr::var("b") - 1;

    assert_eq!(expr1.reduce(&config), expr2);
}

#[test]
fn expr_to_value() {
    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") - 4 * Expr::var("c") + 1;
    let expr2 = Expr::constant(42);

    assert_eq!(expr1.to_value(), None);
    assert_eq!(expr2.to_value(), Some(42));
}

#[test]
fn expr_eval() {
    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") - 4 * Expr::var("c") + 1;
    let expr2 = -2* Expr::var("a") - 4 * Expr::var("c") + 1;

    let config = Config {
        values: BTreeMap::from([
            ("a".into(), true),
            ("c".into(), false),
        ]),
    };

    assert_eq!(expr1.eval(&config), None);
    assert_eq!(expr2.eval(&config), Some(-1));
}

#[test]
fn constraint_eval() {
    let expr1 = -2* Expr::var("a") + 3 *  Expr::var("b") - 4 * Expr::var("c") + 1;
    let expr2 = -2* Expr::var("a") - 4 * Expr::var("c") + 1;

    let constraint1 = expr1.leq(&Expr::constant(0));
    let constraint2 = expr2.eq(&Expr::constant(0));
    let constraint3 = expr2.geq(&Expr::constant(-2));

    let config = Config {
        values: BTreeMap::from([
            ("a".into(), true),
            ("c".into(), false),
        ]),
    };

    assert_eq!(constraint1.eval(&config), None);
    assert_eq!(constraint2.eval(&config), Some(false));
    assert_eq!(constraint3.eval(&config), Some(true));
}
