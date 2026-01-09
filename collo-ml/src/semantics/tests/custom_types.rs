use super::*;

// =============================================================================
// CUSTOM TYPE DECLARATION TESTS
// =============================================================================

#[test]
fn custom_type_declaration_basic() {
    let input = r#"
        type MyInt = Int;
        let f() -> MyInt = MyInt(5);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_declaration_with_tuple() {
    let input = r#"
        type Point = (Int, Int);
        let origin() -> Point = Point(0, 0);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_declaration_with_list() {
    let input = r#"
        type IntList = [Int];
        let empty() -> IntList = IntList([]);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_declaration_with_sum_type() {
    // Note: Custom types wrapping sum types can be declared, but converting
    // TO them is currently limited because the conversion check requires
    // concrete types. We just verify the declaration works.
    let input = r#"
        type MaybeInt = Int | None;
        let get_type() -> Bool = true;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_referencing_previous_custom_type() {
    let input = r#"
        type MyInt = Int;
        type MyIntList = [MyInt];
        let make_list() -> MyIntList = MyIntList([MyInt(5)]);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// TYPE CONVERSION (INTO) TESTS
// =============================================================================

#[test]
fn into_custom_type_basic() {
    let input = r#"
        type MyInt = Int;
        let wrap(x: Int) -> MyInt = MyInt(x);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn into_underlying_type() {
    let input = r#"
        type MyInt = Int;
        let unwrap(x: MyInt) -> Int = Int(x);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn into_roundtrip() {
    let input = r#"
        type MyInt = Int;
        let roundtrip(x: Int) -> Int = Int(MyInt(x));
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// CUSTOM TYPE FUNCTION PARAMETER/RETURN TESTS
// =============================================================================

#[test]
fn custom_type_as_parameter() {
    let input = r#"
        type MyInt = Int;
        let process(x: MyInt) -> Int = Int(x);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_as_return_type() {
    let input = r#"
        type MyInt = Int;
        let create() -> MyInt = MyInt(42);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn multiple_custom_types_in_function() {
    let input = r#"
        type TypeA = Int;
        type TypeB = Bool;
        let combine(a: TypeA, b: TypeB) -> Int = if Bool(b) { Int(a) } else { 0 };
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// FIELD ACCESS THROUGH CUSTOM TYPES
// =============================================================================

#[test]
fn field_access_through_custom_type_with_tuple() {
    let input = r#"
        type Point = (Int, Int);
        let get_x(p: Point) -> Int = p.0;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn field_access_through_custom_type_with_object() {
    let input = r#"
        type MyStudent = Student;
        let get_age(s: MyStudent) -> Int = s.age;
    "#;
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let (_, errors, _warnings) = analyze(input, types, HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn nested_custom_type_field_access() {
    let input = r#"
        type Point = (Int, Int);
        type NamedPoint = (String, Point);
        let get_x(np: NamedPoint) -> Int = np.1.0;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// ERROR CASES - SHADOWING
// =============================================================================

// Note: The error_shadowing_primitive_type test is removed because
// all primitive type names (Int, Bool, String, None, LinExpr, Constraint, Never)
// are reserved keywords in the grammar, so the parser rejects them before
// the semantic analysis can check for shadowing. The semantic check is still
// implemented as a defense-in-depth measure but cannot be triggered from valid syntax.

#[test]
fn error_shadowing_object_type() {
    let input = r#"
        type Student = Int;
    "#;
    let types = simple_object("Student");
    let (_, errors, _warnings) = analyze(input, types, HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should error when shadowing object type"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeShadowsObject { .. })));
}

#[test]
fn error_shadowing_previous_custom_type() {
    let input = r#"
        type MyType = Int;
        type MyType = Bool;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should error when shadowing custom type"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeShadowsCustomType { .. })));
}

// =============================================================================
// ERROR CASES - RECURSIVE TYPES
// =============================================================================

// =============================================================================
// GUARDED RECURSIVE TYPES (NOW ALLOWED)
// Recursion inside containers (List, Tuple) is valid
// =============================================================================

#[test]
fn recursive_type_in_list() {
    // Self-reference inside a list is guarded recursion - now allowed
    let input = r#"
        type A = [A];
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Guarded recursion in list should be allowed: {:?}",
        errors
    );
}

#[test]
fn recursive_type_in_tuple() {
    // Self-reference inside a tuple is guarded recursion - now allowed
    let input = r#"
        type A = (Int, A);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Guarded recursion in tuple should be allowed: {:?}",
        errors
    );
}

#[test]
fn recursive_type_indirect_guarded() {
    // Indirect recursion inside containers is guarded - now allowed
    let input = r#"
        type A = Int;
        type B = [A];
        type C = (B, C);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Guarded indirect recursion should be allowed: {:?}",
        errors
    );
}

// =============================================================================
// ERROR CASES - TYPE MISMATCH
// =============================================================================

#[test]
fn error_wrong_type_return() {
    let input = r#"
        type MyInt = Int;
        let f(x: Int) -> MyInt = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should error: Int doesn't match MyInt without into"
    );
}

#[test]
fn error_wrong_type_parameter() {
    let input = r#"
        type MyInt = Int;
        let f(x: MyInt) -> Int = x;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should error: MyInt doesn't convert to Int without into"
    );
}

#[test]
fn error_incompatible_custom_types() {
    let input = r#"
        type TypeA = Int;
        type TypeB = Int;
        let f(x: TypeA) -> TypeB = TypeB(x);
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    // TypeA cannot be converted to TypeB directly, even though both are Int underneath
    // The user must first convert to Int, then to TypeB
    assert!(
        !errors.is_empty(),
        "Should error: TypeA cannot convert directly to TypeB"
    );
}

// =============================================================================
// CUSTOM TYPES IN COLLECTIONS
// =============================================================================

#[test]
fn custom_type_in_list() {
    let input = r#"
        type MyInt = Int;
        let make_list() -> [MyInt] = [MyInt(1), MyInt(2)];
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn sum_with_custom_type() {
    let input = r#"
        type MyInt = Int;
        let total(xs: [MyInt]) -> Int = sum x in xs { Int(x) };
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// CUSTOM TYPES WITH FORALL
// =============================================================================

#[test]
fn forall_with_custom_type() {
    let input = r#"
        type MyInt = Int;
        let check(xs: [MyInt]) -> Bool = forall x in xs { (Int(x)) > 0 };
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

// =============================================================================
// ERROR CASES - UNKNOWN TYPE
// =============================================================================

#[test]
fn error_unknown_custom_type() {
    let input = r#"
        let f(x: UnknownType) -> Int = 5;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(!errors.is_empty(), "Should error on unknown type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownType { .. })));
}

#[test]
fn forward_reference_to_custom_type_now_allowed() {
    // Forward references to types are now allowed
    let input = r#"
        let f() -> B = B(5);
        type B = Int;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Forward reference to custom type should now be allowed: {:?}",
        errors
    );
}

// =============================================================================
// CUSTOM TYPES WRAPPING UNION TYPES
// =============================================================================

#[test]
fn custom_type_wrapping_union_tuple_index_same_type() {
    // Custom type wraps union of tuples, field access should work
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool);
        let f(x: MyType) -> Bool = x.1;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_wrapping_union_tuple_index_different_types() {
    // Custom type wraps union of tuples with different first element types
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool);
        let f(x: MyType) -> Int | String = x.0;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_wrapping_nested_custom_type_union() {
    // type A wraps tuple, type B is union containing A
    let input = r#"
        type A = (Int, Int);
        type B = A | (String, Int);
        let f(x: B) -> Int = x.1;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn custom_type_wrapping_union_with_triple_tuple() {
    // Union of tuples with different lengths - both have index 0 and 1
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool, Int);
        let f(x: MyType) -> Bool = x.1;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}

#[test]
fn error_custom_type_wrapping_union_invalid_index() {
    // Union where one variant doesn't have the index
    let input = r#"
        type MyType = (Int, Bool) | String;
        let f(x: MyType) -> Bool = x.1;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should error: String has no tuple index"
    );
}

#[test]
fn error_custom_type_wrapping_union_index_out_of_bounds() {
    // Union where one variant doesn't have sufficient elements (only index 0 and 1)
    let input = r#"
        type MyType = (Int, Bool, String) | (Int, Bool);
        let f(x: MyType) -> String = x.2;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Should error: second variant has no index 2"
    );
}

#[test]
fn custom_type_wrapping_custom_type_tuple_index() {
    // Custom type wrapping another custom type that wraps a tuple
    let input = r#"
        type UnionType = (Int, Bool);
        type MyType = UnionType;
        let f(x: MyType) -> Int = x.0;
    "#;
    let (_, errors, _warnings) = analyze(input, HashMap::new(), HashMap::new());
    assert!(errors.is_empty(), "Errors: {:?}", errors);
}
