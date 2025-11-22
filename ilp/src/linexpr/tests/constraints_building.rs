use super::*;

#[test]
fn sign_default_is_less_than() {
    let sign = EqSymbol::default();

    assert_eq!(sign, EqSymbol::LessThan);
}

#[test]
fn leq_gives_right_symbol() {
    let expr1 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B");
    let expr2 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B")
        + 2.0 * LinExpr::<String>::constant(3.0);

    let constraint = expr1.leq(&expr2);

    assert_eq!(constraint.get_symbol(), EqSymbol::LessThan);
}

#[test]
fn leq_gives_right_lhs() {
    let expr1 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B");
    let expr2 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B")
        + 2.0 * LinExpr::<String>::constant(3.0);

    let expr = &expr1 - &expr2;

    let constraint = expr1.leq(&expr2);

    assert_eq!(*constraint.get_lhs(), expr);
}

#[test]
fn geq_gives_right_symbol() {
    let expr1 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B");
    let expr2 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B")
        + 2.0 * LinExpr::<String>::constant(3.0);

    let constraint = expr1.geq(&expr2);

    assert_eq!(constraint.get_symbol(), EqSymbol::LessThan);
}

#[test]
fn geq_gives_right_lhs() {
    let expr1 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B");
    let expr2 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B")
        + 2.0 * LinExpr::<String>::constant(3.0);

    let expr = &expr2 - &expr1;

    let constraint = expr1.geq(&expr2);

    assert_eq!(*constraint.get_lhs(), expr);
}

#[test]
fn eq_gives_right_symbol() {
    let expr1 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B");
    let expr2 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B")
        + 2.0 * LinExpr::<String>::constant(3.0);

    let constraint = expr1.eq(&expr2);

    assert_eq!(constraint.get_symbol(), EqSymbol::Equals);
}

#[test]
fn eq_gives_right_lhs() {
    let expr1 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B");
    let expr2 = 2.0 * LinExpr::<String>::var("A") - 3.0 * LinExpr::<String>::var("B")
        + 2.0 * LinExpr::<String>::constant(3.0);

    let expr = &expr1 - &expr2;

    let constraint = expr1.eq(&expr2);

    assert_eq!(*constraint.get_lhs(), expr);
}
