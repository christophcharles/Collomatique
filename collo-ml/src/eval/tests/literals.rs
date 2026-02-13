use super::*;
use std::sync::Arc;

#[tokio::test]
async fn simple_string() {
    let input = r#"pub let f() -> String = "Hello world!";"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::String("Hello world!".into()));
}

#[tokio::test]
async fn string_with_quotes() {
    let input = r#"pub let f() -> String = ~"Hello "quotes""~;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::String(r#"Hello "quotes""#.into()));
}

#[tokio::test]
async fn pass_string() {
    let input = "pub let f(str: String) -> String = str;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::String("Hello world!".into())])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::String("Hello world!".into()));
}

#[tokio::test]
async fn simple_number() {
    let input = "pub let f() -> Int = 42;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[tokio::test]
async fn negative_number() {
    let input = "pub let f() -> Int = -5;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-5));
}

#[tokio::test]
async fn boolean_true() {
    let input = "pub let f() -> Bool = true;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn boolean_false() {
    let input = "pub let f() -> Bool = false;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[tokio::test]
async fn boolean_list() {
    let input = "pub let f() -> [Bool] = [true, false, true];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            Arc::new(ExprValue::Bool(true)),
            Arc::new(ExprValue::Bool(false)),
            Arc::new(ExprValue::Bool(true)),
        ]))
    );
}

#[tokio::test]
async fn number_list() {
    let input = "pub let f() -> [Int] = [0, 42, -1];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            Arc::new(ExprValue::Int(0)),
            Arc::new(ExprValue::Int(42)),
            Arc::new(ExprValue::Int(-1))
        ]))
    );
}

#[tokio::test]
async fn cardinality_of_fixed_list() {
    let input = "pub let f() -> Int = |[0, 42, -1]|;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[tokio::test]
async fn cardinality_of_list_in_param() {
    let input = "pub let f(list: [Int]) -> Int = |list|;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(Vec::from([
                Arc::new(ExprValue::Int(0)),
                Arc::new(ExprValue::Int(42)),
                Arc::new(ExprValue::Int(-1)),
            ]))],
        )
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[tokio::test]
async fn range() {
    let input = "pub let f() -> [Int] = [-3..2];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            Arc::new(ExprValue::Int(-3)),
            Arc::new(ExprValue::Int(-2)),
            Arc::new(ExprValue::Int(-1)),
            Arc::new(ExprValue::Int(0)),
            Arc::new(ExprValue::Int(1)),
        ]))
    );
}

#[tokio::test]
async fn empty_range() {
    let input = "pub let f() -> [Int] = [0..0];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[tokio::test]
async fn empty_range_with_end_below_start() {
    let input = "pub let f() -> [Int] = [3..-2];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[tokio::test]
async fn range_with_one_element() {
    let input = "pub let f() -> [Int] = [4..5];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([Arc::new(ExprValue::Int(4))]))
    );
}
