use super::*;

#[test]
fn expr_var_has_correct_coef() {
    let expr = Expr::<String>::var("A");

    assert_eq!(expr.get("A"), Some(1.0));
}

#[test]
fn expr_var_has_empty_coef_for_other_var() {
    let expr = Expr::<String>::var("A");

    assert_eq!(expr.get("B"), None);
}

#[test]
fn expr_var_has_correct_list_of_vars() {
    let expr = Expr::<String>::var("A");

    assert_eq!(expr.variables(), BTreeSet::from([String::from("A")]));
}

#[test]
fn expr_var_returns_zero_constant() {
    let expr = Expr::<String>::var("A");

    assert_eq!(expr.get_constant(), 0.);
}

#[test]
fn expr_constant_returns_correct_coef() {
    let expr = Expr::<String>::constant(3.0);

    assert_eq!(expr.get("A"), None);
}

#[test]
fn expr_constant_returns_correct_constant() {
    let expr = Expr::<String>::constant(3.0);

    assert_eq!(expr.get_constant(), 3.0);
}

#[test]
fn expr_constant_has_empty_list_of_vars() {
    let expr = Expr::<String>::constant(3.0);

    assert_eq!(expr.variables(), BTreeSet::new());
}

#[test]
fn expr_default_has_zero_constant() {
    let expr = Expr::<String>::default();
    assert_eq!(expr.get_constant(), 0.0);
}

#[test]
fn expr_default_has_no_coef() {
    let expr = Expr::<String>::default();
    assert_eq!(expr.get("A"), None);
}

#[test]
fn expr_default_has_empty_list_of_vars() {
    let expr = Expr::<String>::default();

    assert_eq!(expr.variables(), BTreeSet::new());
}
