use super::*;

// =============================================================================
// STRUCT TYPE INFERENCE
// =============================================================================

#[test]
fn struct_literal_basic_inference() {
    let input = "pub let f() -> {x: Int, y: Bool} = {x: 1, y: true};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Basic struct literal should work: {:?}",
        errors
    );
}

#[test]
fn struct_literal_single_field() {
    let input = "pub let f() -> {value: Int} = {value: 42};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Single field struct should work: {:?}",
        errors
    );
}

#[test]
fn struct_literal_multiple_fields() {
    let input = "pub let f() -> {a: Int, b: Bool, c: String} = {a: 1, b: true, c: \"hello\"};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Multi-field struct should work: {:?}",
        errors
    );
}

#[test]
fn struct_literal_with_expressions() {
    let input = "pub let f(x: Int) -> {total: Int, doubled: Int} = {total: x + 1, doubled: x * 2};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct with expressions should work: {:?}",
        errors
    );
}

#[test]
fn struct_empty() {
    let input = "pub let f() -> {} = {};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Empty struct should work: {:?}", errors);
}

// =============================================================================
// STRUCT FIELD ORDER INDEPENDENCE
// =============================================================================

#[test]
fn struct_field_order_type_matches_literal() {
    // Type has {x, y} order, literal has {y, x} order
    let input = "pub let f() -> {x: Int, y: Bool} = {y: true, x: 1};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct field order should be independent: {:?}",
        errors
    );
}

#[test]
fn struct_field_order_three_fields() {
    // Type has {a, b, c} order, literal has {c, a, b} order
    let input = "pub let f() -> {a: Int, b: Bool, c: String} = {c: \"hi\", a: 42, b: false};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Three-field struct order should be independent: {:?}",
        errors
    );
}

// =============================================================================
// STRUCT FIELD ACCESS
// =============================================================================

#[test]
fn struct_access_single_field() {
    let input = "pub let f(s: {x: Int}) -> Int = s.x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Single field access should work: {:?}",
        errors
    );
}

#[test]
fn struct_access_multiple_fields() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> Int = s.x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Field access on multi-field struct should work: {:?}",
        errors
    );
}

#[test]
fn struct_access_second_field() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> Bool = s.y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Access to second field should work: {:?}",
        errors
    );
}

#[test]
fn struct_access_on_literal() {
    let input = "pub let f() -> Int = {x: 42, y: true}.x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Field access on literal should work: {:?}",
        errors
    );
}

#[test]
fn struct_access_unknown_field() {
    let input = "pub let f(s: {x: Int}) -> Int = s.y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Access to unknown field should fail");
}

#[test]
fn struct_access_wrong_type() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> Int = s.y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Returning Bool as Int should fail");
}

// =============================================================================
// NESTED STRUCTS
// =============================================================================

#[test]
fn nested_struct_type() {
    let input = "pub let f() -> {inner: {x: Int}} = {inner: {x: 42}};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Nested struct should work: {:?}", errors);
}

#[test]
fn nested_struct_field_access() {
    let input = "pub let f(s: {inner: {x: Int}}) -> Int = s.inner.x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested struct field access should work: {:?}",
        errors
    );
}

#[test]
fn deeply_nested_struct() {
    let input = "pub let f() -> {a: {b: {c: Int}}} = {a: {b: {c: 1}}};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Deeply nested struct should work: {:?}",
        errors
    );
}

#[test]
fn deeply_nested_struct_access() {
    let input = "pub let f(s: {a: {b: {c: Int}}}) -> Int = s.a.b.c;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Deeply nested struct access should work: {:?}",
        errors
    );
}

// =============================================================================
// STRUCTS WITH LISTS
// =============================================================================

#[test]
fn struct_containing_list() {
    let input = "pub let f() -> {items: [Int]} = {items: [1, 2, 3]};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct containing list should work: {:?}",
        errors
    );
}

#[test]
fn list_of_structs() {
    let input = "pub let f() -> [{x: Int, y: Bool}] = [{x: 1, y: true}, {x: 2, y: false}];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List of structs should work: {:?}",
        errors
    );
}

#[test]
fn struct_field_access_in_list_comprehension() {
    let input = "pub let f(points: [{x: Int, y: Int}]) -> [Int] = [p.x for p in points];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct field access in list comprehension should work: {:?}",
        errors
    );
}

// =============================================================================
// STRUCTS WITH TUPLES
// =============================================================================

#[test]
fn struct_containing_tuple() {
    let input = "pub let f() -> {point: (Int, Int)} = {point: (1, 2)};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct containing tuple should work: {:?}",
        errors
    );
}

#[test]
fn tuple_containing_struct() {
    let input = "pub let f() -> ({x: Int}, Bool) = ({x: 42}, true);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple containing struct should work: {:?}",
        errors
    );
}

#[test]
fn struct_field_then_tuple_access() {
    let input = "pub let f(s: {point: (Int, Int)}) -> Int = s.point.0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct field then tuple access should work: {:?}",
        errors
    );
}

#[test]
fn tuple_then_struct_field_access() {
    let input = "pub let f(t: ({x: Int}, Bool)) -> Int = t.0.x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple then struct field access should work: {:?}",
        errors
    );
}

// =============================================================================
// STRUCTS WITH UNION TYPES
// =============================================================================

#[test]
fn struct_with_union_field() {
    let input =
        "pub let f(b: Bool) -> {value: Int | Bool} = if b { {value: 1} } else { {value: true} };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct with union field should work: {:?}",
        errors
    );
}

#[test]
fn struct_subtyping_covariant() {
    // {x: Int} should fit in {x: Int | String}
    let input = "pub let f() -> {x: Int | String} = {x: 1};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct subtyping should be covariant: {:?}",
        errors
    );
}

