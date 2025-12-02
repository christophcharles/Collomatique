use collo_ml::{EvalObject, EvalVar, ViewBuilder, ViewObject};
use std::collections::{BTreeSet, HashMap};

// ============================================================================
// Test Environment with Dynamic Configuration
// ============================================================================

#[derive(Debug, Clone)]
struct StudentData {
    name: String,
    absent_weeks: Vec<i32>,
}

#[derive(Debug, Clone)]
struct SubjectData {
    name: String,
}

struct DynamicEnv {
    students: HashMap<u64, StudentData>,
    subjects: HashMap<u64, SubjectData>,
    max_week: i32,
    last_hour: i32,
    lunch_mandatory: bool,
}

impl DynamicEnv {
    fn test_env() -> Self {
        DynamicEnv {
            students: HashMap::from([
                (
                    0,
                    StudentData {
                        name: "Harry Potter".to_string(),
                        absent_weeks: vec![1], // Harry is absent week 1
                    },
                ),
                (
                    1,
                    StudentData {
                        name: "Hermione Granger".to_string(),
                        absent_weeks: vec![],
                    },
                ),
                (
                    2,
                    StudentData {
                        name: "Ron Weasley".to_string(),
                        absent_weeks: vec![2], // Ron is absent week 2
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
            ]),
            max_week: 4,   // 4 weeks: 0..4
            last_hour: 17, // Hours: 8..17
            lunch_mandatory: true,
        }
    }

    fn relaxed_env() -> Self {
        let mut env = Self::test_env();
        env.lunch_mandatory = false;
        env
    }

    fn short_schedule_env() -> Self {
        let mut env = Self::test_env();
        env.max_week = 2; // Only 2 weeks
        env.last_hour = 15; // Shorter days
        env
    }

    fn is_student_absent(&self, student: &StudentId, week: i32) -> bool {
        if let Some(data) = self.students.get(&student.0) {
            data.absent_weeks.contains(&week)
        } else {
            false
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
// EvalObject
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(DynamicEnv)]
enum ObjectId {
    Student(StudentId),
    Subject(SubjectId),
}

// ============================================================================
// View Objects
// ============================================================================

#[derive(Clone, ViewObject)]
#[eval_object(ObjectId)]
struct Student {
    #[hidden]
    name: String,
}

#[derive(Clone, ViewObject)]
#[eval_object(ObjectId)]
struct Subject {
    #[hidden]
    name: String,
}

// ============================================================================
// ViewBuilder Implementations
// ============================================================================

impl ViewBuilder<DynamicEnv, StudentId> for ObjectId {
    type Object = Student;

    fn enumerate(env: &DynamicEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &DynamicEnv, id: &StudentId) -> Option<Self::Object> {
        let data = env.students.get(&id.0)?;
        Some(Student {
            name: data.name.clone(),
        })
    }
}

impl ViewBuilder<DynamicEnv, SubjectId> for ObjectId {
    type Object = Subject;

    fn enumerate(env: &DynamicEnv) -> BTreeSet<SubjectId> {
        env.subjects.keys().map(|&id| SubjectId(id)).collect()
    }

    fn build(env: &DynamicEnv, id: &SubjectId) -> Option<Self::Object> {
        let data = env.subjects.get(&id.0)?;
        Some(Subject {
            name: data.name.clone(),
        })
    }
}

// ============================================================================
// EvalVar with Dynamic Features
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(DynamicEnv)]
#[fix_with(0.0)]
enum DynamicVar {
    // Test 1: Dynamic range from env
    // Range depends on env.max_week
    StudentInWeek {
        student: StudentId,
        #[range(0..env.max_week)]
        week: i32,
    },

    // Test 2: Dynamic range with dynamic fix_with based on env
    // Fix value depends on whether lunch is mandatory
    #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
    TimeSlot {
        #[range(0..7)]
        day: i32,
        #[range(8..env.last_hour)]
        hour: i32,
    },

    // Test 3: Dynamic fix_with based on field values (named fields)
    // Lunch hours (12-14) get special fix value
    #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
    WorkSlot {
        #[range(0..5)]
        day: i32,
        #[range(8..18)]
        hour: i32,
    },

    // Test 4: Dynamic fix_with based on field values (unnamed fields)
    #[fix_with(if *v1 >= 12 && *v1 < 14 { 3.0 } else { 0.0 })]
    UnnamedSlot(#[range(0..5)] i32, #[range(8..18)] i32),

    // Test 5: defer_fix with complex logic
    #[defer_fix(Self::check_student_availability(env, student, week))]
    StudentAvailable {
        student: StudentId,
        #[range(0..env.max_week)]
        week: i32,
    },

    // Test 6: defer_fix with inline expression
    #[defer_fix({
        if *week >= 3 {
            Some(5.0)
        } else {
            None
        }
    })]
    LateWeekPenalty {
        subject: SubjectId,
        #[range(0..env.max_week)]
        week: i32,
    },
}

impl DynamicVar {
    fn check_student_availability(
        env: &DynamicEnv,
        student: &StudentId,
        week: &i32,
    ) -> Option<f64> {
        let env = env.as_ref();
        if env.is_student_absent(student, *week) {
            Some(10.0) // High penalty for assigning when absent
        } else {
            None
        }
    }
}

// Helper to make DynamicEnv work with the generic trait bound
impl AsRef<DynamicEnv> for DynamicEnv {
    fn as_ref(&self) -> &DynamicEnv {
        self
    }
}

// ============================================================================
// Tests for Dynamic Ranges
// ============================================================================

#[test]
fn test_dynamic_range_vars_generation() {
    let env = DynamicEnv::test_env();

    // Test with max_week = 4 (range 0..4)
    let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // Count StudentInWeek vars: 3 students × 4 weeks = 12
    let student_in_week_count = vars
        .keys()
        .filter(|v| matches!(v, DynamicVar::StudentInWeek { .. }))
        .count();
    assert_eq!(
        student_in_week_count, 12,
        "Should have 12 StudentInWeek vars with 4 weeks"
    );

    // Now test with shorter schedule
    let short_env = DynamicEnv::short_schedule_env();
    let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&short_env).expect("Should generate vars");

