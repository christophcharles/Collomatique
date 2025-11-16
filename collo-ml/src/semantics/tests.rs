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
    assert!(matches!(errors[0], SemError::UnknownInputType { .. }));
}

#[test]
fn test_body_type_mismatch() {
    let input = "pub let f() -> LinExpr = 5 <= 10;"; // Constraint body, LinExpr expected
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
    let input = "pub let f() -> Constraint = $UnknownVar(5) <= 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.len() > 0);
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownVariable { .. })));
}

#[test]
fn test_variable_argument_type_mismatch() {
    let mut vars = HashMap::new();
    vars.insert("MyVar".to_string(), vec![InputType::Int]);

    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> Constraint = $MyVar(5) <= 10;";
    let (_, errors, _) = analyze(input, types.clone(), vars.clone());

    assert_eq!(errors.len(), 0, "Should accept Int argument: {:?}", errors);

    // Wrong type
    let input2 = "pub let f(s: Student) -> Constraint = $MyVar(s) <= 10;";
    let (_, errors2, _) = analyze(input2, types, vars);

    assert!(errors2
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn test_forall_with_collection() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> Constraint = forall s in @[Student]: 0 <= 1;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Should accept forall with Student type");
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
    student_fields.insert("age".to_string(), InputType::Int);
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

    let input = "pub let f() -> Constraint = forall s in @[Student]: 0 <= 1;"; // s unused
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })));
}

#[test]
fn test_unused_sum_variable() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "pub let f() -> LinExpr = sum s in @[Student]: 5;"; // s unused
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
    student_fields.insert("age".to_string(), InputType::Int);
    types.insert("Student".to_string(), student_fields);

    let mut vars = HashMap::new();
    vars.insert(
        "V".to_string(),
        vec![InputType::Object("Student".to_string())],
    );

    let input = "pub let f() -> Constraint = forall s in @[Student]: $V(s) >= 0;"; // s is used
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
        vec![InputType::Object("Student".to_string())],
    );

    let input = "pub let f() -> LinExpr = sum s in @[Student]: $V(s);"; // s is used
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
        pub let c(y: Int) -> Constraint = f(y) == y;
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
        let f(x: Int) -> Constraint = x <= 10;
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
        let f(x: Int) -> Constraint = x <= 10;
        reify f as $MyVar;
        pub let g(y: Int) -> Constraint = $MyVar(y) == 1;
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_reify_variable_shadowing_should_fail() {
    let input = r#"
        let f(x: Int) -> Constraint = x <= 10;
        reify f as $MyVar;
        let g(y: Int) -> Constraint = y >= 5;
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
    vars.insert("MyVar".to_string(), vec![InputType::Int]); // already defined externally

    let input = r#"
        let f(x: Int) -> Constraint = x <= 10;
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
        let f(x: Int) -> Constraint = x <= 10;
        # redefine f with the same signature
        let f(x: Int) -> Constraint = x >= 0;
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
        let f(x: Int) -> Constraint = x <= 10;
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
        pub let f(x: Int) -> Constraint = x <= 10;
        # attempt to redefine locally
        let f(x: Int) -> Constraint = x >= 0;
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
        let f(x: Int) -> Constraint = x <= 10;
        # use f in another function to ensure it's referenced
        let g(y: Int) -> Constraint = f(y);
        # redefine f later
        let f(z: Int) -> Constraint = z >= 0;
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
        let f(x: Int) -> Constraint = x <= 10;
        reify f as $my_var; # variable name in snake_case instead of PascalCase
        pub let g() -> Constraint = $my_var(42) <= 2;
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
        pub let f(MyParam: Int) -> Constraint = MyParam <= 10; # parameter should be snake_case
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
            forall MyElem in xs: MyElem <= 10; # parameter should be snake_case
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
            sum MyElem in xs: MyElem; # parameter should be snake_case
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
            forall my_elem in xs: my_elem <= 10;

        # Variable name in PascalCase
        reify my_function as $MyVar;

        # Public function using the variable
        pub let another_function(xs: [Int]) -> Constraint = $MyVar(xs) == 1;
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
            forall elem in xs:
                if elem < 5 {
                    elem <= 10
                } else {
                    sum i in xs: i <= 100
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
            forall elem in x: elem <= 10; # x is Int, not a collection
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
            forall x in xs:
                forall y in ys:
                    sum i in xs: i + x + y <= 200;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_function_call_wrong_type_should_fail() {
    let input = r#"
        pub let f(x: Int) -> Constraint = x <= 10;
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
        pub let f(x: Int, y: Int) -> Constraint = x <= y;
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
        let f(x: Int) -> Constraint = x <= 10;
        reify f as $MyVar;
        pub let g(b: Bool) -> Constraint = $MyVar(b); # wrong type: expects Int, got Bool
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
        let f(x: Int, y: Int) -> Constraint = x <= y;
        reify f as $MyVar;
        pub let g(z: Int) -> Constraint = $MyVar(z); # wrong number: expected 2 args, got 1
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
        pub let f(x: Int) -> Constraint = x <= 10;

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
        let f(x: Int) -> Constraint = x <= 10;

        # Reify f as a variable
        reify f as $MyVar;

        # Public function g calls the reified variable with the correct type
        pub let g(y: Int) -> Constraint = $MyVar(y) == 1;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_function_name_starting_with_underscore_should_work() {
    let input = r#"
        pub let _f(x: Int) -> Constraint = x <= 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_parameter_name_starting_with_underscore_should_work() {
    let input = r#"
        pub let f(_x: Int) -> Constraint = _x <= 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_variable_name_starting_with_underscore_should_work() {
    let input = r#"
        let f(x: Int) -> Constraint = x <= 10;
        reify f as $_Var;
        pub let g(x: Int) -> Constraint = $_Var(x) == 1;
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
        let _f(x: Int) -> Constraint = x <= 10;
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
        pub let f(_x: Int) -> Constraint = 1 <= 10;
    "#;

    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Final desired behavior: no errors, no warnings (even if unused)
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);
}

#[test]
fn test_variable_name_starting_with_underscore_unused_should_work() {
    let input = r#"
        let f(x: Int) -> Constraint = x <= 10;
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
        let __() -> Constraint = 1 <= 10;
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
        pub let f(___: Int) -> Constraint = ___ <= 10;
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
        let f(x: Int) -> Constraint = x <= 10;
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
        InputType::List(Box::new(InputType::Object("Course".to_string()))),
    );
    types.insert("Student".to_string(), student_fields);
    types.insert("Course".to_string(), HashMap::new());

    let input = "pub let f(s: Student) -> Constraint = forall c in s.courses: 0 <= 1;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty());
}

#[test]
fn test_nested_path() {
    // student.address.city
    let mut types = HashMap::new();
    let mut address_fields = HashMap::new();
    address_fields.insert("city".to_string(), InputType::Object("City".to_string()));
    types.insert("Address".to_string(), address_fields);

    let mut student_fields = HashMap::new();
    student_fields.insert(
        "address".to_string(),
        InputType::Object("Address".to_string()),
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
