use collo_ml::{EvalObject, EvalVar, ViewBuilder, ViewObject};
use std::collections::{BTreeSet, HashMap};

// ============================================================================
// Test Environment and Data Structures
// ============================================================================

#[derive(Debug, Clone)]
struct StudentData {
    name: String,
}

#[derive(Debug, Clone)]
struct SubjectData {
    name: String,
}

struct TestEnv {
    students: HashMap<u64, StudentData>,
    subjects: HashMap<u64, SubjectData>,
}

impl TestEnv {
    fn simple_env() -> Self {
        TestEnv {
            students: HashMap::from([
                (
                    0,
                    StudentData {
                        name: "Harry Potter".to_string(),
                    },
                ),
                (
                    1,
                    StudentData {
                        name: "Hermione Granger".to_string(),
                    },
                ),
                (
                    2,
                    StudentData {
                        name: "Ron Weasley".to_string(),
                    },
                ),
            ]),
            subjects: HashMap::from([
                (
                    0,
                    SubjectData {
                        name: "Potions".to_string(),
                    },
                ),
                (
                    1,
                    SubjectData {
                        name: "Divination".to_string(),
                    },
                ),
                (
                    2,
                    SubjectData {
                        name: "Defense Against the Dark Arts".to_string(),
                    },
                ),
            ]),
        }
    }
}

// ============================================================================
// Object IDs
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct SubjectId(u64);

// ============================================================================
// EvalObject - The object ID enum
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(TestEnv)]
enum ObjectId {
    Student(StudentId),
    Subject(SubjectId),
}

// ============================================================================
// View Objects
// ============================================================================

#[derive(Clone, ViewObject)]
#[eval_object(ObjectId)]
#[pretty("Student \"{name}\"")]
struct Student {
    #[hidden]
    name: String,
}

#[derive(Clone, ViewObject)]
#[eval_object(ObjectId)]
#[pretty("Subject \"{name}\"")]
struct Subject {
    #[hidden]
    name: String,
}

// ============================================================================
// ViewBuilder Implementations
// ============================================================================

impl ViewBuilder<TestEnv, StudentId> for ObjectId {
    type Object = Student;

    fn enumerate(env: &TestEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &TestEnv, id: &StudentId) -> Option<Self::Object> {
        let data = env.students.get(&id.0)?;
        Some(Student {
            name: data.name.clone(),
        })
    }
}

impl ViewBuilder<TestEnv, SubjectId> for ObjectId {
    type Object = Subject;

    fn enumerate(env: &TestEnv) -> BTreeSet<SubjectId> {
        env.subjects.keys().map(|&id| SubjectId(id)).collect()
    }

    fn build(env: &TestEnv, id: &SubjectId) -> Option<Self::Object> {
        let data = env.subjects.get(&id.0)?;
        Some(Subject {
            name: data.name.clone(),
        })
    }
}

// ============================================================================
// EvalVar - The actual thing we're testing!
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
enum SimpleVar {
    // Simple: student takes a subject
    StudentTakesSubject(StudentId, SubjectId),

    // With a week number (range 0-3 for a 3-week schedule)
    #[name("StSiW")]
    StudentTakesSubjectInWeek {
        student: StudentId,
        subject: SubjectId,
        #[range(0..3)]
        week: i32,
    },

