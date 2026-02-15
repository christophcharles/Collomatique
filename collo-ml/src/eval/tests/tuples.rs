use super::*;
use std::sync::Arc;

// =============================================================================
// TUPLE CONSTRUCTION
// =============================================================================

#[tokio::test]
async fn tuple_construction_basic() {
    let input = "pub let f() -> (Int, Bool) = (42, true);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(42)),
            Arc::new(ExprValue::Bool(true))
        ])
    );
}

#[tokio::test]
async fn tuple_construction_three_elements() {
    let input = "pub let f() -> (Int, Bool, String) = (1, false, \"hello\");";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(1)),
            Arc::new(ExprValue::Bool(false)),
            Arc::new(ExprValue::String("hello".to_string()))
        ])
    );
}

#[tokio::test]
async fn tuple_construction_with_params() {
    let input = "pub let f(x: Int, y: Bool) -> (Int, Bool) = (x, y);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Int(10), ExprValue::Bool(true)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(10)),
            Arc::new(ExprValue::Bool(true))
        ])
    );
}

#[tokio::test]
async fn tuple_construction_with_expressions() {
    let input = "pub let f(x: Int) -> (Int, Int) = (x + 1, x * 2);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Int(5)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(6)),
            Arc::new(ExprValue::Int(10))
        ])
    );
}

// =============================================================================
// TUPLE ACCESS
// =============================================================================

#[tokio::test]
async fn tuple_access_first_element() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.0;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(42)),
                Arc::new(ExprValue::Bool(true)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[tokio::test]
async fn tuple_access_second_element() {
    let input = "pub let f(t: (Int, Bool)) -> Bool = t.1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(42)),
                Arc::new(ExprValue::Bool(true)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn tuple_access_third_element() {
    let input = "pub let f(t: (Int, Bool, String)) -> String = t.2;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Bool(false)),
                Arc::new(ExprValue::String("test".to_string())),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("test".to_string()));
}

#[tokio::test]
async fn tuple_access_on_literal() {
    let input = "pub let f() -> Int = (10, 20).0;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

#[tokio::test]
async fn tuple_access_second_on_literal() {
    let input = "pub let f() -> Int = (10, 20).1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(20));
}

// =============================================================================
// NESTED TUPLES
// =============================================================================

#[tokio::test]
async fn nested_tuple_construction() {
    let input = "pub let f() -> ((Int, Bool), String) = ((1, true), \"x\");";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Bool(true))
            ])),
            Arc::new(ExprValue::String("x".to_string()))
        ])
    );
}

#[tokio::test]
async fn nested_tuple_access() {
    let input = "pub let f(t: ((Int, Bool), String)) -> Bool = t.0.1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Bool(true)),
                ])),
                Arc::new(ExprValue::String("x".to_string())),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn deeply_nested_tuple_access() {
    let input = "pub let f() -> Int = (((1, 2), 3), 4).0.0.0;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(1));
}

// =============================================================================
// TUPLES IN ARITHMETIC
// =============================================================================

#[tokio::test]
async fn tuple_elements_in_arithmetic() {
    let input = "pub let f(t: (Int, Int)) -> Int = t.0 + t.1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(10)),
                Arc::new(ExprValue::Int(32)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[tokio::test]
async fn tuple_elements_in_multiplication() {
    let input = "pub let f(t: (Int, Int)) -> Int = t.0 * t.1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(6)),
                Arc::new(ExprValue::Int(7)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

// =============================================================================
// TUPLES IN COMPARISONS
// =============================================================================

#[tokio::test]
async fn tuple_elements_in_comparison() {
    let input = "pub let f(t: (Int, Int)) -> Bool = t.0 < t.1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(5)),
                Arc::new(ExprValue::Int(10)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn tuple_elements_equality() {
    let input = "pub let f(t: (Int, Int)) -> Bool = t.0 == t.1;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(5)),
                Arc::new(ExprValue::Int(5)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

// =============================================================================
// TUPLES WITH LISTS
// =============================================================================

#[tokio::test]
async fn tuple_containing_list() {
    let input = "pub let f() -> ([Int], Bool) = ([1, 2, 3], true);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::List(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Int(3))
            ])),
            Arc::new(ExprValue::Bool(true))
        ])
    );
}

#[tokio::test]
async fn list_of_tuples() {
    let input = "pub let f() -> [(Int, Bool)] = [(1, true), (2, false)];";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Bool(true))
            ])),
            Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Bool(false))
            ]))
        ])
    );
}

