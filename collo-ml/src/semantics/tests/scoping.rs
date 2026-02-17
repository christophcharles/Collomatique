use super::*;

// ========== Parameter Scoping Tests ==========

#[tokio::test]
async fn parameter_accessible_in_body() {
    let input = "pub let f(x: Int) -> Int = x;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Parameter should be accessible: {:?}",
        errors
    );
}

#[tokio::test]
async fn multiple_parameters_accessible() {
    let input = "pub let f(x: Int, y: Int, z: Int) -> Int = x + y + z;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "All parameters should be accessible: {:?}",
        errors
    );
}

#[tokio::test]
async fn parameter_not_accessible_in_other_function() {
    let input = r#"
        pub let f(x: Int) -> Int = x;
        pub let g() -> Int = x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Parameter from other function should not be accessible"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownIdentifer { .. }))
    );
}

// ========== Forall Scoping Tests ==========

#[tokio::test]
async fn forall_variable_accessible_in_body() {
    let vars = var_with_args("V", vec![SimpleType::Int]);

    let input = "pub let f(students: [Int]) -> Constraint = forall s in students { $V(s) >== 0 };";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Forall variable should be accessible: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_variable_not_accessible_outside() {
    let input = r#"
        pub let f(students: [Int]) -> Int = forall s in students { 0 <== 1 } and s;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Forall variable should not leak outside"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownIdentifer { .. }))
    );
}

#[tokio::test]
async fn nested_forall_with_different_variables() {
    let input = r#"
        pub let f(students: [Int]) -> Constraint =
            forall s1 in students {
                forall s2 in students {
                    0 <== 1
                }
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested forall should work: {:?}", errors);
}

#[tokio::test]
async fn forall_variable_shadows_parameter() {
    let input = r#"
        pub let f(s: Int, students: [Int]) -> Constraint =
            forall s in students { 0 <== 1 };
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new()).await;

    // Should have a shadowing warning
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing: {:?}",
        warnings
    );

    // But no errors
    assert!(
        errors.is_empty(),
        "Shadowing should be allowed: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_where_clause_can_access_variable() {
    let input = r#"
        pub let f(students: [{age: Int}]) -> Constraint =
            forall s in students where s.age > 18 { 0 <== 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Where clause should access forall variable: {:?}",
        errors
    );
}

// ========== Sum Scoping Tests ==========

#[tokio::test]
async fn sum_variable_accessible_in_body() {
    let vars = var_with_args("V", vec![SimpleType::Int]);

    let input = "pub let f(students: [Int]) -> LinExpr = sum s in students { $V(s) };";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Sum variable should be accessible: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_variable_not_accessible_outside() {
    let input = r#"
        pub let f(students: [Int]) -> LinExpr = (sum s in students { 5 }) + s;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Sum variable should not leak outside");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownIdentifer { .. }))
    );
}

#[tokio::test]
async fn sum_where_clause_can_access_variable() {
    let input = r#"
        pub let f(students: [{age: Int}]) -> LinExpr =
            sum s in students where s.age > 18 { LinExpr(1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Where clause should access sum variable: {:?}",
        errors
    );
}

#[tokio::test]
async fn nested_sum() {
    let input = r#"
        pub let f(students: [Int]) -> Int =
            sum s1 in students {
                sum s2 in students { 1 }
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested sum should work: {:?}", errors);
}

// ========== List Comprehension Scoping Tests ==========

#[tokio::test]
async fn list_comprehension_variable_accessible_in_body() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "List comprehension variable should be accessible: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_variable_not_accessible_outside() {
    let input = r#"
        pub let f() -> Int = [x * 2 for x in [1, 2, 3]] and x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "List comprehension variable should not leak outside"
    );
}

#[tokio::test]
async fn list_comprehension_where_clause() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3, 4, 5] where x > 2];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "List comprehension with where should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_with_struct_field_access() {
    let input = "pub let f(students: [{age: Int}]) -> [Int] = [s.age for s in students];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Field access in comprehension should work: {:?}",
        errors
    );
}

// ========== Variable Shadowing Tests ==========

#[tokio::test]
async fn sum_shadows_parameter() {
    let input = r#"
        pub let f(x: Int) -> Int = 
            sum x in [1, 2, 3] { x };
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing"
    );
    assert!(
        errors.is_empty(),
        "Shadowing should be allowed: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_shadows_parameter() {
    let input = r#"
        pub let f(x: Int) -> [Int] = 
            [x for x in [1, 2, 3]];
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing"
    );
    assert!(
        errors.is_empty(),
        "Shadowing should be allowed: {:?}",
        errors
    );
}

#[tokio::test]
async fn nested_forall_shadows_outer_variable() {
    let input = r#"
        pub let f(students: [Int]) -> Constraint =
            forall s in students {
                forall s in students {
                    0 <== 1
                }
            };
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn about shadowing"
    );
    assert!(
        errors.is_empty(),
        "Nested shadowing should be allowed: {:?}",
        errors
    );
}

// ========== Complex Scoping Scenarios ==========

#[tokio::test]
async fn multiple_scopes_with_same_name_in_sequence() {
    let input = r#"
        pub let f() -> Int = 
            (sum x in [1, 2, 3] { x }) + 
            (sum x in [4, 5, 6] { x });
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Same name in different scopes should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn nested_different_construct_scopes() {
    let input = r#"
        pub let f(students: [{age: Int}]) -> Int =
            sum s in students where (forall t in students { t.age > 0 }) { s.age };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Nested different constructs should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_expression_maintains_outer_scope() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Int =
            if flag { x + 1 } else { x - 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If expression should access outer scope: {:?}",
        errors
    );
}

// ========== Function Shadowing Tests ==========

#[tokio::test]
async fn local_variable_cannot_shadow_function() {
    let input = r#"
        let f() -> Int = 42;
        let g() -> Int = let f = 43 { f };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::LocalIdentShadowsFunction { .. })),
        "Should error when local shadows function: {:?}",
        errors
    );
}

#[tokio::test]
async fn local_variable_shadowing_local_is_warning() {
    let input = r#"
        let f(x: Int) -> Int = let x = 43 { x };
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new()).await;

    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should warn when local shadows local: {:?}",
        warnings
    );
    assert!(
        errors.is_empty(),
        "Shadowing local with local should not error: {:?}",
        errors
    );
}

#[tokio::test]
async fn function_shadowing_causes_usage_error() {
    let input = r#"
        let f() -> Int = 42;
        let g() -> Int = let f = 43 {
            f + f
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    // Should have the shadowing error
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::LocalIdentShadowsFunction { .. })),
        "Should error on function shadowing: {:?}",
        errors
    );

    // The identifier `f` resolves to the function (not the shadowed local),
    // so using it without parens produces UnknownIdentifer from check_ident_path
    assert!(
        errors.iter().any(
            |e| matches!(e, SemError::UnknownIdentifer { identifier, .. } if identifier == "f")
        ),
        "Should error when using function without parens: {:?}",
        errors
    );
}
