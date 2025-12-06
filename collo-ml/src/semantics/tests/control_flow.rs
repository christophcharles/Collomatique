use super::*;

// ========== If Expressions ==========

#[test]
fn simple_if_expression() {
    let input = "pub let f(x: Bool) -> Int = if x { 1 } else { 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Simple if expression should work: {:?}",
        errors
    );
}

#[test]
fn if_with_comparison_condition() {
    let input = "pub let f(x: Int) -> Int = if x > 5 { 10 } else { 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If with comparison should work: {:?}",
        errors
    );
}

#[test]
fn if_condition_must_be_bool() {
    let input = "pub let f(x: Int) -> Int = if x { 1 } else { 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "If condition must be Bool");
}

#[test]
fn if_branches_must_have_same_type() {
    let input = "pub let f(x: Bool) -> Int = if x { 1 } else { true };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "If branches must unify");
}

#[test]
fn if_unifies_int_and_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int, flag: Bool) -> LinExpr = if flag { 5 } else { $V(x) };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "If should unify Int and LinExpr: {:?}",
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
fn nested_if_expressions() {
    let input = r#"
        pub let f(a: Bool, b: Bool) -> Int = 
            if a { 
                if b { 1 } else { 2 } 
            } else { 
                3 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested if should work: {:?}", errors);
}

#[test]
fn if_with_complex_expressions() {
    let input = r#"
        pub let f(x: Int, y: Int, flag: Bool) -> Int = 
            if flag { x + y } else { x * y };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If with complex expressions should work: {:?}",
        errors
    );
}

#[test]
fn if_returning_constraint() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Constraint = 
            if flag { x === 0 } else { x === 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If returning Constraint should work: {:?}",
        errors
    );
}

#[test]
fn if_returning_bool() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Bool = 
            if flag { x > 0 } else { x < 0 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If returning Bool should work: {:?}",
        errors
    );
}

// ========== Forall Expressions ==========

#[test]
fn simple_forall() {
    let types = simple_object("Student");
    let input = "pub let f() -> Constraint = forall s in @[Student] { 0 <== 1 };";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Simple forall should work: {:?}", errors);
}

#[test]
fn forall_with_bool_body() {
    let input = "pub let f(xs: [Int]) -> Bool = forall x in xs { x > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall with Bool body should work: {:?}",
        errors
    );
}

#[test]
fn forall_with_constraint_body() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(xs: [Int]) -> Constraint = forall x in xs { $V(x) >== 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Forall with Constraint body should work: {:?}",
        errors
    );
}

#[test]
fn forall_body_must_be_bool_or_constraint() {
    let input = "pub let f(xs: [Int]) -> Int = forall x in xs { x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Forall body must be Bool or Constraint");
}

#[test]
fn forall_with_where_clause() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] where s.age > 18 { 0 <== 1 };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall with where should work: {:?}",
        errors
    );
}

#[test]
fn forall_where_must_be_bool() {
    let types = simple_object("Student");
    let input = r#"
        pub let f() -> Constraint = 
            forall s in @[Student] where 5 { 0 <== 1 };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(!errors.is_empty(), "Forall where clause must be Bool");
}

#[test]
fn nested_forall() {
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
fn forall_over_list_parameter() {
    let input = "pub let f(xs: [Int]) -> Bool = forall x in xs { x > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall over parameter should work: {:?}",
        errors
    );
}

#[test]
fn forall_over_list_literal() {
    let input = "pub let f() -> Bool = forall x in [1, 2, 3, 4, 5] { x > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall over literal should work: {:?}",
        errors
    );
}

#[test]
fn forall_over_list_comprehension() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            forall x in [y * 2 for y in xs] { x > 0 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall over comprehension should work: {:?}",
        errors
    );
}

#[test]
fn forall_must_iterate_over_list() {
    let input = "pub let f(x: Int) -> Bool = forall y in x { y > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Forall must iterate over list");
}

// ========== Sum Expressions ==========

#[test]
fn simple_sum() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Simple sum should work: {:?}", errors);
}

#[test]
fn sum_returns_int_for_int_body() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { 1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum should return Int for Int body: {:?}",
        errors
    );
}

#[test]
fn sum_returns_linexpr_for_linexpr_body() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f() -> LinExpr = sum x in [1, 2, 3] { $V(x) };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Sum should return LinExpr for LinExpr body: {:?}",
        errors
    );
}

#[test]
fn sum_body_must_be_arithmetic() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { true };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Sum body must be arithmetic type");
}

#[test]
fn sum_with_where_clause() {
    let input = "pub let f(xs: [Int]) -> Int = sum x in xs where x > 10 { x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum with where should work: {:?}",
        errors
    );
}

#[test]
fn sum_where_must_be_bool() {
    let input = "pub let f(xs: [Int]) -> Int = sum x in xs where x { x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Sum where clause must be Bool");
}

#[test]
fn sum_over_global_collection() {
    let types = simple_object("Student");
    let input = "pub let f() -> Int = sum s in @[Student] { 1 };";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum over global collection should work: {:?}",
        errors
    );
}

#[test]
fn sum_over_list_comprehension() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            sum x in [y * 2 for y in xs] { x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum over comprehension should work: {:?}",
        errors
    );
}

#[test]
fn sum_must_iterate_over_list() {
    let input = "pub let f(x: Int) -> Int = sum y in x { y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Sum must iterate over list");
}

#[test]
fn nested_sum() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            sum row in matrix { sum x in row { x } };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested sum should work: {:?}", errors);
}

#[test]
fn sum_with_field_access() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = r#"
        pub let f(students: [Student]) -> Int = 
            sum s in students { s.age };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum with field access should work: {:?}",
        errors
    );
}

#[test]
fn sum_in_comparison() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            (sum x in xs { 1 }) > 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum in comparison should work: {:?}",
        errors
    );
}

#[test]
fn sum_in_constraint() {
    let input = r#"
        pub let f(xs: [Int]) -> Constraint = 
            (sum x in xs { 1 }) === 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum in constraint should work: {:?}",
        errors
    );
}

// ========== Complex Control Flow Combinations ==========

#[test]
fn if_with_forall() {
    let input = r#"
        pub let f(xs: [Int], flag: Bool) -> Bool = 
            if flag { 
                forall x in xs { x > 0 } 
            } else { 
                true 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "If with forall should work: {:?}",
        errors
    );
}

#[test]
fn if_with_sum() {
    let input = r#"
        pub let f(xs: [Int], flag: Bool) -> Int = 
            if flag { 
                sum x in xs { x } 
            } else { 
                0 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "If with sum should work: {:?}", errors);
}

#[test]
fn forall_with_nested_if() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            forall x in xs { 
                if x > 0 { x < 100 } else { true } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall with nested if should work: {:?}",
        errors
    );
}

#[test]
fn sum_with_nested_if() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            sum x in xs { 
                if x > 0 { x } else { 0 } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum with nested if should work: {:?}",
        errors
    );
}

#[test]
fn forall_containing_sum() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Bool = 
            forall row in matrix { 
                (sum x in row { x }) > 0 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall containing sum should work: {:?}",
        errors
    );
}

#[test]
fn sum_containing_forall_in_where() {
    let input = r#"
        pub let f(lists: [[Int]]) -> Int = 
            sum xs in lists where (forall x in xs { x > 0 }) { |xs| };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum with forall in where should work: {:?}",
        errors
    );
}
