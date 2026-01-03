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
// EvalVar with Option support - The actual thing we're testing!
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
enum OptionVar {
    // Optional student - generates vars for None + each student
    OptionalStudent(Option<StudentId>),

    // Optional subject
    OptionalSubject(Option<SubjectId>),

    // Optional week
    OptionalWeek(#[range(0..3)] Option<i32>),

    // Student with optional mentor (another student)
    #[name("StuMentor")]
    StudentWithMentor {
        student: StudentId,
        mentor: Option<StudentId>,
    },

    // Optional student takes subject
    OptionalStudentTakesSubject(Option<StudentId>, SubjectId),

    // Student takes optional subject
    StudentTakesOptionalSubject(StudentId, Option<SubjectId>),

    // Both optional
    BothOptional(Option<StudentId>, Option<SubjectId>),

    // Named with optional week
    StudentInOptionalWeek {
        student: StudentId,
        #[range(0..3)]
        week: Option<i32>,
    },

    // Multiple optionals
    MultipleOptionals {
        student: Option<StudentId>,
        subject: Option<SubjectId>,
        #[range(0..3)]
        week: Option<i32>,
    },

    // Optional bool for testing
    OptionalFlag(Option<bool>),
}

// ============================================================================
// Tests - Field Schema
// ============================================================================

use collo_ml::traits::SimpleFieldType;

#[test]
fn test_optional_field_schema() {
    let schema = <OptionVar as EvalVar<ObjectId>>::field_schema();

    // Check OptionalStudent: should have 1 field of type None | Student
    let opt_student_schema = schema.get("OptionalStudent").unwrap();
    assert_eq!(opt_student_schema.len(), 1);

    let field_type = &opt_student_schema[0];
    assert!(!field_type.is_simple(), "Should be a sum type (union)");

    let variants = field_type.get_variants();
    assert_eq!(variants.len(), 2, "Should have None and Object variants");
    assert!(variants.contains(&SimpleFieldType::None));
    // Check for Object variant (can't easily check TypeId)
    assert!(variants
        .iter()
        .any(|v| matches!(v, SimpleFieldType::Object(_))));
}

#[test]
fn test_optional_int_field_schema() {
    let schema = <OptionVar as EvalVar<ObjectId>>::field_schema();

    // Check OptionalWeek: should be None | Int
    let opt_week_schema = schema.get("OptionalWeek").unwrap();
    assert_eq!(opt_week_schema.len(), 1);

    let field_type = &opt_week_schema[0];
    let variants = field_type.get_variants();
    assert_eq!(variants.len(), 2);
    assert!(variants.contains(&SimpleFieldType::None));
    assert!(variants.contains(&SimpleFieldType::Int));
}

#[test]
fn test_optional_bool_field_schema() {
    let schema = <OptionVar as EvalVar<ObjectId>>::field_schema();

    // Check OptionalFlag: should be None | Bool
    let opt_flag_schema = schema.get("OptionalFlag").unwrap();
    assert_eq!(opt_flag_schema.len(), 1);

    let field_type = &opt_flag_schema[0];
    let variants = field_type.get_variants();
    assert_eq!(variants.len(), 2);
    assert!(variants.contains(&SimpleFieldType::None));
    assert!(variants.contains(&SimpleFieldType::Bool));
}

#[test]
fn test_mixed_optional_schema() {
    let schema = <OptionVar as EvalVar<ObjectId>>::field_schema();

    // Check StudentWithMentor: [Student, None | Student]
    let mentor_schema = schema.get("StuMentor").unwrap();
    assert_eq!(mentor_schema.len(), 2);

    // First field is Student (not optional)
    assert!(mentor_schema[0].is_simple());

    // Second field is None | Student (optional)
    assert!(!mentor_schema[1].is_simple());
    let variants = mentor_schema[1].get_variants();
    assert_eq!(variants.len(), 2);
    assert!(variants.contains(&SimpleFieldType::None));
}

#[test]
fn test_multiple_optionals_schema() {
    let schema = <OptionVar as EvalVar<ObjectId>>::field_schema();

    // Check MultipleOptionals: all three fields should be optional
    let multi_schema = schema.get("MultipleOptionals").unwrap();
    assert_eq!(multi_schema.len(), 3);

    // All three should be sum types with None
    for field_type in multi_schema {
        assert!(!field_type.is_simple());
        let variants = field_type.get_variants();
        assert!(variants.contains(&SimpleFieldType::None));
    }
}

// ============================================================================
// Tests - Vars Generation
// ============================================================================

#[test]
fn test_optional_student_vars_generation() {
    let env = TestEnv::simple_env();
    let vars = <OptionVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible");

    // OptionalStudent should generate: None + 3 students = 4 vars
    let opt_student_vars: Vec<_> = vars
        .keys()
        .filter(|v| matches!(v, OptionVar::OptionalStudent(_)))
        .collect();

    assert_eq!(opt_student_vars.len(), 4, "Should have None + 3 students");

    // Check that None is present
    assert!(
        opt_student_vars
            .iter()
            .any(|v| matches!(v, OptionVar::OptionalStudent(None))),
        "Should have None variant"
    );

    // Check that all students are present
    for student_id in 0..3 {
        assert!(
            opt_student_vars.iter().any(|v|
                matches!(v, OptionVar::OptionalStudent(Some(StudentId(id))) if *id == student_id)
            ),
            "Should have student {}", student_id
        );
    }
}

#[test]
fn test_optional_week_vars_generation() {
    let env = TestEnv::simple_env();
    let vars = <OptionVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible");

    // OptionalWeek with range 0..3 should generate: None + [0,1,2] = 4 vars
    let opt_week_vars: Vec<_> = vars
        .keys()
        .filter(|v| matches!(v, OptionVar::OptionalWeek(_)))
        .collect();

    assert_eq!(opt_week_vars.len(), 4, "Should have None + 3 weeks");

    // Check None
    assert!(opt_week_vars
        .iter()
        .any(|v| matches!(v, OptionVar::OptionalWeek(None))));

    // Check weeks 0, 1, 2
    for week in 0..3 {
        assert!(opt_week_vars
            .iter()
            .any(|v| matches!(v, OptionVar::OptionalWeek(Some(w)) if *w == week)));
    }
}

#[test]
fn test_optional_bool_vars_generation() {
    let env = TestEnv::simple_env();
    let vars = <OptionVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible");

    // OptionalFlag should generate: None + true + false = 3 vars
    let opt_flag_vars: Vec<_> = vars
        .keys()
        .filter(|v| matches!(v, OptionVar::OptionalFlag(_)))
        .collect();

    assert_eq!(opt_flag_vars.len(), 3, "Should have None + true + false");

    assert!(opt_flag_vars
        .iter()
        .any(|v| matches!(v, OptionVar::OptionalFlag(None))));
    assert!(opt_flag_vars
        .iter()
        .any(|v| matches!(v, OptionVar::OptionalFlag(Some(true)))));
    assert!(opt_flag_vars
        .iter()
        .any(|v| matches!(v, OptionVar::OptionalFlag(Some(false)))));
}

#[test]
fn test_student_with_optional_mentor_vars_generation() {
    let env = TestEnv::simple_env();
    let vars = <OptionVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible");

    // StudentWithMentor: 3 students x (None + 3 mentors) = 3 x 4 = 12 vars
    let mentor_vars: Vec<_> = vars
        .keys()
        .filter(|v| matches!(v, OptionVar::StudentWithMentor { .. }))
        .collect();

    assert_eq!(
        mentor_vars.len(),
        12,
        "Should have 3 students x 4 mentor options"
    );

    // Check that each student has None mentor option
    for student_id in 0..3 {
        assert!(
            mentor_vars.iter().any(
                |v| matches!(v, OptionVar::StudentWithMentor { student, mentor }
                    if student.0 == student_id && mentor.is_none())
            ),
            "Student {} should have None mentor option",
            student_id
        );
    }

    // Check that student 0 with mentor 1 exists
    assert!(mentor_vars.iter().any(
        |v| matches!(v, OptionVar::StudentWithMentor { student, mentor }
                if student.0 == 0 && matches!(mentor, Some(StudentId(1))))
    ));
}

#[test]
fn test_both_optional_vars_generation() {
    let env = TestEnv::simple_env();
    let vars = <OptionVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible");

    // BothOptional: (None + 3 students) x (None + 3 subjects) = 4 x 4 = 16 vars
    let both_opt_vars: Vec<_> = vars
        .keys()
        .filter(|v| matches!(v, OptionVar::BothOptional(_, _)))
        .collect();

    assert_eq!(both_opt_vars.len(), 16, "Should have 4 x 4 combinations");

    // Check None, None
    assert!(both_opt_vars
        .iter()
        .any(|v| matches!(v, OptionVar::BothOptional(None, None))));

    // Check Some student, None subject
    assert!(both_opt_vars
        .iter()
        .any(|v| matches!(v, OptionVar::BothOptional(Some(StudentId(0)), None))));

    // Check None student, Some subject
    assert!(both_opt_vars
        .iter()
        .any(|v| matches!(v, OptionVar::BothOptional(None, Some(SubjectId(0))))));

    // Check Some student, Some subject
    assert!(both_opt_vars.iter().any(|v| matches!(
        v,
        OptionVar::BothOptional(Some(StudentId(0)), Some(SubjectId(1)))
    )));
}

#[test]
fn test_multiple_optionals_vars_generation() {
    let env = TestEnv::simple_env();
    let vars = <OptionVar as EvalVar<ObjectId>>::vars(&env).expect("Should be compatible");

    // MultipleOptionals: 4 students x 4 subjects x 4 weeks = 64 vars
    let multi_vars: Vec<_> = vars
        .keys()
        .filter(|v| matches!(v, OptionVar::MultipleOptionals { .. }))
        .collect();

    assert_eq!(multi_vars.len(), 64, "Should have 4 x 4 x 4 combinations");

    // Check all None
    assert!(multi_vars.iter().any(
        |v| matches!(v, OptionVar::MultipleOptionals { student, subject, week } 
                if student.is_none() && subject.is_none() && week.is_none())
    ));

    // Check all Some
    assert!(multi_vars.iter().any(
        |v| matches!(v, OptionVar::MultipleOptionals { student, subject, week }
                if matches!(student, Some(StudentId(0)))
                    && matches!(subject, Some(SubjectId(1)))
                    && matches!(week, Some(2)))
    ));
}

// ============================================================================
// Tests - Fix with Optional i32
// ============================================================================

#[test]
fn test_optional_week_fix_none() {
    let env = TestEnv::simple_env();

    // None is always valid (no range check needed)
    let var = OptionVar::OptionalWeek(None);
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), None);
}

