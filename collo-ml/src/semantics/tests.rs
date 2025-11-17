use super::*;
use crate::parser::{ColloMLParser, Rule};
use pest::Parser;

fn analyze(
    input: &str,
    types: HashMap<String, ObjectFields>,
    vars: HashMap<String, ArgsType>,
) -> (TypeInfo, Vec<SemError>, Vec<SemWarning>) {
    let pairs = ColloMLParser::parse(Rule::file, input).expect("Parse failed");
    let file = crate::ast::File::from_pest(pairs.into_iter().next().unwrap())
        .expect("AST conversion failed");

    let (_global_env, type_info, errors, warnings) =
        GlobalEnv::new(types, vars, &file).expect("GlobalEnv creation failed");

    (type_info, errors, warnings)
}

#[test]
fn test_simple_function_definition() {
    let input = "pub let add(x: Int, y: Int) -> LinExpr = x + y;";
    let (_type_info, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 0, "Should have no errors: {:?}", errors);
    assert_eq!(warnings.len(), 0, "Should have no warnings: {:?}", warnings);
}

#[test]
fn test_unknown_type_in_parameter() {
    let input = "pub let f(x: UnknownType) -> LinExpr = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemError::UnknownType { .. }));
}

#[test]
fn test_body_type_mismatch() {
    let input = "pub let f() -> LinExpr = 5 <== 10;"; // Constraint body, LinExpr expected
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemError::BodyTypeMismatch { .. }));
}

#[test]
fn test_duplicate_parameter() {
    let input = "pub let f(x: Int, x: Int) -> LinExpr = x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0],
        SemError::ParameterAlreadyDefined { .. }
    ));
}

#[test]
fn test_unknown_variable_in_linexpr() {
    let input = "pub let f() -> Constraint = $UnknownVar(5) <== 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.len() > 0);
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownVariable { .. })));
}

#[test]
fn test_variable_argument_type_mismatch() {
    let mut vars = HashMap::new();
    vars.insert("MyVar".to_string(), vec![ExprType::Int]);

    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> Constraint = $MyVar(5) <== 10;";
    let (_, errors, _) = analyze(input, types.clone(), vars.clone());

    assert_eq!(errors.len(), 0, "Should accept Int argument: {:?}", errors);

    // Wrong type
    let input2 = "pub let f(s: Student) -> Constraint = $MyVar(s) <== 10;";
    let (_, errors2, _) = analyze(input2, types, vars);

    assert!(errors2
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn test_forall_with_collection() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> Constraint = forall s in @[Student] { 0 <== 1 };";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Should accept forall with Student type: {:?}",
        errors
    );
}

#[test]
fn test_naming_convention_warnings() {
    let input = "pub let MyFunction() -> LinExpr = 5;"; // PascalCase instead of snake_case
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::FunctionNamingConvention { .. }
    ));
}

#[test]
fn test_path_field_access() {
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert("age".to_string(), ExprType::Int);
    types.insert("Student".to_string(), student_fields);

    let input = "pub let f(s: Student) -> LinExpr = s.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert_eq!(
        errors.len(),
        0,
        "Should access field successfully: {:?}",
        errors
    );
}

#[test]
fn test_unknown_field_access() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f(s: Student) -> LinExpr = s.unknown_field;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownField { .. })));
}

#[test]
fn test_unused_parameter_warning() {
    let input = "pub let f(x: Int, y: Int) -> LinExpr = x;"; // y is unused
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], SemWarning::UnusedIdentifier { .. }));
    if let SemWarning::UnusedIdentifier { identifier, .. } = &warnings[0] {
        assert_eq!(identifier, "y");
    }
}

#[test]
fn test_unused_forall_variable() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> Constraint = forall s in @[Student] { 0 <== 1 };"; // s unused
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })));
}

#[test]
fn test_unused_sum_variable() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> LinExpr = sum s in @[Student] { 5 };"; // s unused
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })));
    if let Some(SemWarning::UnusedIdentifier { identifier, .. }) = warnings
        .iter()
        .find(|w| matches!(w, SemWarning::UnusedIdentifier { .. }))
    {
        assert_eq!(identifier, "s");
    }
}