    // Count StudentInWeek vars: 3 students × 2 weeks = 6
    let student_in_week_count = vars
        .keys()
        .filter(|v| matches!(v, DynamicVar::StudentInWeek { .. }))
        .count();
    assert_eq!(
        student_in_week_count, 6,
        "Should have 6 StudentInWeek vars with 2 weeks"
    );
}

#[test]
fn test_dynamic_range_fix() {
    let env = DynamicEnv::test_env();

    // Within range (0..4)
    let var = DynamicVar::StudentInWeek {
        student: StudentId(0),
        week: 2,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Week 2 should be valid with max_week=4"
    );

    // Outside range (week 5 >= max_week 4)
    let var = DynamicVar::StudentInWeek {
        student: StudentId(0),
        week: 5,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Week 5 should be fixed with max_week=4"
    );

    // Test with different env
    let short_env = DynamicEnv::short_schedule_env();
    let var = DynamicVar::StudentInWeek {
        student: StudentId(0),
        week: 3,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &short_env),
        Some(0.0),
        "Week 3 should be fixed with max_week=2"
    );
}

// ============================================================================
// Tests for Dynamic fix_with Based on Env
// ============================================================================

#[test]
fn test_dynamic_fix_with_env() {
    let env = DynamicEnv::test_env();
    let relaxed = DynamicEnv::relaxed_env();

    // TimeSlot with hour=20 (outside range 8..17)
    let var = DynamicVar::TimeSlot { day: 3, hour: 20 };

    // Mandatory lunch → fix_with = 1.0
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(1.0),
        "Should use fix value 1.0 when lunch_mandatory=true"
    );

    // Relaxed lunch → fix_with = 0.5
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &relaxed),
        Some(0.5),
        "Should use fix value 0.5 when lunch_mandatory=false"
    );

    // Test with shorter schedule (last_hour = 15)
    let short_env = DynamicEnv::short_schedule_env();
    let var = DynamicVar::TimeSlot {
        day: 3,
        hour: 16, // Outside 8..15
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &short_env),
        Some(1.0),
        "Should fix hour=16 when last_hour=15"
    );
}

#[test]
fn test_dynamic_range_with_vars_filtering() {
    let env = DynamicEnv::test_env(); // last_hour = 17

    let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // Check that TimeSlot vars only go up to hour 16 (range 8..17)
    let has_invalid_hour = vars
        .keys()
        .filter_map(|v| match v {
            DynamicVar::TimeSlot { hour, .. } => Some(*hour),
            _ => None,
        })
        .any(|hour| hour >= 17 || hour < 8);

    assert!(
        !has_invalid_hour,
        "Should not generate TimeSlot vars with invalid hours"
    );

    // Count TimeSlot vars: 7 days × 9 hours (8..17) = 63
    let timeslot_count = vars
        .keys()
        .filter(|v| matches!(v, DynamicVar::TimeSlot { .. }))
        .count();
    assert_eq!(timeslot_count, 63, "Should have 63 TimeSlot vars");
}

// ============================================================================
// Tests for Dynamic fix_with Based on Field Values
// ============================================================================

