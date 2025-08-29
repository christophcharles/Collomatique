use super::*;

use std::collections::BTreeMap;

#[test]
fn expr_display() {
    let expr =
        2 * Expr::<String>::var("a") - 3 * Expr::<String>::var("b") + 4 * Expr::<String>::var("c");
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c");

    let expr = 2 * Expr::<String>::var("a") - 3 * Expr::<String>::var("b")
        + 4 * Expr::<String>::var("c")
        + 1;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + 1");

    let expr = 2 * Expr::<String>::var("a") - 3 * Expr::<String>::var("b")
        + 4 * Expr::<String>::var("c")
        - 2;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + (-2)");

    let expr = Expr::<String>::constant(3);
    assert_eq!(format!("{}", expr), "3");

    let expr = Expr::<String>::constant(-42);
    assert_eq!(format!("{}", expr), "(-42)");
}

#[test]
fn expr_add() {
    let expr1 = Expr::<String> {
        coefs: BTreeMap::from([("a".into(), -2), ("b".into(), 3)]),
        constant: 2,
    };
    let expr2 = Expr::<String> {
        coefs: BTreeMap::from([("b".into(), -4), ("c".into(), 5)]),
        constant: 3,
    };

    let expr3 = Expr::<String> {
        coefs: BTreeMap::from([("a".into(), -2), ("b".into(), -1), ("c".into(), 5)]),
        constant: 5,
    };

    assert_eq!(expr1 + expr2, expr3);
}

#[test]
fn expr_sub() {
    let expr1 = Expr::<String> {
        coefs: BTreeMap::from([("a".into(), -2), ("b".into(), 3)]),
        constant: 2,
    };
    let expr2 = Expr::<String> {
        coefs: BTreeMap::from([("b".into(), -4), ("c".into(), 5)]),
        constant: 3,
    };

    let expr3 = Expr::<String> {
        coefs: BTreeMap::from([("a".into(), -2), ("b".into(), 7), ("c".into(), -5)]),
        constant: -1,
    };

    assert_eq!(expr1 - expr2, expr3);
}

#[test]
fn expr_mul() {
    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b") + 2;
    let expr2 = -4 * Expr::<String>::var("a") + 6 * Expr::<String>::var("b") + 4;
    assert_eq!(2 * expr1, expr2);

    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b") + 2;
    let expr2 = 6 * Expr::<String>::var("a") - 9 * Expr::<String>::var("b") - 6;
    assert_eq!((-3) * expr1, expr2);

    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b") + 2;
    let expr2 = Expr::<String>::constant(0);
    assert_eq!(0 * expr1, expr2);
}