#[test]
fn test_no_warning_when_parameter_used() {
    let input = "pub let f(x: Int, y: Int) -> LinExpr = x + y;"; // Both used
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Should only have warnings for naming conventions, not unused
    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn about unused when all parameters are used: {:?}",
        warnings
    );
}

#[test]
fn test_no_warning_when_forall_variable_used() {
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert("age".to_string(), ExprType::Int);
    types.insert("Student".to_string(), student_fields);

    let mut vars = HashMap::new();
    vars.insert(
        "V".to_string(),
        vec![ExprType::Object("Student".to_string())],
    );

    let input = "pub let f() -> Constraint = forall s in @[Student] { $V(s) >== 0 };"; // s is used
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn about unused when forall variable is used: {:?}",
        warnings
    );
}

#[test]
fn test_no_warning_when_sum_variable_used() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let mut vars = HashMap::new();
    vars.insert(
        "V".to_string(),
        vec![ExprType::Object("Student".to_string())],
    );

    let input = "pub let f() -> LinExpr = sum s in @[Student] { $V(s) };"; // s is used
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn about unused when sum variable is used: {:?}",
        warnings
    );
}

#[test]
fn test_unused_function_warning() {
    let input = "let foo(x: Int) -> LinExpr = x;"; // foo is never called
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], SemWarning::UnusedFunction { .. }));
    if let SemWarning::UnusedFunction { identifier, .. } = &warnings[0] {
        assert_eq!(identifier, "foo");
    }
}

#[test]
fn test_multiple_unused_functions_warning() {
    let input = r#"
        let f(x: Int) -> LinExpr = x;
        let g(y: Int) -> LinExpr = y;
    "#; // both f and g are unused
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 2);
    assert!(warnings
        .iter()
        .all(|w| matches!(w, SemWarning::UnusedFunction { .. })));

    let identifiers: Vec<_> = warnings
        .iter()
        .filter_map(|w| {
            if let SemWarning::UnusedFunction { identifier, .. } = w {
                Some(identifier.as_str())
            } else {
                None
            }
        })
        .collect();

    assert!(identifiers.contains(&"f"));
    assert!(identifiers.contains(&"g"));
}

#[test]
fn test_function_used_no_warning() {
    let input = r#"
        let f(x: Int) -> LinExpr = x;
        pub let g(y: Int) -> LinExpr = f(y);
    "#; // f is used inside g, so only g is unused
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 0);
}

#[test]
fn test_function_used_in_constraint_no_warning() {
    let input = r#"
        let f(x: Int) -> LinExpr = x;
        pub let c(y: Int) -> Constraint = f(y) === y;
    "#; // f is used in c, so only c is unused
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 0);
}

#[test]
fn test_reify_without_function_should_fail() {
    let input = "reify f as $MyVar;"; // f is not defined
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

#[test]
fn test_reify_with_wrong_function_type_should_fail() {
    let input = r#"
        let f(x: Int) -> LinExpr = x;
        reify f as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionTypeMismatch { .. })));
}

#[test]
fn test_reify_with_unused_variable_warning() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $MyVar;
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedVariable { .. })));
}

#[test]
fn test_reify_then_use_variable_should_work_no_warning() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $MyVar;
        pub let g(y: Int) -> Constraint = $MyVar(y) === 1;
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_reify_variable_shadowing_should_fail() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $MyVar;
        let g(y: Int) -> Constraint = y >== 5;
        reify g as $MyVar; # shadowing: $MyVar already defined
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::VariableAlreadyDefined { .. })),
        "Expected VariableAlreadyDefined error, got: {:?}",
        errors
    );
}

#[test]
fn test_variable_then_reify_shadowing_should_fail() {
    let mut vars = HashMap::new();
    vars.insert("MyVar".to_string(), vec![ExprType::Int]); // already defined externally

    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $MyVar; # conflicts with external definition
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::VariableAlreadyDefined { .. })),
        "Expected VariableAlreadyDefined error, got: {:?}",
        errors
    );
}

#[test]
fn test_function_shadowing_same_signature_should_fail() {
    let input = r#"
        # define f once
        let f(x: Int) -> Constraint = x <== 10;
        # redefine f with the same signature
        let f(x: Int) -> Constraint = x >== 0;
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::FunctionAlreadyDefined { .. })),
        "Expected FunctionAlreadyDefined error, got: {:?}",
        errors
    );
}

