use collo_ml::{EvalObject, EvalVar, ViewBuilder, ViewObject};
use std::collections::{BTreeSet, HashMap};

// ============================================================================
// Test Environment
// ============================================================================

struct EdgeCaseEnv {
    students: HashMap<u64, String>,
    max_week: i32,
}

impl EdgeCaseEnv {
    fn empty_env() -> Self {
        EdgeCaseEnv {
            students: HashMap::new(),
            max_week: 0,
        }
    }

    fn single_student_env() -> Self {
        EdgeCaseEnv {
            students: HashMap::from([(0, "Student".to_string())]),
            max_week: 1,
        }
    }
}

// ============================================================================
// Object IDs
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(EdgeCaseEnv)]
enum ObjectId {
    Student(StudentId),
}

#[derive(Clone, ViewObject)]
#[eval_object(ObjectId)]
#[pretty("Student {name}")]
struct Student {
    #[hidden]
    name: String,
}

impl ViewBuilder<EdgeCaseEnv, StudentId> for ObjectId {
    type Object = Student;

    fn enumerate(env: &EdgeCaseEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &EdgeCaseEnv, id: &StudentId) -> Option<Self::Object> {
        let name = env.students.get(&id.0)?;
        Some(Student { name: name.clone() })
    }
}

// ============================================================================
// Test 1: Edge Case - Empty Range
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum EmptyRangeVar {
    // When max_week = 0, range is 0..0 (empty)
    StudentInWeek {
        student: StudentId,
        #[range(0..env.max_week)]
        week: i32,
    },
}

#[test]
fn test_empty_range() {
    let env = EdgeCaseEnv::empty_env();

    let vars = <EmptyRangeVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // With empty range and no students, should generate 0 variables
    assert_eq!(vars.len(), 0, "Empty range should produce no variables");
}

#[test]
fn test_single_element_range() {
    let env = EdgeCaseEnv::single_student_env();

    let vars = <EmptyRangeVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // 1 student × 1 week (0..1) = 1 variable
    assert_eq!(
        vars.len(),
        1,
        "Single element range should produce 1 variable"
    );

    // Verify the variable
    let expected = EmptyRangeVar::StudentInWeek {
        student: StudentId(0),
        week: 0,
    };
    assert!(vars.contains_key(&expected));
}

// ============================================================================
// Test 2: Boundary Values
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
#[fix_with(0.0)]
enum BoundaryVar {
    TimeSlot {
        #[range(0..env.max_week)]
        slot: i32,
    },
}

#[test]
fn test_boundary_values_fix() {
    let mut env = EdgeCaseEnv::empty_env();
    env.max_week = 10;

    // Exactly at lower bound
    let var = BoundaryVar::TimeSlot { slot: 0 };
    assert_eq!(
        <BoundaryVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Lower bound should be valid"
    );

    // One before lower bound
    let var = BoundaryVar::TimeSlot { slot: -1 };
    assert_eq!(
        <BoundaryVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Below lower bound should be fixed"
    );

    // One before upper bound (9 is valid in 0..10)
    let var = BoundaryVar::TimeSlot { slot: 9 };
    assert_eq!(
        <BoundaryVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "One before upper bound should be valid"
    );

    // Exactly at upper bound (10 is invalid in 0..10)
    let var = BoundaryVar::TimeSlot { slot: 10 };
    assert_eq!(
        <BoundaryVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Upper bound should be fixed (exclusive)"
    );

    // One past upper bound
    let var = BoundaryVar::TimeSlot { slot: 11 };
    assert_eq!(
        <BoundaryVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Above upper bound should be fixed"
    );
}

// ============================================================================
// Test 3: Negative Ranges
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum NegativeRangeVar {
    Temperature {
        #[range(-10..10)]
        celsius: i32,
    },
}

