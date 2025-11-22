use super::*;

// ========== Basic Coercion Tests ==========

#[test]
fn int_coerces_to_linexpr_in_return() {
    let input = "pub let f() -> LinExpr = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr: {:?}",
        errors
    );
}

#[test]
fn int_coerces_to_linexpr_in_arithmetic() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = x + $V(x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Int should coerce to LinExpr in arithmetic: {:?}",
        errors
    );
}

#[test]
fn int_coerces_to_linexpr_in_function_argument() {
    let input = r#"
        pub let double(x: LinExpr) -> LinExpr = x + x;
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
fn linexpr_does_not_coerce_to_int() {
    let vars = var_with_args("V", vec![ExprType::Int]);
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
    let vars = var_with_args("V", vec![ExprType::Int]);
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
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int, flag: Bool) -> LinExpr = if flag { 5 } else { $V(x) };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If should unify Int and LinExpr to LinExpr: {:?}",
        errors
    );
}

#[test]
fn if_unifies_linexpr_and_int() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int, flag: Bool) -> LinExpr = if flag { $V(x) } else { 5 };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If should unify LinExpr and Int to LinExpr: {:?}",
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
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> [LinExpr] = [5, $V(x), 10];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "List should unify Int and LinExpr to [LinExpr]: {:?}",
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
fn collection_union_unifies_types() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> [LinExpr] = [5] union [$V(x)];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Union should unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn collection_inter_unifies_types() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> [LinExpr] = [5, 10] inter [$V(x)];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Inter should unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn collection_diff_unifies_types() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> [LinExpr] = [5, 10] \\ [$V(x)];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Diff should unify [Int] and [LinExpr]: {:?}",
        errors
    );
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
fn forced_type_prohibits_coercion_in_constraint() {
    let input = "pub let f() -> Constraint = 5 as Int === 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Forced Int should not coerce to LinExpr in constraint"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn forced_type_prohibits_coercion_in_list() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> [LinExpr] = [$V(x), 5 as Int];";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Forced Int should not coerce to LinExpr in list"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn forced_type_allows_recast() {
    let input = "pub let f() -> LinExpr = (5 as Int) as LinExpr;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained type annotation should be valid: {:?}",
        errors
    );
}

#[test]
fn forced_type_in_arithmetic_operation() {
    let input = "pub let f() -> LinExpr = (5 as Int) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forced type should loosen after operation: {:?}",
        errors
    );
}

// ========== Nested List Coercion ==========

#[test]
fn nested_list_int_coerces_to_nested_list_linexpr() {
    let input = "pub let f() -> [[LinExpr]] = [[1, 2], [3, 4]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "[[Int]] should coerce to [[LinExpr]]: {:?}",
        errors
    );
}

#[test]
fn nested_emptylist_coercion_should_fail_without_annotation() {
    let input = "pub let f() -> [[Int]] = [[], [], []];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Nested empty lists should not coerce without type annotation: {:?}",
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
