use super::*;

#[test]
fn expr_var_mul_by_f64() {
    let expr = LinExpr::<String>::var("a");
    let expr_mul = 2.4 * expr;

    assert_eq!(expr_mul.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr_mul.get("a"), Some(2.4));
    assert_eq!(expr_mul.get_constant(), 0.0);
}

#[test]
fn expr_var_mul_by_i32() {
    let expr = LinExpr::<String>::var("a");
    let expr_mul = 3 * expr;

    assert_eq!(expr_mul.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr_mul.get("a"), Some(3.0));
    assert_eq!(expr_mul.get_constant(), 0.0);
}

#[test]
fn expr_var_add_f64() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = expr + 2.4;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 2.4);
}

#[test]
fn expr_var_add_i32() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = expr + 3;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 3.0);
}

#[test]
fn expr_var_add_to_f64() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = 2.4 + expr;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 2.4);
}

#[test]
fn expr_var_add_to_i32() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = 3 + expr;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), 3.0);
}

#[test]
fn expr_var_sub_f64() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = expr - 2.4;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), -2.4);
}

#[test]
fn expr_var_sub_i32() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = expr - 3;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(1.0));
    assert_eq!(expr2.get_constant(), -3.0);
}

#[test]
fn expr_var_sub_to_f64() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = 2.4 - expr;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(-1.0));
    assert_eq!(expr2.get_constant(), 2.4);
}

#[test]
fn expr_var_sub_to_i32() {
    let expr = LinExpr::<String>::var("a");
    let expr2 = 3 - expr;

    assert_eq!(expr2.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr2.get("a"), Some(-1.0));
    assert_eq!(expr2.get_constant(), 3.0);
}

#[test]
fn expr_add_together() {
    let expr1 = LinExpr::<String>::var("a");
    let expr2 = LinExpr::<String>::var("b");
    let expr = expr1 + expr2;

    assert_eq!(
        expr.variables(),
        HashSet::from([String::from("a"), String::from("b")])
    );
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get("b"), Some(1.0));
    assert_eq!(expr.get_constant(), 0.0);
}

#[test]
fn expr_add_together_with_constant() {
    let expr1 = LinExpr::<String>::var("a");
    let expr2 = LinExpr::<String>::constant(2.0);
    let expr = expr1 + expr2;

    assert_eq!(expr.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), 2.0);
}

#[test]
fn expr_sub_together() {
    let expr1 = LinExpr::<String>::var("a");
    let expr2 = LinExpr::<String>::var("b");
    let expr = expr1 - expr2;

    assert_eq!(
        expr.variables(),
        HashSet::from([String::from("a"), String::from("b")])
    );
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get("b"), Some(-1.0));
    assert_eq!(expr.get_constant(), 0.0);
}

#[test]
fn expr_sub_together_with_constant() {
    let expr1 = LinExpr::<String>::var("a");
    let expr2 = LinExpr::<String>::constant(2.0);
    let expr = expr1 - expr2;

    assert_eq!(expr.variables(), HashSet::from([String::from("a")]));
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), -2.0);
}

#[test]
fn expr_add_assign_f64() {
    let mut expr = LinExpr::<String>::var("a");
    expr += 3.5;
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), 3.5);
}

#[test]
fn expr_add_assign_i32() {
    let mut expr = LinExpr::<String>::var("a");
    expr += 3;
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), 3.0);
}

#[test]
fn expr_sub_assign_f64() {
    let mut expr = LinExpr::<String>::var("a");
    expr -= 2.5;
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), -2.5);
}

#[test]
fn expr_sub_assign_i32() {
    let mut expr = LinExpr::<String>::var("a");
    expr -= 2;
    assert_eq!(expr.get("a"), Some(1.0));
    assert_eq!(expr.get_constant(), -2.0);
}

#[test]
fn expr_sum_iterator() {
    let exprs = vec![
        LinExpr::<String>::var("a"),
        LinExpr::<String>::var("b"),
        LinExpr::constant(3.0),
    ];
    let sum: LinExpr<String> = exprs.into_iter().sum();
    assert_eq!(sum.get("a"), Some(1.0));
    assert_eq!(sum.get("b"), Some(1.0));
    assert_eq!(sum.get_constant(), 3.0);
}

#[test]
fn expr_sum_ref_iterator() {
    let exprs = vec![
        LinExpr::<String>::var("a"),
        LinExpr::<String>::var("b"),
        LinExpr::constant(3.0),
    ];
    let sum: LinExpr<String> = exprs.iter().sum();
    assert_eq!(sum.get("a"), Some(1.0));
    assert_eq!(sum.get("b"), Some(1.0));
    assert_eq!(sum.get_constant(), 3.0);
}

#[test]
fn expr_sum_empty_iterator() {
    let sum: LinExpr<String> = Vec::<LinExpr<String>>::new().into_iter().sum();
    assert!(sum.variables().is_empty());
    assert_eq!(sum.get_constant(), 0.0);
}