#[test]
fn test_negative_range() {
    let env = EdgeCaseEnv::empty_env();

    let vars = <NegativeRangeVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // Range -10..10 = 20 values
    assert_eq!(vars.len(), 20, "Negative range should work correctly");

    // Check boundaries
    let var = NegativeRangeVar::Temperature { celsius: -10 };
    assert_eq!(
        <NegativeRangeVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "Lower bound -10 should be valid"
    );

    let var = NegativeRangeVar::Temperature { celsius: -11 };
    assert_eq!(
        <NegativeRangeVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Below -10 should be fixed"
    );

    let var = NegativeRangeVar::Temperature { celsius: 9 };
    assert_eq!(
        <NegativeRangeVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "9 should be valid in -10..10"
    );

    let var = NegativeRangeVar::Temperature { celsius: 10 };
    assert_eq!(
        <NegativeRangeVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "10 should be fixed (exclusive)"
    );
}

// ============================================================================
// Test 4: Multiple Fields with Different Fix Behaviors
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum MultiFieldVar {
    // First field out of range triggers fix
    #[fix_with(1.0)]
    Slot1 {
        #[range(0..5)]
        x: i32,
        #[range(0..5)]
        y: i32,
    },

    // Second field out of range triggers fix
    #[fix_with(2.0)]
    Slot2 {
        #[range(0..5)]
        x: i32,
        #[range(0..5)]
        y: i32,
    },

    // Both fields out of range - still returns single fix value
    #[fix_with(3.0)]
    Slot3 {
        #[range(0..5)]
        x: i32,
        #[range(0..5)]
        y: i32,
    },
}

#[test]
fn test_multiple_field_fix_priority() {
    let env = EdgeCaseEnv::empty_env();

    // First field out of range
    let var = MultiFieldVar::Slot1 { x: 10, y: 2 };
    assert_eq!(
        <MultiFieldVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(1.0),
        "First field out of range should trigger fix"
    );

    // Second field out of range
    let var = MultiFieldVar::Slot2 { x: 2, y: 10 };
    assert_eq!(
        <MultiFieldVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(2.0),
        "Second field out of range should trigger fix"
    );

    // Both fields out of range - returns the variant's fix value
    let var = MultiFieldVar::Slot3 { x: 10, y: 10 };
    assert_eq!(
        <MultiFieldVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(3.0),
        "Multiple fields out of range should return variant fix value"
    );

    // Both in range
    let var = MultiFieldVar::Slot1 { x: 2, y: 2 };
    assert_eq!(
        <MultiFieldVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "All fields in range should not fix"
    );
}

// ============================================================================
// Test 5: defer_fix Returns None for All Values
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum AlwaysValidVar {
    #[defer_fix(None)]
    AlwaysValid {
        student: StudentId,
        #[range(0..10)]
        slot: i32,
    },
}

#[test]
fn test_defer_fix_always_none() {
    let env = EdgeCaseEnv::single_student_env();

    let vars = <AlwaysValidVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // Should generate all combinations since defer_fix always returns None
    // 1 student × 10 slots = 10 variables
    assert_eq!(
        vars.len(),
        10,
        "Should generate all variables when defer_fix returns None"
    );

    // Verify fix returns None for all
    for var in vars.keys() {
        assert_eq!(
            <AlwaysValidVar as EvalVar<ObjectId>>::fix(var, &env),
            None,
            "All variables should have fix = None"
        );
    }
}

// ============================================================================
// Test 6: defer_fix Returns Some for All Values
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum AlwaysInvalidVar {
    #[defer_fix(Some(99.0))]
    AlwaysInvalid {
        student: StudentId,
        #[range(0..10)]
        slot: i32,
    },
}

#[test]
fn test_defer_fix_always_some() {
    let env = EdgeCaseEnv::single_student_env();

    let vars = <AlwaysInvalidVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // Should generate NO variables since defer_fix always returns Some
    assert_eq!(
        vars.len(),
        0,
        "Should generate no variables when defer_fix always returns Some"
    );
}

// ============================================================================
// Test 7: Complex fix_with Expression
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum ComplexFixVar {
    #[fix_with(
        if x * y > 10 {
            (x * y) as f64
        } else if x + y > 5 {
            0.5
        } else {
            0.0
        }
    )]
    ComplexLogic {
        #[range(0..10)]
        x: i32,
        #[range(0..10)]
        y: i32,
    },
}