#[test]
fn test_function_shadowing_different_signature_should_fail() {
    let input = r#"
        # first definition of f
        let f(x: Int) -> Constraint = x <== 10;
        # second definition of f with different parameter/return type
        let f(y: Int) -> LinExpr = y;
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::FunctionAlreadyDefined { .. })),
        "Expected FunctionAlreadyDefined error, got: {:?}",
        errors
    );
}

#[test]
fn test_function_shadowing_public_first_should_fail() {
    let input = r#"
        # public function
        pub let f(x: Int) -> Constraint = x <== 10;
        # attempt to redefine locally
        let f(x: Int) -> Constraint = x >== 0;
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::FunctionAlreadyDefined { .. })),
        "Expected FunctionAlreadyDefined error, got: {:?}",
        errors
    );
}

#[test]
fn test_function_shadowing_after_use_should_fail() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        # use f in another function to ensure it's referenced
        let g(y: Int) -> Constraint = f(y);
        # redefine f later
        let f(z: Int) -> Constraint = z >== 0;
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::FunctionAlreadyDefined { .. })),
        "Expected FunctionAlreadyDefined error, got: {:?}",
        errors
    );
}

#[test]
fn test_variable_naming_convention_warning() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $my_var; # variable name in snake_case instead of PascalCase
        pub let g() -> Constraint = $my_var(42) <== 2;
    "#;

    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::VariableNamingConvention { .. }
    ));
    if let SemWarning::VariableNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "my_var");
        assert_eq!(suggestion, "MyVar");
    }
}

#[test]
fn test_parameter_naming_convention_warning_in_function() {
    let input = r#"
        pub let f(MyParam: Int) -> Constraint = MyParam <== 10; # parameter should be snake_case
    "#;

    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::ParameterNamingConvention { .. }
    ));
    if let SemWarning::ParameterNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "MyParam");
        assert_eq!(suggestion, "my_param");
    }
}

#[test]
fn test_parameter_naming_convention_warning_in_forall() {
    let input = r#"
        pub let f(xs: [Int]) -> Constraint =
            forall MyElem in xs { MyElem <== 10 }; # parameter should be snake_case
    "#;

    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::ParameterNamingConvention { .. }
    ));
    if let SemWarning::ParameterNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "MyElem");
        assert_eq!(suggestion, "my_elem");
    }
}

#[test]
fn test_parameter_naming_convention_warning_in_sum() {
    let input = r#"
        pub let f(xs: [Int]) -> LinExpr =
            sum MyElem in xs { MyElem }; # parameter should be snake_case
    "#;

    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::ParameterNamingConvention { .. }
    ));
    if let SemWarning::ParameterNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "MyElem");
        assert_eq!(suggestion, "my_elem");
    }
}

#[test]
fn test_naming_convention_positive_no_warnings() {
    let input = r#"
        # Function name in snake_case; parameter is a list of Int
        let my_function(xs: [Int]) -> Constraint =
            # forall parameter in snake_case, iterating over a collection
            forall my_elem in xs { my_elem <== 10 };

        # Variable name in PascalCase
        reify my_function as $MyVar;

        # Public function using the variable
        pub let another_function(xs: [Int]) -> Constraint = $MyVar(xs) === 1;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_complex_constraint_should_work() {
    let input = r#"
        # Function takes a list of Ints
        pub let f(xs: [Int]) -> Constraint =
            # For all elements, either they are <= 10 or the sum is bounded
            forall elem in xs {
                if elem < 5 {
                    elem <== 10
                } else {
                    sum i in xs { i } <== 100
                }
            };
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_complex_constraint_wrong_collection_type() {
    let input = r#"
        pub let f(x: Int) -> Constraint =
            forall elem in x { elem <== 10 }; # x is Int, not a collection
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::TypeMismatch { .. })),
        "Expected TypeMismatch error, got: {:?}",
        errors
    );
}

