use super::*;

#[test]
fn expr_display_no_constant() {
    let expr =
        2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b") + 4 * LinExpr::<String>::var("c");
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c");
}

#[test]
fn expr_display_floats_no_constant() {
    let expr = 2.5 * LinExpr::<String>::var("a") - 3.2 * LinExpr::<String>::var("b")
        + 4.1 * LinExpr::<String>::var("c");
    assert_eq!(format!("{}", expr), "2.5*a + (-3.2)*b + 4.1*c");
}

#[test]
fn expr_display_with_constant() {
    let expr = 2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b")
        + 4 * LinExpr::<String>::var("c")
        + 1;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + 1");
}

#[test]
fn expr_display_floats_with_constant() {
    let expr = 2.5 * LinExpr::<String>::var("a") - 3.2 * LinExpr::<String>::var("b")
        + 4.1 * LinExpr::<String>::var("c")
        + 1.3;
    assert_eq!(format!("{}", expr), "2.5*a + (-3.2)*b + 4.1*c + 1.3");
}

#[test]
fn expr_display_with_negative_constant() {
    let expr = 2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b")
        + 4 * LinExpr::<String>::var("c")
        - 2;
    assert_eq!(format!("{}", expr), "2*a + (-3)*b + 4*c + (-2)");
}

#[test]
fn expr_display_floats_with_negative_constant() {
    let expr = 2.5 * LinExpr::<String>::var("a") - 3.2 * LinExpr::<String>::var("b")
        + 4.1 * LinExpr::<String>::var("c")
        - 2.3;
    assert_eq!(format!("{}", expr), "2.5*a + (-3.2)*b + 4.1*c + (-2.3)");
}

#[test]
fn expr_display_constant_only() {
    let expr = LinExpr::<String>::constant(3.0);
    assert_eq!(format!("{}", expr), "3");
}

#[test]
fn expr_display_negative_constant_only() {
    let expr = LinExpr::<String>::constant(-42.0);
    assert_eq!(format!("{}", expr), "(-42)");
}

#[test]
fn expr_display_floats_constant_only() {
    let expr = LinExpr::<String>::constant(3.5);
    assert_eq!(format!("{}", expr), "3.5");
}

#[test]
fn expr_display_floats_negative_constant_only() {
    let expr = LinExpr::<String>::constant(-42.1);
    assert_eq!(format!("{}", expr), "(-42.1)");
}

#[test]
fn symbol_display_less_than() {
    let symbol = EqSymbol::LessThan;
    assert_eq!(format!("{}", symbol), "<=");
}

#[test]
fn symbol_display_equals() {
    let symbol = EqSymbol::Equals;
    assert_eq!(format!("{}", symbol), "=");
}

#[test]
fn constraint_display_no_constant_less_than() {
    let expr =
        2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b") + 4 * LinExpr::<String>::var("c");
    let constraint = expr.leq(&LinExpr::<String>::constant(0.0));
    assert_eq!(format!("{}", constraint), "2*a + (-3)*b + 4*c <= 0");
}

#[test]
fn constraint_display_no_constant_equals() {
    let expr =
        2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b") + 4 * LinExpr::<String>::var("c");
    let constraint = expr.eq(&LinExpr::<String>::constant(0.0));
    assert_eq!(format!("{}", constraint), "2*a + (-3)*b + 4*c = 0");
}

#[test]
fn constraint_display_with_constant_less_than() {
    let expr = 2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b")
        + 4 * LinExpr::<String>::var("c")
        + 2;
    let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "2*a + (-3)*b + 4*c + 1 <= 0");
}

#[test]
fn constraint_display_with_constant_equals() {
    let expr = 2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b")
        + 4 * LinExpr::<String>::var("c")
        + 2;
    let constraint = expr.eq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "2*a + (-3)*b + 4*c + 1 = 0");
}

#[test]
fn constraint_display_with_negative_constant_less_than() {
    let expr = 2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b")
        + 4 * LinExpr::<String>::var("c")
        - 2;
    let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "2*a + (-3)*b + 4*c + (-3) <= 0");
}

#[test]
fn constraint_display_with_negative_constant_equals() {
    let expr = 2 * LinExpr::<String>::var("a") - 3 * LinExpr::<String>::var("b")
        + 4 * LinExpr::<String>::var("c")
        - 2;
    let constraint = expr.eq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "2*a + (-3)*b + 4*c + (-3) = 0");
}

#[test]
fn constraint_display_constant_only_less_than() {
    let expr = LinExpr::<String>::constant(3.0);
    let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "2 <= 0");
}

#[test]
fn constraint_display_constant_only_equals() {
    let expr = LinExpr::<String>::constant(3.0);
    let constraint = expr.eq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "2 = 0");
}

#[test]
fn constraint_display_negative_constant_only_less_than() {
    let expr = LinExpr::<String>::constant(-42.0);
    let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "(-43) <= 0");
}

#[test]
fn constraint_display_negative_constant_only_equals() {
    let expr = LinExpr::<String>::constant(-42.0);
    let constraint = expr.eq(&LinExpr::<String>::constant(1.0));
    assert_eq!(format!("{}", constraint), "(-43) = 0");
}
