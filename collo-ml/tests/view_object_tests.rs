use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ExprType, ExprValue, FieldType, ViewObject};

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
    fn type_schemas(
    ) -> std::collections::HashMap<String, std::collections::HashMap<String, ExprType>> {
        HashMap::new()
    }
}

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
    assert!(matches!(schema.get("age"), Some(FieldType::Int)));
}

#[test]
fn test_get_field() {
    let student = TestStudent {
        age: 20,
        enrolled: true,
    };

    let age = student.get_field("age");
    assert_eq!(age, Some(ExprValue::Int(20)));
}

#[test]
fn test_hidden_fields() {
    #[derive(ViewObject)]
    #[eval_object(TestObjectId)]
    struct StudentWithHidden {
        age: i32,
        #[hidden]
        _secret: String,
    }

    let schema = StudentWithHidden::field_schema();
    assert_eq!(schema.len(), 1); // Only age, not secret
    assert!(!schema.contains_key("_secret"));
}
