use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ExprValue, SimpleType, ViewBuilder, ViewObject};

// ============================================================================
// Setup: Define our environment and ID types
// ============================================================================

struct TestEnv {
    students: HashMap<usize, StudentData>,
    rooms: HashMap<usize, RoomData>,
}

struct StudentData {
    age: i32,
    enrolled: bool,
    room_id: usize,
}

struct RoomData {
    number: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct RoomId(usize);

// ============================================================================
// Define the ObjectId enum with EvalObject derive
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(TestEnv)]
enum ObjectId {
    Student(StudentId),
    Room(RoomId),
}

// ============================================================================
// Define ViewObjects
// ============================================================================

#[derive(ViewObject)]
#[eval_object(ObjectId)]
struct Student {
    age: i32,
    enrolled: bool,
    room: RoomId,
}

#[derive(ViewObject)]
#[eval_object(ObjectId)]
struct Room {
    number: i32,
}

// ============================================================================
// Implement ViewBuilder for each ID type
// ============================================================================

impl ViewBuilder<TestEnv, StudentId> for ObjectId {
    type Object = Student;

    fn enumerate(env: &TestEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &TestEnv, id: &StudentId) -> Option<Self::Object> {
        let data = env.students.get(&id.0)?;
        Some(Student {
            age: data.age,
            enrolled: data.enrolled,
            room: RoomId(data.room_id),
        })
    }
}

impl ViewBuilder<TestEnv, RoomId> for ObjectId {
    type Object = Room;

    fn enumerate(env: &TestEnv) -> BTreeSet<RoomId> {
        env.rooms.keys().map(|&id| RoomId(id)).collect()
    }

    fn build(env: &TestEnv, id: &RoomId) -> Option<Self::Object> {
        let data = env.rooms.get(&id.0)?;
        Some(Room {
            number: data.number,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_from_impls() {
    // Test that From<StudentId> and From<RoomId> are implemented
    let student_obj: ObjectId = StudentId(1).into();
    assert!(matches!(student_obj, ObjectId::Student(_)));

    let room_obj: ObjectId = RoomId(42).into();
    assert!(matches!(room_obj, ObjectId::Room(_)));
}

#[test]
fn test_try_from_impls() {
    // Test that TryFrom<ObjectId> are implemented
    let student_obj = ObjectId::Student(StudentId(1));
    assert_eq!(student_obj.try_into(), Ok(StudentId(1)));
    assert_eq!(
        student_obj.try_into(),
        Result::<RoomId, _>::Err(collo_ml::traits::TypeConversionError::BadType)
    );

    let room_obj = ObjectId::Room(RoomId(42));
    assert_eq!(room_obj.try_into(), Ok(RoomId(42)));
    assert_eq!(
        room_obj.try_into(),
        Result::<StudentId, _>::Err(collo_ml::traits::TypeConversionError::BadType)
    );
}

#[test]
fn test_typ_name() {
    let env = TestEnv {
        students: HashMap::new(),
        rooms: HashMap::new(),
    };

    let student = ObjectId::Student(StudentId(1));
    assert_eq!(student.typ_name(&env), "Student");

    let room = ObjectId::Room(RoomId(42));
    assert_eq!(room.typ_name(&env), "Room");
}

#[test]
fn test_objects_with_typ() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            enrolled: true,
            room_id: 101,
        },
    );
    students.insert(
        2,
        StudentData {
            age: 22,
            enrolled: false,
            room_id: 102,
        },
    );

    let mut rooms = HashMap::new();
    rooms.insert(101, RoomData { number: 101 });

    let env = TestEnv { students, rooms };

    // Get all students
    let student_objects = ObjectId::objects_with_typ(&env, "Student");
    assert_eq!(student_objects.len(), 2);
    assert!(student_objects.contains(&ObjectId::Student(StudentId(1))));
    assert!(student_objects.contains(&ObjectId::Student(StudentId(2))));

    // Get all rooms
    let room_objects = ObjectId::objects_with_typ(&env, "Room");
    assert_eq!(room_objects.len(), 1);
    assert!(room_objects.contains(&ObjectId::Room(RoomId(101))));

    // Unknown type
    let unknown = ObjectId::objects_with_typ(&env, "Unknown");
    assert_eq!(unknown.len(), 0);
}

#[test]
fn test_type_schemas() {
    let schemas = ObjectId::type_schemas();

    // Should have schemas for Student and Room
    assert_eq!(schemas.len(), 2);

    // Check Student schema
    let student_schema = schemas.get("Student").unwrap();
    assert_eq!(student_schema.get("age"), Some(&SimpleType::Int.into()));
    assert_eq!(
        student_schema.get("enrolled"),
        Some(&SimpleType::Bool.into())
    );
    assert_eq!(
        student_schema.get("room"),
        Some(&SimpleType::Object("Room".to_string()).into())
    );

    // Check Room schema
    let room_schema = schemas.get("Room").unwrap();
    assert_eq!(room_schema.get("number"), Some(&SimpleType::Int.into()));
}

#[test]
fn test_field_access() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            enrolled: true,
            room_id: 101,
        },
    );

    let rooms = HashMap::new();

    let env = TestEnv { students, rooms };
    let mut cache = <ObjectId as EvalObject>::Cache::default();

    let student = ObjectId::Student(StudentId(1));

    // Access age field
    assert_eq!(
        student.field_access(&env, &mut cache, "age"),
        Some(ExprValue::Int(20))
    );

    // Access enrolled field
    assert_eq!(
        student.field_access(&env, &mut cache, "enrolled"),
        Some(ExprValue::Bool(true))
    );

    // Access room field (should be converted to ObjectId)
    assert_eq!(
        student.field_access(&env, &mut cache, "room"),
        Some(ExprValue::Object(ObjectId::Room(RoomId(101))))
    );

    // Non-existent field
    assert_eq!(student.field_access(&env, &mut cache, "nonexistent"), None);
}

#[test]
fn test_field_access_with_nonexistent_object() {
    let env = TestEnv {
        students: HashMap::new(),
        rooms: HashMap::new(),
    };
    let mut cache = <ObjectId as EvalObject>::Cache::default();

    // Student ID 999 doesn't exist
    let student = ObjectId::Student(StudentId(999));

    // Should return None because the object can't be built
    assert_eq!(student.field_access(&env, &mut cache, "age"), None);
}
