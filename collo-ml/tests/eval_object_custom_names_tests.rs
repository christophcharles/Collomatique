use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ExprType, ExprValue, FieldType, FieldValue, ViewBuilder, ViewObject};

// ============================================================================
// Setup: Define our environment and ID types
// ============================================================================

struct TestEnv {
    students: HashMap<usize, StudentData>,
    rooms: HashMap<usize, RoomData>,
    teachers: HashMap<usize, TeacherData>,
}

struct StudentData {
    age: i32,
}

struct RoomData {
    number: i32,
}

struct TeacherData {
    age: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct RoomId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct TeacherId(usize);

// ============================================================================
// Define ObjectId with custom names
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(TestEnv)]
enum CustomObjectId {
    Student(StudentId),
    #[name("Classroom")]
    RoomNumber(RoomId),
    #[name("Instructor")]
    Teacher(TeacherId),
}

// ============================================================================
// Define ViewObjects
// ============================================================================

#[derive(ViewObject)]
#[eval_object(CustomObjectId)]
struct CustomStudent {
    age: i32,
}

#[derive(ViewObject)]
#[eval_object(CustomObjectId)]
struct CustomRoom {
    number: i32,
}

#[derive(ViewObject)]
#[eval_object(CustomObjectId)]
struct CustomTeacher {
    age: i32,
}

// ============================================================================
// Implement ViewBuilder
// ============================================================================

impl ViewBuilder<TestEnv, StudentId> for CustomObjectId {
    type Object = CustomStudent;

    fn enumerate(env: &TestEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &TestEnv, id: &StudentId) -> Option<Self::Object> {
        let data = env.students.get(&id.0)?;
        Some(CustomStudent { age: data.age })
    }
}

impl ViewBuilder<TestEnv, RoomId> for CustomObjectId {
    type Object = CustomRoom;

    fn enumerate(env: &TestEnv) -> BTreeSet<RoomId> {
        env.rooms.keys().map(|&id| RoomId(id)).collect()
    }

    fn build(env: &TestEnv, id: &RoomId) -> Option<Self::Object> {
        let data = env.rooms.get(&id.0)?;
        Some(CustomRoom {
            number: data.number,
        })
    }
}

impl ViewBuilder<TestEnv, TeacherId> for CustomObjectId {
    type Object = CustomTeacher;

    fn enumerate(env: &TestEnv) -> BTreeSet<TeacherId> {
        env.teachers.keys().map(|&id| TeacherId(id)).collect()
    }

    fn build(env: &TestEnv, id: &TeacherId) -> Option<Self::Object> {
        let data = env.teachers.get(&id.0)?;
        Some(CustomTeacher { age: data.age })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_custom_type_names() {
    let env = TestEnv {
        students: HashMap::new(),
        rooms: HashMap::new(),
        teachers: HashMap::new(),
    };

    // Test typ_name uses custom names
    let student = CustomObjectId::Student(StudentId(1));
    assert_eq!(student.typ_name(&env), "Student"); // No custom name, uses variant

    let room = CustomObjectId::RoomNumber(RoomId(101));
    assert_eq!(room.typ_name(&env), "Classroom"); // Custom name from #[name]

    let teacher = CustomObjectId::Teacher(TeacherId(5));
    assert_eq!(teacher.typ_name(&env), "Instructor"); // Custom name from #[name]
}

#[test]
fn test_custom_type_names_in_schemas() {
    let schemas = CustomObjectId::type_schemas();

    // Should use custom names as keys
    assert!(schemas.contains_key("Student"));
    assert!(schemas.contains_key("Classroom"));
    assert!(schemas.contains_key("Instructor"));

    // Should NOT contain variant names when custom name is specified
    assert!(!schemas.contains_key("RoomNumber"));
    assert!(!schemas.contains_key("Teacher"));
}

#[test]
fn test_objects_with_typ_uses_custom_names() {
    let mut rooms = HashMap::new();
    rooms.insert(101, RoomData { number: 101 });
    rooms.insert(102, RoomData { number: 102 });

    let env = TestEnv {
        students: HashMap::new(),
        rooms,
        teachers: HashMap::new(),
    };

    // Should work with custom name "Classroom", not "RoomNumber"
    let classrooms = CustomObjectId::objects_with_typ(&env, "Classroom");
    assert_eq!(classrooms.len(), 2);

    // Should NOT work with variant name
    let rooms = CustomObjectId::objects_with_typ(&env, "RoomNumber");
    assert_eq!(rooms.len(), 0);
}

#[test]
fn test_from_impls_with_custom_names() {
    // From should still use the Rust type, not the DSL name
    let student: CustomObjectId = StudentId(1).into();
    assert!(matches!(student, CustomObjectId::Student(_)));

    let room: CustomObjectId = RoomId(101).into();
    assert!(matches!(room, CustomObjectId::RoomNumber(_)));

    let teacher: CustomObjectId = TeacherId(5).into();
    assert!(matches!(teacher, CustomObjectId::Teacher(_)));
}
