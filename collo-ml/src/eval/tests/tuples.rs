use super::*;

// =============================================================================
// TUPLE CONSTRUCTION
// =============================================================================

#[test]
fn tuple_construction_basic() {
    let input = "pub let f() -> (Int, Bool) = (42, true);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![ExprValue::Int(42), ExprValue::Bool(true)])
    );
}

#[test]
fn tuple_construction_three_elements() {
    let input = "pub let f() -> (Int, Bool, String) = (1, false, \"hello\");";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            ExprValue::Int(1),
            ExprValue::Bool(false),
            ExprValue::String("hello".to_string())
        ])
    );
}

#[test]
fn tuple_construction_with_params() {
    let input = "pub let f(x: Int, y: Bool) -> (Int, Bool) = (x, y);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10), ExprValue::Bool(true)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![ExprValue::Int(10), ExprValue::Bool(true)])
    );
}

#[test]
fn tuple_construction_with_expressions() {
    let input = "pub let f(x: Int) -> (Int, Int) = (x + 1, x * 2);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![ExprValue::Int(6), ExprValue::Int(10)])
    );
}

// =============================================================================
// TUPLE ACCESS
// =============================================================================

#[test]
fn tuple_access_first_element() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.0;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(42),
                ExprValue::Bool(true),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn tuple_access_second_element() {
    let input = "pub let f(t: (Int, Bool)) -> Bool = t.1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(42),
                ExprValue::Bool(true),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn tuple_access_third_element() {
    let input = "pub let f(t: (Int, Bool, String)) -> String = t.2;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(1),
                ExprValue::Bool(false),
                ExprValue::String("test".to_string()),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("test".to_string()));
}

#[test]
fn tuple_access_on_literal() {
    let input = "pub let f() -> Int = (10, 20).0;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn tuple_access_second_on_literal() {
    let input = "pub let f() -> Int = (10, 20).1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(20));
}

// =============================================================================
// NESTED TUPLES
// =============================================================================

#[test]
fn nested_tuple_construction() {
    let input = "pub let f() -> ((Int, Bool), String) = ((1, true), \"x\");";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Bool(true)]),
            ExprValue::String("x".to_string())
        ])
    );
}

#[test]
fn nested_tuple_access() {
    let input = "pub let f(t: ((Int, Bool), String)) -> Bool = t.0.1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Bool(true)]),
                ExprValue::String("x".to_string()),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn deeply_nested_tuple_access() {
    let input = "pub let f() -> Int = (((1, 2), 3), 4).0.0.0;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(1));
}

// =============================================================================
// TUPLES IN ARITHMETIC
// =============================================================================

#[test]
fn tuple_elements_in_arithmetic() {
    let input = "pub let f(t: (Int, Int)) -> Int = t.0 + t.1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(10),
                ExprValue::Int(32),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn tuple_elements_in_multiplication() {
    let input = "pub let f(t: (Int, Int)) -> Int = t.0 * t.1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![ExprValue::Int(6), ExprValue::Int(7)])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

// =============================================================================
// TUPLES IN COMPARISONS
// =============================================================================

#[test]
fn tuple_elements_in_comparison() {
    let input = "pub let f(t: (Int, Int)) -> Bool = t.0 < t.1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(5),
                ExprValue::Int(10),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn tuple_elements_equality() {
    let input = "pub let f(t: (Int, Int)) -> Bool = t.0 == t.1;";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![ExprValue::Int(5), ExprValue::Int(5)])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

// =============================================================================
// TUPLES WITH LISTS
// =============================================================================

#[test]
fn tuple_containing_list() {
    let input = "pub let f() -> ([Int], Bool) = ([1, 2, 3], true);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            ExprValue::List(vec![
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3)
            ]),
            ExprValue::Bool(true)
        ])
    );
}

#[test]
fn list_of_tuples() {
    let input = "pub let f() -> [(Int, Bool)] = [(1, true), (2, false)];";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Bool(true)]),
            ExprValue::Tuple(vec![ExprValue::Int(2), ExprValue::Bool(false)])
        ])
    );
}

