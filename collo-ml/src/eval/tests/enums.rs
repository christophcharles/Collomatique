use super::*;
use std::sync::Arc;

// =============================================================================
// BASIC ENUM DECLARATION AND CONSTRUCTION
// =============================================================================

#[tokio::test]
async fn enum_basic_construction() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_ok(x: Int) -> Result = Result::Ok(x);
        pub let make_error(msg: String) -> Result = Result::Error(msg);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let ok_result = checked_ast
        .eval_fn("main", "make_ok", vec![ExprValue::Int(42)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        ok_result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: Arc::new(ExprValue::Int(42)),
        })
    );

    let error_result = checked_ast
        .eval_fn(
            "main",
            "make_error",
            vec![ExprValue::String("oops".to_string())],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(
        error_result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Error".to_string()),
            content: Arc::new(ExprValue::String("oops".to_string())),
        })
    );
}

#[tokio::test]
async fn enum_unit_variant() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_some(x: Int) -> Option = Option::Some(x);
        pub let make_none() -> Option = Option::None;
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let some_result = checked_ast
        .eval_fn("main", "make_some", vec![ExprValue::Int(42)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        some_result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("Some".to_string()),
            content: Arc::new(ExprValue::Int(42)),
        })
    );

    let none_result = checked_ast
        .eval_fn("main", "make_none", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        none_result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("None".to_string()),
            content: Arc::new(ExprValue::None),
        })
    );
}

#[tokio::test]
async fn enum_unit_variant_with_empty_parens() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_none() -> Option = Option::None();
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let none_result = checked_ast
        .eval_fn("main", "make_none", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        none_result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("None".to_string()),
            content: Arc::new(ExprValue::None),
        })
    );
}

#[tokio::test]
async fn enum_unit_variant_with_explicit_none() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_none() -> Option = Option::None(none);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let none_result = checked_ast
        .eval_fn("main", "make_none", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        none_result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("None".to_string()),
            content: Arc::new(ExprValue::None),
        })
    );
}

// =============================================================================
// ENUM VARIANT TYPES
// =============================================================================

#[tokio::test]
async fn enum_variant_as_return_type() {
    // Returning a specific variant type guarantees the function can't fail
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_ok(x: Int) -> Result::Ok = Result::Ok(x);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "make_ok", vec![ExprValue::Int(42)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: Arc::new(ExprValue::Int(42)),
        })
    );
}

#[tokio::test]
async fn enum_variant_subtype_of_root() {
    // Result::Ok is a subtype of Result, so it should work where Result is expected
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let identity(x: Result) -> Result = x;
        pub let make_and_pass() -> Result = identity(Result::Ok(42));
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "make_and_pass", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: Arc::new(ExprValue::Int(42)),
        })
    );
}

// =============================================================================
// ENUM WITH TUPLE VARIANTS
// =============================================================================

#[tokio::test]
async fn enum_tuple_variant() {
    let input = r#"
        enum MyEnum = TupleCase(Int, Bool);
        pub let make(x: Int, b: Bool) -> MyEnum = MyEnum::TupleCase(x, b);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "make",
            vec![ExprValue::Int(42), ExprValue::Bool(true)],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyEnum".to_string(),
            variant: Some("TupleCase".to_string()),
            content: Arc::new(ExprValue::Tuple(vec![
                Arc::new(ExprValue::Int(42)),
                Arc::new(ExprValue::Bool(true))
            ])),
        })
    );
}

// =============================================================================
// ENUM WITH STRUCT VARIANTS
// =============================================================================

#[tokio::test]
async fn enum_struct_variant() {
    let input = r#"
        enum MyEnum = StructCase { x: Int, y: Bool };
        pub let make(x: Int, b: Bool) -> MyEnum = MyEnum::StructCase { x: x, y: b };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn(
            "main",
            "make",
            vec![ExprValue::Int(42), ExprValue::Bool(true)],
        )
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "MyEnum".to_string(),
            variant: Some("StructCase".to_string()),
            content: Arc::new(ExprValue::Struct(
                [
                    ("x".to_string(), Arc::new(ExprValue::Int(42))),
                    ("y".to_string(), Arc::new(ExprValue::Bool(true)))
                ]
                .into_iter()
                .collect()
            )),
        })
    );
}

// =============================================================================
// ENUM IN MATCH EXPRESSIONS
// =============================================================================

#[tokio::test]
async fn enum_match_expression() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let extract(r: Result) -> Int = match r {
            x as Result::Ok { Int(x) }
            _ as Result::Error { 0 }
        };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let ok_value = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "Result".to_string(),
        variant: Some("Ok".to_string()),
        content: Arc::new(ExprValue::Int(42)),
    });
    let result1 = checked_ast
        .eval_fn("main", "extract", vec![ok_value])
        .await
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    let error_value = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "Result".to_string(),
        variant: Some("Error".to_string()),
        content: Arc::new(ExprValue::String("oops".to_string())),
    });
    let result2 = checked_ast
        .eval_fn("main", "extract", vec![error_value])
        .await
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(0));
}

// =============================================================================
// ENUM IN CONDITIONALS
// =============================================================================

#[tokio::test]
async fn enum_in_if_expression() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let f(b: Bool) -> Result = if b { Result::Ok(1) } else { Result::Error("no") };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result_true = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: Arc::new(ExprValue::Int(1)),
        })
    );

    let result_false = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Error".to_string()),
            content: Arc::new(ExprValue::String("no".to_string())),
        })
    );
}

// =============================================================================
// QUALIFIED TYPES IN ANNOTATIONS
// =============================================================================

#[tokio::test]
async fn qualified_type_in_function_param() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let extract_ok(x: Result::Ok) -> Int = Int(x);
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let value = ExprValue::Custom(CustomValue {
        module: "main".to_string(),
        type_name: "Result".to_string(),
        variant: Some("Ok".to_string()),
        content: Arc::new(ExprValue::Int(42)),
    });

    let result = checked_ast
        .eval_fn("main", "extract_ok", vec![value])
        .await
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[tokio::test]
async fn qualified_type_in_list() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_list() -> [Result::Ok] = [Result::Ok(1), Result::Ok(2)];
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "make_list", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "Result".to_string(),
                variant: Some("Ok".to_string()),
                content: Arc::new(ExprValue::Int(1)),
            })),
            Arc::new(ExprValue::Custom(CustomValue {
                module: "main".to_string(),
                type_name: "Result".to_string(),
                variant: Some("Ok".to_string()),
                content: Arc::new(ExprValue::Int(2)),
            }))
        ])
    );
}

#[tokio::test]
async fn qualified_type_maybe() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let maybe_ok(b: Bool) -> ?Result::Ok = if b { Result::Ok(42) } else { none };
    "#;
    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), HashMap::new())
            .await
            .expect("Should compile");

    let result_some = checked_ast
        .eval_fn("main", "maybe_ok", vec![ExprValue::Bool(true)])
        .await
        .expect("Should evaluate");
    assert_eq!(
        result_some,
        ExprValue::Custom(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: Arc::new(ExprValue::Int(42)),
        })
    );

    let result_none = checked_ast
        .eval_fn("main", "maybe_ok", vec![ExprValue::Bool(false)])
        .await
        .expect("Should evaluate");
    assert_eq!(result_none, ExprValue::None);
}
