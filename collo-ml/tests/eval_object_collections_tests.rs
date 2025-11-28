use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ExprType, ExprValue, ViewBuilder, ViewObject};

// ============================================================================
// Setup: Define our environment and ID types
// ============================================================================

struct TestEnv {
    students: HashMap<usize, StudentData>,
    teachers: HashMap<usize, TeacherData>,
    courses: HashMap<usize, CourseData>,
}

struct StudentData {
    age: i32,
}

struct TeacherData {
    age: i32,
    student_ids: Vec<usize>,
}

struct CourseData {
    student_groups: Vec<Vec<usize>>, // Nested groups
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct TeacherId(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct CourseId(usize);

// ============================================================================
// Define ObjectId
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(TestEnv)]
enum CollectionObjectId {
    Student(StudentId),
    Teacher(TeacherId),
    Course(CourseId),
}

// ============================================================================
// Define ViewObjects
// ============================================================================

#[derive(ViewObject)]
#[eval_object(CollectionObjectId)]
struct CollectionStudent {
    age: i32,
}

#[derive(ViewObject)]
#[eval_object(CollectionObjectId)]
struct CollectionTeacher {
    age: i32,
    students: Vec<StudentId>,
}

#[derive(ViewObject)]
#[eval_object(CollectionObjectId)]
struct CollectionCourse {
    student_groups: Vec<Vec<StudentId>>,
}

// ============================================================================
// Implement ViewBuilder
// ============================================================================

impl ViewBuilder<TestEnv, StudentId> for CollectionObjectId {
    type Object = CollectionStudent;

    fn enumerate(env: &TestEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &TestEnv, id: &StudentId) -> Option<Self::Object> {
        let data = env.students.get(&id.0)?;
        Some(CollectionStudent { age: data.age })
    }
}

impl ViewBuilder<TestEnv, TeacherId> for CollectionObjectId {
    type Object = CollectionTeacher;

    fn enumerate(env: &TestEnv) -> BTreeSet<TeacherId> {
        env.teachers.keys().map(|&id| TeacherId(id)).collect()
    }

    fn build(env: &TestEnv, id: &TeacherId) -> Option<Self::Object> {
        let data = env.teachers.get(&id.0)?;
        Some(CollectionTeacher {
            age: data.age,
            students: data.student_ids.iter().map(|&id| StudentId(id)).collect(),
        })
    }
}

impl ViewBuilder<TestEnv, CourseId> for CollectionObjectId {
    type Object = CollectionCourse;

    fn enumerate(env: &TestEnv) -> BTreeSet<CourseId> {
        env.courses.keys().map(|&id| CourseId(id)).collect()
    }

