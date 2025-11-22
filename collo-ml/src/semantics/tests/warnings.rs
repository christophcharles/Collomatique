use super::*;

// ========== Naming Convention Warnings ==========

#[test]
fn function_naming_convention_pascal_case() {
    let input = "pub let MyFunction() -> Int = 5;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::FunctionNamingConvention { .. })),
        "Should warn about function naming: {:?}",
        warnings
    );
}

#[test]
fn function_naming_convention_correct() {
    let input = "pub let my_function() -> Int = 5;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::FunctionNamingConvention { .. })),
        "Should not warn about correct naming: {:?}",
        warnings
    );
}

#[test]
fn parameter_naming_convention_pascal_case() {
    let input = "pub let f(MyParam: Int) -> Int = MyParam;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::ParameterNamingConvention { .. })),
        "Should warn about parameter naming: {:?}",
        warnings
    );
}

#[test]
fn parameter_naming_convention_correct() {
    let input = "pub let f(my_param: Int) -> Int = my_param;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::ParameterNamingConvention { .. })),
        "Should not warn about correct naming: {:?}",
        warnings
    );
}

#[test]
fn variable_naming_convention_snake_case() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $my_var;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::VariableNamingConvention { .. })),
        "Should warn about variable naming (should be PascalCase): {:?}",
        warnings
    );
}

#[test]
fn variable_naming_convention_correct() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $MyVar;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::VariableNamingConvention { .. })),
        "Should not warn about correct variable naming: {:?}",
        warnings
    );
}

// ========== Unused Parameter Warnings ==========

#[test]
fn unused_parameter_warning() {
    let input = "pub let f(x: Int, y: Int) -> Int = x;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused parameter: {:?}",
        warnings
    );

    if let Some(SemWarning::UnusedIdentifier { identifier, .. }) = warnings
        .iter()
        .find(|w| matches!(w, SemWarning::UnusedIdentifier { .. }))
    {
        assert_eq!(identifier, "y", "Should identify 'y' as unused");
    }
}

#[test]
fn all_parameters_unused_warning() {
    let input = "pub let f(x: Int, y: Int) -> Int = 42;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    let unused_count = warnings
        .iter()
        .filter(|w| matches!(w, SemWarning::UnusedIdentifier { .. }))
        .count();

    assert_eq!(unused_count, 2, "Should warn about both unused parameters");
}

#[test]
fn no_warning_when_parameter_used() {
    let input = "pub let f(x: Int, y: Int) -> Int = x + y;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when all parameters are used: {:?}",
        warnings
    );
}

#[test]
fn parameter_used_in_nested_expression() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Int = 
            if flag { x } else { 0 };
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when parameter used in nested expression: {:?}",
        warnings
    );
}

// ========== Unused Forall Variable Warnings ==========

#[test]
fn unused_forall_variable() {
    let types = simple_object("Student");
    let input = "pub let f() -> Constraint = forall s in @[Student] { 0 <== 1 };";
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused forall variable: {:?}",
        warnings
    );
}

#[test]
fn no_warning_when_forall_variable_used() {
    let types = simple_object("Student");
    let vars = var_with_args("V", vec![ExprType::Object("Student".to_string())]);

    let input = "pub let f() -> Constraint = forall s in @[Student] { $V(s) >== 0 };";
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when forall variable is used: {:?}",
        warnings
    );
}

#[test]
fn forall_variable_used_in_where_clause() {
    let types = object_with_fields("Student", vec![("age", ExprType::Int)]);
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] where s.age > 18 { 0 <== 1 };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Variable used in where clause should not be marked unused: {:?}",
        warnings
    );
}

// ========== Unused Sum Variable Warnings ==========

#[test]
fn unused_sum_variable() {
    let types = simple_object("Student");
    let input = "pub let f() -> Int = sum s in @[Student] { 5 };";
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused sum variable: {:?}",
        warnings
    );
}

#[test]
fn no_warning_when_sum_variable_used() {
    let types = simple_object("Student");
    let vars = var_with_args("V", vec![ExprType::Object("Student".to_string())]);

    let input = "pub let f() -> LinExpr = sum s in @[Student] { $V(s) };";
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when sum variable is used: {:?}",
        warnings
    );
}

#[test]
fn sum_variable_used_in_where_clause() {
    let types = object_with_fields("Student", vec![("age", ExprType::Int)]);
    let input = r#"
        pub let f() -> LinExpr = 
            sum s in @[Student] where s.age > 18 { 1 };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Variable used in where clause should not be marked unused: {:?}",
        warnings
    );
}

// ========== Unused List Comprehension Variable Warnings ==========

#[test]
fn unused_list_comprehension_variable() {
    let input = "pub let f() -> [Int] = [5 for x in [1, 2, 3]];";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused comprehension variable: {:?}",
        warnings
    );
}

#[test]
fn no_warning_when_comprehension_variable_used() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when comprehension variable is used: {:?}",
        warnings
    );
}

#[test]
fn comprehension_variable_used_in_where_clause() {
    let input = "pub let f() -> [Int] = [1 for x in [1, 2, 3, 4, 5] where x > 2];";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Variable used in where clause should not be marked unused: {:?}",
        warnings
    );
}

// ========== Unused Function Warnings ==========

#[test]
fn unused_private_function_warning() {
    let input = "let foo(x: Int) -> Int = x;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Should warn about unused function: {:?}",
        warnings
    );
}

#[test]
fn multiple_unused_functions() {
    let input = r#"
        let f(x: Int) -> Int = x;
        let g(y: Int) -> Int = y;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    let unused_count = warnings
        .iter()
        .filter(|w| matches!(w, SemWarning::UnusedFunction { .. }))
        .count();

    assert_eq!(unused_count, 2, "Should warn about both unused functions");
}

#[test]
fn no_warning_for_public_function() {
    let input = "pub let foo(x: Int) -> Int = x;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Should not warn about unused public function: {:?}",
        warnings
    );
}

#[test]
fn no_warning_when_private_function_called() {
    let input = r#"
        let helper(x: Int) -> Int = x;
        pub let main() -> Int = helper(5);
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Should not warn about used private function: {:?}",
        warnings
    );
}

#[test]
fn function_used_in_reify() {
    let input = r#"
        let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $MyVar;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Function used in reify should not be marked unused: {:?}",
        warnings
    );
}

// ========== Shadowing Warnings ==========

#[test]
fn shadowing_parameter_with_forall() {
    let types = simple_object("Student");
    let input = r#"
        pub let f(s: Student) -> Constraint = 
            forall s in @[Student] { 0 <== 1 };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing: {:?}",
        warnings
    );
}

#[test]
fn shadowing_parameter_with_sum() {
    let input = r#"
        pub let f(x: Int) -> Int = sum x in [1, 2, 3] { x };
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing: {:?}",
        warnings
    );
}

#[test]
fn shadowing_in_nested_forall() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] { 
                forall s in @[Student] { 0 <== 1 } 
            };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing in nested forall: {:?}",
        warnings
    );
}

// ========== No Warnings in Valid Cases ==========

#[test]
fn no_warnings_for_well_written_code() {
    let types = object_with_fields("Student", vec![("age", ExprType::Int)]);
    let vars = var_with_args("StudentVar", vec![ExprType::Object("Student".to_string())]);

    let input = r#"
        pub let compute_total(students: [Student]) -> LinExpr = 
            sum s in students where s.age > 18 { $StudentVar(s) };
    "#;
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        warnings.is_empty(),
        "Well-written code should have no warnings: {:?}",
        warnings
    );
}