#[test]
fn tuple_access_in_list_comprehension() {
    let input = "pub let f(pairs: [(Int, Int)]) -> [Int] = [p.0 + p.1 for p in pairs];";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(2)]),
                ExprValue::Tuple(vec![ExprValue::Int(3), ExprValue::Int(4)]),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![ExprValue::Int(3), ExprValue::Int(7)])
    );
}

#[test]
fn tuple_creation_in_list_comprehension() {
    let input = "pub let f(xs: [Int]) -> [(Int, Int)] = [(x, x * 2) for x in xs];";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(2)]),
            ExprValue::Tuple(vec![ExprValue::Int(2), ExprValue::Int(4)]),
            ExprValue::Tuple(vec![ExprValue::Int(3), ExprValue::Int(6)])
        ])
    );
}

// =============================================================================
// TUPLES IN CONTROL FLOW
// =============================================================================

#[test]
fn tuple_in_if_expression() {
    let input = "pub let f(b: Bool) -> (Int, Bool) = if b { (1, true) } else { (2, false) };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Bool(true)])
    );
}

#[test]
fn tuple_in_if_expression_else() {
    let input = "pub let f(b: Bool) -> (Int, Bool) = if b { (1, true) } else { (2, false) };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![ExprValue::Int(2), ExprValue::Bool(false)])
    );
}

#[test]
fn tuple_in_let_expression() {
    let input = "pub let f() -> Int = let t = (3, 7) { t.0 + t.1 };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

// =============================================================================
// TUPLES IN AGGREGATIONS
// =============================================================================

#[test]
fn tuple_access_in_sum() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Int = sum p in pairs { p.0 };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(10)]),
                ExprValue::Tuple(vec![ExprValue::Int(2), ExprValue::Int(20)]),
                ExprValue::Tuple(vec![ExprValue::Int(3), ExprValue::Int(30)]),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn tuple_access_in_forall() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Bool = forall p in pairs { p.0 <= p.1 };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(10)]),
                ExprValue::Tuple(vec![ExprValue::Int(5), ExprValue::Int(5)]),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn tuple_access_in_forall_false() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Bool = forall p in pairs { p.0 < p.1 };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(10)]),
                ExprValue::Tuple(vec![ExprValue::Int(5), ExprValue::Int(5)]), // Not strictly less
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(false));
}

// =============================================================================
// TUPLE STRING CONVERSION
// =============================================================================

#[test]
fn tuple_to_string() {
    let input = "pub let f(t: (Int, Bool)) -> String = String(t);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(42),
                ExprValue::Bool(true),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("(42, true)".to_string()));
}

#[test]
fn tuple_to_string_three_elements() {
    let input = "pub let f(t: (Int, Bool, String)) -> String = String(t);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(1),
                ExprValue::Bool(false),
                ExprValue::String("hi".to_string()),
            ])],
        )
        .expect("Should evaluate");

    // Strings are displayed with quotes in the tuple string representation
    assert_eq!(result, ExprValue::String("(1, false, \"hi\")".to_string()));
}

#[test]
fn nested_tuple_to_string() {
    let input = "pub let f(t: ((Int, Int), Bool)) -> String = String(t);";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(2)]),
                ExprValue::Bool(true),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("((1, 2), true)".to_string()));
}

// =============================================================================
// TUPLES WITH FOLDS
// =============================================================================

#[test]
fn tuple_in_fold() {
    let input =
        "pub let f(pairs: [(Int, Int)]) -> Int = fold p in pairs with acc = 0 { acc + p.0 + p.1 };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(2)]),
                ExprValue::Tuple(vec![ExprValue::Int(3), ExprValue::Int(4)]),
            ])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10)); // 1+2+3+4
}

#[test]
fn tuple_as_fold_accumulator() {
    let input = "pub let f(xs: [Int]) -> (Int, Int) = fold x in xs with acc = (0, 1) { (acc.0 + x, acc.1 * x) };";
    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "f",
            vec![ExprValue::List(vec![
                ExprValue::Int(2),
                ExprValue::Int(3),
                ExprValue::Int(4),
            ])],
        )
        .expect("Should evaluate");

    // sum: 0+2+3+4 = 9, product: 1*2*3*4 = 24
    assert_eq!(
        result,
        ExprValue::Tuple(vec![ExprValue::Int(9), ExprValue::Int(24)])
    );
}
