use super::*;

// ========== Parameter Scoping Tests ==========

#[test]
fn parameter_accessible_in_body() {
    let input = "pub let f(x: Int) -> Int = x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Parameter should be accessible: {:?}",
        errors
    );
}

#[test]
fn multiple_parameters_accessible() {
    let input = "pub let f(x: Int, y: Int, z: Int) -> Int = x + y + z;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "All parameters should be accessible: {:?}",
        errors
    );
}

#[test]
fn parameter_not_accessible_in_other_function() {
    let input = r#"
        pub let f(x: Int) -> Int = x;
        pub let g() -> Int = x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Parameter from other function should not be accessible"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

// ========== Forall Scoping Tests ==========

#[test]
fn forall_variable_accessible_in_body() {
    let types = simple_object("Student");
    let vars = var_with_args("V", vec![SimpleType::Object("Student".to_string())]);

    let input = "pub let f() -> Constraint = forall s in @[Student] { $V(s) >== 0 };";
    let (_, errors, _) = analyze(input, types, vars);

    assert!(
        errors.is_empty(),
        "Forall variable should be accessible: {:?}",
        errors
    );
}

#[test]
fn forall_variable_not_accessible_outside() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Int = forall s in @[Student] { 0 <== 1 } and s;
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        !errors.is_empty(),
        "Forall variable should not leak outside"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

#[test]
fn nested_forall_with_different_variables() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Constraint = 
            forall s1 in @[Student] { 
                forall s2 in @[Student] { 
                    0 <== 1 
                } 
            };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Nested forall should work: {:?}", errors);
}

#[test]
fn forall_variable_shadows_parameter() {
    let types = simple_object("Student");
    let input = r#"
        pub let f(s: Student) -> Constraint = 
            forall s in @[Student] { 0 <== 1 };
    "#;
    let (_, errors, warnings) = analyze(input, types, HashMap::new());

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

#[test]
fn forall_where_clause_can_access_variable() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] where s.age > 18 { 0 <== 1 };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Where clause should access forall variable: {:?}",
        errors
    );
}

// ========== Sum Scoping Tests ==========

#[test]
fn sum_variable_accessible_in_body() {
    let types = simple_object("Student");
    let vars = var_with_args("V", vec![SimpleType::Object("Student".to_string())]);

    let input = "pub let f() -> LinExpr = sum s in @[Student] { $V(s) };";
    let (_, errors, _) = analyze(input, types, vars);

    assert!(
        errors.is_empty(),
        "Sum variable should be accessible: {:?}",
        errors
    );
}

#[test]
fn sum_variable_not_accessible_outside() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> LinExpr = (sum s in @[Student] { 5 }) + s;
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(!errors.is_empty(), "Sum variable should not leak outside");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

#[test]
fn sum_where_clause_can_access_variable() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f() -> LinExpr = 
            sum s in @[Student] where s.age > 18 { 1 };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Where clause should access sum variable: {:?}",
        errors
    );
}

#[test]
fn nested_sum() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Int = 
            sum s1 in @[Student] { 
                sum s2 in @[Student] { 1 } 
            };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Nested sum should work: {:?}", errors);
}

// ========== List Comprehension Scoping Tests ==========

#[test]
fn list_comprehension_variable_accessible_in_body() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List comprehension variable should be accessible: {:?}",
        errors
    );
}

#[test]
fn list_comprehension_variable_not_accessible_outside() {
    let input = r#"
        pub let f() -> Int = [x * 2 for x in [1, 2, 3]] and x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "List comprehension variable should not leak outside"
    );
}

#[test]
fn list_comprehension_where_clause() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3, 4, 5] where x > 2];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List comprehension with where should work: {:?}",
        errors
    );
}

#[test]
fn list_comprehension_with_object_field_access() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = "pub let f(students: [Student]) -> [Int] = [s.age for s in students];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Field access in comprehension should work: {:?}",
        errors
    );
}

// ========== Variable Shadowing Tests ==========

#[test]
fn sum_shadows_parameter() {
    let input = r#"
        pub let f(x: Int) -> Int = 
            sum x in [1, 2, 3] { x };
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

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

#[test]
fn list_comprehension_shadows_parameter() {
    let input = r#"
        pub let f(x: Int) -> [Int] = 
            [x for x in [1, 2, 3]];
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

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

#[test]
fn nested_forall_shadows_outer_variable() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] { 
                forall s in @[Student] { 
                    0 <== 1 
                } 
            };
    "#;
    let (_, errors, warnings) = analyze(input, types, HashMap::new());

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

#[test]
fn multiple_scopes_with_same_name_in_sequence() {
    let input = r#"
        pub let f() -> Int = 
            (sum x in [1, 2, 3] { x }) + 
            (sum x in [4, 5, 6] { x });
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Same name in different scopes should work: {:?}",
        errors
    );
}

#[test]
fn nested_different_construct_scopes() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f(students: [Student]) -> Int = 
            sum s in students where (forall t in students { t.age > 0 }) { s.age };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested different constructs should work: {:?}",
        errors
    );
}

#[test]
fn if_expression_maintains_outer_scope() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Int = 
            if flag { x + 1 } else { x - 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If expression should access outer scope: {:?}",
        errors
    );
}