#[test]
fn option_struct() {
    let input = "pub let f(b: Bool) -> ?{x: Int} = if b { {x: 1} } else { none };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Option struct should work: {:?}", errors);
}

// =============================================================================
// STRUCT TYPE ERRORS
// =============================================================================

#[test]
fn struct_type_mismatch_field_type() {
    let input = "pub let f() -> {x: Int} = {x: true};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Struct with wrong field type should fail"
    );
}

#[test]
fn struct_type_mismatch_missing_field() {
    let input = "pub let f() -> {x: Int, y: Bool} = {x: 1};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Struct with missing field should fail");
}

#[test]
fn struct_type_mismatch_extra_field() {
    let input = "pub let f() -> {x: Int} = {x: 1, y: 2};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Struct with extra field should fail");
}

#[test]
fn struct_duplicate_field_in_literal() {
    let input = "pub let f() -> {x: Int} = {x: 1, x: 2};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Struct with duplicate field should fail"
    );
}

// =============================================================================
// STRUCTS IN EXPRESSIONS
// =============================================================================

#[test]
fn struct_in_if_expression() {
    let input = "pub let f(b: Bool) -> {x: Int} = if b { {x: 1} } else { {x: 2} };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct in if expression should work: {:?}",
        errors
    );
}

#[test]
fn struct_in_let_expression() {
    let input = "pub let f() -> Int = let s = {x: 10, y: 20} { s.x + s.y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct in let expression should work: {:?}",
        errors
    );
}

#[test]
fn struct_access_in_sum() {
    let input = "pub let f(points: [{x: Int, y: Int}]) -> Int = sum p in points { p.x + p.y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct access in sum should work: {:?}",
        errors
    );
}

#[test]
fn struct_access_in_forall() {
    let input =
        "pub let f(points: [{x: Int, y: Int}]) -> Bool = forall p in points { p.x <= p.y };";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct access in forall should work: {:?}",
        errors
    );
}

#[test]
fn struct_creation_in_list_comprehension() {
    let input = "pub let f(xs: [Int]) -> [{val: Int, double: Int}] = [{val: x, double: x * 2} for x in xs];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct creation in list comprehension should work: {:?}",
        errors
    );
}

// =============================================================================
// STRUCT TYPE CONVERSION
// =============================================================================

#[test]
fn struct_to_string_conversion() {
    let input = "pub let f(s: {x: Int, y: Bool}) -> String = String(s);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct to string conversion should work: {:?}",
        errors
    );
}

#[test]
fn struct_field_to_string_conversion() {
    let input = "pub let f(s: {x: Int}) -> String = String(s.x);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct field to string conversion should work: {:?}",
        errors
    );
}

// =============================================================================
// NAMED STRUCTS VIA TYPE ALIAS
// =============================================================================
// Note: Type aliases to struct types create distinct Custom types.
// A struct literal {x: 1, y: 2} has type Struct, not Custom("Point").
// To use named struct types, you must pass values of that named type.

#[test]
fn named_struct_field_access() {
    // Field access works on named struct types
    let input = r#"
        type Point = {x: Int, y: Int};
        pub let f(p: Point) -> Int = p.x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Named struct field access should work: {:?}",
        errors
    );
}

#[test]
fn named_struct_nested() {
    let input = r#"
        type Inner = {value: Int};
        type Outer = {inner: Inner};
        pub let f(o: Outer) -> Int = o.inner.value;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested named struct should work: {:?}",
        errors
    );
}

// =============================================================================
// STRUCTS WITH OBJECTS
// =============================================================================

#[test]
fn struct_with_object_field() {
    let types = simple_object("Student");
    let input = "pub let f(s: Student) -> {student: Student, age: Int} = {student: s, age: 20};";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct with object field should work: {:?}",
        errors
    );
}

#[test]
fn struct_field_then_object_field() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = "pub let f(data: {student: Student}) -> Int = data.student.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct field then object field access should work: {:?}",
        errors
    );
}

// =============================================================================
// STRUCTS WITH LINEXPR
// =============================================================================

#[test]
fn struct_with_linexpr_field() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> {expr: LinExpr, val: Int} = {expr: $V(x), val: x};";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Struct with LinExpr field should work: {:?}",
        errors
    );
}

// =============================================================================
// TRAILING COMMA TESTS
// =============================================================================

#[test]
fn struct_type_trailing_comma() {
    let input = "pub let f() -> {x: Int, y: Bool,} = {x: 1, y: true};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct type with trailing comma should work: {:?}",
        errors
    );
}

#[test]
fn struct_literal_trailing_comma() {
    let input = "pub let f() -> {x: Int, y: Bool} = {x: 1, y: true,};";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Struct literal with trailing comma should work: {:?}",
        errors
    );
}

#[test]
fn tuple_type_trailing_comma() {
    let input = "pub let f() -> (Int, Bool,) = (1, true);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple type with trailing comma should work: {:?}",
        errors
    );
}

#[test]
fn tuple_literal_trailing_comma() {
    let input = "pub let f() -> (Int, Bool) = (1, true,);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Tuple literal with trailing comma should work: {:?}",
        errors
    );
}

#[test]
fn function_params_trailing_comma() {
    let input = "pub let f(x: Int, y: Bool,) -> Int = x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function params with trailing comma should work: {:?}",
        errors
    );
}

#[test]
fn function_call_trailing_comma() {
    let input = r#"
        let add(a: Int, b: Int) -> Int = a + b;
        pub let f() -> Int = add(1, 2,);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function call with trailing comma should work: {:?}",
        errors
    );
}