#[test]
fn test_complex_constraint_nested_forall_sum_should_work() {
    let input = r#"
        pub let f(xs: [Int], ys: [Int]) -> Constraint =
            forall x in xs {
                forall y in ys {
                    sum i in xs { i + x + y } <== 200
                }
            };
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_function_call_wrong_type_should_fail() {
    let input = r#"
        pub let f(x: Int) -> Constraint = x <== 10;
        pub let g(s: Bool) -> Constraint = f(s); # wrong type: f expects Int, got Bool
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::TypeMismatch { .. })),
        "Expected TypeMismatch error, got: {:?}",
        errors
    );
}

#[test]
fn test_function_call_wrong_argument_count_should_fail() {
    let input = r#"
        pub let f(x: Int, y: Int) -> Constraint = x <== y;
        pub let g(z: Int) -> Constraint = f(z); # wrong number: expected 2 args, got 1
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::ArgumentCountMismatch { .. })),
        "Expected ArgumentCountMismatch error, got: {:?}",
        errors
    );
}

#[test]
fn test_reify_variable_call_wrong_type_should_fail() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $MyVar;
        pub let g(b: Bool) -> Constraint = $MyVar(b) === 1; # wrong type: expects Int, got Bool
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::TypeMismatch { .. })),
        "Expected TypeMismatch error, got: {:?}",
        errors
    );
}

#[test]
fn test_reify_variable_call_wrong_argument_count_should_fail() {
    let input = r#"
        let f(x: Int, y: Int) -> Constraint = x <== y;
        reify f as $MyVar;
        pub let g(z: Int) -> Constraint = $MyVar(z) === 0; # wrong number: expected 2 args, got 1
    "#;

    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::ArgumentCountMismatch { .. })),
        "Expected ArgumentCountMismatch error, got: {:?}",
        errors
    );
}

#[test]
fn test_function_call_correct_should_work() {
    let input = r#"
        # Function f takes an Int and returns a Constraint
        pub let f(x: Int) -> Constraint = x <== 10;

        # Function g calls f with the correct type and arity
        pub let g(y: Int) -> Constraint = f(y);
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_reify_variable_call_correct_should_work() {
    let input = r#"
        # Function f takes an Int and returns a Constraint
        let f(x: Int) -> Constraint = x <== 10;

        # Reify f as a variable
        reify f as $MyVar;

        # Public function g calls the reified variable with the correct type
        pub let g(y: Int) -> Constraint = $MyVar(y) === 1;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_function_name_starting_with_underscore_should_work() {
    let input = r#"
        pub let _f(x: Int) -> Constraint = x <== 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_parameter_name_starting_with_underscore_should_work() {
    let input = r#"
        pub let f(_x: Int) -> Constraint = _x <== 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_variable_name_starting_with_underscore_should_work() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $_Var;
        pub let g(x: Int) -> Constraint = $_Var(x) === 1;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_function_name_starting_with_underscore_unused_should_work() {
    let input = r#"
        # Function is defined but never called
        let _f(x: Int) -> Constraint = x <== 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings (even if unused)
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_parameter_name_starting_with_underscore_unused_should_work() {
    let input = r#"
        # Parameter _x is never used in the body
        pub let f(_x: Int) -> Constraint = 1 <== 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings (even if unused)
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_variable_name_starting_with_underscore_unused_should_work() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $_Var;
        # $_Var is never used
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings (even if unused)
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_function_name_only_underscores_should_be_rejected() {
    let input = r#"
        let __() -> Constraint = 1 <== 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: rejected with a naming convention warning suggesting "__name"
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::FunctionNamingConvention { .. }
    ));

    if let SemWarning::FunctionNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "__");
        assert_eq!(suggestion, "__name");
    }
}

#[test]
fn test_parameter_name_only_underscores_should_be_rejected() {
    let input = r#"
        pub let f(___: Int) -> Constraint = ___ <== 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: rejected with a naming convention warning suggesting "__name"
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::ParameterNamingConvention { .. }
    ));

    if let SemWarning::ParameterNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "___");
        assert_eq!(suggestion, "___name");
    }
}

#[test]
fn test_variable_name_only_underscores_should_be_rejected() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $_;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: rejected with a naming convention warning suggesting "_Name"
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::VariableNamingConvention { .. }
    ));

    if let SemWarning::VariableNamingConvention {
        identifier,
        suggestion,
        ..
    } = &warnings[0]
    {
        assert_eq!(identifier, "_");
        assert_eq!(suggestion, "_Name");
    }
}

