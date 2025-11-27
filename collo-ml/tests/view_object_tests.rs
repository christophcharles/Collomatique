use std::any::TypeId;
use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ExprType, ExprValue, FieldType, FieldValue, ViewObject};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct TestObjectId;

// Dummy implementation for testing
impl EvalObject for TestObjectId {
    type Env = ();

    fn field_access(&self, _env: &Self::Env, _field: &str) -> Option<ExprValue<Self>> {
        None
    }
    fn objects_with_typ(_env: &Self::Env, _name: &str) -> BTreeSet<Self> {
        BTreeSet::new()
    }
    fn typ_name(&self, _env: &Self::Env) -> String {
        String::new()
    }
    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        HashMap::new()
    }
}

// Dummy ID types for testing object references
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct RoomId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct CourseId(usize);

// Implement From for TestObjectId to allow conversion
impl From<RoomId> for TestObjectId {
    fn from(_: RoomId) -> Self {
        TestObjectId
    }
}

impl From<CourseId> for TestObjectId {
    fn from(_: CourseId) -> Self {
        TestObjectId
    }
}

// Test 1: Basic fields (Int, Bool)
#[derive(ViewObject)]
#[eval_object(TestObjectId)]
struct TestStudent {
    age: i32,
    enrolled: bool,
}

#[test]
fn test_field_schema_generation() {
    let schema = TestStudent::field_schema();
    assert_eq!(schema.len(), 2);
    assert_eq!(schema.get("age"), Some(&FieldType::Int));
    assert_eq!(schema.get("enrolled"), Some(&FieldType::Bool));
}

#[test]
fn test_get_field() {
    let student = TestStudent {
        age: 20,
        enrolled: true,
    };

    assert_eq!(student.get_field("age"), Some(FieldValue::Int(20)));
    assert_eq!(student.get_field("enrolled"), Some(FieldValue::Bool(true)));
    assert_eq!(student.get_field("nonexistent"), None);
}

// Test 2: Hidden fields
#[test]
fn test_hidden_fields() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithHidden {
        age: i32,
        #[hidden]
        secret: String,
    }

    let schema = StudentWithHidden::field_schema();
    assert_eq!(schema.len(), 1); // Only age, not secret
    assert!(schema.contains_key("age"));
    assert!(!schema.contains_key("secret"));

    let student = StudentWithHidden {
        age: 25,
        secret: "hidden data".to_string(),
    };

    // Can still access the visible field
    assert_eq!(student.get_field("age"), Some(FieldValue::Int(25)));
    // Cannot access hidden field through get_field
    assert_eq!(student.get_field("secret"), None);
    // But the field still exists in the struct for other purposes
    assert_eq!(student.secret, "hidden data");
}

// Test 3: Object references
#[test]
fn test_object_references() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithRoom {
        age: i32,
        room: RoomId,
    }

    let schema = StudentWithRoom::field_schema();
    assert_eq!(schema.len(), 2);
    assert_eq!(schema.get("age"), Some(&FieldType::Int));
    assert_eq!(
        schema.get("room"),
        Some(&FieldType::Object(TypeId::of::<RoomId>()))
    );

    let student = StudentWithRoom {
        age: 20,
        room: RoomId(42),
    };

    assert_eq!(student.get_field("age"), Some(FieldValue::Int(20)));
    assert_eq!(
        student.get_field("room"),
        Some(FieldValue::Object(TestObjectId))
    );
}

// Test 4: Collections of basic types
#[test]
fn test_collections_of_ints() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithGrades {
        name_length: i32,
        grades: BTreeSet<i32>,
    }

    let schema = StudentWithGrades::field_schema();
    assert_eq!(schema.len(), 2);
    assert_eq!(
        schema.get("grades"),
        Some(&FieldType::List(Box::new(FieldType::Int)))
    );

    let mut grades = BTreeSet::new();
    grades.insert(85);
    grades.insert(90);
    grades.insert(78);

    let student = StudentWithGrades {
        name_length: 5,
        grades: grades.clone(),
    };

    if let Some(FieldValue::List(FieldType::Int, values)) = student.get_field("grades") {
        assert_eq!(values.len(), 3);
        assert!(values.contains(&FieldValue::Int(85)));
        assert!(values.contains(&FieldValue::Int(90)));
        assert!(values.contains(&FieldValue::Int(78)));
    } else {
        panic!("Expected List of Ints");
    }
}

#[test]
fn test_collections_of_bools() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithFlags {
        flags: BTreeSet<bool>,
    }

    let schema = StudentWithFlags::field_schema();
    assert_eq!(
        schema.get("flags"),
        Some(&FieldType::List(Box::new(FieldType::Bool)))
    );

    let mut flags = BTreeSet::new();
    flags.insert(true);
    flags.insert(false);

    let student = StudentWithFlags { flags };

    if let Some(FieldValue::List(_, values)) = student.get_field("flags") {
        assert_eq!(values.len(), 2);
        assert!(values.contains(&FieldValue::Bool(true)));
        assert!(values.contains(&FieldValue::Bool(false)));
    } else {
        panic!("Expected List of Bools");
    }
}