#[test]
fn test_optional_week_fix_within_range() {
    let env = TestEnv::simple_env();

    // Some(1) is within range 0..3
    let var = OptionVar::OptionalWeek(Some(1));
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), None);

    let var = OptionVar::OptionalWeek(Some(0));
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), None);

    let var = OptionVar::OptionalWeek(Some(2));
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), None);
}

#[test]
fn test_optional_week_fix_outside_range() {
    let env = TestEnv::simple_env();

    // Some(5) is outside range 0..3
    let var = OptionVar::OptionalWeek(Some(5));
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), Some(0.0));

    let var = OptionVar::OptionalWeek(Some(-1));
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), Some(0.0));

    let var = OptionVar::OptionalWeek(Some(10));
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), Some(0.0));
}

#[test]
fn test_named_optional_week_fix() {
    let env = TestEnv::simple_env();

    // None is valid
    let var = OptionVar::StudentInOptionalWeek {
        student: StudentId(0),
        week: None,
    };
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), None);

    // Some(1) is valid
    let var = OptionVar::StudentInOptionalWeek {
        student: StudentId(0),
        week: Some(1),
    };
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), None);

    // Some(10) is invalid
    let var = OptionVar::StudentInOptionalWeek {
        student: StudentId(0),
        week: Some(10),
    };
    assert_eq!(<OptionVar as EvalVar<ObjectId>>::fix(&var, &env), Some(0.0));
}

