use super::*;

// ========== Primitive Type Tests ==========

#[test]
fn int_type() {
    let input = "pub let f() -> Int = 42;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Int literal should be valid: {:?}",
        errors
    );
}

#[test]
fn bool_type() {
    let input = "pub let f() -> Bool = true;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Bool literal should be valid: {:?}",
        errors
    );
}

#[test]
fn linexpr_type_from_arithmetic() {
    let input = "pub let f(x: Int, y: Int) -> LinExpr = x + y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Arithmetic should produce LinExpr: {:?}",
        errors
    );
}

#[test]
fn constraint_type_from_comparison() {
    let input = "pub let f(x: Int) -> Constraint = x === 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "LinExpr comparison should produce Constraint: {:?}",
        errors
    );
}

// ========== List Type Tests ==========

#[test]
fn list_type_int() {
    let input = "pub let f() -> [Int] = [1, 2, 3];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Int list should be valid: {:?}", errors);
}

#[test]
fn list_type_bool() {
    let input = "pub let f() -> [Bool] = [true, false, true];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Bool list should be valid: {:?}", errors);
}

#[test]
fn nested_list_type() {
    let input = "pub let f() -> [[Int]] = [[1, 2], [3, 4]];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested lists should be valid: {:?}",
        errors
    );
}

#[test]
fn empty_list() {
    let input = "pub let f() -> [Int] = [];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Empty list should coerce to typed list: {:?}",
        errors
    );
}

#[test]
fn list_type_mismatch_in_elements() {
    let input = "pub let f() -> [Int] = [1, true, 3];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Mixed type list should error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

// ========== Object Type Tests ==========

#[test]
fn object_type_with_no_fields() {
    let types = simple_object("Student");
    let input = "pub let f(s: Student) -> Student = s;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Object type should be valid: {:?}",
        errors
    );
}

#[test]
fn object_type_with_fields() {
    let mut types = object_with_fields(
        "Student",
        vec![
            ("age", SimpleType::Int),
            ("name", SimpleType::Object("String".to_string())),
        ],
    );
    types.insert("String".to_string(), HashMap::new());
    let input = "pub let f(s: Student) -> Int = s.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Object field access should be valid: {:?}",
        errors
    );
}

#[test]
fn unknown_object_type() {
    let input = "pub let f(s: UnknownObject) -> Int = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Unknown object type should error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownType { .. })));
}

#[test]
fn unknown_field_access() {
    let types = simple_object("Student");
    let input = "pub let f(s: Student) -> Int = s.unknown_field;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(!errors.is_empty(), "Unknown field should error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownField { .. })));
}

#[test]
fn nested_field_access() {
    let mut types = HashMap::new();
    types.insert("String".to_string(), HashMap::new());

    let mut address_fields = HashMap::new();
    address_fields.insert(
        "city".to_string(),
        ExprType::simple(SimpleType::Object("String".to_string()))
            .try_into()
            .unwrap(),
    );
    types.insert("Address".to_string(), address_fields);

    let mut student_fields = HashMap::new();
    student_fields.insert(
        "address".to_string(),
        ExprType::simple(SimpleType::Object("Address".to_string()))
            .try_into()
            .unwrap(),
    );
    types.insert("Student".to_string(), student_fields);

    let input = "pub let f(s: Student) -> String = s.address.city;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Nested field access should be valid: {:?}",
        errors
    );
}

#[test]
fn field_access_on_non_object() {
    let input = "pub let f(x: Int) -> Int = x.field;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Field access on Int should error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FieldAccessOnNonObject { .. })));
}

// ========== Type Annotation Tests (as keyword) ==========

#[test]
fn explicit_type_annotation_valid() {
    let input = "pub let f() -> Int = 5 as Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Explicit type annotation should be valid: {:?}",
        errors
    );
}

#[test]
fn type_annotation_upcast() {
    let input = "pub let f() -> LinExpr = 5 as LinExpr;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Type annotation to LinExpr should be valid: {:?}",
        errors
    );
}

#[test]
fn type_annotation_invalid_cast() {
    let input = "pub let f() -> Int = true as Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Invalid type cast should error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn chained_type_annotations() {
    let input = "pub let f() -> LinExpr = (5 as Int) as LinExpr;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained type annotations should be valid: {:?}",
        errors
    );
}

// ========== List of Object Types ==========

#[test]
fn list_of_objects() {
    let types = simple_object("Student");
    let input = "pub let f(students: [Student]) -> [Student] = students;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "List of objects should be valid: {:?}",
        errors
    );
}

#[test]
fn list_of_objects_with_field_access_in_comprehension() {
    let types = object_with_fields("Student", vec![("age", SimpleType::Int)]);
    let input = "pub let f(students: [Student]) -> [Int] = [s.age for s in students];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Field access in list comprehension should be valid: {:?}",
        errors
    );
}

// ========== Global Collections ==========

#[test]
fn global_collection_primitive() {
    let input = "pub let f() -> [Int] = @[Int];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Global collection of Int should not be valid: {:?}",
        errors
    );
}

#[test]
fn global_collection_object() {
    let types = simple_object("Student");
    let input = "pub let f() -> [Student] = @[Student];";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Global collection of objects should be valid: {:?}",
        errors
    );
}

#[test]
fn global_collection_unknown_type() {
    let input = "pub let f() -> [UnknownType] = @[UnknownType];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Global collection of unknown type should error"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownType { .. })));
}
