use super::*;

// =============================================================================
// OPTION TYPE SEMANTIC TESTS - ?Type
// =============================================================================

#[test]
fn option_int_type_valid() {
    let input = "pub let f() -> ?Int = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Option Int should be valid: {:?}",
        errors
    );
}

#[test]
fn option_bool_type_valid() {
    let input = "pub let f() -> ?Bool = true;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Option Bool should be valid: {:?}",
        errors
    );
}

#[test]
fn option_custom_object_valid() {
    let types = simple_object("Student");
    let input = "pub let f(s: ?Student) -> ?Student = s;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Option custom object should be valid: {:?}",
        errors
    );
}

#[test]
fn option_list_type_valid() {
    let input = "pub let f() -> ?[Int] = [1, 2, 3];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Option list should be valid: {:?}",
        errors
    );
}

#[test]
fn list_of_option_type_valid() {
    let input = "pub let f() -> [?Int] = [1, none];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List of option should be valid: {:?}",
        errors
    );
}

#[test]
fn none_literal_valid() {
    let input = "pub let f() -> ?Int = none;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "None literal should be valid: {:?}",
        errors
    );
}

#[test]
fn option_type_annotation_valid() {
    let input = "pub let f() -> ?Int = 5 as ?Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Option type annotation should be valid",);
}

#[test]
fn option_type_can_hold_value() {
    // Option types are valid as return types and can hold concrete values
    let input = "pub let f() -> ?Int = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Option type should accept Int value: {:?}",
        errors
    );
}

// =============================================================================
// MULTIPLE OPTION MARKERS - SEMANTIC ERRORS
// =============================================================================

#[test]
fn double_question_mark_error() {
    let input = "pub let f() -> ??Int = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Double question mark should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleOptionMarkers { .. })),
        "Should have MultipleOptionMarkers error: {:?}",
        errors
    );
}

#[test]
fn triple_question_mark_error() {
    let input = "pub let f() -> ???Bool = true;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Triple question mark should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleOptionMarkers { .. })),
        "Should have MultipleOptionMarkers error: {:?}",
        errors
    );
}

#[test]
fn option_marker_on_none_error() {
    let input = "pub let f() -> ?None = none;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Option marker on None should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::OptionMarkerOnNone { .. })),
        "Should have OptionMarkerOnNone error: {:?}",
        errors
    );
}

// =============================================================================
// SUM TYPE SEMANTIC TESTS - Type1 | Type2
// =============================================================================

#[test]
fn sum_type_int_bool_valid() {
    let input = "pub let f() -> Int | Bool = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum type Int | Bool should be valid: {:?}",
        errors
    );
}

#[test]
fn sum_type_three_variants_valid() {
    let input = "pub let f() -> Int | Bool | LinExpr = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum type with three variants should be valid: {:?}",
        errors
    );
}

#[test]
fn sum_type_with_none_valid() {
    let input = "pub let f() -> None | Int | Bool = none;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum type with None should be valid: {:?}",
        errors
    );
}

#[test]
fn sum_type_custom_objects_valid() {
    let mut types = simple_object("Student");
    types.extend(simple_object("Teacher"));

    let input = "pub let f(p: Student | Teacher) -> Student | Teacher = p;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum of custom objects should be valid: {:?}",
        errors
    );
}

#[test]
fn list_of_sum_type_valid() {
    let input = "pub let f() -> [Int | Bool] = [1, true];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List of sum type should be valid: {:?}",
        errors
    );
}

#[test]
fn sum_of_list_types_valid() {
    let input = "pub let f() -> [Int] | [Bool] = [1, 2];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Sum of list types should be valid: {:?}",
        errors
    );
}

// =============================================================================
// DUPLICATE TYPES IN SUM - SEMANTIC ERRORS
// =============================================================================

#[test]
fn duplicate_types_in_sum_error() {
    let input = "pub let f() -> Int | Int = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Duplicate types in sum should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleTypeInSum { .. })),
        "Should have MultipleTypeInSum error: {:?}",
        errors
    );
}

