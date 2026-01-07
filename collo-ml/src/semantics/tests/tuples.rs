use super::*;

// =============================================================================
// TUPLE TYPE INFERENCE
// =============================================================================

#[test]
fn tuple_literal_basic_inference() {
    let input = "pub let f() -> (Int, Bool) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Basic tuple literal should work: {:?}",
        errors
    );
}

#[test]
fn tuple_literal_three_elements() {
    let input = "pub let f() -> (Int, Bool, String) = (1, true, \"hello\");";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Triple tuple literal should work: {:?}",
        errors
    );
}

#[test]
fn tuple_literal_with_expressions() {
    let input = "pub let f(x: Int, y: Bool) -> (Int, Bool) = (x + 1, y);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple with expressions should work: {:?}",
        errors
    );
}

#[test]
fn tuple_type_mismatch() {
    let input = "pub let f() -> (Int, Bool) = (true, 1);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Tuple with swapped types should fail");
}

#[test]
fn tuple_element_count_mismatch() {
    let input = "pub let f() -> (Int, Bool) = (1, true, 3);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Tuple with wrong number of elements should fail"
    );
}

#[test]
fn tuple_element_count_mismatch_fewer() {
    let input = "pub let f() -> (Int, Bool, String) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Tuple with fewer elements than expected should fail"
    );
}

// =============================================================================
// TUPLE ACCESS
// =============================================================================

#[test]
fn tuple_access_first_element() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Accessing first element should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_second_element() {
    let input = "pub let f(t: (Int, Bool)) -> Bool = t.1;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Accessing second element should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_third_element() {
    let input = "pub let f(t: (Int, Bool, String)) -> String = t.2;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Accessing third element should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_out_of_bounds() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.2;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Accessing out of bounds index should fail"
    );
}

#[test]
fn tuple_access_wrong_type() {
    let input = "pub let f(t: (Int, Bool)) -> Int = t.1;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Returning Bool as Int should fail");
}

#[test]
fn tuple_access_on_non_tuple() {
    let input = "pub let f(x: Int) -> Int = x.0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Accessing tuple index on Int should fail"
    );
}

#[test]
fn tuple_access_chained() {
    let input = "pub let f(t: ((Int, Bool), String)) -> Bool = t.0.1;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained tuple access should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_on_literal() {
    let input = "pub let f() -> Int = (1, 2).0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple access on literal should work: {:?}",
        errors
    );
}

// =============================================================================
// NESTED TUPLES
// =============================================================================

#[test]
fn nested_tuple_type() {
    let input = "pub let f() -> ((Int, Bool), String) = ((1, true), \"x\");";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested tuple should work: {:?}", errors);
}

#[test]
fn deeply_nested_tuple() {
    let input = "pub let f() -> ((Int, (Bool, String)), Int) = ((1, (true, \"x\")), 2);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Deeply nested tuple should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH LISTS
// =============================================================================

#[test]
fn tuple_containing_list() {
    let input = "pub let f() -> ([Int], Bool) = ([1, 2, 3], true);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple containing list should work: {:?}",
        errors
    );
}

#[test]
fn list_of_tuples() {
    let input = "pub let f() -> [(Int, Bool)] = [(1, true), (2, false)];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List of tuples should work: {:?}",
        errors
    );
}

#[test]
fn list_of_tuples_type_mismatch() {
    let input = "pub let f() -> [(Int, Bool)] = [(1, true), (false, 2)];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "List of tuples with type mismatch should fail"
    );
}

#[test]
fn tuple_access_in_list_comprehension() {
    let input = "pub let f(pairs: [(Int, Bool)]) -> [Int] = [p.0 for p in pairs];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple access in list comprehension should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH UNION TYPES
// =============================================================================

#[test]
fn tuple_with_union_element() {
    let input =
        "pub let f(b: Bool) -> (Int | Bool, String) = if b { (1, \"a\") } else { (true, \"b\") };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple with union element should work: {:?}",
        errors
    );
}

#[test]
fn tuple_subtyping_covariant() {
    // (Int, Bool) should fit in (Int | String, Bool | Int)
    let input = "pub let f() -> (Int | String, Bool | Int) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple subtyping should be covariant: {:?}",
        errors
    );
}

#[test]
fn option_tuple() {
    let input = "pub let f(b: Bool) -> ?(Int, Bool) = if b { (1, true) } else { none };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Option tuple should work: {:?}", errors);
}

// =============================================================================
// TUPLES IN EXPRESSIONS
// =============================================================================

#[test]
fn tuple_in_if_expression() {
    let input = "pub let f(b: Bool) -> (Int, Bool) = if b { (1, true) } else { (2, false) };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple in if expression should work: {:?}",
        errors
    );
}

#[test]
fn tuple_in_let_expression() {
    let input = "pub let f() -> Int = let t = (1, 2) { t.0 + t.1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple in let expression should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_in_sum() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Int = sum p in pairs { p.0 + p.1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple access in sum should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_in_forall() {
    let input = "pub let f(pairs: [(Int, Int)]) -> Bool = forall p in pairs { p.0 <= p.1 };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple access in forall should work: {:?}",
        errors
    );
}

#[test]
fn tuple_creation_in_list_comprehension() {
    let input = "pub let f(xs: [Int]) -> [(Int, Int)] = [(x, x * 2) for x in xs];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple creation in list comprehension should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLE TYPE CONVERSION
// =============================================================================

#[test]
fn tuple_to_string_conversion() {
    let input = "pub let f(t: (Int, Bool)) -> String = t into String;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple to string conversion should work: {:?}",
        errors
    );
}

#[test]
fn tuple_element_to_string_conversion() {
    let input = "pub let f(t: (Int, Bool)) -> String = t.0 into String;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple element to string conversion should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH OBJECTS
// =============================================================================

#[test]
fn tuple_with_object() {
    let types = simple_object("Student");
    let input = "pub let f(s: Student) -> (Student, Int) = (s, 42);";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple with object should work: {:?}",
        errors
    );
}

#[test]
fn tuple_access_then_field_access() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = "pub let f(t: (Student, Int)) -> Int = t.0.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple access then field access should work: {:?}",
        errors
    );
}

#[test]
fn field_access_then_tuple_access() {
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert(
        "coords".to_string(),
        ExprType::simple(SimpleType::Tuple(vec![
            ExprType::simple(SimpleType::Int),
            ExprType::simple(SimpleType::Int),
        ])),
    );
    types.insert("Student".to_string(), student_fields);

    let input = "pub let f(s: Student) -> Int = s.coords.0;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Field access then tuple access should work: {:?}",
        errors
    );
}

// =============================================================================
// TUPLES WITH LINEXPR
// =============================================================================

#[test]
fn tuple_with_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> (LinExpr, Int) = ($V(x), x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Tuple with LinExpr should work: {:?}",
        errors
    );
}

#[test]
fn tuple_element_explicit_conversion_to_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    // Explicit conversion is needed for Int to LinExpr in tuple elements
    let input = "pub let f(x: Int) -> (LinExpr, LinExpr) = (x into LinExpr, $V(x));";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Tuple element explicit conversion should work: {:?}",
        errors
    );
}

#[test]
fn tuple_no_implicit_coercion_int_to_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    // Implicit coercion is NOT supported - this should fail
    let input = "pub let f(x: Int) -> (LinExpr, LinExpr) = (x, $V(x));";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Tuple should not implicitly coerce Int to LinExpr"
    );
}
