use super::*;

// ========== Basic Fold Expressions ==========

#[test]
fn simple_fold() {
    let input = "pub let f() -> Int = fold x in [1, 2, 3] with acc = 0 { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Simple fold should work: {:?}", errors);
}

#[test]
fn fold_returns_accumulator_type() {
    let input = "pub let f() -> Int = fold x in [1, 2, 3] with total = 0 { total + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold should return accumulator type: {:?}",
        errors
    );
}

#[test]
fn fold_with_linexpr_accumulator() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = fold i in [1, 2, 3] with acc = $V(x) { acc + i };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Fold with LinExpr accumulator should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_list_accumulator() {
    let input = "pub let f() -> [Int] = fold x in [1, 2, 3] with acc = [] as [Int] { acc + [x] };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with list accumulator should work: {:?}",
        errors
    );
}

#[test]
fn fold_body_must_match_accumulator_type() {
    let input = "pub let f() -> Int = fold x in [1, 2, 3] with acc = 0 { true };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Fold body must match accumulator type");
}

#[test]
fn fold_body_can_coerce_to_accumulator_type() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = fold i in [1, 2, 3] with acc = $V(x) { acc + 1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Fold body should coerce Int to LinExpr: {:?}",
        errors
    );
}

#[test]
fn fold_must_iterate_over_list() {
    let input = "pub let f(x: Int) -> Int = fold y in x with acc = 0 { acc + y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Fold must iterate over list");
}

// ========== Fold with Where Clause ==========

#[test]
fn fold_with_where_clause() {
    let input = "pub let f(xs: [Int]) -> Int = fold x in xs with acc = 0 where x > 10 { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with where should work: {:?}",
        errors
    );
}

#[test]
fn fold_where_must_be_bool() {
    let input = "pub let f(xs: [Int]) -> Int = fold x in xs with acc = 0 where x { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Fold where clause must be Bool");
}

#[test]
fn fold_with_complex_where() {
    let input = "pub let f(xs: [Int]) -> Int = fold x in xs with acc = 0 where x > 0 and x < 100 { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with complex where should work: {:?}",
        errors
    );
}

// ========== Different Collection Types ==========

#[test]
fn fold_over_parameter() {
    let input = "pub let f(xs: [Int]) -> Int = fold x in xs with acc = 0 { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold over parameter should work: {:?}",
        errors
    );
}

#[test]
fn fold_over_global_collection() {
    let types = simple_object("Student");
    let input = "pub let f() -> Int = fold s in @[Student] with acc = 0 { acc + 1 };";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold over global collection should work: {:?}",
        errors
    );
}

#[test]
fn fold_over_list_comprehension() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            fold x in [y * 2 for y in xs] with acc = 0 { acc + x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold over comprehension should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_field_access() {
    let types = object_with_fields("Student", vec![("age", ExprType::Int)]);
    let input = r#"
        pub let f(students: [Student]) -> Int = 
            fold s in students with acc = 0 { acc + s.age };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with field access should work: {:?}",
        errors
    );
}

// ========== Different Accumulator Initial Values ==========

#[test]
fn fold_with_computed_init_value() {
    let input = "pub let f(x: Int) -> Int = fold i in [1, 2, 3] with acc = x * 2 { acc + i };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with computed init should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_function_call_init_value() {
    let input = r#"
        pub let get_init() -> Int = 0;
        pub let f() -> Int = fold x in [1, 2, 3] with acc = get_init() { acc + x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with function call init should work: {:?}",
        errors
    );
}

#[test]
fn fold_init_value_determines_accumulator_type() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = fold i in [1, 2, 3] with acc = $V(x) { acc };";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Fold init determines accumulator type: {:?}",
        errors
    );
}

// ========== Nested Folds ==========

#[test]
fn nested_fold() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            fold row in matrix with acc1 = 0 { 
                fold x in row with acc2 = acc1 { 
                    acc2 + x 
                } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested fold should work: {:?}", errors);
}

#[test]
fn fold_accessing_outer_accumulator() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            fold row in matrix with outer = 0 { 
                fold x in row with inner = 0 { 
                    inner + x + outer
                } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold accessing outer accumulator should work: {:?}",
        errors
    );
}

// ========== Complex Body Expressions ==========

#[test]
fn fold_with_if_in_body() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            fold x in xs with acc = 0 { 
                if x > 0 { acc + x } else { acc } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with if in body should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_let_in_body() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            fold x in xs with acc = 0 { 
                let double = x * 2 { acc + double } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with let in body should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_sum_in_body() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            fold row in matrix with acc = 0 { 
                acc + sum x in row { x } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with sum in body should work: {:?}",
        errors
    );
}

// ========== RFold ==========

#[test]
fn simple_rfold() {
    let input = "pub let f() -> Int = rfold x in [1, 2, 3] with acc = 0 { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Simple rfold should work: {:?}", errors);
}

