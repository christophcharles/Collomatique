use super::*;

#[test]
fn expr_var_mul_by_f64() {
    let expr = Expr::<String>::var("a");
    let expr_mul = 2.4 * expr;

    assert_eq!(expr_mul.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr_mul.get("a"), Some(2.4));
    assert_eq!(expr_mul.get_constant(), 0.0);
}

#[test]
fn expr_var_mul_by_i32() {
    let expr = Expr::<String>::var("a");
    let expr_mul = 3 * expr;

    assert_eq!(expr_mul.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr_mul.get("a"), Some(3.0));
    assert_eq!(expr_mul.get_constant(), 0.0);
}

#[test]
fn expr_var_add_f64() {
    let expr = Expr::<String>::var("a");
    let expr2 = expr + 2.4;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 2.4);
}

#[test]
fn expr_var_add_i32() {
    let expr = Expr::<String>::var("a");
    let expr2 = expr + 3;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 3.0);
}

#[test]
fn expr_var_add_to_f64() {
    let expr = Expr::<String>::var("a");
    let expr2 = 2.4 + expr;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 2.4);
}

#[test]
fn expr_var_add_to_i32() {
    let expr = Expr::<String>::var("a");
    let expr2 = 3 + expr;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 3.0);
}

#[test]
fn expr_var_sub_f64() {
    let expr = Expr::<String>::var("a");
    let expr2 = expr - 2.4;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), -2.4);
}

#[test]
fn expr_var_sub_i32() {
    let expr = Expr::<String>::var("a");
    let expr2 = expr - 3;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), -3.0);
}

#[test]
fn expr_var_sub_to_f64() {
    let expr = Expr::<String>::var("a");
    let expr2 = 2.4 - expr;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(-1.0));
    assert_eq!(expr2.get_constant(), 2.4);
}

#[test]
fn expr_var_sub_to_i32() {
    let expr = Expr::<String>::var("a");
    let expr2 = 3 - expr;

    assert_eq!(expr2.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(-1.0));
    assert_eq!(expr2.get_constant(), 3.0);
}

#[test]
fn expr_add_together() {
    let expr1 = Expr::<String>::var("a");
    let expr2 = Expr::<String>::var("b");
    let expr = expr1 + expr2;

    assert_eq!(
        expr.variables(),
        BTreeSet::from([String::from("a"), String::from("b")])
    );
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get("b"), Some(1.0));
    assert_eq!(expr.get_constant(), 0.0);
}

#[test]
fn expr_add_together_with_constant() {
    let expr1 = Expr::<String>::var("a");
    let expr2 = Expr::<String>::constant(2.0);
    let expr = expr1 + expr2;

    assert_eq!(expr.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), 2.0);
}

#[test]
fn expr_sub_together() {
    let expr1 = Expr::<String>::var("a");
    let expr2 = Expr::<String>::var("b");
    let expr = expr1 - expr2;

    assert_eq!(
        expr.variables(),
        BTreeSet::from([String::from("a"), String::from("b")])
    );
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get("b"), Some(-1.0));
    assert_eq!(expr.get_constant(), 0.0);
}

#[test]
fn expr_sub_together_with_constant() {
    let expr1 = Expr::<String>::var("a");
    let expr2 = Expr::<String>::constant(2.0);
    let expr = expr1 - expr2;

    assert_eq!(expr.variables(), BTreeSet::from([String::from("a")]));
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), -2.0);
}
