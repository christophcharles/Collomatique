use super::*;

// ========== Boolean AND Tests ==========

#[test]
fn and_two_bools_true() {
    let input = "pub let f() -> Bool = true and true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn and_two_bools_false() {
    let input = "pub let f() -> Bool = true and false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn and_both_false() {
    let input = "pub let f() -> Bool = false and false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn and_with_comparisons() {
    let input = "pub let f(x: Int) -> Bool = x > 0 and x < 10;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn and_chain() {
    let input = "pub let f() -> Bool = true and true and true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn and_chain_with_false() {
    let input = "pub let f() -> Bool = true and false and true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn and_with_params() {
    let input = "pub let f(a: Bool, b: Bool) -> Bool = a and b;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Bool(true), ExprValue::Bool(true)],
        )
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn and_with_ampersands() {
    let input = "pub let f() -> Bool = true && true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Boolean OR Tests ==========

#[test]
fn or_two_bools_true() {
    let input = "pub let f() -> Bool = true or false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn or_both_false() {
    let input = "pub let f() -> Bool = false or false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn or_both_true() {
    let input = "pub let f() -> Bool = true or true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn or_with_comparisons() {
    let input = "pub let f(x: Int) -> Bool = x < 0 or x > 10;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn or_chain() {
    let input = "pub let f() -> Bool = false or false or true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn or_with_params() {
    let input = "pub let f(a: Bool, b: Bool) -> Bool = a or b;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Bool(false), ExprValue::Bool(true)],
        )
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn or_with_pipes() {
    let input = "pub let f() -> Bool = false || true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Boolean NOT Tests ==========

#[test]
fn not_true() {
    let input = "pub let f() -> Bool = not true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn not_false() {
    let input = "pub let f() -> Bool = not false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn not_with_exclamation() {
    let input = "pub let f() -> Bool = !true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn not_with_comparison() {
    let input = "pub let f(x: Int) -> Bool = not (x > 10);";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn not_with_param() {
    let input = "pub let f(b: Bool) -> Bool = not b;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn double_negation() {
    let input = "pub let f() -> Bool = not (not true);";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== Complex Boolean Logic Tests ==========

#[test]
fn and_or_precedence() {
    // AND has higher precedence than OR
    let input = "pub let f() -> Bool = true or false and false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Should be: true or (false and false) = true or false = true
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn and_or_with_parentheses() {
    let input = "pub let f() -> Bool = (true or false) and false;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Should be: (true or false) and false = true and false = false
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn complex_boolean_expression() {
    let input = "pub let f(x: Int) -> Bool = (x > 0 and x < 10) or (x > 100 and x < 110);";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_first_range = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_first_range, ExprValue::Bool(true));

    let result_second_range = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(105)])
        .expect("Should evaluate");
    assert_eq!(result_second_range, ExprValue::Bool(true));

    let result_outside = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(50)])
        .expect("Should evaluate");
    assert_eq!(result_outside, ExprValue::Bool(false));
}

#[test]
fn not_with_and_or() {
    let input = "pub let f() -> Bool = not (true and false) or true;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // not (true and false) or true = not false or true = true or true = true
    assert_eq!(result, ExprValue::Bool(true));
}
