use super::*;

// ========== Basic Match Expressions ==========

#[test]
fn simple_match_with_one_branch() {
    let input = "pub let f(x: Int) -> Int = match x { y as Int { 10 } };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Simple match with one branch should work: {:?}",
        errors
    );
}

#[test]
fn match_with_multiple_branches() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { 1 } 
            b as Bool { 2 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with multiple branches should work: {:?}",
        errors
    );
}

#[test]
fn match_with_catchall_branch() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Int = match x { 
            i as Int { 1 } 
            other { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with catch-all should work: {:?}",
        errors
    );
}

#[test]
fn match_only_catchall() {
    let input = "pub let f(x: Int) -> Int = match x { y { 42 } };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with only catch-all should work: {:?}",
        errors
    );
}

// ========== Type Refinement ==========

#[test]
fn match_refines_union_type() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { i + 1 } 
            b as Bool { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match should refine union types: {:?}",
        errors
    );
}

#[test]
fn match_binding_has_refined_type() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { i * 2 }
            b as Bool { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Binding should have refined type (Int supports *): {:?}",
        errors
    );
}

#[test]
fn match_catchall_has_remaining_type() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Bool = match x { 
            i as Int { true }
            other { other == none } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Catch-all should have remaining type (Bool | None): {:?}",
        errors
    );
}

// ========== Type Conversion in Body ==========

#[test]
fn match_converts_int_to_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::LinExpr]);
    let input = r#"
        pub let f(x: Int) -> LinExpr = match x {
            i as Int { $V(LinExpr(i)) }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Match should convert Int to LinExpr in body: {:?}",
        errors
    );
}

#[test]
fn match_converts_emptylist_to_list() {
    let input = r#"
        pub let f(x: [] | Int) -> [Int] | Int = match x {
            i as Int { i }
            lst as [] { [Int]([1, 2, 3]) }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match should convert EmptyList to List in body: {:?}",
        errors
    );
}

#[test]
fn match_non_exhaustive_with_only_where() {
    // Match with only a filtered branch is not exhaustive
    let input = "pub let f(x: Int) -> Int = match x { i as Int where i > 0 { i } };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Match with only filtered branch should error (non-exhaustive)"
    );
}

// ========== Where Filters ==========

#[test]
fn match_with_where_filter() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            i as Int where i > 0 { i } 
            j as Int { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with where filter should work: {:?}",
        errors
    );
}

#[test]
fn match_where_must_be_bool() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            i as Int where i { i } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Match where clause must be Bool");
}

#[test]
fn match_where_does_not_refine_type() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            i as Int where i > 0 { i }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Match with only filtered branch should not be exhaustive"
    );
}

#[test]
fn match_where_with_conversion_in_body() {
    let vars = var_with_args("V", vec![SimpleType::LinExpr]);
    let input = r#"
        pub let f(x: Int) -> LinExpr | Int = match x {
            i as Int where x > 0 { $V(LinExpr(i)) }
            j as Int { j }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Match with where and conversion in body should work: {:?}",
        errors
    );
}

// ========== Exhaustiveness Checking ==========

#[test]
fn match_non_exhaustive_simple() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { 1 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Non-exhaustive match should error (missing Bool)"
    );
}

#[test]
fn match_exhaustive_with_catchall() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Int = match x { 
            i as Int { 1 } 
            other { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Exhaustive match with catch-all should work: {:?}",
        errors
    );
}

#[test]
fn match_exhaustive_all_branches() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Int = match x { 
            i as Int { 1 } 
            b as Bool { 2 } 
            n as None { 3 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Exhaustive match with all branches should work: {:?}",
        errors
    );
}

#[test]
fn match_non_exhaustive_partial_union() {
    let input = r#"
        pub let f(x: Int | Bool | None) -> Int = match x { 
            i as Int | Bool { 1 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Non-exhaustive match should error (missing None)"
    );
}

// ========== Over-matching / Unreachable Branches ==========

#[test]
fn match_detects_unreachable_after_exhaustive() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { 1 } 
            b as Bool { 2 } 
            other { 3 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Match should error on unreachable branch after exhaustive matching"
    );
}

#[test]
fn match_detects_overlapping_branches() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int | Bool { 1 } 
            j as Int { 2 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Match should error on overlapping branches (Int already matched)"
    );
}

#[test]
fn match_detects_duplicate_type() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { 1 } 
            j as Int { 2 } 
            b as Bool { 3 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Match should error on duplicate type branches"
    );
}

// ========== Branch Output Unification ==========

#[test]
fn match_unifies_branch_outputs() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int | Bool = match x { 
            i as Int { 5 } 
            b as Bool { true } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match should unify branch outputs: {:?}",
        errors
    );
}

#[test]
fn match_unifies_int_and_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = r#"
        pub let f(x: Int | Bool) -> Int | LinExpr = match x { 
            i as Int { 5 } 
            b as Bool { $V(0) } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Match should unify Int and LinExpr: {:?}",
        errors
    );
}

#[test]
fn match_unifies_emptylist_and_list() {
    let input = r#"
        pub let f(x: Int | Bool) -> [Int] = match x { 
            i as Int { [1, 2, 3] } 
            b as Bool { [] } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match should unify EmptyList and [Int]: {:?}",
        errors
    );
}

// ========== Lists and Subtypes ==========

#[test]
fn match_handles_emptylist() {
    let input = r#"
        pub let f(x: [Int]) -> Int = match x { 
            empty as [] { 0 } 
            lst as [Int] { |lst| }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match should handle EmptyList separately: {:?}",
        errors
    );
}