    fn build(env: &TestEnv, id: &CourseId) -> Option<Self::Object> {
        let data = env.courses.get(&id.0)?;
        Some(CollectionCourse {
            student_groups: data
                .student_groups
                .iter()
                .map(|group| group.iter().map(|&id| StudentId(id)).collect())
                .collect(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_collection_field_schema() {
    let schemas = CollectionObjectId::type_schemas();
    let teacher_schema = schemas.get("Teacher").unwrap();

    // BTreeSet<StudentId> should become List(Object("Student"))
    assert_eq!(
        teacher_schema.get("students"),
        Some(&ExprType::List(Box::new(ExprType::Object(
            "Student".to_string()
        ))))
    );
}

#[test]
fn test_collection_field_access() {
    let mut students = HashMap::new();
    students.insert(1, StudentData { age: 20 });
    students.insert(2, StudentData { age: 22 });

    let mut teachers = HashMap::new();
    teachers.insert(
        10,
        TeacherData {
            age: 45,
            student_ids: vec![1, 2],
        },
    );

    let env = TestEnv {
        students,
        teachers,
        courses: HashMap::new(),
    };
    let mut cache = <CollectionObjectId as EvalObject>::Cache::default();

    let teacher = CollectionObjectId::Teacher(TeacherId(10));
    let students_field = teacher.field_access(&env, &mut cache, "students");

    // Should be a List of Objects
    if let Some(ExprValue::List(expr_type, values)) = students_field {
        assert_eq!(expr_type, ExprType::Object("Student".to_string()));
        assert_eq!(values.len(), 2);
        assert!(
            values.contains(&ExprValue::Object(CollectionObjectId::Student(StudentId(
                1
            ))))
        );
        assert!(
            values.contains(&ExprValue::Object(CollectionObjectId::Student(StudentId(
                2
            ))))
        );
    } else {
        panic!("Expected List of Objects");
    }
}

#[test]
fn test_empty_collection_field_access() {
    let mut teachers = HashMap::new();
    teachers.insert(
        10,
        TeacherData {
            age: 45,
            student_ids: vec![], // Empty list
        },
    );

    let env = TestEnv {
        students: HashMap::new(),
        teachers,
        courses: HashMap::new(),
    };
    let mut cache = <CollectionObjectId as EvalObject>::Cache::default();

    let teacher = CollectionObjectId::Teacher(TeacherId(10));
    let students_field = teacher.field_access(&env, &mut cache, "students");

    // Should be an empty List with correct type
    if let Some(ExprValue::List(expr_type, values)) = students_field {
        assert_eq!(expr_type, ExprType::Object("Student".to_string()));
        assert_eq!(values.len(), 0);
    } else {
        panic!("Expected empty List of Objects");
    }
}

#[test]
fn test_nested_collection_schema() {
    let schemas = CollectionObjectId::type_schemas();
    let course_schema = schemas.get("Course").unwrap();

    // BTreeSet<BTreeSet<StudentId>> should become List(List(Object("Student")))
    assert_eq!(
        course_schema.get("student_groups"),
        Some(&ExprType::List(Box::new(ExprType::List(Box::new(
            ExprType::Object("Student".to_string())
        )))))
    );
}

#[test]
fn test_nested_collection_field_access() {
    let mut students = HashMap::new();
    students.insert(1, StudentData { age: 20 });
    students.insert(2, StudentData { age: 22 });

    let mut courses = HashMap::new();
    courses.insert(
        100,
        CourseData {
            student_groups: vec![
                vec![1, 2], // Group 1
                vec![1],    // Group 2
            ],
        },
    );

    let env = TestEnv {
        students,
        teachers: HashMap::new(),
        courses,
    };
    let mut cache = <CollectionObjectId as EvalObject>::Cache::default();

    let course = CollectionObjectId::Course(CourseId(100));
    let groups_field = course.field_access(&env, &mut cache, "student_groups");

    // Should be a List of Lists
    if let Some(ExprValue::List(outer_type, outer_values)) = groups_field {
        assert_eq!(
            outer_type,
            ExprType::List(Box::new(ExprType::Object("Student".to_string())))
        );
        assert_eq!(outer_values.len(), 2);

        // Check that we have nested lists
        for value in outer_values {
            if let ExprValue::List(inner_type, _inner_values) = value {
                assert_eq!(inner_type, ExprType::Object("Student".to_string()));
            } else {
                panic!("Expected nested List");
            }
        }
    } else {
        panic!("Expected List of Lists");
    }
}

#[test]
fn test_collection_of_primitives() {
    #[derive(ViewObject)]
    #[eval_object(CollectionObjectId)]
    struct TeacherWithGrades {
        grades: Vec<i32>,
    }

    // This would require adding a variant and implementing ViewBuilder
    // Just testing that the schema generation works
    let schema = TeacherWithGrades::field_schema();
    assert_eq!(
        schema.get("grades"),
        Some(&collo_ml::traits::FieldType::List(Box::new(
            collo_ml::traits::FieldType::Int
        )))
    );
}

#[test]
fn test_empty_nested_collection() {
    let mut courses = HashMap::new();
    courses.insert(
        100,
        CourseData {
            student_groups: vec![], // Empty outer list
        },
    );

    let env = TestEnv {
        students: HashMap::new(),
        teachers: HashMap::new(),
        courses,
    };
    let mut cache = <CollectionObjectId as EvalObject>::Cache::default();

    let course = CollectionObjectId::Course(CourseId(100));
    let groups_field = course.field_access(&env, &mut cache, "student_groups");

    // Should be an empty List with correct nested type
    if let Some(ExprValue::List(outer_type, values)) = groups_field {
        assert_eq!(
            outer_type,
            ExprType::List(Box::new(ExprType::Object("Student".to_string())))
        );
        assert_eq!(values.len(), 0);
    } else {
        panic!("Expected empty List of Lists");
    }
}
