use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ViewBuilder, ViewObject};

// ============================================================================
// Setup: Define our environment and ID types
// ============================================================================

struct TestEnv {
    students: HashMap<usize, StudentData>,
    teachers: HashMap<usize, TeacherData>,
}

struct StudentData {
    age: i32,
    name: String,
}

struct TeacherData {
    name: String,
    title: String,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct TeacherId(usize);

// ============================================================================
// Define ObjectId
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(TestEnv)]
enum PrettyObjectId {
    Student(StudentId),
    Teacher(TeacherId),
}

// ============================================================================
// Define ViewObjects with pretty print
// ============================================================================

#[derive(ViewObject)]
#[eval_object(PrettyObjectId)]
#[pretty("{name} (age {age})")]
struct PrettyStudent {
    age: i32,
    #[hidden]
    name: String,
}

#[derive(ViewObject)]
#[eval_object(PrettyObjectId)]
#[pretty("{title} {name}")]
struct PrettyTeacher {
    #[hidden]
    name: String,
    #[hidden]
    title: String,
}

// ============================================================================
// Implement ViewBuilder
// ============================================================================

impl ViewBuilder<TestEnv, StudentId> for PrettyObjectId {
    type Object = PrettyStudent;

    fn enumerate(env: &TestEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &TestEnv, id: &StudentId) -> Option<Self::Object> {
        let data = env.students.get(&id.0)?;
        Some(PrettyStudent {
            age: data.age,
            name: data.name.clone(),
        })
    }
}

impl ViewBuilder<TestEnv, TeacherId> for PrettyObjectId {
    type Object = PrettyTeacher;

    fn enumerate(env: &TestEnv) -> BTreeSet<TeacherId> {
        env.teachers.keys().map(|&id| TeacherId(id)).collect()
    }

    fn build(env: &TestEnv, id: &TeacherId) -> Option<Self::Object> {
        let data = env.teachers.get(&id.0)?;
        Some(PrettyTeacher {
            name: data.name.clone(),
            title: data.title.clone(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_pretty_print_through_eval_object() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );

    let mut teachers = HashMap::new();
    teachers.insert(
        10,
        TeacherData {
            name: "Smith".to_string(),
            title: "Prof.".to_string(),
        },
    );

    let env = TestEnv { students, teachers };
    let mut cache = <PrettyObjectId as EvalObject>::Cache::default();

    // Test student pretty print
    let student = PrettyObjectId::Student(StudentId(1));
    assert_eq!(
        student.pretty_print(&env, &mut cache),
        Some("Alice (age 20)".to_string())
    );

    // Test teacher pretty print
    let teacher = PrettyObjectId::Teacher(TeacherId(10));
    assert_eq!(
        teacher.pretty_print(&env, &mut cache),
        Some("Prof. Smith".to_string())
    );
}

#[test]
fn test_pretty_print_with_nonexistent_object() {
    let env = TestEnv {
        students: HashMap::new(),
        teachers: HashMap::new(),
    };
    let mut cache = <PrettyObjectId as EvalObject>::Cache::default();

    // Object doesn't exist, so ViewBuilder::build returns None
    let student = PrettyObjectId::Student(StudentId(999));
    assert_eq!(student.pretty_print(&env, &mut cache), None);
}

#[test]
fn test_pretty_print_uses_hidden_fields() {
    let schemas = PrettyObjectId::type_schemas();

    // name should be hidden from schema
    let student_schema = schemas.get("Student").unwrap();
    assert!(!student_schema.contains_key("name"));
    assert!(student_schema.contains_key("age"));

    let teacher_schema = schemas.get("Teacher").unwrap();
    assert!(!teacher_schema.contains_key("name"));
    assert!(!teacher_schema.contains_key("title"));

    // But pretty_print should still work (tested above)
}

#[test]
fn test_pretty_print_with_multiple_hidden_fields() {
    let mut teachers = HashMap::new();
    teachers.insert(
        10,
        TeacherData {
            name: "Jones".to_string(),
            title: "Dr.".to_string(),
        },
    );

    let env = TestEnv {
        students: HashMap::new(),
        teachers,
    };
    let mut cache = <PrettyObjectId as EvalObject>::Cache::default();

    let teacher = PrettyObjectId::Teacher(TeacherId(10));
    assert_eq!(
        teacher.pretty_print(&env, &mut cache),
        Some("Dr. Jones".to_string())
    );
}

#[test]
fn test_no_pretty_print_attribute() {
    #[derive(ViewObject)]
    #[eval_object(PrettyObjectId)]
    struct PlainStudent {
        age: i32,
    }

    // ViewObject without #[pretty] should have None for pretty_print
    let student = PlainStudent { age: 20 };
    assert_eq!(student.pretty_print(), None);
}

#[test]
fn test_pretty_print_with_format_specifiers() {
    #[derive(ViewObject)]
    #[eval_object(PrettyObjectId)]
    #[pretty("{name:?} is {age} years old")]
    struct DebugStudent {
        age: i32,
        #[hidden]
        name: String,
    }

    let student = DebugStudent {
        age: 20,
        name: "Alice".to_string(),
    };

    // Should use Debug formatting for name
    assert_eq!(
        student.pretty_print(),
        Some(format!("{:?} is 20 years old", "Alice"))
    );
}