#[test]
fn test_path_on_list_field() {
    // student.courses where courses is [Course]
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert(
        "courses".to_string(),
        ExprType::List(Box::new(ExprType::Object("Course".to_string()))),
    );
    types.insert("Student".to_string(), student_fields);
    types.insert("Course".to_string(), HashMap::new());

    let input = "pub let f(s: Student) -> Constraint = forall c in s.courses { 0 <== 1 };";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty());
}

#[test]
fn test_nested_path() {
    // student.address.city
    let mut types = HashMap::new();
    let mut address_fields = HashMap::new();
    address_fields.insert("city".to_string(), ExprType::Object("City".to_string()));
    types.insert("Address".to_string(), address_fields);

    let mut student_fields = HashMap::new();
    student_fields.insert(
        "address".to_string(),
        ExprType::Object("Address".to_string()),
    );
    types.insert("Student".to_string(), student_fields);
    types.insert("City".to_string(), HashMap::new());

    let input = "pub let f(s: Student, allowed_cities: [City]) -> LinExpr = if s.address.city in allowed_cities { 1 } else { 0 };";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty());
}

#[test]
fn test_cardinality_returns_int() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> LinExpr = |@[Student]|;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty());
}

// ========== Type Coercion Tests ==========

#[test]
fn test_int_coerces_to_linexpr_in_addition() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> LinExpr = $V(x) + 5;"; // 5 coerces to LinExpr
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr: {:?}",
        errors
    );
}

#[test]
fn test_int_coerces_to_linexpr_in_comparison() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Constraint = $V(x) <== 10;"; // 10 coerces to LinExpr
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr in comparison: {:?}",
        errors
    );
}

#[test]
fn test_int_coerces_to_linexpr_in_multiplication() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> LinExpr = 5 * $V(x);"; // 5 coerces to LinExpr
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr in multiplication: {:?}",
        errors
    );
}

#[test]
fn test_int_coerces_to_linexpr_in_function_return() {
    let input = "pub let f() -> LinExpr = 42;"; // 42 coerces to LinExpr
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr in return: {:?}",
        errors
    );
}

#[test]
fn test_bool_does_not_coerce_to_constraint_in_forall() {
    let input = r#"
        pub let f() -> Constraint = 
            forall x in [1, 2, 3] { x > 0 }; # x > 0 is Bool, should not coerce to Constraint
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Bool should not coerce to Constraint in forall: {:?}",
        errors
    );
}

#[test]
fn test_bool_does_not_coerce_to_constraint_in_function_return() {
    let input = "pub let f() -> Constraint = 5 > 3;"; // Bool should not coerce to Constraint
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Bool should not coerce to Constraint in return: {:?}",
        errors
    );
}

#[test]
fn test_no_coercion_linexpr_to_int() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Int = $V(x);"; // LinExpr cannot coerce to Int
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(!errors.is_empty(), "LinExpr should NOT coerce to Int");
    assert!(matches!(errors[0], SemError::BodyTypeMismatch { .. }));
}

#[test]
fn test_no_coercion_constraint_to_bool() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Bool = $V(x) <== 10;"; // Constraint cannot coerce to Bool
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(!errors.is_empty(), "Constraint should NOT coerce to Bool");
    assert!(matches!(errors[0], SemError::BodyTypeMismatch { .. }));
}

// ========== Functions Returning Different Types ==========

#[test]
fn test_function_returning_int() {
    let input = "pub let count(xs: [Int]) -> Int = |xs|;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should return Int: {:?}",
        errors
    );
}

#[test]
fn test_function_returning_bool() {
    let input = "pub let is_valid(x: Int) -> Bool = x > 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should return Bool: {:?}",
        errors
    );
}

#[test]
fn test_function_returning_list() {
    let input = "pub let get_filtered(xs: [Int]) -> [Int] = [x for x in xs where x > 0];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should return [Int]: {:?}",
        errors
    );
}

#[test]
fn test_function_returning_object() {
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert("id".to_string(), ExprType::Int);
    types.insert("Student".to_string(), student_fields);

    let input = "pub let get_student(s: Student) -> Student = s;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should return Student: {:?}",
        errors
    );
}