#[test]
fn triplicate_types_in_sum_error() {
    let types = simple_object("Student");
    let input = "pub let f(x: Student) -> Student | Student | Student = x;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(!errors.is_empty(), "Triplicate types in sum should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleTypeInSum { .. })),
        "Should have MultipleTypeInSum error: {:?}",
        errors
    );
}

#[test]
fn triplicate_types_in_sum_error_with_extra_error() {
    let types = simple_object("Student");
    let input = "pub let f() -> Student | Student | Student = get();";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(!errors.is_empty(), "Triplicate types in sum should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleTypeInSum { .. })),
        "Should have MultipleTypeInSum error: {:?}",
        errors
    );
}

#[test]
fn duplicate_after_flattening_error() {
    // None | Int | None should error after flattening
    let input = "pub let f() -> None | Int | None = none;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Duplicate types after flattening should error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleTypeInSum { .. })),
        "Should have MultipleTypeInSum error: {:?}",
        errors
    );
}

// =============================================================================
// OPTION IN SUM TYPE - SEMANTIC ERRORS
// =============================================================================

#[test]
fn multiple_options_in_sum_type_error() {
    let input = "pub let f() -> ?Int | ?Bool = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Multiple options in sum type should error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleTypeInSum { .. })),
        "Should have MultipleTypeInSum error: {:?}",
        errors
    );
}

#[test]
fn option_with_none_in_sum_error() {
    // ?Int | None should error - redundant
    let input = "pub let f() -> ?Int | None = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Option with None in sum should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::MultipleTypeInSum { .. })),
        "Should have MultipleTypeInSum error: {:?}",
        errors
    );
}

// =============================================================================
// GLOBAL COLLECTIONS WITH SUM TYPES
// =============================================================================

#[test]
fn global_collection_of_sum_of_objects_valid() {
    let mut types = simple_object("Student");
    types.extend(simple_object("Teacher"));

    let input = "pub let f() -> [Student | Teacher] = @[Student | Teacher];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Global collection of object sum should be valid: {:?}",
        errors
    );
}

#[test]
fn global_collection_of_sum_with_primitives_error() {
    let input = "pub let f() -> [Int | Bool] = @[Int | Bool];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Global collection of primitive sum should error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::GlobalCollectionsMustBeAListOfObjects { .. })),
        "Should have GlobalCollectionsMustBeAListOfObjects error: {:?}",
        errors
    );
}

#[test]
fn global_collection_of_mixed_sum_error() {
    let types = simple_object("Student");
    let input = "pub let f() -> [Student | Int] = @[Student | Int];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        !errors.is_empty(),
        "Global collection of mixed sum should error"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::GlobalCollectionsMustBeAListOfObjects { .. })),
        "Should have GlobalCollectionsMustBeAListOfObjects error: {:?}",
        errors
    );
}

#[test]
fn global_collection_of_option_object_error() {
    let types = simple_object("Student");
    let input = "pub let f() -> [?Student] = @[?Student];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        !errors.is_empty(),
        "Global collection of option object should error"
    );
    // ?Student is sugar for None | Student, which includes None (not an object)
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::GlobalCollectionsMustBeAListOfObjects { .. })),
        "Should have GlobalCollectionsMustBeAListOfObjects error: {:?}",
        errors
    );
}

// =============================================================================
// TYPE COERCION WITH SUM TYPES
// =============================================================================

#[test]
fn coercion_to_sum_type_valid() {
    let input = "pub let f() -> Int | Bool = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Int should coerce to Int | Bool: {:?}",
        errors
    );
}

#[test]
fn conversion_within_sum_type_explicit() {
    // LinExpr(5) should work when target is LinExpr | Bool
    let input = "pub let f() -> LinExpr | Bool = LinExpr(5);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Type conversion should work: {:?}",
        errors
    );
}

#[test]
fn implicit_conversion_not_allowed() {
    // 5 should NOT implicitly convert to LinExpr | Bool (Int not in sum)
    let input = "pub let f() -> LinExpr | Bool = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Implicit conversion should not work without explicit cast"
    );
}

