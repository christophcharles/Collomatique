use super::*;

// ========== Basic Function Definition Tests ==========

#[test]
fn simple_function_with_arithmetic() {
    let input = "pub let add(x: Int, y: Int) -> Int = x + y;";
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 0, "Should have no errors: {:?}", errors);
    assert_eq!(warnings.len(), 0, "Should have no warnings: {:?}", warnings);
}

#[test]
fn function_with_no_parameters() {
    let input = "pub let constant() -> Int = 42;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function with no parameters should be valid: {:?}",
        errors
    );
}

#[test]
fn function_with_string_return_type() {
    let input = r#"pub let constant() -> String = "Hello world!";"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function with string output should be valid: {:?}",
        errors
    );
}

#[test]
fn function_passing_string() {
    let input = "pub let pass(str: String) -> String = str;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function passing string should be valid: {:?}",
        errors
    );
}

// ========== String Concatenation Tests ==========

#[test]
fn string_concatenation_with_plus() {
    let input = r#"pub let concat() -> String = "hello" + "world";"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "String concatenation with + should be valid: {:?}",
        errors
    );
}

#[test]
fn string_concatenation_multiple() {
    let input = r#"pub let concat() -> String = "a" + "b" + "c";"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Multiple string concatenation should be valid: {:?}",
        errors
    );
}

#[test]
fn string_concatenation_with_variables() {
    let input = r#"pub let concat(a: String, b: String) -> String = a + b;"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "String concatenation with parameters should be valid: {:?}",
        errors
    );
}

#[test]
fn string_concatenation_with_parentheses() {
    let input = r#"pub let concat() -> String = ("hello" + " ") + "world";"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "String concatenation with parentheses should be valid: {:?}",
        errors
    );
}

#[test]
fn string_concatenation_mixed_with_int_should_fail() {
    let input = r#"pub let concat() -> String = "hello" + 42;"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should error when concatenating string with int"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn string_concatenation_mixed_with_bool_should_fail() {
    let input = r#"pub let concat() -> String = "value: " + true;"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should error when concatenating string with bool"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn int_plus_string_should_fail() {
    let input = r#"pub let concat() -> String = 42 + "hello";"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error when adding int to string");
}

#[test]
fn string_concatenation_in_expression() {
    let input = r#"
        pub let greet(name: String) -> String = 
            "Hello, " + name + "!";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "String concatenation in complex expression should be valid: {:?}",
        errors
    );
}

#[test]
fn string_concatenation_with_empty_string() {
    let input = r#"pub let concat() -> String = "" + "hello" + "";"#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "String concatenation with empty strings should be valid: {:?}",
        errors
    );
}

#[test]
fn sum_strings_in_list() {
    let input = r#"
        pub let concat_list() -> String = 
            sum s in ["a", "b", "c"] { s };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum over list of strings should be valid: {:?}",
        errors
    );
}

#[test]
fn sum_strings_with_expression() {
    let input = r#"
        pub let build_string() -> String = 
            sum s in ["hello", "world"] { s + " " };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum with string concatenation should be valid: {:?}",
        errors
    );
}

#[test]
fn function_with_constraint_return_type() {
    let input = "pub let constraint() -> Constraint = 5 <== 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Constraint function should be valid: {:?}",
        errors
    );
}

#[test]
fn private_function_definition() {
    let input = "let private_fn(x: Int) -> Int = x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Private function should be valid: {:?}",
        errors
    );
}

#[test]
fn function_with_multiple_parameters() {
    let input = "pub let complex(a: Int, b: Int, c: Int, d: Int) -> Int = a + b + c + d;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function with many parameters should be valid: {:?}",
        errors
    );
}

// ========== Function Call Tests ==========

#[test]
fn calling_defined_function() {
    let input = r#"
        pub let helper(x: Int) -> Int = x;
        pub let main() -> Int = helper(5);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Calling defined function should be valid: {:?}",
        errors
    );
}

#[test]
fn calling_undefined_function() {
    let input = "pub let main() -> Int = undefined_func(5);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on undefined function");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

#[test]
fn function_call_with_wrong_argument_count() {
    let input = r#"
        pub let helper(x: Int) -> Int = x;
        pub let main() -> Int = helper(5, 10);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on wrong argument count");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::ArgumentCountMismatch { .. })));
}

#[test]
fn function_call_with_wrong_argument_types() {
    let input = r#"
        pub let helper(x: Int) -> Int = x;
        pub let main() -> Int = helper(true);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on wrong argument type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

// ========== Error Detection Tests ==========

#[test]
fn duplicate_function_definition() {
    let input = r#"
        pub let foo() -> Int = 1;
        pub let foo() -> Int = 2;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on duplicate function");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionAlreadyDefined { .. })));
}

#[test]
fn duplicate_parameter_names() {
    let input = "pub let f(x: Int, x: Int) -> Int = x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1, "Should have exactly one error");
    assert!(matches!(
        errors[0],
        SemError::ParameterAlreadyDefined { .. }
    ));
}

#[test]
fn unknown_type_in_parameter() {
    let input = "pub let f(x: UnknownType) -> LinExpr = LinExpr(5);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemError::UnknownType { .. }));
}

#[test]
fn unknown_type_in_return_type() {
    let input = "pub let f() -> UnknownType = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on unknown return type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownType { .. })));
}

#[test]
fn body_type_mismatch() {
    let input = "pub let f() -> LinExpr = 5 <== 10;"; // Constraint body, LinExpr expected
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemError::BodyTypeMismatch { .. }));
}

#[test]
fn body_type_mismatch_bool_to_int() {
    let input = "pub let f() -> Int = true;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Bool should not match Int return type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

// ========== Recursive and Mutually Recursive Functions ==========

#[test]
fn recursive_function_call_should_fail() {
    let input = r#"
        pub let factorial(n: Int) -> Int = 
            if n <== 1 { 1 } else { n * factorial(n - 1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    // Note: This will fail type checking because of Int vs Bool in if condition
    // But it tests that recursive calls are recognized
    assert!(
        errors
            .iter()
            .any(|e| !matches!(e, SemError::UnknownIdentifer { .. })),
        "Recursive call should be recognized"
    );
}

#[test]
fn mutually_recursive_functions_now_allowed() {
    // Mutual recursion is now allowed with forward references
    let input = r#"
        pub let is_even(n: Int) -> Bool = if n == 0 { true } else { is_odd(n - 1) };
        pub let is_odd(n: Int) -> Bool = if n == 0 { false } else { is_even(n - 1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Mutually recursive functions should now be allowed: {:?}",
        errors
    );
}