#[test]
fn match_emptylist_exhausts_list() {
    let input = r#"
        pub let f(x: [Int]) -> Int = match x { 
            lst as [Int] { |lst| } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match on [Int] should be exhaustive (includes []): {:?}",
        errors
    );
}

#[test]
fn match_list_conversion() {
    let input = r#"
        pub let f(x: [Int]) -> [LinExpr] = match x {
            lst as [Int] { [LinExpr](lst) }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match should convert [Int] to [LinExpr] in body: {:?}",
        errors
    );
}

// ========== Object Types ==========

#[test]
fn match_with_object_types() {
    let types = {
        let mut map = HashMap::new();
        map.insert("Student".to_string(), HashMap::new());
        map.insert("Teacher".to_string(), HashMap::new());
        map
    };
    let input = r#"
        pub let f(x: Student | Teacher) -> Int = match x { 
            s as Student { 1 } 
            t as Teacher { 2 } 
        };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with object types should work: {:?}",
        errors
    );
}

#[test]
fn match_with_field_access_in_branch() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f(x: Student | Int) -> Int = match x { 
            s as Student { s.age } 
            i as Int { i } 
        };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with field access should work: {:?}",
        errors
    );
}

#[test]
fn match_unknown_type_in_pattern() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            u as UnknownType { 1 } 
            i as Int { i } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Match should error on unknown type in pattern"
    );
}

// ========== Nested Match ==========

#[test]
fn nested_match_expressions() {
    let input = r#"
        pub let f(x: Int | Bool, y: Int | Bool) -> Int = match x { 
            i as Int { 
                match y { 
                    j as Int { i + j } 
                    b as Bool { i } 
                } 
            } 
            b as Bool { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested match should work: {:?}", errors);
}

#[test]
fn match_in_branch_body() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { i } 
            b as Bool { match 5 { y as Int { y } } } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match in branch body should work: {:?}",
        errors
    );
}

// ========== Match with Other Expressions ==========

#[test]
fn match_with_if_in_branch() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = match x { 
            i as Int { if i > 0 { i } else { 0 } } 
            b as Bool { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with if in branch should work: {:?}",
        errors
    );
}

#[test]
fn match_with_quantifier_in_branch() {
    let input = r#"
        pub let f(x: [Int] | Int) -> Int = match x { 
            lst as [Int] { sum i in lst { i } } 
            i as Int { i } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with quantifier should work: {:?}",
        errors
    );
}

#[test]
fn match_with_list_comprehension_in_branch() {
    let input = r#"
        pub let f(x: [Int] | Int) -> [Int] = match x { 
            lst as [Int] { [i * 2 for i in lst] } 
            i as Int { [i] } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with list comprehension should work: {:?}",
        errors
    );
}

#[test]
fn if_with_match_in_branches() {
    let input = r#"
        pub let f(x: Int | Bool, flag: Bool) -> Int = 
            if flag { 
                match x { 
                    i as Int { i } 
                    b as Bool { 0 } 
                } 
            } else { 
                5 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "If with match should work: {:?}", errors);
}

#[test]
fn forall_with_match_in_body() {
    let input = r#"
        pub let f(xs: [Int | Bool]) -> Bool = 
            forall x in xs { 
                match x { 
                    i as Int { i > 0 } 
                    b as Bool { b } 
                } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall with match should work: {:?}",
        errors
    );
}

// ========== Complex Real-World Examples ==========

#[test]
fn match_complex_type_dispatch() {
    let vars = var_with_args("V", vec![SimpleType::LinExpr]);
    let input = r#"
        pub let f(value: Int | Bool | [Int]) -> Constraint = match value {
            i as Int { $V(LinExpr(i)) === 0 }
            b as Bool { if b { 0 === 0 } else { 1 === 0 } }
            lst as [Int] { sum x in lst { $V(LinExpr(x)) } === 10 }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Complex type dispatch should work: {:?}",
        errors
    );
}

#[test]
fn match_with_filtered_branches() {
    let input = r#"
        pub let f(x: Int) -> Int = match x { 
            i as Int where i > 10 { i * 2 } 
            j as Int where j > 0 { j } 
            k as Int { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with multiple filtered branches should work: {:?}",
        errors
    );
}

#[test]
fn match_optional_handling() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f(student: Student | None) -> Int = match student { 
            s as Student { s.age } 
            n as None { 0 } 
        };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Match for optional handling should work: {:?}",
        errors
    );
}

#[test]
fn match_list_processing_with_conversion() {
    let vars = var_with_args("V", vec![SimpleType::LinExpr]);
    let input = r#"
        pub let f(items: [Int] | Int) -> Constraint = match items {
            lst as [Int] {
                sum x in lst { $V(LinExpr(x)) } === 100
            }
            i as Int { $V(LinExpr(i)) === 0 }
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Match with list processing and conversion should work: {:?}",
        errors
    );
}

#[test]
fn match_in_arithmetic_expression() {
    let input = r#"
        pub let f(x: Int | Bool) -> Int = 
            (match x { i as Int { i } b as Bool { 0 } }) + 5;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match in arithmetic should work: {:?}",
        errors
    );
}

#[test]
fn match_with_cardinality_in_filter() {
    let input = r#"
        pub let f(items: [Int] | Int) -> Int = match items { 
            lst as [Int] where |lst| > 0 { |lst| } 
            empty_list as [Int] { 0 }
            i as Int { i } 
        };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Match with cardinality in filter should work: {:?}",
        errors
    );
}