// ============================================================================
// Tests - TryFrom ExternVar with Options
// ============================================================================

#[test]
fn test_try_from_optional_student_none() {
    use collo_ml::eval::{ExprValue, ExternVar};

    let extern_var = ExternVar::new_no_env(
        "OptionalStudent".to_string(),
        vec![ExprValue::<ObjectId>::None],
    );

    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalStudent(None));
}

#[test]
fn test_try_from_optional_student_some() {
    use collo_ml::eval::{ExprValue, ExternVar};

    let extern_var = ExternVar::new_no_env(
        "OptionalStudent".to_string(),
        vec![ExprValue::Object(ObjectId::Student(StudentId(1)))],
    );

    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalStudent(Some(StudentId(1))));
}

#[test]
fn test_try_from_optional_week_none() {
    use collo_ml::eval::{ExprValue, ExternVar};

    let extern_var = ExternVar::new_no_env(
        "OptionalWeek".to_string(),
        vec![ExprValue::<ObjectId>::None],
    );

    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalWeek(None));
}

#[test]
fn test_try_from_optional_week_some() {
    use collo_ml::eval::{ExprValue, ExternVar};

    let extern_var = ExternVar::new_no_env(
        "OptionalWeek".to_string(),
        vec![ExprValue::<ObjectId>::Int(2)],
    );

    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalWeek(Some(2)));
}