#[test]
fn test_complex_fix_with_expression() {
    let env = EdgeCaseEnv::empty_env();

    // x * y > 10 case (out of range triggers check)
    let var = ComplexFixVar::ComplexLogic { x: 100, y: 2 };
    assert_eq!(
        <ComplexFixVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(200.0),
        "x * y = 200 > 10, should return 200.0"
    );

    // x + y > 5 but x * y <= 10 (out of range triggers check)
    let var = ComplexFixVar::ComplexLogic { x: 100, y: 0 };
    assert_eq!(
        <ComplexFixVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.5),
        "x + y = 100 > 5 but x*y = 0, should return 0.5"
    );

    // Neither condition (out of range triggers check)
    let var = ComplexFixVar::ComplexLogic { x: -1, y: 0 };
    assert_eq!(
        <ComplexFixVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "Neither condition met, should return 0.0"
    );

    // In range - no fix
    let var = ComplexFixVar::ComplexLogic { x: 2, y: 2 };
    assert_eq!(
        <ComplexFixVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "In range should not fix"
    );
}

// ============================================================================
// Test 8: Unit Variant (No Fields)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
enum UnitVariantVar {
    #[fix_with(5.0)]
    NoFields,

    #[defer_fix(Some(10.0))]
    NoFieldsDefer,
}

#[test]
fn test_unit_variant() {
    let env = EdgeCaseEnv::empty_env();

    let vars = <UnitVariantVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // NoFields: should generate 1 var (no defer_fix or defer_fix returns None)
    // NoFieldsDefer: should NOT generate (defer_fix returns Some)
    // Total: 1 variable
    assert_eq!(
        vars.len(),
        1,
        "Should generate 1 variable for unit variants"
    );

    // Verify NoFields exists
    assert!(
        vars.contains_key(&UnitVariantVar::NoFields),
        "NoFields should be generated"
    );

    // Verify fix behavior
    assert_eq!(
        <UnitVariantVar as EvalVar<ObjectId>>::fix(&UnitVariantVar::NoFields, &env),
        None,
        "Unit variant with no out-of-range fields should not fix"
    );

    assert_eq!(
        <UnitVariantVar as EvalVar<ObjectId>>::fix(&UnitVariantVar::NoFieldsDefer, &env),
        Some(10.0),
        "Unit variant with defer_fix should return defer_fix value"
    );
}

// ============================================================================
// Test 9: Boolean Fields
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum BoolVar {
    WithBool {
        #[range(0..2)]
        x: i32,
        flag: bool,
    },
}

#[test]
fn test_boolean_field_enumeration() {
    let env = EdgeCaseEnv::empty_env();

    let vars = <BoolVar as EvalVar<ObjectId>>::vars(&env).expect("Should generate vars");

    // 2 values for x (0..2) × 2 values for bool = 4 variables
    assert_eq!(vars.len(), 4, "Should enumerate both true and false");

    // Verify all combinations exist
    for x in 0..2 {
        for flag in [false, true] {
            let var = BoolVar::WithBool { x, flag };
            assert!(
                vars.contains_key(&var),
                "Should contain x={}, flag={}",
                x,
                flag
            );
        }
    }
}

// ============================================================================
// Test 10: Unnamed Fields with Complex Expressions
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(EdgeCaseEnv)]
enum UnnamedComplexVar {
    #[fix_with(if *v0 + *v1 > 10 { (*v0 + *v1) as f64 } else { 0.0 })]
    UnnamedSlot(#[range(0..10)] i32, #[range(0..10)] i32),
}

#[test]
fn test_unnamed_fields_complex_fix() {
    let env = EdgeCaseEnv::empty_env();

    // Out of range triggers complex expression
    let var = UnnamedComplexVar::UnnamedSlot(100, 5);
    assert_eq!(
        <UnnamedComplexVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(105.0),
        "100 + 5 = 105 > 10, should return 105.0"
    );

    // Out of range but sum <= 10
    let var = UnnamedComplexVar::UnnamedSlot(-5, 3);
    assert_eq!(
        <UnnamedComplexVar as EvalVar<ObjectId>>::fix(&var, &env),
        Some(0.0),
        "-5 + 3 = -2 <= 10, should return 0.0"
    );

    // In range, no fix
    let var = UnnamedComplexVar::UnnamedSlot(5, 3);
    assert_eq!(
        <UnnamedComplexVar as EvalVar<ObjectId>>::fix(&var, &env),
        None,
        "In range should not fix"
    );
}
