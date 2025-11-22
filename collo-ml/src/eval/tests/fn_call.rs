use super::*;

#[test]
fn simple_fn_call() {
    let input = r#"
    let g() -> Int = 42;
    pub let f() -> Int = g();
    "#;
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn fn_call_should_shadow_params() {
    let input = r#"
    let g(x: Int) -> Int = x;
    pub let f(x: Int) -> Int = g(43);
    "#;
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(43));
}

#[test]
fn fn_multi_call() {
    let input = r#"
    let g() -> Int = 0;
    let h(x: Int) -> Int = x;
    pub let f() -> [Int] = [g(), h(42)];
    "#;
    let types = HashMap::new();
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(0), ExprValue::Int(42)])
        )
    );
}