#[test]
fn test_function_chain_different_types() {
    let input = r#"
        let get_count(xs: [Int]) -> Int = |xs|;
        let is_large(n: Int) -> Bool = n > 10;
        pub let check(xs: [Int]) -> Bool = is_large(get_count(xs));
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function chain should work: {:?}",
        errors
    );
}

// ========== List Literals and Comprehensions ==========

#[test]
fn test_list_literal_empty() {
    let input = "pub let f() -> [Int] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Empty list should coerce to output type");
}

#[test]
fn test_list_literal_integers() {
    let input = "pub let f() -> [Int] = [1, 2, 3, 4, 5];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "List literal should work: {:?}", errors);
}

#[test]
fn test_list_literal_mixed_types_should_fail() {
    let input = "pub let f() -> [Int] = [1, 2, 5 > 3, 4];"; // Bool in Int list
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Mixed types should fail");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn test_list_comprehension_simple() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List comprehension should work: {:?}",
        errors
    );
}

#[test]
fn test_list_comprehension_with_filter() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3, 4, 5] where x > 2];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Filtered comprehension should work: {:?}",
        errors
    );
}

#[test]
fn test_list_comprehension_with_transformation() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Comprehension with transformation should work: {:?}",
        errors
    );
}

#[test]
fn test_list_comprehension_from_global_collection() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> [Student] = [s for s in @[Student] where 5 > 3];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Comprehension from global collection should work: {:?}",
        errors
    );
}

#[test]
fn test_list_comprehension_wrong_filter_type_should_fail() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3] where x];"; // x is Int, not Bool
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Filter must be Bool");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

// ========== Collection Operations ==========

#[test]
fn test_collection_union() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input =
        "pub let f(group_a: [Student], group_b: [Student]) -> [Student] = group_a union group_b;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Union should work: {:?}", errors);
}

#[test]
fn test_collection_intersection() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f(all: [Student], active: [Student]) -> [Student] = all inter active;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Intersection should work: {:?}", errors);
}

#[test]
fn test_collection_difference() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = r#"pub let f(all: [Student], excluded: [Student]) -> [Student] = all \ excluded;"#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Difference should work: {:?}", errors);
}

#[test]
fn test_collection_operations_type_mismatch_should_fail() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());
    types.insert("Teacher".to_string(), HashMap::new());

    let input = "pub let f(students: [Student], teachers: [Teacher]) -> [Student] = students union teachers;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(!errors.is_empty(), "Union of different types should fail");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn test_collection_complex_operations() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = r#"
        pub let f(all: [Student], group_a: [Student], group_b: [Student], excluded: [Student]) -> [Student] = 
            (all \ excluded) inter (group_a union group_b);
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Complex collection operations should work: {:?}",
        errors
    );
}

// ========== Reification Tests ==========

#[test]
fn test_reify_non_constraint_function_should_fail() {
    let input = r#"
        let f(x: Int) -> Int = x + 5;
        reify f as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Cannot reify non-Constraint function");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionTypeMismatch { .. })));
}

#[test]
fn test_reify_linexpr_function_should_fail() {
    let input = r#"
        let f(x: Int) -> LinExpr = x + 5;
        reify f as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Cannot reify LinExpr function");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionTypeMismatch { .. })));
}

#[test]
fn test_reify_bool_function_should_fail() {
    let input = r#"
        let f(x: Int) -> Bool = x > 5;
        reify f as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Cannot reify Bool function");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionTypeMismatch { .. })));
}

#[test]
fn test_reify_constraint_function_should_work() {
    let input = r#"
        let f(x: Int) -> Constraint = x <== 10;
        reify f as $MyVar;
        pub let g(y: Int) -> Constraint = $MyVar(y) === 1;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Should reify Constraint function: {:?}",
        errors
    );
}

// ========== Functions Taking LinExpr/Constraint Parameters ==========

#[test]
fn test_function_taking_linexpr_parameter() {
    let input = r#"
        pub let double(e: LinExpr) -> LinExpr = e + e;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should take LinExpr parameter: {:?}",
        errors
    );
}

