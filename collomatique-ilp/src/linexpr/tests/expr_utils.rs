use super::*;

#[test]
fn expr_clean_removes_zeros() {
    let mut expr = 2.0 * Expr::<String>::var("A") - 3.0 * Expr::<String>::var("B")
        + 0.0 * Expr::<String>::var("C")
        + 2.0 * Expr::<String>::constant(3.0);

    expr.clean();

    assert_eq!(expr.get("C"), None);
}

#[test]
fn expr_clean_keeps_non_zeros() {
    let mut expr = 2.0 * Expr::<String>::var("A") - 3.0 * Expr::<String>::var("B")
        + 0.0 * Expr::<String>::var("C")
        + 2.0 * Expr::<String>::constant(3.0);

    expr.clean();

    assert_eq!(
        expr.variables(),
        BTreeSet::from([String::from("A"), String::from("B")])
    );
}

#[test]
fn expr_clean_keeps_constant() {
    let mut expr = 2.0 * Expr::<String>::var("A") - 3.0 * Expr::<String>::var("B")
        + 0.0 * Expr::<String>::var("C")
        + 2.0 * Expr::<String>::constant(3.0);

    expr.clean();

    assert_eq!(expr.get_constant(), 6.0);
}

#[test]
fn expr_clean_and_cleaned_match() {
    let mut expr = 2.0 * Expr::<String>::var("A") - 3.0 * Expr::<String>::var("B")
        + 0.0 * Expr::<String>::var("C")
        + 2.0 * Expr::<String>::constant(3.0);

    let expr_copy = expr.clone();

    expr.clean();

    assert_eq!(expr, expr_copy.cleaned());
}