#[test]
fn rfold_with_where() {
    let input =
        "pub let f(xs: [Int]) -> Int = rfold x in xs with acc = 0 where x > 10 { acc + x };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "RFold with where should work: {:?}",
        errors
    );
}

#[test]
fn rfold_body_must_match_accumulator() {
    let input = "pub let f() -> Int = rfold x in [1, 2, 3] with acc = 0 { true };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "RFold body must match accumulator type");
}

#[test]
fn nested_rfold() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            rfold row in matrix with acc1 = 0 { 
                rfold x in row with acc2 = 0 { 
                    acc2 + x 
                } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested rfold should work: {:?}", errors);
}

#[test]
fn mixing_fold_and_rfold() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            fold row in matrix with acc1 = 0 { 
                rfold x in row with acc2 = 0 { 
                    acc2 + x 
                } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Mixing fold and rfold should work: {:?}",
        errors
    );
}

// ========== Fold in Larger Expressions ==========

#[test]
fn fold_in_arithmetic() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            (fold x in xs with acc = 0 { acc + x }) * 2;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold in arithmetic should work: {:?}",
        errors
    );
}

#[test]
fn fold_in_comparison() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            (fold x in xs with acc = 0 { acc + x }) > 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold in comparison should work: {:?}",
        errors
    );
}

#[test]
fn fold_in_constraint() {
    let input = r#"
        pub let f(xs: [Int]) -> Constraint = 
            (fold x in xs with acc = 0 { acc + x }) === 10;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold in constraint should work: {:?}",
        errors
    );
}

// ========== Fold with Other Control Flow ==========

#[test]
fn if_with_fold() {
    let input = r#"
        pub let f(xs: [Int], flag: Bool) -> Int = 
            if flag { 
                fold x in xs with acc = 0 { acc + x } 
            } else { 
                0 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "If with fold should work: {:?}", errors);
}

#[test]
fn forall_with_fold() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Bool = 
            forall row in matrix { 
                (fold x in row with acc = 0 { acc + x }) > 0 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Forall with fold should work: {:?}",
        errors
    );
}

#[test]
fn sum_with_fold_in_body() {
    let input = r#"
        pub let f(matrix: [[Int]]) -> Int = 
            sum row in matrix { 
                fold x in row with acc = 0 { acc + x } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum with fold in body should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_forall_in_where() {
    let input = r#"
        pub let f(lists: [[Int]]) -> Int = 
            fold xs in lists with acc = 0 where (forall x in xs { x > 0 }) { acc + |xs| };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with forall in where should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_sum_in_where() {
    let input = r#"
        pub let f(lists: [[Int]]) -> Int = 
            fold xs in lists with acc = 0 where (sum x in xs { x }) > 10 { acc + 1 };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with sum in where should work: {:?}",
        errors
    );
}

// ========== Edge Cases ==========

#[test]
fn fold_returning_bool() {
    let input = r#"
        pub let f(xs: [Int]) -> Bool = 
            fold x in xs with acc = true { acc and (x > 0) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold returning Bool should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_nested_list_accumulator() {
    let input = r#"
        pub let f(xs: [Int]) -> [[Int]] = 
            fold x in xs with acc = [] as [[Int]] { acc + [[x]] };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with nested list accumulator should work: {:?}",
        errors
    );
}

#[test]
fn fold_building_list() {
    let input = r#"
        pub let f(xs: [Int]) -> [Int] = 
            fold x in xs with acc = [] as [Int] { 
                if x > 0 { acc + [x] } else { acc } 
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold building list should work: {:?}",
        errors
    );
}

#[test]
fn fold_with_object_type() {
    let types = object_with_fields("Student", vec![("age", ExprType::Int)]);
    let input = r#"
        pub let f(students: [Student], count: Int) -> Int = 
            fold s in students with acc = count { acc + 1 };
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold with object type should work: {:?}",
        errors
    );
}

#[test]
fn fold_accumulator_shadows_parameter() {
    let input = r#"
        pub let f(acc: Int, xs: [Int]) -> Int = 
            fold x in xs with acc = 0 { acc + x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold accumulator can shadow parameter: {:?}",
        errors
    );
}

#[test]
fn fold_iterator_shadows_accumulator_name() {
    let input = r#"
        pub let f(xs: [[Int]]) -> Int = 
            fold acc in xs with result = 0 { 
                fold x in acc with inner = 0 { inner + x }
            };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fold iterator can shadow outer accumulator: {:?}",
        errors
    );
}

#[test]
fn multiple_independent_folds() {
    let input = r#"
        pub let f(xs: [Int]) -> Int = 
            (fold x in xs with a = 0 { a + x }) + (fold y in xs with b = 0 { b + y });
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Multiple independent folds should work: {:?}",
        errors
    );
}
