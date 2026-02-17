use super::*;

// =============================================================================
// TUPLE TYPE INFERENCE
// =============================================================================

#[tokio::test]
async fn tuple_literal_basic_inference() {
    let input = "pub let f() -> (Int, Bool) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Basic tuple literal should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_literal_three_elements() {
    let input = "pub let f() -> (Int, Bool, String) = (1, true, \"hello\");";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Triple tuple literal should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_literal_with_expressions() {
    let input = "pub let f(x: Int, y: Bool) -> (Int, Bool) = (x + 1, y);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple with expressions should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_type_mismatch() {
    let input = "pub let f() -> (Int, Bool) = (true, 1);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Tuple with swapped types should fail");
}

#[tokio::test]
async fn tuple_element_count_mismatch() {
    let input = "pub let f() -> (Int, Bool) = (1, true, 3);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Tuple with wrong number of elements should fail"
    );
}

#[tokio::test]
async fn tuple_element_count_mismatch_fewer() {
    let input = "pub let f() -> (Int, Bool, String) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Tuple with fewer elements than expected should fail"
    );
}

// =============================================================================
// TUPLE ACCESS
// =============================================================================

#[tokio::test]
async fn tuple_access_first_element() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.0;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Accessing first element should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_second_element() {
    let input = "pub let f(t: (Int, Bool)) -> Bool = t.1;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Accessing second element should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_third_element() {
    let input = "pub let f(t: (Int, Bool, String)) -> String = t.2;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Accessing third element should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_out_of_bounds() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.2;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Accessing out of bounds index should fail"
    );
}

#[tokio::test]
async fn tuple_access_wrong_type() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.1;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(!errors.is_empty(), "Returning Bool as Int should fail");
}

#[tokio::test]
async fn tuple_access_on_non_tuple() {
    let input = "pub let f(x: Int) -> Int = x.0;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Accessing tuple index on Int should fail"
    );
}

#[tokio::test]
async fn tuple_access_chained() {
    let input = "pub let f(t: ((Int, Bool), String)) -> Bool = t.0.1;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Chained tuple access should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_on_literal() {
    let input = "pub let f() -> Int = (1, 2).0;";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple access on literal should work: {:?}",
        errors
    );
}

// =============================================================================
// NESTED TUPLES
// =============================================================================

#[tokio::test]
async fn nested_tuple_type() {
    let input = "pub let f() -> ((Int, Bool), String) = ((1, true), \"x\");";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Nested tuple should work: {:?}", errors);
}

#[tokio::test]
async fn deeply_nested_tuple() {
    let input = "pub let f() -> ((Int, (Bool, String)), Int) = ((1, (true, \"x\")), 2);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Deeply nested tuple should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH LISTS
// =============================================================================

#[tokio::test]
async fn tuple_containing_list() {
    let input = "pub let f() -> ([Int], Bool) = ([1, 2, 3], true);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple containing list should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_of_tuples() {
    let input = "pub let f() -> [(Int, Bool)] = [(1, true), (2, false)];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "List of tuples should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn list_of_tuples_type_mismatch() {
    let input = "pub let f() -> [(Int, Bool)] = [(1, true), (false, 2)];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "List of tuples with type mismatch should fail"
    );
}

#[tokio::test]
async fn tuple_access_in_list_comprehension() {
    let input = "pub let f(pairs: [(Int, Bool)]) -> [Int] = [p.0 for p in pairs];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple access in list comprehension should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH UNION TYPES
// =============================================================================

#[tokio::test]
async fn tuple_with_union_element() {
    let input =
        "pub let f(b: Bool) -> (Int | Bool, String) = if b { (1, \"a\") } else { (true, \"b\") };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple with union element should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_subtyping_covariant() {
    // (Int, Bool) should fit in (Int | String, Bool | Int)
    let input = "pub let f() -> (Int | String, Bool | Int) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple subtyping should be covariant: {:?}",
        errors
    );
}

#[tokio::test]
async fn option_tuple() {
    let input = "pub let f(b: Bool) -> ?(Int, Bool) = if b { (1, true) } else { none };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(errors.is_empty(), "Option tuple should work: {:?}", errors);
}

// =============================================================================
// TUPLES IN EXPRESSIONS
// =============================================================================

#[tokio::test]
async fn tuple_in_if_expression() {
    let input = "pub let f(b: Bool) -> (Int, Bool) = if b { (1, true) } else { (2, false) };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple in if expression should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_in_let_expression() {
    let input = "pub let f() -> Int = let t = (1, 2) { t.0 + t.1 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple in let expression should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_in_sum() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Int = sum p in pairs { p.0 + p.1 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple access in sum should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_in_forall() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Bool = forall p in pairs { p.0 <= p.1 };";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple access in forall should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_creation_in_list_comprehension() {
    let input = "pub let f(xs: [Int]) -> [(Int, Int)] = [(x, x * 2) for x in xs];";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple creation in list comprehension should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLE TYPE CONVERSION
// =============================================================================

#[tokio::test]
async fn tuple_to_string_conversion() {
    let input = "pub let f(t: (Int, Bool)) -> String = String(t);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple to string conversion should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_element_to_string_conversion() {
    let input = "pub let f(t: (Int, Bool)) -> String = String(t.0);";
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple element to string conversion should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH STRUCTS
// =============================================================================

#[tokio::test]
async fn tuple_with_struct() {
    let input = r#"
        type Student = {age: Int};
        pub let f(s: Student) -> (Student, Int) = (s, 42);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple with struct should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_access_then_field_access() {
    let input = r#"
        type Student = {age: Int};
        pub let f(t: (Student, Int)) -> Int = t.0.age;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Tuple access then field access should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn field_access_then_tuple_access() {
    let input = r#"
        type Student = {coords: (Int, Int)};
        pub let f(s: Student) -> Int = s.coords.0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Field access then tuple access should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH LINEXPR
// =============================================================================

#[tokio::test]
async fn tuple_with_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> (LinExpr, Int) = ($V(x), x);";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Tuple with LinExpr should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_element_explicit_conversion_to_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    // Explicit conversion is needed for Int to LinExpr in tuple elements
    let input = "pub let f(x: Int) -> (LinExpr, LinExpr) = (LinExpr(x), $V(x));";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        errors.is_empty(),
        "Tuple element explicit conversion should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn tuple_no_implicit_coercion_int_to_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    // Implicit coercion is NOT supported - this should fail
    let input = "pub let f(x: Int) -> (LinExpr, LinExpr) = (x, $V(x));";
    let (_, errors, _) = analyze(input, vars).await;

    assert!(
        !errors.is_empty(),
        "Tuple should not implicitly coerce Int to LinExpr"
    );
}