#[tokio::test]
async fn tuple_access_in_list_comprehension() {
    let input = "pub let f(pairs: [(Int, Int)]) -> [Int] = [p.0 + p.1 for p in pairs];";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Int(2)),
                ])),
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(3)),
                    Arc::new(ExprValue::Int(4)),
                ])),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Int(3)),
            Arc::new(ExprValue::Int(7))
        ])
    );
}

#[tokio::test]
async fn tuple_creation_in_list_comprehension() {
    let input = "pub let f(xs: [Int]) -> [(Int, Int)] = [(x, x * 2) for x in xs];";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Int(3)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Int(2))
            ])),
            Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Int(4))
            ])),
            Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(3)),
                Arc::new(ExprValue::Int(6))
            ]))
        ])
    );
}

// =============================================================================
// TUPLES IN CONTROL FLOW
// =============================================================================

#[tokio::test]
async fn tuple_in_if_expression() {
    let input = "pub let f(b: Bool) -> (Int, Bool) = if b { (1, true) } else { (2, false) };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(1)),
            Arc::new(ExprValue::Bool(true))
        ])
    );
}

#[tokio::test]
async fn tuple_in_if_expression_else() {
    let input = "pub let f(b: Bool) -> (Int, Bool) = if b { (1, true) } else { (2, false) };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(2)),
            Arc::new(ExprValue::Bool(false))
        ])
    );
}

#[tokio::test]
async fn tuple_in_let_expression() {
    let input = "pub let f() -> Int = let t = (3, 7) { t.0 + t.1 };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

// =============================================================================
// TUPLES IN AGGREGATIONS
// =============================================================================

#[tokio::test]
async fn tuple_access_in_sum() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Int = sum p in pairs { p.0 };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Int(10)),
                ])),
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(2)),
                    Arc::new(ExprValue::Int(20)),
                ])),
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(3)),
                    Arc::new(ExprValue::Int(30)),
                ])),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

#[tokio::test]
async fn tuple_access_in_forall() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Bool = forall p in pairs { p.0 <= p.1 };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Int(10)),
                ])),
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(5)),
                    Arc::new(ExprValue::Int(5)),
                ])),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn tuple_access_in_forall_false() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Bool = forall p in pairs { p.0 < p.1 };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Int(10)),
                ])),
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(5)),
                    Arc::new(ExprValue::Int(5)),
                ])), // Not strictly less
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(false));
}

// =============================================================================
// TUPLE STRING CONVERSION
// =============================================================================

#[tokio::test]
async fn tuple_to_string() {
    let input = "pub let f(t: (Int, Bool)) -> String = String(t);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(42)),
                Arc::new(ExprValue::Bool(true)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("(42, true)".to_string()));
}

#[tokio::test]
async fn tuple_to_string_three_elements() {
    let input = "pub let f(t: (Int, Bool, String)) -> String = String(t);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Bool(false)),
                Arc::new(ExprValue::String("hi".to_string())),
            ])],
        )
        .await
        .expect("Should evaluate");

    // Strings are displayed with quotes in the tuple string representation
    assert_eq!(result, ExprValue::String("(1, false, \"hi\")".to_string()));
}

#[tokio::test]
async fn nested_tuple_to_string() {
    let input = "pub let f(t: ((Int, Int), Bool)) -> String = String(t);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Int(2)),
                ])),
                Arc::new(ExprValue::Bool(true)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("((1, 2), true)".to_string()));
}

// =============================================================================
// TUPLES WITH FOLDS
// =============================================================================

#[tokio::test]
async fn tuple_in_fold() {
    let input =
        "pub let f(pairs: [(Int, Int)]) -> Int = fold p in pairs with acc = 0 { acc + p.0 + p.1 };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(1)),
                    Arc::new(ExprValue::Int(2)),
                ])),
                Arc::new(ExprValue::Tuple(vec![
                    Arc::new(ExprValue::Int(3)),
                    Arc::new(ExprValue::Int(4)),
                ])),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10)); // 1+2+3+4
}

#[tokio::test]
async fn tuple_as_fold_accumulator() {
    let input = "pub let f(xs: [Int]) -> (Int, Int) = fold x in xs with acc = (0, 1) { (acc.0 + x, acc.1 * x) };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Int(3)),
                Arc::new(ExprValue::Int(4)),
            ])],
        )
        .await
        .expect("Should evaluate");

    // sum: 0+2+3+4 = 9, product: 1*2*3*4 = 24
    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(9)),
            Arc::new(ExprValue::Int(24))
        ])
    );
}
