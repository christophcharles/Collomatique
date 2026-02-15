use super::*;

// ========== List Literals ==========

#[tokio::test]
async fn empty_list() {
    let input = "pub let f() -> [Int] = [];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Empty list should work: {:?}", errors);
}

#[tokio::test]
async fn simple_list() {
    let input = "pub let f() -> [Int] = [1, 2, 3];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Simple list should work: {:?}", errors);
}

#[tokio::test]
async fn list_with_expressions() {
    let input = "pub let f(x: Int) -> [Int] = [x, x + 1, x * 2];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "List with expressions should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_type_mismatch() {
    let input = "pub let f() -> [Int] = [1, true, 3];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "List with mixed types should fail");
}

#[tokio::test]
async fn list_of_bool() {
    let input = "pub let f() -> [Bool] = [true, false, true];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "List of Bool should work: {:?}", errors);
}

#[tokio::test]
async fn list_of_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [LinExpr] = [$V(x), $V(x + 1)];";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "List of LinExpr should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn nested_lists() {
    let input = "pub let f() -> [[Int]] = [[1, 2], [3, 4], [5, 6]];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested lists should work: {:?}", errors);
}

#[tokio::test]
async fn nested_lists_with_empty() {
    let input = "pub let f() -> [[Int]] = [[], [1, 2], []];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Nested lists with empty should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_with_coercion() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> [Int | LinExpr] = [5, $V(x), 10];";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "List should unify Int and LinExpr: {:?}",
        errors
    );
}

// ============= List Ranges =============

#[tokio::test]
async fn collection_accepts_lists_range_with_numbers() {
    let input = "pub let f() -> [Int] = [0..42];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Simple list range should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn collection_accepts_lists_range_with_expr() {
    let input = r#"
    let count() -> Int = 32;
    pub let f(students: [Int]) -> [Int] = [count()..|students|];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Complex list range with expressions should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn collection_rejects_lists_range_with_wrong_type() {
    let input = "pub let f() -> [Int] = [0..true];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Wrong type in list range with expressions should not work: {:?}",
        errors
    );
    assert!(matches!(errors[0], SemError::TypeMismatch { .. }));
}

// ========== List Comprehensions ==========

#[tokio::test]
async fn simple_list_comprehension() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Simple comprehension should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_with_transformation() {
    let input = "pub let f() -> [Int] = [x * 2 for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Comprehension with transformation should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_with_where() {
    let input = "pub let f() -> [Int] = [x for x in [1, 2, 3, 4, 5] where x > 2];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Comprehension with where should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_type_transformation() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f() -> [LinExpr] = [$V(x) for x in [1, 2, 3]];";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Comprehension should transform types: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_with_struct_fields() {
    let input = "pub let f(students: [{age: Int}]) -> [Int] = [s.age for s in students];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Field access in comprehension should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_where_uses_field() {
    let input = "pub let f(students: [{age: Int}]) -> [{age: Int}] = [s for s in students where s.age > 18];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Where with field access should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn nested_list_comprehension() {
    let input = r#"
        pub let f() -> [[Int]] = 
            [[x * y for x in [1, 2, 3]] for y in [1, 2, 3]];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Nested comprehension should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_multiple_for_typechecks_correctly() {
    let input = r#"
        type Student = {age: Int, enroled: Bool};
        type Class = {num: Int, students: [Student]};
        pub let cross_ages(classes: [Class], all_students: [Student]) -> [Int] =
            [x.age + y.num
             for x in all_students
             for y in classes
             where x in y.students];
    "#;

    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Multiple for + where with membership should typecheck: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_where_can_reference_all_for_variables() {
    let input = r#"
        type Person = {id: Int, active: Bool};
        type Group = {members: [Person], min_age: Int};
        pub let active_pairs(groups: [Group]) -> [Bool] =
            [p1.active and p2.active
             for g in groups
             for p1 in g.members
             for p2 in g.members
             where p1.id < p2.id and g.min_age > 18];
    "#;

    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Where clause should access variables from all for-clauses: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_comprehension_rejects_non_iterable_in_second_for() {
    let input = "
        let f(s: {age: Int}, students: [Int]) -> [Int] = [x for y in s.age for x in students];
    ";

    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Second for loop over non-iterable (Int) should fail"
    );
}

// ========== Collection Operations ==========

#[tokio::test]
async fn union_of_lists() {
    let input = "pub let f() -> [Int] = [1, 2] + [3, 4];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Union should work: {:?}", errors);
}

#[tokio::test]
async fn difference_of_lists() {
    let input = "pub let f() -> [Int] = [1, 2, 3] - [2];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Difference should work: {:?}", errors);
}

#[tokio::test]
async fn union_with_coercion() {
    let input = "pub let f(xs: [Int], ys: [LinExpr]) -> [Int | LinExpr] = xs + ys;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Union with coercion should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn chained_collection_operations() {
    let input = "pub let f(a: [Int], b: [Int], c: [Int]) -> [Int] = a + b - c;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Chained collection ops should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn collection_operation_with_structs() {
    let input = "pub let f(a: [{x: Int}], b: [{x: Int}]) -> [{x: Int}] = a + b;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Collection ops with structs should work: {:?}",
        errors
    );
}

// ========== Cardinality ==========

#[tokio::test]
async fn cardinality_of_list_literal() {
    let input = "pub let f() -> Int = |[1, 2, 3, 4, 5]|;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Cardinality of list literal should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn cardinality_of_parameter() {
    let input = "pub let f(xs: [Int]) -> Int = |xs|;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Cardinality of parameter should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn cardinality_of_comprehension() {
    let input = "pub let f() -> Int = |[x for x in [1, 2, 3]]|;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Cardinality of comprehension should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn cardinality_in_comparison() {
    let input = "pub let f(xs: [Int]) -> Bool = |xs| > 10;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Cardinality in comparison should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn cardinality_in_constraint() {
    let input = "pub let f(xs: [Int]) -> Constraint = |xs| === 10;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Cardinality in constraint should work: {:?}",
        errors
    );
}

// ========== Membership ==========

#[tokio::test]
async fn element_in_list() {
    let input = "pub let f(x: Int, xs: [Int]) -> Bool = x in xs;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Membership test should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn membership_with_conversion() {
    let input = "pub let f(x: Int, xs: [LinExpr]) -> Bool = LinExpr(x) in xs;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Membership with coercion should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn membership_type_mismatch() {
    let input = "pub let f(x: Bool, xs: [Int]) -> Bool = x in xs;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Membership with wrong type should fail");
}

// ========== Complex Scenarios ==========

#[tokio::test]
async fn list_of_list_comprehensions() {
    let input =
        "pub let f() -> [[Int]] = [[x * 2 for x in [1, 2, 3]], [y + 1 for y in [4, 5, 6]]];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "List of comprehensions should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn comprehension_over_union() {
    let input = "pub let f(a: [Int], b: [Int]) -> [Int] = [x * 2 for x in a + b];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Comprehension over + should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn filtering_with_cardinality() {
    let input = r#"
        pub let f(lists: [[Int]]) -> [[Int]] = 
            [xs for xs in lists where |xs| > 3];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Filtering with cardinality should work: {:?}",
        errors
    );
}

// ============= Var list =============

#[tokio::test]
async fn var_list_can_be_used_as_source_collection() {
    let input = r#"
    let constraints(vals: [Int]) -> [Constraint] = [0 <== v for v in vals];
    reify constraints as $[MyConstraints];
    pub let f() -> LinExpr = sum x in $[MyConstraints]([0..42]) { x };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Var lists should work as source collections: {:?}",
        errors
    );
}