#[test]
fn test_function_taking_constraint_parameter() {
    let input = r#"
        pub let negate(c: Constraint) -> Constraint = c;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should take Constraint parameter: {:?}",
        errors
    );
}

#[test]
fn test_function_call_with_linexpr_argument() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = r#"
        let double(e: LinExpr) -> LinExpr = e + e;
        pub let f(x: Int) -> LinExpr = double($V(x));
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should pass LinExpr to function: {:?}",
        errors
    );
}

#[test]
fn test_function_call_with_constraint_argument() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = r#"
        let combine(c1: Constraint, c2: Constraint) -> Constraint = c1 and c2;
        pub let f(x: Int) -> Constraint = combine($V(x) >== 0, $V(x) <== 10);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should pass Constraint to function: {:?}",
        errors
    );
}

#[test]
fn test_function_taking_list_of_linexpr() {
    let input = r#"
        pub let sum_all(exprs: [LinExpr]) -> LinExpr = 
            sum e in exprs { e };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function should take [LinExpr]: {:?}",
        errors
    );
}

// ========== Complex Mixed-Type Scenarios ==========

#[test]
fn test_if_expression_returning_different_expr_types() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = r#"
        pub let f(x: Int, use_var: Bool) -> LinExpr = 
            if use_var { $V(x) } else { x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If with Int/LinExpr should work: {:?}",
        errors
    );
}

#[test]
fn test_sum_returning_int_in_condition() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            (sum x in xs { 1 }) > 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum of Int should work in condition: {:?}",
        errors
    );
}

#[test]
fn test_function_composition_across_types() {
    let input = r#"
        let count(xs: [Int]) -> Int = |xs|;
        let is_large(n: Int) -> Bool = n > 10;
        let as_int(b: Bool) -> Int = if b { 1 } else { 0 };
        pub let check(xs: [Int]) -> Int = as_int(is_large(count(xs)));
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function composition should work: {:?}",
        errors
    );
}

// ========== Constraint Operator Tests ==========

#[test]
fn test_constraint_equality_operator() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Constraint = $V(x) === 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should accept === operator: {:?}",
        errors
    );
}

#[test]
fn test_constraint_le_operator() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Constraint = $V(x) <== 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should accept <== operator: {:?}",
        errors
    );
}

#[test]
fn test_constraint_ge_operator() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Constraint = $V(x) >== 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should accept >== operator: {:?}",
        errors
    );
}

#[test]
fn test_constraint_operators_with_int_coercion() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = r#"
        pub let f(x: Int) -> Constraint = 
            $V(x) === 5 and $V(x) <== 10 and $V(x) >== 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should accept all constraint operators with Int coercion: {:?}",
        errors
    );
}

#[test]
fn test_regular_eq_vs_constraint_eq() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    // Regular == returns Bool
    let input1 = "pub let f(x: Int) -> Bool = 5 == 10;";
    let (_, errors1, _) = analyze(input1, HashMap::new(), vars.clone());
    assert!(
        errors1.is_empty(),
        "Regular == should return Bool: {:?}",
        errors1
    );

    // Constraint === returns Constraint
    let input2 = "pub let f(x: Int) -> Constraint = $V(x) === 10;";
    let (_, errors2, _) = analyze(input2, HashMap::new(), vars);
    assert!(
        errors2.is_empty(),
        "Constraint === should return Constraint: {:?}",
        errors2
    );
}

#[test]
fn test_linexpr_with_regular_eq_fails() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Constraint = $V(x) == 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(!errors.is_empty(), "LinExpr == should fail");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

#[test]
fn test_constraint_operators_reject_non_arithmetic() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f(s: Student) -> Constraint = s === s;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should reject Object in constraint operators"
    );
}

// ========== Type Annotation Tests ==========

#[test]
fn test_empty_list_with_type_annotation() {
    let input = "pub let f() -> [Int] = [] as [Int];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Should accept empty list with type annotation: {:?}",
        errors
    );
}

#[test]
fn test_empty_list_without_annotation_in_return_coerces() {
    let input = "pub let f() -> [Int] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "EmptyList should coerce to [Int] in return: {:?}",
        errors
    );
}

#[test]
fn test_type_annotation_nested_lists() {
    let input = "pub let f() -> [[Int]] = [] as [[Int]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Should accept nested type annotation: {:?}",
        errors
    );
}

