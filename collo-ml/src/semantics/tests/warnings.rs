use super::*;

// ========== Naming Convention Warnings ==========

#[tokio::test]
async fn function_naming_convention_pascal_case() {
    let input = "pub let MyFunction() -> Int = 5;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::FunctionNamingConvention { .. })),
        "Should warn about function naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn function_naming_convention_correct() {
    let input = "pub let my_function() -> Int = 5;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::FunctionNamingConvention { .. })),
        "Should not warn about correct naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn parameter_naming_convention_pascal_case() {
    let input = "pub let f(MyParam: Int) -> Int = MyParam;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::ParameterNamingConvention { .. })),
        "Should warn about parameter naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn parameter_naming_convention_correct() {
    let input = "pub let f(my_param: Int) -> Int = my_param;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::ParameterNamingConvention { .. })),
        "Should not warn about correct naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn variable_naming_convention_snake_case() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $my_var;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::VariableNamingConvention { .. })),
        "Should warn about variable naming (should be PascalCase): {:?}",
        warnings
    );
}

#[tokio::test]
async fn variable_naming_convention_correct() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $MyVar;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::VariableNamingConvention { .. })),
        "Should not warn about correct variable naming: {:?}",
        warnings
    );
}

// ========== Unused Parameter Warnings ==========

#[tokio::test]
async fn unused_parameter_warning() {
    let input = "pub let f(x: Int, y: Int) -> Int = x;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

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

#[tokio::test]
async fn all_parameters_unused_warning() {
    let input = "pub let f(x: Int, y: Int) -> Int = 42;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    let unused_count = warnings
        .iter()
        .filter(|w| matches!(w, SemWarning::UnusedIdentifier { .. }))
        .count();

    assert_eq!(unused_count, 2, "Should warn about both unused parameters");
}

#[tokio::test]
async fn no_warning_when_parameter_used() {
    let input = "pub let f(x: Int, y: Int) -> Int = x + y;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when all parameters are used: {:?}",
        warnings
    );
}

#[tokio::test]
async fn parameter_used_in_nested_expression() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Int = 
            if flag { x } else { 0 };
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when parameter used in nested expression: {:?}",
        warnings
    );
}

// ========== Unused Forall Variable Warnings ==========

#[tokio::test]
async fn unused_forall_variable() {
    let types = simple_object("Student");
    let input = "pub let f() -> Constraint = forall s in @[Student] { 0 <== 1 };";
    let (_, _, warnings) = analyze(input, types, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused forall variable: {:?}",
        warnings
    );
}

#[tokio::test]
async fn no_warning_when_forall_variable_used() {
    let types = simple_object("Student");
    let vars = var_with_args("V", vec![SimpleType::Object("Student".to_string())]);

    let input = "pub let f() -> Constraint = forall s in @[Student] { $V(s) >== 0 };";
    let (_, _, warnings) = analyze(input, types, vars).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when forall variable is used: {:?}",
        warnings
    );
}

#[tokio::test]
async fn forall_variable_used_in_where_clause() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] where s.age > 18 { 0 <== 1 };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Variable used in where clause should not be marked unused: {:?}",
        warnings
    );
}

// ========== Unused Sum Variable Warnings ==========

#[tokio::test]
async fn unused_sum_variable() {
    let types = simple_object("Student");
    let input = "pub let f() -> Int = sum s in @[Student] { 5 };";
    let (_, _, warnings) = analyze(input, types, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused sum variable: {:?}",
        warnings
    );
}

#[tokio::test]
async fn no_warning_when_sum_variable_used() {
    let types = simple_object("Student");
    let vars = var_with_args("V", vec![SimpleType::Object("Student".to_string())]);

    let input = "pub let f() -> LinExpr = sum s in @[Student] { $V(s) };";
    let (_, _, warnings) = analyze(input, types, vars).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when sum variable is used: {:?}",
        warnings
    );
}

#[tokio::test]
async fn sum_variable_used_in_where_clause() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f() -> LinExpr = 
            sum s in @[Student] where s.age > 18 { 1 };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Variable used in where clause should not be marked unused: {:?}",
        warnings
    );
}

// ========== Unused List Comprehension Variable Warnings ==========

#[tokio::test]
async fn unused_list_comprehension_variable() {
    let input = "pub let f() -> [Int] = [5 for x in [1, 2, 3]];";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should warn about unused comprehension variable: {:?}",
        warnings
    );
}

#[tokio::test]
async fn no_warning_when_comprehension_variable_used() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn when comprehension variable is used: {:?}",
        warnings
    );
}

#[tokio::test]
async fn comprehension_variable_used_in_where_clause() {
    let input = "pub let f() -> [Int] = [1 for x in [1, 2, 3, 4, 5] where x > 2];";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Variable used in where clause should not be marked unused: {:?}",
        warnings
    );
}

// ========== Unused Function Warnings ==========

#[tokio::test]
async fn unused_private_function_warning() {
    let input = "let foo(x: Int) -> Int = x;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Should warn about unused function: {:?}",
        warnings
    );
}

#[tokio::test]
async fn multiple_unused_functions() {
    let input = r#"
        let f(x: Int) -> Int = x;
        let g(y: Int) -> Int = y;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    let unused_count = warnings
        .iter()
        .filter(|w| matches!(w, SemWarning::UnusedFunction { .. }))
        .count();

    assert_eq!(unused_count, 2, "Should warn about both unused functions");
}