// Test 5: Collections of object references
#[test]
fn test_collections_of_objects() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithCourses {
        age: i32,
        courses: BTreeSet<CourseId>,
    }

    let schema = StudentWithCourses::field_schema();
    assert_eq!(schema.len(), 2);
    assert_eq!(
        schema.get("courses"),
        Some(&FieldType::List(Box::new(FieldType::Object(TypeId::of::<
            CourseId,
        >()))))
    );

    let mut courses = BTreeSet::new();
    courses.insert(CourseId(101));
    courses.insert(CourseId(102));

    let student = StudentWithCourses { age: 20, courses };

    if let Some(FieldValue::List(_, values)) = student.get_field("courses") {
        assert_eq!(values.len(), 1); // All types fuse into one possible object in this simple test
                                     // All should be converted to TestObjectId
        assert!(values.contains(&FieldValue::Object(TestObjectId)));
    } else {
        panic!("Expected List of Objects");
    }
}

// Test 6: Pretty printing with format string
#[test]
fn test_pretty_print_with_format() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    #[pretty("Student aged {age}, enrolled: {enrolled}")]
    struct StudentWithPretty {
        age: i32,
        enrolled: bool,
    }

    let student = StudentWithPretty {
        age: 22,
        enrolled: true,
    };

    assert_eq!(
        student.pretty_print(),
        Some("Student aged 22, enrolled: true".to_string())
    );
}

// Test 7: Pretty printing with hidden field
#[test]
fn test_pretty_print_with_hidden_field() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    #[pretty("{name} is {age} years old")]
    struct StudentWithHiddenName {
        age: i32,
        #[hidden]
        name: String,
    }

    let student = StudentWithHiddenName {
        age: 20,
        name: "Alice".to_string(),
    };

    // Hidden field can still be used in pretty print
    assert_eq!(
        student.pretty_print(),
        Some("Alice is 20 years old".to_string())
    );

    // But not in schema
    let schema = StudentWithHiddenName::field_schema();
    assert!(!schema.contains_key("name"));
}

// Test 8: No pretty print attribute (default)
#[test]
fn test_default_pretty_print() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentNoPretty {
        age: i32,
    }

    let student = StudentNoPretty { age: 20 };
    assert_eq!(student.pretty_print(), None);
}

// Test 9: Complex nested structure
#[test]
fn test_complex_structure() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct ComplexStudent {
        age: i32,
        enrolled: bool,
        room: RoomId,
        courses: BTreeSet<CourseId>,
        grades: BTreeSet<i32>,
        #[hidden]
        _internal_id: usize,
    }

    let schema = ComplexStudent::field_schema();

    // Should have 5 visible fields
    assert_eq!(schema.len(), 5);
    assert_eq!(schema.get("age"), Some(&FieldType::Int));
    assert_eq!(schema.get("enrolled"), Some(&FieldType::Bool));
    assert_eq!(
        schema.get("room"),
        Some(&FieldType::Object(TypeId::of::<RoomId>()))
    );
    assert_eq!(
        schema.get("courses"),
        Some(&FieldType::List(Box::new(FieldType::Object(TypeId::of::<
            CourseId,
        >()))))
    );
    assert_eq!(
        schema.get("grades"),
        Some(&FieldType::List(Box::new(FieldType::Int)))
    );

    // Hidden field should not be in schema
    assert!(!schema.contains_key("_internal_id"));
}

// Test 10: Empty struct (edge case)
#[test]
fn test_empty_struct() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct EmptyStudent {}

    let schema = EmptyStudent::field_schema();
    assert_eq!(schema.len(), 0);

    let student = EmptyStudent {};
    assert_eq!(student.get_field("anything"), None);
}

// Test 11: Pretty printing with same field multiple times
#[test]
fn test_pretty_print_with_smae_field_multiple_times() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    #[pretty("{name} is really {name} and is {age} years old")]
    struct StudentWithHiddenName {
        age: i32,
        #[hidden]
        name: String,
    }

    let student = StudentWithHiddenName {
        age: 20,
        name: "Alice".to_string(),
    };

    // Hidden field can still be used in pretty print
    assert_eq!(
        student.pretty_print(),
        Some("Alice is really Alice and is 20 years old".to_string())
    );

    // But not in schema
    let schema = StudentWithHiddenName::field_schema();
    assert!(!schema.contains_key("name"));
}

// Test 12: Pretty printing with same field multiple times
#[test]
fn test_pretty_print_with_debug_output() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    #[pretty("{name:?} is {age} years old")]
    struct StudentWithHiddenName {
        age: i32,
        #[hidden]
        name: String,
    }

    let student = StudentWithHiddenName {
        age: 20,
        name: "Alice".to_string(),
    };

    // Hidden field can still be used in pretty print
    assert_eq!(
        student.pretty_print(),
        Some(format!("{:?} is 20 years old", "Alice".to_string()))
    );

    // But not in schema
    let schema = StudentWithHiddenName::field_schema();
    assert!(!schema.contains_key("name"));
}