#[test]
fn test_fix_with_field_values_named() {
    let env = DynamicEnv::test_env();

    // Lunch hours (12-14) should get fix value 2.0
    let var = DynamicVar::WorkSlot {
        day: 20,  // Outside range 0..5
        hour: 13, // Lunch hour
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(2.0),
        "Lunch hours should use fix value 2.0"
    );

    // Non-lunch hours should get fix value 0.0
    let var = DynamicVar::WorkSlot {
        day: 20,  // Outside range
        hour: 10, // Not lunch
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Non-lunch hours should use fix value 0.0"
    );

    // Within all ranges - no fix
    let var = DynamicVar::WorkSlot { day: 2, hour: 13 };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Valid values should not be fixed"
    );
}

#[test]
fn test_fix_with_field_values_unnamed() {
    let env = DynamicEnv::test_env();

    // Lunch hours (12-14) should get fix value 3.0
    let var = DynamicVar::UnnamedSlot(20, 13); // day outside range, hour in lunch
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(3.0),
        "Unnamed lunch hours should use fix value 3.0"
    );

    // Non-lunch hours should get fix value 0.0
    let var = DynamicVar::UnnamedSlot(20, 10);
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Unnamed non-lunch hours should use fix value 0.0"
    );
}

// ============================================================================
// Tests for defer_fix
// ============================================================================

#[test]
fn test_defer_fix_with_function() {
    let env = DynamicEnv::test_env();

    // Harry is absent week 1
    let var = DynamicVar::StudentAvailable {
        student: StudentId(0), // Harry
        week: 1,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(10.0),
        "Harry should be fixed (penalty 10.0) for week 1"
    );

    // Harry is available week 2
    let var = DynamicVar::StudentAvailable {
        student: StudentId(0),
        week: 2,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Harry should be available for week 2"
    );

    // Ron is absent week 2
    let var = DynamicVar::StudentAvailable {
        student: StudentId(2), // Ron
        week: 2,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(10.0),
        "Ron should be fixed for week 2"
    );

    // Hermione is never absent
    let var = DynamicVar::StudentAvailable {
        student: StudentId(1), // Hermione
        week: 1,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Hermione should always be available"
    );
}

#[test]
fn test_defer_fix_with_inline_expression() {
    let env = DynamicEnv::test_env();

    // Week 3+ should be penalized
    let var = DynamicVar::LateWeekPenalty {
        subject: SubjectId(0),
        week: 3,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(5.0),
        "Week 3 should have penalty"
    );

    // Week < 3 should not be penalized
    let var = DynamicVar::LateWeekPenalty {
        subject: SubjectId(0),
        week: 2,
    };
    assert_eq!(
        <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Week 2 should not have penalty"
    );
}

#[test]
fn test_defer_fix_filters_vars() {
    let env = DynamicEnv::test_env();

    let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // StudentAvailable should not include variables that would be fixed
    // Harry (0) should not have week 1
    let harry_week_1_exists = vars.keys().any(|v| match v {
        DynamicVar::StudentAvailable { student, week } => student.0 == 0 && *week == 1,
        _ => false,
    });
    assert!(
        !harry_week_1_exists,
        "Should not generate Harry in week 1 (he's absent)"
    );

    // Harry should have week 2
    let harry_week_2_exists = vars.keys().any(|v| match v {
        DynamicVar::StudentAvailable { student, week } => student.0 == 0 && *week == 2,
        _ => false,
    });
    assert!(
        harry_week_2_exists,
        "Should generate Harry in week 2 (he's available)"
    );

    // Ron (2) should not have week 2
    let ron_week_2_exists = vars.keys().any(|v| match v {
        DynamicVar::StudentAvailable { student, week } => student.0 == 2 && *week == 2,
        _ => false,
    });
    assert!(
        !ron_week_2_exists,
        "Should not generate Ron in week 2 (he's absent)"
    );

    // LateWeekPenalty should not include week 3+ (they get fixed)
    let has_late_weeks = vars.keys().any(|v| match v {
        DynamicVar::LateWeekPenalty { week, .. } => *week >= 3,
        _ => false,
    });
    assert!(
        !has_late_weeks,
        "Should not generate LateWeekPenalty for weeks >= 3"
    );
}

// ============================================================================
// Integration Test
// ============================================================================

#[test]
fn test_complete_dynamic_scenario() {
    let env = DynamicEnv::test_env();

    let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // Verify no fixed variables are in the set
    for var in vars.keys() {
        let fix_result = <DynamicVar as EvalVar<ObjectId>>::fix(var, &env);
        assert_eq!(
            fix_result, None,
            "Variable in vars() should not have a fix value. Found: {:?}",
            var
        );
    }
}