    // Just a week number
    #[fix_with(1.)]
    WeekUsed(#[range(0..3)] i32),
}

// ============================================================================
// Tests
// ============================================================================

use collo_ml::traits::SimpleFieldType;

#[test]
fn test_field_schema() {
    let schema = <SimpleVar as EvalVar<ObjectId>>::field_schema();

    // Check that all variants are present
    assert!(schema.contains_key("StudentTakesSubject"));
    assert!(schema.contains_key("StSiW"));
    assert!(schema.contains_key("WeekUsed"));

    // Check field counts
    assert_eq!(schema.get("StudentTakesSubject").unwrap().len(), 2);
    assert_eq!(schema.get("StSiW").unwrap().len(), 3);
    assert_eq!(schema.get("WeekUsed").unwrap().len(), 1);

    // Check field types
    let sts_schema = schema.get("StudentTakesSubject").unwrap();
    assert!(matches!(
        sts_schema[0].as_simple(),
        Some(SimpleFieldType::Object(_))
    ));
    assert!(matches!(
        sts_schema[1].as_simple(),
        Some(SimpleFieldType::Object(_))
    ));

    let week_schema = schema.get("WeekUsed").unwrap();
    assert!(matches!(
        week_schema[0].as_simple(),
        Some(SimpleFieldType::Int)
    ));
}

#[test]
fn test_vars_generation() {
    let env = TestEnv::simple_env();

    // Now vars() takes the environment as a generic parameter
    let vars =
        <SimpleVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible with ObjectId");

    // Should have variables for all combinations
    // 3 students × 3 subjects = 9 StudentTakesSubject vars
    // 3 students × 3 subjects × 3 weeks = 27 StudentTakesSubjectInWeek vars
    // 3 weeks = 3 WeekUsed vars
    // Total: 39 variables

    assert_eq!(vars.len(), 39, "Should generate 39 total variables");
}

#[test]
fn test_fix_within_range() {
    let env = TestEnv::simple_env();

    let var = SimpleVar::WeekUsed(1);
    assert_eq!(<SimpleVar as EvalVar<ObjectId>>::fix(&var, &env), None);

    let var = SimpleVar::StudentTakesSubjectInWeek {
        student: StudentId(0),
        subject: SubjectId(0),
        week: 2,
    };
    assert_eq!(<SimpleVar as EvalVar<ObjectId>>::fix(&var, &env), None);
}

#[test]
fn test_fix_outside_range() {
    let env = TestEnv::simple_env();

    let var = SimpleVar::WeekUsed(5); // Outside range 0..3
    assert_eq!(<SimpleVar as EvalVar<ObjectId>>::fix(&var, &env), Some(1.0));

    let var = SimpleVar::WeekUsed(-1); // Outside range 0..3
    assert_eq!(<SimpleVar as EvalVar<ObjectId>>::fix(&var, &env), Some(1.0));

    let var = SimpleVar::StudentTakesSubjectInWeek {
        student: StudentId(0),
        subject: SubjectId(0),
        week: 10, // Outside range 0..3
    };
    assert_eq!(<SimpleVar as EvalVar<ObjectId>>::fix(&var, &env), Some(0.0));
}

#[test]
fn test_try_from_extern_var() {
    use collo_ml::eval::{ExprValue, ExternVar};

    // Test successful conversion for StudentTakesSubject
    let extern_var = ExternVar::new_no_env(
        "StudentTakesSubject".to_string(),
        vec![
            ExprValue::Object(ObjectId::Student(StudentId(0))),
            ExprValue::Object(ObjectId::Subject(SubjectId(1))),
        ],
    );

    let var: Result<SimpleVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        SimpleVar::StudentTakesSubject(StudentId(0), SubjectId(1))
    );

    // Test successful conversion for WeekUsed
    let extern_var =
        ExternVar::new_no_env("WeekUsed".to_string(), vec![ExprValue::<ObjectId>::Int(2)]);

    let var: Result<SimpleVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), SimpleVar::WeekUsed(2));
}

#[test]
fn test_try_from_wrong_param_count() {
    use collo_ml::eval::{ExprValue, ExternVar};
    use collo_ml::traits::VarConversionError;

    // Wrong number of parameters
    let extern_var = ExternVar::new_no_env(
        "StudentTakesSubject".to_string(),
        vec![ExprValue::Object(ObjectId::Student(StudentId(0)))], // Only 1, need 2
    );

    let var: Result<SimpleVar, _> = (&extern_var).try_into();
    assert!(var.is_err());
    assert!(matches!(
        var.unwrap_err(),
        VarConversionError::WrongParameterCount {
            expected: 2,
            found: 1,
            ..
        }
    ));
}

#[test]
fn test_try_from_wrong_param_type() {
    use collo_ml::eval::{ExprValue, ExternVar};
    use collo_ml::traits::VarConversionError;

    // Wrong parameter type (Int instead of Object)
    let extern_var = ExternVar::new_no_env(
        "StudentTakesSubject".to_string(),
        vec![
            ExprValue::Int(42), // Wrong type!
            ExprValue::Object(ObjectId::Subject(SubjectId(1))),
        ],
    );

    let var: Result<SimpleVar, _> = (&extern_var).try_into();
    assert!(var.is_err());
    assert!(matches!(
        var.unwrap_err(),
        VarConversionError::WrongParameterType { param: 0, .. }
    ));
}

#[test]
fn test_try_from_unknown_variant() {
    use collo_ml::eval::ExternVar;
    use collo_ml::traits::VarConversionError;

    let extern_var = ExternVar::<ObjectId>::new_no_env("NonExistentVariant".to_string(), vec![]);

    let var: Result<SimpleVar, _> = (&extern_var).try_into();
    assert!(var.is_err());
    assert!(matches!(var.unwrap_err(), VarConversionError::Unknown(_)));
}
