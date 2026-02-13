use std::sync::Arc;

use super::*;

// ========== Equality Tests (==) ==========

#[tokio::test]
async fn eq_ints_true() {
    let input = "pub let f() -> Bool = 42 == 42;";

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
async fn eq_ints_false() {
    let input = "pub let f() -> Bool = 42 == 43;";

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
async fn eq_ints_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x == y;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Int(10)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Int(11)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[tokio::test]
async fn eq_bools_true() {
    let input = "pub let f() -> Bool = true == true;";

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
async fn eq_bools_false() {
    let input = "pub let f() -> Bool = true == false;";

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
async fn eq_bools_with_params() {
    let input = "pub let f(a: Bool, b: Bool) -> Bool = a == b;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Bool(false), ExprValue::Bool(false)],
        )
        .await
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Bool(true), ExprValue::Bool(false)],
        )
        .await
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[tokio::test]
async fn eq_lists_empty() {
    let input = "pub let f() -> Bool = [] == [];";

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
async fn eq_lists_same_content() {
    let input = "pub let f() -> Bool = [1, 2, 3] == [1, 2, 3];";

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
async fn eq_lists_different_content() {
    let input = "pub let f() -> Bool = [1, 2, 3] == [1, 2, 4];";

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
async fn eq_lists_different_length() {
    let input = "pub let f() -> Bool = [1, 2] == [1, 2, 3];";

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
async fn eq_lists_order_dependent() {
    // Lists are Vec, so order should matter
    let input = "pub let f() -> Bool = [3, 1, 2] == [1, 2, 3];";

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
async fn eq_lists_with_params() {
    let input = "pub let f(list1: [Int], list2: [Int]) -> Bool = list1 == list2;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let list1 = ExprValue::List(Vec::from([
        Arc::new(ExprValue::Int(1)),
        Arc::new(ExprValue::Int(2)),
    ]));
    let list2 = ExprValue::List(Vec::from([
        Arc::new(ExprValue::Int(1)),
        Arc::new(ExprValue::Int(2)),
    ]));

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![list1, list2])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Inequality Tests (!=) ==========

#[tokio::test]
async fn ne_ints_true() {
    let input = "pub let f() -> Bool = 42 != 43;";

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
async fn ne_ints_false() {
    let input = "pub let f() -> Bool = 42 != 42;";

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
async fn ne_ints_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x != y;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Int(11)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Int(10)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[tokio::test]
async fn ne_bools_true() {
    let input = "pub let f() -> Bool = true != false;";

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
async fn ne_bools_false() {
    let input = "pub let f() -> Bool = false != false;";

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
async fn ne_lists_different() {
    let input = "pub let f() -> Bool = [1, 2] != [3, 4];";

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
async fn ne_lists_same() {
    let input = "pub let f() -> Bool = [1, 2, 3] != [1, 2, 3];";

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
async fn ne_empty_lists() {
    let input = "pub let f() -> Bool = [] != [];";

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

// ========== Less Than Tests (<) ==========

#[tokio::test]
async fn lt_ints_true() {
    let input = "pub let f() -> Bool = 5 < 10;";

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
async fn lt_ints_false() {
    let input = "pub let f() -> Bool = 10 < 5;";

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
async fn lt_ints_equal_false() {
    let input = "pub let f() -> Bool = 10 < 10;";

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
async fn lt_with_negative() {
    let input = "pub let f() -> Bool = -5 < 3;";

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
async fn lt_both_negative() {
    let input = "pub let f() -> Bool = -10 < -5;";

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
async fn lt_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x < y;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(3), ExprValue::Int(7)])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Less Than or Equal Tests (<=) ==========

#[tokio::test]
async fn le_ints_less_true() {
    let input = "pub let f() -> Bool = 5 <= 10;";

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
async fn le_ints_equal_true() {
    let input = "pub let f() -> Bool = 10 <= 10;";

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
async fn le_ints_greater_false() {
    let input = "pub let f() -> Bool = 15 <= 10;";

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
async fn le_with_negative() {
    let input = "pub let f() -> Bool = -5 <= 0;";

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
async fn le_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x <= y;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Int(10)])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Greater Than Tests (>) ==========

#[tokio::test]
async fn gt_ints_true() {
    let input = "pub let f() -> Bool = 10 > 5;";

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
async fn gt_ints_false() {
    let input = "pub let f() -> Bool = 5 > 10;";

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
async fn gt_ints_equal_false() {
    let input = "pub let f() -> Bool = 10 > 10;";

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
async fn gt_with_negative() {
    let input = "pub let f() -> Bool = 3 > -5;";

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
async fn gt_both_negative() {
    let input = "pub let f() -> Bool = -5 > -10;";

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
async fn gt_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x > y;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(7), ExprValue::Int(3)])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Greater Than or Equal Tests (>=) ==========

#[tokio::test]
async fn ge_ints_greater_true() {
    let input = "pub let f() -> Bool = 10 >= 5;";

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
async fn ge_ints_equal_true() {
    let input = "pub let f() -> Bool = 10 >= 10;";

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
async fn ge_ints_less_false() {
    let input = "pub let f() -> Bool = 5 >= 10;";

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
async fn ge_with_negative() {
    let input = "pub let f() -> Bool = 0 >= -5;";

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
async fn ge_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x >= y;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Int(10)])
        .await
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Comparison Chains Tests ==========

#[tokio::test]
async fn comparison_chain_with_and() {
    let input = "pub let f(x: Int) -> Bool = x > 0 and x < 10;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[tokio::test]
async fn comparison_in_arithmetic_context() {
    let input = "pub let f(x: Int) -> Int = if x > 5 { x } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), vars)
        .await
        .expect("Should compile");

    let result_greater = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_greater, ExprValue::Int(10));

    let result_not_greater = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(3)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_not_greater, ExprValue::Int(0));
}
