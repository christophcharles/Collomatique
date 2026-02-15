use super::*;
use std::collections::BTreeMap;
use std::sync::Arc;

// =============================================================================
// STRUCT CONSTRUCTION
// =============================================================================

#[tokio::test]
async fn struct_construction_basic() {
    let input = "pub let f() -> {x: Int, y: Bool} = {x: 42, y: true};";
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

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    expected.insert("y".to_string(), Arc::new(ExprValue::Bool(true)));
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_construction_single_field() {
    let input = "pub let f() -> {value: Int} = {value: 100};";
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

    let mut expected = BTreeMap::new();
    expected.insert("value".to_string(), Arc::new(ExprValue::Int(100)));
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_construction_three_fields() {
    let input = "pub let f() -> {a: Int, b: Bool, c: String} = {a: 1, b: false, c: \"hello\"};";
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

    let mut expected = BTreeMap::new();
    expected.insert("a".to_string(), Arc::new(ExprValue::Int(1)));
    expected.insert("b".to_string(), Arc::new(ExprValue::Bool(false)));
    expected.insert(
        "c".to_string(),
        Arc::new(ExprValue::String("hello".to_string())),
    );
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_construction_with_params() {
    let input = "pub let f(x: Int, y: Bool) -> {x: Int, y: Bool} = {x: x, y: y};";
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

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), Arc::new(ExprValue::Int(10)));
    expected.insert("y".to_string(), Arc::new(ExprValue::Bool(true)));
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_construction_with_expressions() {
    let input = "pub let f(x: Int) -> {total: Int, doubled: Int} = {total: x + 1, doubled: x * 2};";
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

    let mut expected = BTreeMap::new();
    expected.insert("total".to_string(), Arc::new(ExprValue::Int(6)));
    expected.insert("doubled".to_string(), Arc::new(ExprValue::Int(10)));
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_empty() {
    let input = "pub let f() -> {} = {};";
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

    assert_eq!(result, ExprValue::Struct(BTreeMap::new()));
}

// =============================================================================
// STRUCT FIELD ORDER INDEPENDENCE
// =============================================================================

#[tokio::test]
async fn struct_field_order_in_literal() {
    // Fields written in different order than type declaration
    let input = "pub let f() -> {x: Int, y: Bool} = {y: true, x: 42};";
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

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    expected.insert("y".to_string(), Arc::new(ExprValue::Bool(true)));
    assert_eq!(result, ExprValue::Struct(expected));
}

// =============================================================================
// STRUCT FIELD ACCESS
// =============================================================================

#[tokio::test]
async fn struct_access_first_field() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> Int = s.x;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    struct_val.insert("y".to_string(), Arc::new(ExprValue::Bool(true)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[tokio::test]
async fn struct_access_second_field() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> Bool = s.y;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    struct_val.insert("y".to_string(), Arc::new(ExprValue::Bool(true)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn struct_access_on_literal() {
    let input = "pub let f() -> Int = {x: 10, y: 20}.x;";
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
async fn struct_access_second_on_literal() {
    let input = "pub let f() -> Int = {x: 10, y: 20}.y;";
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
// NESTED STRUCTS
// =============================================================================

#[tokio::test]
async fn nested_struct_construction() {
    let input = "pub let f() -> {inner: {x: Int}} = {inner: {x: 42}};";
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

    let mut inner = BTreeMap::new();
    inner.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    let mut outer = BTreeMap::new();
    outer.insert("inner".to_string(), Arc::new(ExprValue::Struct(inner)));
    assert_eq!(result, ExprValue::Struct(outer));
}

#[tokio::test]
async fn nested_struct_access() {
    let input = "pub let f(s: {inner: {x: Int}}) -> Int = s.inner.x;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut inner = BTreeMap::new();
    inner.insert("x".to_string(), Arc::new(ExprValue::Int(99)));
    let mut outer = BTreeMap::new();
    outer.insert("inner".to_string(), Arc::new(ExprValue::Struct(inner)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(outer)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[tokio::test]
async fn deeply_nested_struct_access() {
    let input = "pub let f() -> Int = {a: {b: {c: 123}}}.a.b.c;";
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

    assert_eq!(result, ExprValue::Int(123));
}

// =============================================================================
// STRUCTS IN ARITHMETIC
// =============================================================================

#[tokio::test]
async fn struct_fields_in_arithmetic() {
    let input = "pub let f(s: {x: Int, y: Int}) -> Int = s.x + s.y;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(10)));
    struct_val.insert("y".to_string(), Arc::new(ExprValue::Int(32)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[tokio::test]
async fn struct_fields_in_multiplication() {
    let input = "pub let f(s: {a: Int, b: Int}) -> Int = s.a * s.b;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("a".to_string(), Arc::new(ExprValue::Int(6)));
    struct_val.insert("b".to_string(), Arc::new(ExprValue::Int(7)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

// =============================================================================
// STRUCTS IN COMPARISONS
// =============================================================================

#[tokio::test]
async fn struct_fields_in_comparison() {
    let input = "pub let f(s: {x: Int, y: Int}) -> Bool = s.x < s.y;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(5)));
    struct_val.insert("y".to_string(), Arc::new(ExprValue::Int(10)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn struct_fields_equality() {
    let input = "pub let f(s: {x: Int, y: Int}) -> Bool = s.x == s.y;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(5)));
    struct_val.insert("y".to_string(), Arc::new(ExprValue::Int(5)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

// =============================================================================
// STRUCTS WITH LISTS
// =============================================================================

#[tokio::test]
async fn struct_containing_list() {
    let input = "pub let f() -> {items: [Int]} = {items: [1, 2, 3]};";
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

    let mut expected = BTreeMap::new();
    expected.insert(
        "items".to_string(),
        Arc::new(ExprValue::List(vec![
            Arc::new(ExprValue::Int(1)),
            Arc::new(ExprValue::Int(2)),
            Arc::new(ExprValue::Int(3)),
        ])),
    );
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn list_of_structs() {
    let input = "pub let f() -> [{x: Int}] = [{x: 1}, {x: 2}];";
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

    let mut s1 = BTreeMap::new();
    s1.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    let mut s2 = BTreeMap::new();
    s2.insert("x".to_string(), Arc::new(ExprValue::Int(2)));

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Struct(s1)),
            Arc::new(ExprValue::Struct(s2))
        ])
    );
}

#[tokio::test]
async fn struct_field_access_in_list_comprehension() {
    let input = "pub let f(points: [{x: Int, y: Int}]) -> [Int] = [p.x + p.y for p in points];";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut p1 = BTreeMap::new();
    p1.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    p1.insert("y".to_string(), Arc::new(ExprValue::Int(2)));
    let mut p2 = BTreeMap::new();
    p2.insert("x".to_string(), Arc::new(ExprValue::Int(3)));
    p2.insert("y".to_string(), Arc::new(ExprValue::Int(4)));

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Struct(p1)),
                Arc::new(ExprValue::Struct(p2)),
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
async fn struct_creation_in_list_comprehension() {
    let input = "pub let f(xs: [Int]) -> [{val: Int}] = [{val: x} for x in xs];";
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

    let mut s1 = BTreeMap::new();
    s1.insert("val".to_string(), Arc::new(ExprValue::Int(1)));
    let mut s2 = BTreeMap::new();
    s2.insert("val".to_string(), Arc::new(ExprValue::Int(2)));
    let mut s3 = BTreeMap::new();
    s3.insert("val".to_string(), Arc::new(ExprValue::Int(3)));

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Struct(s1)),
            Arc::new(ExprValue::Struct(s2)),
            Arc::new(ExprValue::Struct(s3))
        ])
    );
}

// =============================================================================
// STRUCTS IN CONTROL FLOW
// =============================================================================

#[tokio::test]
async fn struct_in_if_expression() {
    let input = "pub let f(b: Bool) -> {x: Int} = if b { {x: 1} } else { {x: 2} };";
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

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_in_if_expression_else() {
    let input = "pub let f(b: Bool) -> {x: Int} = if b { {x: 1} } else { {x: 2} };";
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

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), Arc::new(ExprValue::Int(2)));
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn struct_in_let_expression() {
    let input = "pub let f() -> Int = let s = {x: 3, y: 7} { s.x + s.y };";
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
// STRUCTS IN AGGREGATIONS
// =============================================================================

#[tokio::test]
async fn struct_access_in_sum() {
    let input = "pub let f(points: [{x: Int, y: Int}]) -> Int = sum p in points { p.x };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut p1 = BTreeMap::new();
    p1.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    p1.insert("y".to_string(), Arc::new(ExprValue::Int(10)));
    let mut p2 = BTreeMap::new();
    p2.insert("x".to_string(), Arc::new(ExprValue::Int(2)));
    p2.insert("y".to_string(), Arc::new(ExprValue::Int(20)));
    let mut p3 = BTreeMap::new();
    p3.insert("x".to_string(), Arc::new(ExprValue::Int(3)));
    p3.insert("y".to_string(), Arc::new(ExprValue::Int(30)));

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Struct(p1)),
                Arc::new(ExprValue::Struct(p2)),
                Arc::new(ExprValue::Struct(p3)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

#[tokio::test]
async fn struct_access_in_forall() {
    let input =
        "pub let f(points: [{x: Int, y: Int}]) -> Bool = forall p in points { p.x <= p.y };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut p1 = BTreeMap::new();
    p1.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    p1.insert("y".to_string(), Arc::new(ExprValue::Int(10)));
    let mut p2 = BTreeMap::new();
    p2.insert("x".to_string(), Arc::new(ExprValue::Int(5)));
    p2.insert("y".to_string(), Arc::new(ExprValue::Int(5)));

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Struct(p1)),
                Arc::new(ExprValue::Struct(p2)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[tokio::test]
async fn struct_access_in_forall_false() {
    let input = "pub let f(points: [{x: Int, y: Int}]) -> Bool = forall p in points { p.x < p.y };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut p1 = BTreeMap::new();
    p1.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    p1.insert("y".to_string(), Arc::new(ExprValue::Int(10)));
    let mut p2 = BTreeMap::new();
    p2.insert("x".to_string(), Arc::new(ExprValue::Int(5)));
    p2.insert("y".to_string(), Arc::new(ExprValue::Int(5))); // Not strictly less

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Struct(p1)),
                Arc::new(ExprValue::Struct(p2)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(false));
}

// =============================================================================
// STRUCT STRING CONVERSION
// =============================================================================

#[tokio::test]
async fn struct_to_string() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> String = String(s);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    struct_val.insert("y".to_string(), Arc::new(ExprValue::Bool(true)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    // BTreeMap orders keys alphabetically
    assert_eq!(result, ExprValue::String("{x: 42, y: true}".to_string()));
}

#[tokio::test]
async fn struct_to_string_three_fields() {
    let input = "pub let f(s: {a: Int, b: Bool, c: String}) -> String = String(s);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("a".to_string(), Arc::new(ExprValue::Int(1)));
    struct_val.insert("b".to_string(), Arc::new(ExprValue::Bool(false)));
    struct_val.insert(
        "c".to_string(),
        Arc::new(ExprValue::String("hi".to_string())),
    );

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    // Strings are displayed with quotes
    assert_eq!(
        result,
        ExprValue::String("{a: 1, b: false, c: \"hi\"}".to_string())
    );
}

#[tokio::test]
async fn nested_struct_to_string() {
    let input = "pub let f(s: {inner: {x: Int}}) -> String = String(s);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut inner = BTreeMap::new();
    inner.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    let mut outer = BTreeMap::new();
    outer.insert("inner".to_string(), Arc::new(ExprValue::Struct(inner)));

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(outer)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("{inner: {x: 42}}".to_string()));
}

#[tokio::test]
async fn empty_struct_to_string() {
    let input = "pub let f(s: {}) -> String = String(s);";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(BTreeMap::new())])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("{}".to_string()));
}

// =============================================================================
// STRUCTS WITH FOLDS
// =============================================================================

#[tokio::test]
async fn struct_in_fold() {
    let input = "pub let f(points: [{x: Int, y: Int}]) -> Int = fold p in points with acc = 0 { acc + p.x + p.y };";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut p1 = BTreeMap::new();
    p1.insert("x".to_string(), Arc::new(ExprValue::Int(1)));
    p1.insert("y".to_string(), Arc::new(ExprValue::Int(2)));
    let mut p2 = BTreeMap::new();
    p2.insert("x".to_string(), Arc::new(ExprValue::Int(3)));
    p2.insert("y".to_string(), Arc::new(ExprValue::Int(4)));

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![
                Arc::new(ExprValue::Struct(p1)),
                Arc::new(ExprValue::Struct(p2)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10)); // 1+2+3+4
}

#[tokio::test]
async fn struct_as_fold_accumulator() {
    let input = "pub let f(xs: [Int]) -> {total: Int, prod: Int} = fold x in xs with acc = {total: 0, prod: 1} { {total: acc.total + x, prod: acc.prod * x} };";
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

    let mut expected = BTreeMap::new();
    expected.insert("total".to_string(), Arc::new(ExprValue::Int(9))); // 0+2+3+4
    expected.insert("prod".to_string(), Arc::new(ExprValue::Int(24))); // 1*2*3*4
    assert_eq!(result, ExprValue::Struct(expected));
}

// =============================================================================
// STRUCTS WITH TUPLES
// =============================================================================

#[tokio::test]
async fn struct_containing_tuple() {
    let input = "pub let f() -> {point: (Int, Int)} = {point: (1, 2)};";
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

    let mut expected = BTreeMap::new();
    expected.insert(
        "point".to_string(),
        Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(1)),
            Arc::new(ExprValue::Int(2)),
        ])),
    );
    assert_eq!(result, ExprValue::Struct(expected));
}

#[tokio::test]
async fn tuple_containing_struct() {
    let input = "pub let f() -> ({x: Int}, Bool) = ({x: 42}, true);";
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

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(42)));
    assert_eq!(
        result,
        ExprValue::Tuple(vec![
            Arc::new(ExprValue::Struct(struct_val)),
            Arc::new(ExprValue::Bool(true))
        ])
    );
}

#[tokio::test]
async fn struct_field_then_tuple_access() {
    let input = "pub let f(s: {point: (Int, Int)}) -> Int = s.point.0;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert(
        "point".to_string(),
        Arc::new(ExprValue::Tuple(vec![
            Arc::new(ExprValue::Int(10)),
            Arc::new(ExprValue::Int(20)),
        ])),
    );

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Struct(struct_val)])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

#[tokio::test]
async fn tuple_then_struct_field_access() {
    let input = "pub let f(t: ({x: Int}, Bool)) -> Int = t.0.x;";
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut struct_val = BTreeMap::new();
    struct_val.insert("x".to_string(), Arc::new(ExprValue::Int(99)));

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                Arc::new(ExprValue::Struct(struct_val)),
                Arc::new(ExprValue::Bool(true)),
            ])],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

// =============================================================================
// NAMED STRUCTS VIA TYPE ALIAS
// =============================================================================
// Note: Type aliases to struct types create distinct Custom types.
// A struct literal {x: 1, y: 2} has type Struct, not Custom("Point").
// To use named struct types, you must pass values of that named type.
// These tests focus on field access which works on both.

#[tokio::test]
async fn named_struct_field_access() {
    let input = r#"
        type Point = {x: Int, y: Int};
        pub let f(p: Point) -> Int = p.x + p.y;
    "#;
    let checked_ast = CheckedAST::<NoObject, SqliteDatabaseDriver>::new(
        &BTreeMap::from([("main", input)]),
        HashMap::new(),
    )
    .await
    .expect("Should compile");

    let mut point = BTreeMap::new();
    point.insert("x".to_string(), Arc::new(ExprValue::Int(3)));
    point.insert("y".to_string(), Arc::new(ExprValue::Int(7)));

    let result = checked_ast
        .eval_fn(
            "main",
            "f",
            vec![ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "Point".to_string(),
                variant: None,
                content: Arc::new(ExprValue::Struct(point)),
            })],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}