#[test]
fn test_type_annotation_in_union() {
    let input = "pub let f() -> [Int] = ([] as [Int]) union [1, 2, 3];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Should accept type annotation in union: {:?}",
        errors
    );
}

#[test]
fn test_type_annotation_invalid_coercion() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Int = $V(x) as Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(!errors.is_empty(), "LinExpr cannot coerce to Int");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn test_type_annotation_unknown_type() {
    let input = "pub let f() -> [Int] = [] as [UnknownType];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should reject unknown type in annotation"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownType { .. })));
}

// ========== Coercion Tests ==========

#[test]
fn test_int_coerces_to_linexpr_in_return() {
    let input = "pub let f() -> LinExpr = 42;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr: {:?}",
        errors
    );
}

#[test]
fn test_int_coerces_to_linexpr_in_function_arg() {
    let input = r#"
        let double(e: LinExpr) -> LinExpr = e + e;
        pub let f() -> LinExpr = double(5);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr in function arg: {:?}",
        errors
    );
}

#[test]
fn test_linexpr_does_not_coerce_to_int() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Int = $V(x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(!errors.is_empty(), "LinExpr should not coerce to Int");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

#[test]
fn test_emptylist_coerces_to_typed_list() {
    let input = "pub let f() -> [Int] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "EmptyList should coerce to [Int]: {:?}",
        errors
    );
}

// ========== No Bool -> Constraint Coercion ==========

#[test]
fn test_bool_does_not_coerce_to_constraint_in_return() {
    let input = "pub let f() -> Constraint = 5 > 3;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Bool should not coerce to Constraint");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

#[test]
fn test_bool_allowed_in_forall_body() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            forall x in xs { x > 0 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Bool should be allowed in forall body: {:?}",
        errors
    );
}

#[test]
fn test_bool_and_constraint_cannot_mix() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> Constraint = (x > 5) and ($V(x) === 1);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Cannot mix Bool and Constraint with 'and'"
    );
}

// ========== Unify Tests ==========

#[test]
fn test_if_unifies_int_and_linexpr() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int, flag: Bool) -> LinExpr = if flag { 5 } else { $V(x) };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If should unify Int and LinExpr to LinExpr: {:?}",
        errors
    );
}

#[test]
fn test_if_unifies_emptylist_and_list() {
    let input = "pub let f(flag: Bool) -> [Int] = if flag { [] } else { [1, 2, 3] };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If should unify EmptyList and [Int]: {:?}",
        errors
    );
}

#[test]
fn test_list_literal_unifies_mixed_types() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> [LinExpr] = [5, $V(x), 10];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "List should unify Int and LinExpr to [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn test_collection_union_unifies_types() {
    let mut vars = HashMap::new();
    vars.insert("V1".to_string(), vec![ExprType::Int]);
    vars.insert("V2".to_string(), vec![ExprType::Int]);

    let input = "pub let f(x: Int) -> [LinExpr] = [5] union [$V1(x)];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Union should unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

// ========== Sum Return Type Tests ==========

#[test]
fn test_sum_returns_int_when_body_is_int() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { 1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum should return Int when body is Int: {:?}",
        errors
    );
}

#[test]
fn test_sum_returns_linexpr_when_body_is_linexpr() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f() -> LinExpr = sum x in [1, 2, 3] { $V(x) };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Sum should return LinExpr when body is LinExpr: {:?}",
        errors
    );
}

#[test]
fn test_sum_int_can_be_used_in_condition() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            (sum x in xs { 1 }) > 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum returning Int should work in Bool condition: {:?}",
        errors
    );
}

// ========== Complex Scenarios ==========

#[test]
fn test_nested_list_with_empty_list() {
    let input = "pub let f() -> [[Int]] = [[], [1, 2], []];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested list with empty lists should work: {:?}",
        errors
    );
}

#[test]
fn test_list_comprehension_with_type_transformation() {
    let mut vars = HashMap::new();
    vars.insert("V".to_string(), vec![ExprType::Int]);

    let input = "pub let f() -> [LinExpr] = [$V(x) for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "List comprehension should transform types: {:?}",
        errors
    );
}
