use super::*;

// ========== If Expression Tests ==========

#[test]
fn if_simple_true_branch() {
    let input = "pub let f() -> Int = if true { 42 } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn if_simple_false_branch() {
    let input = "pub let f() -> Int = if false { 42 } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn if_with_comparison() {
    let input = "pub let f(x: Int) -> Int = if x > 10 { 100 } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(100));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(0));
}

#[test]
fn if_with_param() {
    let input = "pub let f(cond: Bool, a: Int, b: Int) -> Int = if cond { a } else { b };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn(
            "f",
            vec![
                ExprValue::Bool(true),
                ExprValue::Int(10),
                ExprValue::Int(20),
            ],
        )
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(10));

    let result_false = checked_ast
        .quick_eval_fn(
            "f",
            vec![
                ExprValue::Bool(false),
                ExprValue::Int(10),
                ExprValue::Int(20),
            ],
        )
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(20));
}

#[test]
fn if_returning_bool() {
    let input = "pub let f(x: Int) -> Bool = if x == 0 { true } else { false };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn if_returning_list() {
    let input = "pub let f(x: Int) -> [Int] = if x > 0 { [1, 2, 3] } else { [4, 5] };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::List(
            SimpleType::Int.into(),
            Vec::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::List(
            SimpleType::Int.into(),
            Vec::from([ExprValue::Int(4), ExprValue::Int(5)])
        )
    );
}

#[test]
fn if_nested() {
    let input = "pub let f(x: Int) -> Int = if x > 10 { if x > 20 { 2 } else { 1 } } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_0 = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_0, ExprValue::Int(0));

    let result_1 = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_1, ExprValue::Int(1));

    let result_2 = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(25)])
        .expect("Should evaluate");
    assert_eq!(result_2, ExprValue::Int(2));
}

#[test]
fn if_with_complex_condition() {
    let input = "pub let f(x: Int, y: Int) -> Int = if x > 0 and y > 0 { 1 } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5), ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(1));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5), ExprValue::Int(-3)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(0));
}

#[test]
fn if_with_arithmetic_in_branches() {
    let input = "pub let f(x: Int) -> Int = if x > 0 { x * 2 } else { x + 10 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(10));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(5));
}

#[test]
fn if_with_linexpr() {
    let input = "pub let f(x: Int) -> LinExpr = if x > 0 { $V1() } else { $V2() };";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_true {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar {
                    name: "V1".into(),
                    params: vec![]
                }))
            );
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");

    match result_false {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar {
                    name: "V2".into(),
                    params: vec![]
                }))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn if_with_constraint() {
    let input = "pub let f(x: Int) -> Constraint = if x > 0 { $V1() === 1 } else { $V1() === 0 };";

    let vars = HashMap::from([("V1".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_true {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);
            let constraint1 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![],
            }))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint1));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn if_with_empty_list() {
    let input = "pub let f(x: Int) -> [Int] = if x > 0 { [1, 2] } else { [] };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::List(
            SimpleType::Int.into(),
            Vec::from([ExprValue::Int(1), ExprValue::Int(2)])
        )
    );

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::List(SimpleType::Int.into(), Vec::new())
    );
}
