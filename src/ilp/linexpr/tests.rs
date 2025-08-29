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
    assert_eq!((2 * expr1).cleaned(), expr2.cleaned());

    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b") + 2;
    let expr2 = 6 * Expr::<String>::var("a") - 9 * Expr::<String>::var("b") - 6;
    assert_eq!(((-3) * expr1).cleaned(), expr2.cleaned());

    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b") + 2;
    let expr2 = Expr::<String>::constant(0);
    assert_eq!((0 * expr1).cleaned(), expr2.cleaned());
}

#[test]
fn expr_reduced() {
    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b")
        - 4 * Expr::<String>::var("c")
        + 2;
    let expr2 = -2 * Expr::<String>::var("a") + 5;

    let vars = BTreeMap::from([(String::from("b"), true), (String::from("c"), false)]);

    assert_eq!(expr1.reduced(&vars), expr2)
}

#[test]
fn expr_reduce() {
    let mut expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b")
        - 4 * Expr::<String>::var("c")
        + 2;
    let expr2 = -2 * Expr::<String>::var("a") - 2;

    let vars = BTreeMap::from([(String::from("b"), false), (String::from("c"), true)]);

    expr1.reduce(&vars);

    assert_eq!(expr1, expr2)
}

#[test]
fn constraint_reduce() {
    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b")
        - 4 * Expr::<String>::var("c")
        + 2;
    let expr2 = -2 * Expr::<String>::var("a") + 5;

    let mut constraint1 = expr1.leq(&Expr::constant(42));
    let constraint2 = expr2.leq(&Expr::constant(42));

    let vars = BTreeMap::from([(String::from("b"), true), (String::from("c"), false)]);

    constraint1.reduce(&vars);

    assert_eq!(constraint1, constraint2);
}

#[test]
fn constraint_reduced() {
    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b")
        - 4 * Expr::<String>::var("c")
        + 2;
    let expr2 = -2 * Expr::<String>::var("a") - 2;

    let constraint1 = expr1.leq(&Expr::constant(42));
    let constraint2 = expr2.leq(&Expr::constant(42));

    let vars = BTreeMap::from([(String::from("b"), false), (String::from("c"), true)]);

    assert_eq!(constraint1.reduced(&vars), constraint2);
}

#[test]
fn simple_solve() {
    let expr1 = -2 * Expr::<String>::var("a") + 3 * Expr::<String>::var("b")
        - 4 * Expr::<String>::var("c")
        + 2;
    let expr2 = -2 * Expr::<String>::var("a") - 2;
    let expr3 = Expr::<String>::constant(2);

    let constraint_1_1 = expr1.eq(&Expr::constant(42));
    let constraint_1_2 = expr1.leq(&Expr::constant(42));
    let constraint_2_1 = expr2.leq(&Expr::constant(42));
    let constraint_2_2 = expr2.leq(&Expr::constant(-3));
    let constraint_2_3 = expr2.leq(&Expr::constant(-5));
    let constraint_2_4 = expr2.eq(&Expr::constant(-2));
    let constraint_3_1 = expr3.leq(&Expr::constant(42));
    let constraint_3_2 = expr3.leq(&Expr::constant(-42));

    assert_eq!(
        constraint_1_1.simple_solve(),
        SimpleSolution::NotSimpleSolvable
    );
    assert_eq!(
        constraint_1_2.simple_solve(),
        SimpleSolution::NotSimpleSolvable
    );

    assert_eq!(
        constraint_2_1.simple_solve(),
        SimpleSolution::NotSimpleSolvable
    );
    assert_eq!(
        constraint_2_2.simple_solve(),
        SimpleSolution::Solution(Some((String::from("a"), true)))
    );
    assert_eq!(constraint_2_3.simple_solve(), SimpleSolution::NoSolution);
    assert_eq!(
        constraint_2_4.simple_solve(),
        SimpleSolution::Solution(Some((String::from("a"), false)))
    );

    assert_eq!(
        constraint_3_1.simple_solve(),
        SimpleSolution::Solution(None)
    );
    assert_eq!(constraint_3_2.simple_solve(), SimpleSolution::NoSolution);
}