#[tokio::test]
async fn no_warning_for_public_function() {
    let input = "pub let foo(x: Int) -> Int = x;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Should not warn about unused public function: {:?}",
        warnings
    );
}

#[tokio::test]
async fn no_warning_when_private_function_called() {
    let input = r#"
        let helper(x: Int) -> Int = x;
        pub let main() -> Int = helper(5);
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Should not warn about used private function: {:?}",
        warnings
    );
}

#[tokio::test]
async fn function_used_in_reify() {
    let input = r#"
        let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $MyVar;
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedFunction { .. })),
        "Function used in reify should not be marked unused: {:?}",
        warnings
    );
}

// ========== Shadowing Warnings ==========

#[tokio::test]
async fn shadowing_parameter_with_forall() {
    let types = simple_object("Student");
    let input = r#"
        pub let f(s: Student) -> Constraint = 
            forall s in @[Student] { 0 <== 1 };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing: {:?}",
        warnings
    );
}

#[tokio::test]
async fn shadowing_parameter_with_sum() {
    let input = r#"
        pub let f(x: Int) -> Int = sum x in [1, 2, 3] { x };
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing: {:?}",
        warnings
    );
}

#[tokio::test]
async fn shadowing_in_nested_forall() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] { 
                forall s in @[Student] { 0 <== 1 } 
            };
    "#;
    let (_, _, warnings) = analyze(input, types, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing in nested forall: {:?}",
        warnings
    );
}

// ========== No Warnings in Valid Cases ==========

#[tokio::test]
async fn no_warnings_for_well_written_code() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let vars = var_with_args(
        "StudentVar",
        vec![SimpleType::Object("Student".to_string())],
    );

    let input = r#"
        pub let compute_total(students: [Student]) -> LinExpr =
            sum s in students where s.age > 18 { $StudentVar(s) };
    "#;
    let (_, _, warnings) = analyze(input, types, vars).await;

    assert!(
        warnings.is_empty(),
        "Well-written code should have no warnings: {:?}",
        warnings
    );
}

// ========== Type Naming Convention Warnings ==========

#[tokio::test]
async fn type_naming_convention_warning() {
    let input = "type my_type = Int;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::TypeNamingConvention { .. })),
        "Should warn about type naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn type_correct_naming_no_warning() {
    let input = "type MyType = Int;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::TypeNamingConvention { .. })),
        "Should not warn about correct type naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn enum_root_naming_convention_warning() {
    let input = "enum my_enum = Good;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::TypeNamingConvention { identifier, .. } if identifier == "my_enum")),
        "Should warn about enum root naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn enum_variant_naming_convention_warning() {
    let input = "enum MyEnum = bad_variant;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::TypeNamingConvention { identifier, .. } if identifier == "bad_variant")),
        "Should warn about enum variant naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn enum_correct_naming_no_warning() {
    let input = "enum MyEnum = GoodVariant(Int) | AnotherGood;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::TypeNamingConvention { .. })),
        "Should not warn about correct enum naming: {:?}",
        warnings
    );
}

// ========== Field Naming Convention Warnings ==========

#[tokio::test]
async fn field_naming_convention_warning_struct_literal() {
    let input = r#"pub let f() -> { bad_field: Int } = { BadField: 5 };"#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        warnings.iter().any(|w| matches!(w,
            SemWarning::FieldNamingConvention { identifier, .. } if identifier == "BadField")),
        "Should warn about struct literal field naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn field_naming_convention_warning_enum_struct_variant() {
    let input = "enum MyEnum = Variant { BadField: Int };";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        warnings.iter().any(|w| matches!(w,
            SemWarning::FieldNamingConvention { identifier, .. } if identifier == "BadField")),
        "Should warn about enum struct variant field naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn field_naming_convention_warning_type_alias() {
    let input = "type MyType = { BadField: Int };";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        warnings.iter().any(|w| matches!(w,
            SemWarning::FieldNamingConvention { identifier, .. } if identifier == "BadField")),
        "Should warn about type alias field naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn field_naming_convention_warning_param_type() {
    let input = "pub let f(x: { BadField: Int }) -> Int = 42;";
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        warnings.iter().any(|w| matches!(w,
            SemWarning::FieldNamingConvention { identifier, .. } if identifier == "BadField")),
        "Should warn about parameter type field naming: {:?}",
        warnings
    );
}

#[tokio::test]
async fn field_naming_convention_warning_return_type() {
    let input = r#"pub let f() -> { BadField: Int } = { BadField: 5 };"#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    // Warns twice: once for return type annotation, once for struct literal
    let count = warnings
        .iter()
        .filter(|w| {
            matches!(w,
        SemWarning::FieldNamingConvention { identifier, .. } if identifier == "BadField")
        })
        .count();
    assert_eq!(
        count, 2,
        "Should warn twice (return type + struct literal): {:?}",
        warnings
    );
}

#[tokio::test]
async fn field_correct_naming_no_warning() {
    let input = r#"
        type MyType = { good_field: Int };
        enum MyEnum = Variant { good_field: Int };
        pub let f() -> { good_field: Int } = { good_field: 5 };
    "#;
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::FieldNamingConvention { .. })),
        "Should not warn about correct field naming: {:?}",
        warnings
    );
}
