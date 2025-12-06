use super::*;

#[test]
fn let_expr_with_simple_binding() {
    let input = "pub let f(x: Int) -> Int = let y = 5 { y + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Simple let binding should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_arithmetic_value() {
    let input = "pub let f(x: Int) -> Int = let doubled = x * 2 { doubled + 1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with arithmetic value should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_nested_bindings() {
    let input = "pub let f(x: Int) -> Int = let a = x { let b = a * 2 { b + 1 } };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested let bindings should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_list_value() {
    let input = "pub let f() -> [Int] = let items = [1, 2, 3] { items };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with list value should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_list_range() {
    let input = "pub let f(n: Int) -> [Int] = let range = [0..n] { range };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with list range should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_boolean_value() {
    let input = "pub let f(x: Int) -> Bool = let check = x > 5 { check };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with boolean value should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_membership_test() {
    let input = "pub let f(x: Int, list: [Int]) -> Bool = let is_member = x in list { is_member };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with membership test should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_if_body() {
    let input = "pub let f(x: Int) -> Int = let bound = 10 { if x > bound { 1 } else { 0 } };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with if body should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_forall_body() {
    let input = "pub let f(n: Int) -> Constraint = let bound = n * 2 { forall i in [0..bound] { $V(i) === 1 } };";
    let (_, errors, _) = analyze(
        input,
        HashMap::new(),
        var_with_args("V", vec![SimpleType::Int]),
    );

    assert!(
        errors.is_empty(),
        "Let with forall body should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_sum_body() {
    let input = "pub let f(items: [Int]) -> Int = let doubled_items = items { sum x in doubled_items { x * 2 } };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with sum body should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_constraint_value() {
    let input = "pub let f(x: Int) -> Constraint = let c = $V(x) === 1 { c };";
    let (_, errors, _) = analyze(
        input,
        HashMap::new(),
        var_with_args("V", vec![SimpleType::Int]),
    );

    assert!(
        errors.is_empty(),
        "Let with constraint value should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_function_call() {
    let input = r#"
        let helper(x: Int) -> Int = x * 2;
        pub let f(n: Int) -> Int = let result = helper(n) { result + 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with function call should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_object_field_access() {
    let input = "pub let f(s: Student) -> Int = let age = s.age { age + 1 };";
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with object field access should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_type_mismatch_in_value() {
    let input = "pub let f() -> Int = let x = true { x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Let binding with type mismatch should produce error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })),
        "Should have BodyTypeMismatch error: {:?}",
        errors
    );
}

#[test]
fn let_expr_type_mismatch_in_body() {
    let input = "pub let f() -> Int = let x = 5 { true };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Let with body type mismatch should produce error"
    );
    assert!(
        errors.iter().any(|e| matches!(
            e,
            SemError::BodyTypeMismatch { .. } | SemError::TypeMismatch { .. }
        )),
        "Should have type mismatch error: {:?}",
        errors
    );
}

#[test]
fn let_expr_undefined_variable_in_value() {
    let input = "pub let f() -> Int = let x = undefined_var { x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Let with undefined variable should produce error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownIdentifer { .. })),
        "Should have UnknownIdentifier error: {:?}",
        errors
    );
}

#[test]
fn let_expr_undefined_variable_in_body() {
    let input = "pub let f() -> Int = let x = 5 { undefined_var };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Let with undefined variable in body should produce error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownIdentifer { .. })),
        "Should have UnknownIdentifier error: {:?}",
        errors
    );
}

#[test]
fn let_expr_shadowing_parameter() {
    let input = "pub let f(x: Int) -> Int = let x = 10 { x };";
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let shadowing parameter should be allowed: {:?}",
        errors
    );
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should have shadowing warning: {:?}",
        warnings
    );
}

#[test]
fn let_expr_shadowing_outer_let() {
    let input = "pub let f() -> Int = let x = 5 { let x = 10 { x } };";
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let shadowing outer let should be allowed: {:?}",
        errors
    );
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::IdentifierShadowed { .. })),
        "Should have shadowing warning: {:?}",
        warnings
    );
}

#[test]
fn let_expr_with_list_comprehension() {
    let input = "pub let f(n: Int) -> [Int] = let bound = n * 2 { [i * 2 for i in [0..bound]] };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with list comprehension should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_cardinality() {
    let input = "pub let f(items: [Int]) -> Int = let list = items { |list| };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with cardinality should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_collection_operations() {
    let input = "pub let f(a: [Int], b: [Int]) -> [Int] = let combined = a + b { combined };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with collection operations should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_linexpr() {
    let input = "pub let f(x: Int) -> LinExpr = let expr = $V(x) { expr };";
    let (_, errors, _) = analyze(
        input,
        HashMap::new(),
        var_with_args("V", vec![SimpleType::Int]),
    );

    assert!(
        errors.is_empty(),
        "Let with LinExpr should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_constraint_combination() {
    let input = "pub let f(x: Int) -> Constraint = let c1 = $V(x) === 1 { let c2 = $V(x) <== 10 { c1 and c2 } };";
    let (_, errors, _) = analyze(
        input,
        HashMap::new(),
        var_with_args("V", vec![SimpleType::Int]),
    );

    assert!(
        errors.is_empty(),
        "Let with constraint combination should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_using_bound_var_multiple_times() {
    let input = "pub let f(x: Int) -> Int = let y = x * 2 { y + y + y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Using bound variable multiple times should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_complex_nesting() {
    let input = r#"
        pub let f(x: Int) -> Int = 
            let a = x * 2 {
                let b = a + 5 {
                    let c = b * 3 {
                        if c > 100 {
                            a + b
                        } else {
                            c
                        }
                    }
                }
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Complex nested let expressions should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_with_type_annotation() {
    let input = "pub let f(x: Int) -> Int = let y = (x * 2) as Int { y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with type annotation should work: {:?}",
        errors
    );
}

#[test]
fn let_expr_naming_convention_warning() {
    let input = "pub let f(x: Int) -> Int = let MyVar = 5 { MyVar };";
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with non-snake-case name should be allowed: {:?}",
        errors
    );
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::ParameterNamingConvention { .. })),
        "Should have naming convention warning: {:?}",
        warnings
    );
}

#[test]
fn let_expr_unused_binding_warning() {
    let input = "pub let f(x: Int) -> Int = let y = 10 { x };";
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with unused binding should be allowed: {:?}",
        errors
    );
    assert!(
        warnings.iter().any(
            |w| matches!(w, SemWarning::UnusedIdentifier { identifier, .. } if identifier == "y")
        ),
        "Should have unused identifier warning: {:?}",
        warnings
    );
}
