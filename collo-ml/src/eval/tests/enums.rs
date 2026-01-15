use super::*;

// =============================================================================
// BASIC ENUM DECLARATION AND CONSTRUCTION
// =============================================================================

#[test]
fn enum_basic_construction() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_ok(x: Int) -> Result = Result::Ok(x);
        pub let make_error(msg: String) -> Result = Result::Error(msg);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let ok_result = checked_ast
        .quick_eval_fn("main", "make_ok", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(
        ok_result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: ExprValue::Int(42),
        }))
    );

    let error_result = checked_ast
        .quick_eval_fn(
            "main",
            "make_error",
            vec![ExprValue::String("oops".to_string())],
        )
        .expect("Should evaluate");

    assert_eq!(
        error_result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Error".to_string()),
            content: ExprValue::String("oops".to_string()),
        }))
    );
}

#[test]
fn enum_unit_variant() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_some(x: Int) -> Option = Option::Some(x);
        pub let make_none() -> Option = Option::None;
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let some_result = checked_ast
        .quick_eval_fn("main", "make_some", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(
        some_result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("Some".to_string()),
            content: ExprValue::Int(42),
        }))
    );

    let none_result = checked_ast
        .quick_eval_fn("main", "make_none", vec![])
        .expect("Should evaluate");

    assert_eq!(
        none_result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("None".to_string()),
            content: ExprValue::None,
        }))
    );
}

#[test]
fn enum_unit_variant_with_empty_parens() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_none() -> Option = Option::None();
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let none_result = checked_ast
        .quick_eval_fn("main", "make_none", vec![])
        .expect("Should evaluate");

    assert_eq!(
        none_result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("None".to_string()),
            content: ExprValue::None,
        }))
    );
}

#[test]
fn enum_unit_variant_with_explicit_none() {
    let input = r#"
        enum Option = Some(Int) | None;
        pub let make_none() -> Option = Option::None(none);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let none_result = checked_ast
        .quick_eval_fn("main", "make_none", vec![])
        .expect("Should evaluate");

    assert_eq!(
        none_result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Option".to_string(),
            variant: Some("None".to_string()),
            content: ExprValue::None,
        }))
    );
}

// =============================================================================
// ENUM VARIANT TYPES
// =============================================================================

#[test]
fn enum_variant_as_return_type() {
    // Returning a specific variant type guarantees the function can't fail
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_ok(x: Int) -> Result::Ok = Result::Ok(x);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_ok", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: ExprValue::Int(42),
        }))
    );
}

#[test]
fn enum_variant_subtype_of_root() {
    // Result::Ok is a subtype of Result, so it should work where Result is expected
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let identity(x: Result) -> Result = x;
        pub let make_and_pass() -> Result = identity(Result::Ok(42));
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_and_pass", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: ExprValue::Int(42),
        }))
    );
}

// =============================================================================
// ENUM WITH TUPLE VARIANTS
// =============================================================================

#[test]
fn enum_tuple_variant() {
    let input = r#"
        enum MyEnum = TupleCase(Int, Bool);
        pub let make(x: Int, b: Bool) -> MyEnum = MyEnum::TupleCase(x, b);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "make",
            vec![ExprValue::Int(42), ExprValue::Bool(true)],
        )
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyEnum".to_string(),
            variant: Some("TupleCase".to_string()),
            content: ExprValue::Tuple(vec![ExprValue::Int(42), ExprValue::Bool(true)]),
        }))
    );
}

// =============================================================================
// ENUM WITH STRUCT VARIANTS
// =============================================================================

#[test]
fn enum_struct_variant() {
    let input = r#"
        enum MyEnum = StructCase { x: Int, y: Bool };
        pub let make(x: Int, b: Bool) -> MyEnum = MyEnum::StructCase { x: x, y: b };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "make",
            vec![ExprValue::Int(42), ExprValue::Bool(true)],
        )
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyEnum".to_string(),
            variant: Some("StructCase".to_string()),
            content: ExprValue::Struct(
                [
                    ("x".to_string(), ExprValue::Int(42)),
                    ("y".to_string(), ExprValue::Bool(true))
                ]
                .into_iter()
                .collect()
            ),
        }))
    );
}

// =============================================================================
// ENUM IN MATCH EXPRESSIONS
// =============================================================================

#[test]
fn enum_match_expression() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let extract(r: Result) -> Int = match r {
            x as Result::Ok { Int(x) }
            _ as Result::Error { 0 }
        };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let ok_value = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "Result".to_string(),
        variant: Some("Ok".to_string()),
        content: ExprValue::Int(42),
    }));
    let result1 = checked_ast
        .quick_eval_fn("main", "extract", vec![ok_value])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    let error_value = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "Result".to_string(),
        variant: Some("Error".to_string()),
        content: ExprValue::String("oops".to_string()),
    }));
    let result2 = checked_ast
        .quick_eval_fn("main", "extract", vec![error_value])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(0));
}

// =============================================================================
// ENUM IN CONDITIONALS
// =============================================================================

#[test]
fn enum_in_if_expression() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let f(b: Bool) -> Result = if b { Result::Ok(1) } else { Result::Error("no") };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: ExprValue::Int(1),
        }))
    );

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Error".to_string()),
            content: ExprValue::String("no".to_string()),
        }))
    );
}

// =============================================================================
// QUALIFIED TYPES IN ANNOTATIONS
// =============================================================================

#[test]
fn qualified_type_in_function_param() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let extract_ok(x: Result::Ok) -> Int = Int(x);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let value = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "Result".to_string(),
        variant: Some("Ok".to_string()),
        content: ExprValue::Int(42),
    }));

    let result = checked_ast
        .quick_eval_fn("main", "extract_ok", vec![value])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn qualified_type_in_list() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let make_list() -> [Result::Ok] = [Result::Ok(1), Result::Ok(2)];
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_list", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "Result".to_string(),
                variant: Some("Ok".to_string()),
                content: ExprValue::Int(1),
            })),
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "Result".to_string(),
                variant: Some("Ok".to_string()),
                content: ExprValue::Int(2),
            }))
        ])
    );
}

#[test]
fn qualified_type_maybe() {
    let input = r#"
        enum Result = Ok(Int) | Error(String);
        pub let maybe_ok(b: Bool) -> ?Result::Ok = if b { Result::Ok(42) } else { none };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result_some = checked_ast
        .quick_eval_fn("main", "maybe_ok", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(
        result_some,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Result".to_string(),
            variant: Some("Ok".to_string()),
            content: ExprValue::Int(42),
        }))
    );

    let result_none = checked_ast
        .quick_eval_fn("main", "maybe_ok", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_none, ExprValue::None);
}
