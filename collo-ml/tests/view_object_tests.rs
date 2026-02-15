use std::sync::Arc;

use collo_ml::traits::{FieldConversionError, SimpleFieldType};
use collo_ml::{
    DatabaseConnection, EvalObject, ExprType, ExprValue, SqliteDatabaseConnection, ViewObject,
};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct TestObjectId;

// Dummy implementation for testing
impl EvalObject for TestObjectId {
    type Env = ();
    type Cache = ();

    fn field_access<D: DatabaseConnection>(
        &self,
        _env: &Self::Env,
        _cache: &mut Self::Cache,
        _field: &str,
    ) -> Option<ExprValue<Self, D>> {
        None
    }
    fn type_id_to_name(_field_typ: std::any::TypeId) -> Result<String, FieldConversionError> {
        panic!("Not implemented for test")
    }
    fn objects_with_typ(_env: &Self::Env, _name: &str) -> BTreeSet<Self> {
        BTreeSet::new()
    }
    fn typ_name(&self) -> String {
        String::new()
    }
    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        HashMap::new()
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
    assert_eq!(schema.get("age"), Some(&SimpleFieldType::Int.into()));
    assert_eq!(schema.get("enrolled"), Some(&SimpleFieldType::Bool.into()));
}

#[test]
fn test_get_field() {
    let student = TestStudent {
        age: 20,
        enrolled: true,
    };

    assert_eq!(
        student.get_field::<SqliteDatabaseConnection>("age"),
        Some(ExprValue::Int(20))
    );
    assert_eq!(
        student.get_field::<SqliteDatabaseConnection>("enrolled"),
        Some(ExprValue::Bool(true))
    );
    assert_eq!(
        student.get_field::<SqliteDatabaseConnection>("nonexistent"),
        None
    );
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
    assert_eq!(
        student.get_field::<SqliteDatabaseConnection>("age"),
        Some(ExprValue::Int(25))
    );
    // Cannot access hidden field through get_field
    assert_eq!(
        student.get_field::<SqliteDatabaseConnection>("secret"),
        None
    );
    // But the field still exists in the struct for other purposes
    assert_eq!(student.secret, "hidden data");
}

// Test 4: Collections of basic types
#[test]
fn test_collections_of_ints() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithGrades {
        name_length: i32,
        grades: Vec<i32>,
    }

    let schema = StudentWithGrades::field_schema();
    assert_eq!(schema.len(), 2);
    assert_eq!(
        schema.get("grades"),
        Some(&SimpleFieldType::List(SimpleFieldType::Int.into()).into())
    );

    let mut grades = Vec::new();
    grades.push(85);
    grades.push(90);
    grades.push(78);

    let student = StudentWithGrades {
        name_length: 5,
        grades: grades.clone(),
    };

    if let Some(ExprValue::List(values)) = student.get_field::<SqliteDatabaseConnection>("grades") {
        assert_eq!(values.len(), 3);
        assert!(values.contains(&Arc::new(ExprValue::Int(85))));
        assert!(values.contains(&Arc::new(ExprValue::Int(90))));
        assert!(values.contains(&Arc::new(ExprValue::Int(78))));
    } else {
        panic!("Expected List of Ints");
    }
}

#[test]
fn test_collections_of_bools() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithFlags {
        flags: Vec<bool>,
    }

    let schema = StudentWithFlags::field_schema();
    assert_eq!(
        schema.get("flags"),
        Some(&SimpleFieldType::List(SimpleFieldType::Bool.into()).into())
    );

    let mut flags = Vec::new();
    flags.push(true);
    flags.push(false);

    let student = StudentWithFlags { flags };

    if let Some(ExprValue::List(values)) = student.get_field::<SqliteDatabaseConnection>("flags") {
        assert_eq!(values.len(), 2);
        assert!(values.contains(&Arc::new(ExprValue::Bool(true))));
        assert!(values.contains(&Arc::new(ExprValue::Bool(false))));
    } else {
        panic!("Expected List of Bools");
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

// Test 10: Empty struct (edge case)
#[test]
fn test_empty_struct() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct EmptyStudent {}

    let schema = EmptyStudent::field_schema();
    assert_eq!(schema.len(), 0);

    let student = EmptyStudent {};
    assert_eq!(
        student.get_field::<SqliteDatabaseConnection>("anything"),
        None
    );
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

// Test 13: Recursive Vec of primitives
#[test]
fn test_schemas_for_recursive_vecs() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct ComplexStudent {
        ages: Vec<Vec<i32>>,
    }

    let schema = ComplexStudent::field_schema();

    assert_eq!(schema.len(), 1);
    assert_eq!(
        schema.get("ages"),
        Some(
            &SimpleFieldType::List(SimpleFieldType::List(SimpleFieldType::Int.into()).into())
                .into()
        )
    );
}

// Test 14: Recursive Vec get_field
#[test]
fn test_get_field_for_recursive_vecs() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct ComplexStudent {
        ages: Vec<Vec<i32>>,
    }

    let mut ages = Vec::new();
    ages.push(Vec::from([20, 35]));
    ages.push(Vec::from([50, 75]));

    let student = ComplexStudent { ages };

    if let Some(ExprValue::List(values)) = student.get_field::<SqliteDatabaseConnection>("ages") {
        assert_eq!(values.len(), 2);
        assert_eq!(
            *values[0],
            ExprValue::List(Vec::from([
                Arc::new(ExprValue::Int(20)),
                Arc::new(ExprValue::Int(35))
            ]),)
        );
        assert_eq!(
            *values[1],
            ExprValue::List(Vec::from([
                Arc::new(ExprValue::Int(50)),
                Arc::new(ExprValue::Int(75))
            ]),)
        );
    } else {
        panic!("Expected List of Ints");
    }
}