#[test]
fn test_try_from_optional_bool() {
    use collo_ml::eval::{ExprValue, ExternVar};

    // None
    let extern_var = ExternVar::new_no_env(
        "OptionalFlag".to_string(),
        vec![ExprValue::<ObjectId>::None],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalFlag(None));

    // Some(true)
    let extern_var = ExternVar::new_no_env(
        "OptionalFlag".to_string(),
        vec![ExprValue::<ObjectId>::Bool(true)],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalFlag(Some(true)));

    // Some(false)
    let extern_var = ExternVar::new_no_env(
        "OptionalFlag".to_string(),
        vec![ExprValue::<ObjectId>::Bool(false)],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::OptionalFlag(Some(false)));
}

#[test]
fn test_try_from_student_with_mentor() {
    use collo_ml::eval::{ExprValue, ExternVar};

    // Mentor is None
    let extern_var = ExternVar::new_no_env(
        "StuMentor".to_string(),
        vec![
            ExprValue::Object(ObjectId::Student(StudentId(0))),
            ExprValue::None,
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::StudentWithMentor {
            student: StudentId(0),
            mentor: None
        }
    );

    // Mentor is Some
    let extern_var = ExternVar::new_no_env(
        "StuMentor".to_string(),
        vec![
            ExprValue::Object(ObjectId::Student(StudentId(0))),
            ExprValue::Object(ObjectId::Student(StudentId(2))),
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::StudentWithMentor {
            student: StudentId(0),
            mentor: Some(StudentId(2))
        }
    );
}

#[test]
fn test_try_from_both_optional() {
    use collo_ml::eval::{ExprValue, ExternVar};

    // Both None
    let extern_var = ExternVar::new_no_env(
        "BothOptional".to_string(),
        vec![ExprValue::<ObjectId>::None, ExprValue::None],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(var.unwrap(), OptionVar::BothOptional(None, None));

    // First Some, second None
    let extern_var = ExternVar::new_no_env(
        "BothOptional".to_string(),
        vec![
            ExprValue::Object(ObjectId::Student(StudentId(1))),
            ExprValue::None,
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::BothOptional(Some(StudentId(1)), None)
    );

    // First None, second Some
    let extern_var = ExternVar::new_no_env(
        "BothOptional".to_string(),
        vec![
            ExprValue::None,
            ExprValue::Object(ObjectId::Subject(SubjectId(2))),
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::BothOptional(None, Some(SubjectId(2)))
    );

    // Both Some
    let extern_var = ExternVar::new_no_env(
        "BothOptional".to_string(),
        vec![
            ExprValue::Object(ObjectId::Student(StudentId(0))),
            ExprValue::Object(ObjectId::Subject(SubjectId(1))),
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::BothOptional(Some(StudentId(0)), Some(SubjectId(1)))
    );
}

#[test]
fn test_try_from_multiple_optionals() {
    use collo_ml::eval::{ExprValue, ExternVar};

    // All None
    let extern_var = ExternVar::new_no_env(
        "MultipleOptionals".to_string(),
        vec![
            ExprValue::<ObjectId>::None,
            ExprValue::None,
            ExprValue::None,
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::MultipleOptionals {
            student: None,
            subject: None,
            week: None
        }
    );

    // All Some
    let extern_var = ExternVar::new_no_env(
        "MultipleOptionals".to_string(),
        vec![
            ExprValue::Object(ObjectId::Student(StudentId(1))),
            ExprValue::Object(ObjectId::Subject(SubjectId(2))),
            ExprValue::Int(1),
        ],
    );
    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_ok());
    assert_eq!(
        var.unwrap(),
        OptionVar::MultipleOptionals {
            student: Some(StudentId(1)),
            subject: Some(SubjectId(2)),
            week: Some(1)
        }
    );
}

#[test]
fn test_try_from_optional_wrong_type() {
    use collo_ml::eval::{ExprValue, ExternVar};
    use collo_ml::traits::VarConversionError;

    // Passing Int when expecting Option<StudentId>
    let extern_var = ExternVar::new_no_env(
        "OptionalStudent".to_string(),
        vec![ExprValue::<ObjectId>::Int(42)],
    );

    let var: Result<OptionVar, _> = (&extern_var).try_into();
    assert!(var.is_err());
    assert!(matches!(
        var.unwrap_err(),
        VarConversionError::WrongParameterType { param: 0, .. }
    ));
}
