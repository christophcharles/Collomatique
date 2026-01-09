use super::*;

// ========== Basic Coercion Tests ==========

#[test]
fn int_does_not_coerce_to_linexpr_in_return() {
    let input = "pub let f() -> LinExpr = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Int should not coerce to LinExpr: {:?}",
        errors
    );
}

#[test]
fn int_coerces_to_linexpr_in_arithmetic() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = x + $V(x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr in arithmetic: {:?}",
        errors
    );
}

#[test]
fn int_does_not_coerce_to_linexpr_in_function_argument() {
    let input = r#"
        pub let double(x: LinExpr) -> LinExpr = x + x;
        pub let f() -> LinExpr = double(5);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Int should not coerce to LinExpr in function arg: {:?}",
        errors
    );
}

#[test]
fn linexpr_does_not_coerce_to_int() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> Int = $V(x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(!errors.is_empty(), "LinExpr should not coerce to Int");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

#[test]
fn emptylist_coerces_to_typed_list() {
    let input = "pub let f() -> [Int] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "EmptyList should coerce to [Int]: {:?}",
        errors
    );
}

#[test]
fn emptylist_coerces_to_list_of_linexpr() {
    let input = "pub let f() -> [LinExpr] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "EmptyList should coerce to [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn emptylist_coerces_to_nested_list() {
    let input = "pub let f() -> [[Int]] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "EmptyList should coerce to nested list: {:?}",
        errors
    );
}

// ========== No Bool to Constraint Coercion ==========

#[test]
fn bool_does_not_coerce_to_constraint_in_return() {
    let input = "pub let f() -> Constraint = 5 > 3;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Bool should not coerce to Constraint");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

#[test]
fn bool_allowed_in_forall_body() {
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
fn bool_and_constraint_cannot_mix() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> Constraint = (x > 5) and ($V(x) === 1);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Cannot mix Bool and Constraint with 'and'"
    );
}

#[test]
fn bool_in_logical_and() {
    let input = "pub let f(a: Bool, b: Bool) -> Bool = a and b;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Bool should work in logical AND: {:?}",
        errors
    );
}

#[test]
fn constraint_in_logical_and() {
    let input = "pub let f() -> Constraint = (5 === 0) and (10 === 0);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Constraint should work in logical AND: {:?}",
        errors
    );
}

// ========== Unification Tests ==========

#[test]
fn if_unifies_int_and_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int, flag: Bool) -> Int | LinExpr = if flag { 5 } else { $V(x) };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If should unify Int and LinExpr to Int | LinExpr: {:?}",
        errors
    );
}

#[test]
fn if_unifies_linexpr_and_int() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int, flag: Bool) -> Int | LinExpr = if flag { $V(x) } else { 5 };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If should unify LinExpr and Int to Int | LinExpr: {:?}",
        errors
    );
}

#[test]
fn if_unifies_emptylist_and_list() {
    let input = "pub let f(flag: Bool) -> [Int] = if flag { [] } else { [1, 2, 3] };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If should unify EmptyList and [Int]: {:?}",
        errors
    );
}

#[test]
fn if_unifies_list_and_emptylist() {
    let input = "pub let f(flag: Bool) -> [Int] = if flag { [1, 2, 3] } else { [] };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If should unify [Int] and EmptyList: {:?}",
        errors
    );
}

#[test]
fn if_cannot_unify_incompatible_types() {
    let input = "pub let f(flag: Bool) -> Int = if flag { 5 } else { true };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "If should not unify Int and Bool");
}

#[test]
fn list_literal_unifies_mixed_types() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int | LinExpr] = [5, $V(x), 10];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "List should unify Int and LinExpr to [Int | LinExpr]: {:?}",
        errors
    );
}

#[test]
fn list_literal_with_emptylist() {
    let input = "pub let f() -> [[Int]] = [[], [1, 2], []];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List should unify EmptyList with [Int]: {:?}",
        errors
    );
}

#[test]
fn collection_sum_unifies_types() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int | LinExpr] = [5] + [$V(x)];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Sum should unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn collection_diff_does_not_unify_types() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int] = [5, 10] - [$V(x) as Int | LinExpr];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Diff should not unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn collection_diff_checks_overlapping_types() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int] = [5, 10] - [$V(x)];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Diff should check overlapping types between [Int] and [LinExpr]: {:?}",
        errors
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn collection_diff_checks_not_from_empty() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int] = [] - [5];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Diff should check overlapping types between [] and [Int]: {:?}",
        errors
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn collection_diff_checks_not_by_empty() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int] = [5] - [];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Diff should check overlapping types between [] and [Int]: {:?}",
        errors
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn collection_diff_checks_not_by_and_no_from_empty() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int] = [] - [];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Diff should check that we are not doing [] - []: {:?}",
        errors
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

// ========== Forced Type (as) Prevents Coercion ==========

#[test]
fn forced_type_prohibits_coercion_in_return() {
    let input = "pub let f() -> LinExpr = 5 as Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Forced Int should not coerce to LinExpr"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::BodyTypeMismatch { .. })));
}

#[test]
fn coerced_type_allows_recast() {
    let input = "pub let f() -> Int | LinExpr = (5 as Int) as Int | LinExpr;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained type annotation should be valid: {:?}",
        errors
    );
}

#[test]
fn coerced_type_in_arithmetic_operation() {
    let input = "pub let f() -> Int = (5 as Int) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Coerced type should be allowed in operation: {:?}",
        errors
    );
}

// ========== Nested List Coercion ==========

#[test]
fn nested_list_int_converts_to_nested_list_linexpr() {
    let input = "pub let f() -> [[LinExpr]] = [[LinExpr]]([[1, 2], [3, 4]]);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "[[Int]] should convert to [[LinExpr]]: {:?}",
        errors
    );
}

#[test]
fn nested_emptylist_coercion_should_not_fail_without_annotation() {
    let input = "pub let f() -> [[Int]] = [[], [], []];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested empty lists should coerce even without type annotation: {:?}",
        errors
    );
}

#[test]
fn nested_emptylist_coercion_should_succeed_with_annotation() {
    let input = "pub let f() -> [[Int]] = [[], [], [] as [Int]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested empty lists should coerce with type annotation: {:?}",
        errors
    );
}