#[test]
fn empty_list_coercion_to_sum_with_one_list() {
    // [] should coerce to [Int] | Bool (only one list type)
    let input = "pub let f() -> [Int] | Bool = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Empty list should coerce when only one list in sum: {:?}",
        errors
    );
}

#[test]
fn empty_list_ambiguous_with_multiple_lists() {
    let input = "pub let f() -> [Int] | [Bool] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Empty list should concerce to super-type even if ambiguous"
    );
}

#[test]
fn explicit_empty_list_cast_valid() {
    let input = "pub let f() -> [Int] | [Bool] = [] as [Int];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Explicit cast should resolve ambiguity: {:?}",
        errors
    );
}

// =============================================================================
// COMPLEX NESTING
// =============================================================================

#[test]
fn option_of_list_of_sum_valid() {
    // Elements in [Int | Bool] need explicit casts
    let input = "pub let f() -> ?[Int | Bool] = [1, true];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Option of list of sum should be valid: {:?}",
        errors
    );
}

#[test]
fn list_of_option_valid() {
    // [?Int] where elements are explicitly cast
    let input = "pub let f() -> [?Int] = [1, none];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List of option should be valid: {:?}",
        errors
    );
}

#[test]
fn deeply_nested_types_valid() {
    let input = "pub let f() -> ?[[Int | Bool] | [LinExpr]] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Deeply nested types should be valid: {:?}",
        errors
    );
}

// =============================================================================
// FIELD ACCESS WITH SUM TYPES
// =============================================================================

#[test]
fn field_access_on_sum_type_error() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);

    // Cannot access field on sum type directly
    let input = "pub let f(p: Student | Int) -> Int = p.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        !errors.is_empty(),
        "Field access on sum type should error without matching"
    );
}

#[test]
fn field_access_on_option_type_error() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);

    // Cannot access field on option type directly
    let input = "pub let f(s: ?Student) -> Int = s.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        !errors.is_empty(),
        "Field access on option type should error without matching"
    );
}

// =============================================================================
// FUNCTION PARAMETERS WITH SUM TYPES
// =============================================================================

#[test]
fn function_with_sum_type_parameter_valid() {
    let input = r#"
        pub let process(x: Int | Bool) -> Int = 0;
        pub let use_process() -> Int = process(5);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function with sum type parameter should be valid: {:?}",
        errors
    );
}

#[test]
fn function_with_option_parameter_valid() {
    let input = r#"
        pub let maybe_process(x: ?Int) -> Int = 0;
        pub let use_maybe() -> Int = maybe_process(5);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Function with option parameter should be valid: {:?}",
        errors
    );
}

#[test]
fn function_call_with_wrong_type_for_sum() {
    let input = r#"
        pub let process(x: Int | Bool) -> Int = 0;
        pub let use_process() -> Int = process($V());
    "#;
    let vars = var_with_args("V", vec![]);
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        !errors.is_empty(),
        "Function call with LinExpr when Int | Bool expected should error"
    );
}

// =============================================================================
// UNKNOWN TYPES IN SUM
// =============================================================================

#[test]
fn unknown_type_in_sum_error() {
    let input = "pub let f() -> Int | UnknownType = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Unknown type in sum should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownType { .. })),
        "Should have UnknownType error: {:?}",
        errors
    );
}

#[test]
fn unknown_type_in_option_error() {
    let input = "pub let f() -> ?UnknownType = none;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Unknown type in option should error");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, SemError::UnknownType { .. })),
        "Should have UnknownType error: {:?}",
        errors
    );
}

// =============================================================================
// REALISTIC EXAMPLES
// =============================================================================

#[test]
fn realistic_option_return_type() {
    let types = simple_object("Student");
    let input = r#"
        pub let find_student(id: Int) -> ?Student = none;
        pub let process() -> ?Student = find_student(5);
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Realistic option usage should be valid: {:?}",
        errors
    );
}

#[test]
fn realistic_sum_type_for_mixed_entities() {
    let mut types = simple_object("Student");
    types.extend(simple_object("Teacher"));

    let input = r#"
        pub let count_all() -> Int = |@[Student | Teacher]|;
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Realistic sum for entity list should be valid: {:?}",
        errors
    );
}
