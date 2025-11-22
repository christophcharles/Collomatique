use super::*;

#[test]
fn simple_var_call() {
    let input = r#"
    pub let f() -> LinExpr = $V();
    "#;
    let types = HashMap::new();
    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar {
            name: "V".into(),
            params: vec![]
        })))
    );
}

#[test]
fn simple_var_call_with_param() {
    let input = r#"
    pub let f() -> LinExpr = $V(42);
    "#;
    let types = HashMap::new();
    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar {
            name: "V".into(),
            params: vec![ExprValue::Int(42)]
        })))
    );
}

#[test]
fn simple_var_call_depending_on_fn_param() {
    let input = r#"
    pub let f(x: Int) -> LinExpr = $V(x);
    "#;
    let types = HashMap::new();
    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar {
            name: "V".into(),
            params: vec![ExprValue::Int(42)]
        })))
    );
}
