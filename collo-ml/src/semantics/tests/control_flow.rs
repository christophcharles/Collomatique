use super::*;

// ========== If Expressions ==========

#[tokio::test]
async fn simple_if_expression() {
    let input = "pub let f(x: Bool) -> Int = if x { 1 } else { 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Simple if expression should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_with_comparison_condition() {
    let input = "pub let f(x: Int) -> Int = if x > 5 { 10 } else { 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If with comparison should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_condition_must_be_bool() {
    let input = "pub let f(x: Int) -> Int = if x { 1 } else { 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "If condition must be Bool");
}

#[tokio::test]
async fn if_branches_must_have_same_type() {
    let input = "pub let f(x: Bool) -> Int = if x { 1 } else { true };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "If branches must unify");
}

#[tokio::test]
async fn if_unifies_int_and_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int, flag: Bool) -> Int | LinExpr = if flag { 5 } else { $V(x) };";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "If should unify Int and LinExpr: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_unifies_emptylist_and_list() {
    let input = "pub let f(flag: Bool) -> [Int] = if flag { [] } else { [1, 2, 3] };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If should unify EmptyList and [Int]: {:?}",
        errors
    );
}

#[tokio::test]
async fn nested_if_expressions() {
    let input = r#"
        pub let f(a: Bool, b: Bool) -> Int = 
            if a { 
                if b { 1 } else { 2 } 
            } else { 
                3 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested if should work: {:?}", errors);
}

#[tokio::test]
async fn if_with_complex_expressions() {
    let input = r#"
        pub let f(x: Int, y: Int, flag: Bool) -> Int = 
            if flag { x + y } else { x * y };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If with complex expressions should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_returning_constraint() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Constraint = 
            if flag { x === 0 } else { x === 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If returning Constraint should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_returning_bool() {
    let input = r#"
        pub let f(x: Int, flag: Bool) -> Bool = 
            if flag { x > 0 } else { x < 0 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If returning Bool should work: {:?}",
        errors
    );
}

// ========== Forall Expressions ==========

#[tokio::test]
async fn simple_forall() {
    let input = "pub let f(students: [Int]) -> Constraint = forall s in students { trivial };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Simple forall should work: {:?}", errors);
}

#[tokio::test]
async fn forall_with_bool_body() {
    let input = "pub let f(xs: [Int]) -> Bool = forall x in xs { x > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall with Bool body should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_with_constraint_body() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(xs: [Int]) -> Constraint = forall x in xs { $V(x) >== 0 };";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Forall with Constraint body should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_body_must_be_bool_or_constraint() {
    let input = "pub let f(xs: [Int]) -> Int = forall x in xs { x };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Forall body must be Bool or Constraint");
}

#[tokio::test]
async fn forall_with_where_clause() {
    let input = r#"
        pub let f(students: [{age: Int}]) -> Constraint =
            forall s in students where s.age > 18 { trivial };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall with where should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_where_must_be_bool() {
    let input = r#"
        pub let f(students: [Int]) -> Constraint =
            forall s in students where 5 { trivial };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Forall where clause must be Bool");
}

#[tokio::test]
async fn nested_forall() {
    let input = r#"
        pub let f(students: [Int]) -> Constraint =
            forall s1 in students {
                forall s2 in students {
                    trivial
                }
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested forall should work: {:?}", errors);
}

#[tokio::test]
async fn forall_over_list_parameter() {
    let input = "pub let f(xs: [Int]) -> Bool = forall x in xs { x > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall over parameter should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_over_list_literal() {
    let input = "pub let f() -> Bool = forall x in [1, 2, 3, 4, 5] { x > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall over literal should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_over_list_comprehension() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            forall x in [y * 2 for y in xs] { x > 0 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall over comprehension should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_must_iterate_over_list() {
    let input = "pub let f(x: Int) -> Bool = forall y in x { y > 0 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Forall must iterate over list");
}

// ========== Sum Expressions ==========

#[tokio::test]
async fn simple_sum() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { x };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Simple sum should work: {:?}", errors);
}

#[tokio::test]
async fn sum_returns_int_for_int_body() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { 1 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum should return Int for Int body: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_returns_linexpr_for_linexpr_body() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f() -> LinExpr = sum x in [1, 2, 3] { $V(x) };";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Sum should return LinExpr for LinExpr body: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_body_must_be_arithmetic() {
    let input = "pub let f() -> Int = sum x in [1, 2, 3] { true };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Sum body must be arithmetic type");
}

#[tokio::test]
async fn sum_with_where_clause() {
    let input = "pub let f(xs: [Int]) -> Int = sum x in xs where x > 10 { x };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum with where should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_where_must_be_bool() {
    let input = "pub let f(xs: [Int]) -> Int = sum x in xs where x { x };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Sum where clause must be Bool");
}

#[tokio::test]
async fn sum_over_list_parameter() {
    let input = "pub let f(students: [Int]) -> Int = sum s in students { 1 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum over list parameter should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_over_list_comprehension() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            sum x in [y * 2 for y in xs] { x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum over comprehension should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_must_iterate_over_list() {
    let input = "pub let f(x: Int) -> Int = sum y in x { y };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Sum must iterate over list");
}

#[tokio::test]
async fn nested_sum() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            sum row in matrix { sum x in row { x } };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested sum should work: {:?}", errors);
}

#[tokio::test]
async fn sum_with_field_access() {
    let input = r#"
        pub let f(students: [{age: Int}]) -> Int =
            sum s in students { s.age };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum with field access should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_in_comparison() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            (sum x in xs { 1 }) > 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum in comparison should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_in_constraint() {
    let input = r#"
        pub let f(xs: [Int]) -> Constraint = 
            (sum x in xs { 1 }) === 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum in constraint should work: {:?}",
        errors
    );
}

// ========== Complex Control Flow Combinations ==========

#[tokio::test]
async fn if_with_forall() {
    let input = r#"
        pub let f(xs: [Int], flag: Bool) -> Bool = 
            if flag { 
                forall x in xs { x > 0 } 
            } else { 
                true 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "If with forall should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn if_with_sum() {
    let input = r#"
        pub let f(xs: [Int], flag: Bool) -> Int = 
            if flag { 
                sum x in xs { x } 
            } else { 
                0 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "If with sum should work: {:?}", errors);
}

#[tokio::test]
async fn forall_with_nested_if() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            forall x in xs { 
                if x > 0 { x < 100 } else { true } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall with nested if should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_with_nested_if() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            sum x in xs { 
                if x > 0 { x } else { 0 } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum with nested if should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forall_containing_sum() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Bool = 
            forall row in matrix { 
                (sum x in row { x }) > 0 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forall containing sum should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn sum_containing_forall_in_where() {
    let input = r#"
        pub let f(lists: [[Int]]) -> Int = 
            sum xs in lists where (forall x in xs { x > 0 }) { |xs| };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Sum with forall in where should work: {:?}",
        errors
    );
}
