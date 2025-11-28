use std::cell::Cell;
use std::collections::{BTreeSet, HashMap};

use collo_ml::{EvalObject, ViewBuilder, ViewObject};

// ============================================================================
// Setup: Define our environment and ID types
// ============================================================================

struct TestEnv {
    students: HashMap<usize, StudentData>,
    build_count: Cell<usize>, // Track how many times build() is called
}

struct StudentData {
    age: i32,
    name: String,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct StudentId(usize);

// ============================================================================
// Define ObjectId with caching enabled
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(TestEnv)]
#[cached] // Enable caching with auto-generated name
enum CachedObjectId {
    Student(StudentId),
}

// ============================================================================
// Define ViewObject
// ============================================================================

#[derive(ViewObject, Clone)] // Clone is required for caching
#[eval_object(CachedObjectId)]
struct CachedStudent {
    age: i32,
    #[hidden]
    _name: String,
}

// ============================================================================
// Implement ViewBuilder
// ============================================================================

impl ViewBuilder<TestEnv, StudentId> for CachedObjectId {
    type Object = CachedStudent;

    fn enumerate(env: &TestEnv) -> BTreeSet<StudentId> {
        env.students.keys().map(|&id| StudentId(id)).collect()
    }

    fn build(env: &TestEnv, id: &StudentId) -> Option<Self::Object> {
        // Increment counter to track how many times build is called
        env.build_count.set(env.build_count.get() + 1);

        let data = env.students.get(&id.0)?;
        Some(CachedStudent {
            age: data.age,
            _name: data.name.clone(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_cache_reduces_build_calls() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );

    let env = TestEnv {
        students,
        build_count: Cell::new(0),
    };

    let mut cache = CachedObjectIdCache::default();
    let student = CachedObjectId::Student(StudentId(1));

    // First access - should call build()
    let age1 = student.field_access(&env, &mut cache, "age");
    assert_eq!(age1, Some(collo_ml::ExprValue::Int(20)));
    assert_eq!(env.build_count.get(), 1, "First access should call build()");

    // Second access to same field - should use cache
    let age2 = student.field_access(&env, &mut cache, "age");
    assert_eq!(age2, Some(collo_ml::ExprValue::Int(20)));
    assert_eq!(env.build_count.get(), 1, "Second access should use cache");

    // Third access to different field - should still use cache
    let age3 = student.field_access(&env, &mut cache, "age");
    assert_eq!(age3, Some(collo_ml::ExprValue::Int(20)));
    assert_eq!(env.build_count.get(), 1, "Third access should use cache");
}

#[test]
fn test_cache_stores_different_objects_separately() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );
    students.insert(
        2,
        StudentData {
            age: 22,
            name: "Bob".to_string(),
        },
    );

    let env = TestEnv {
        students,
        build_count: Cell::new(0),
    };

    let mut cache = CachedObjectIdCache::default();

    let student1 = CachedObjectId::Student(StudentId(1));
    let student2 = CachedObjectId::Student(StudentId(2));

    // Access student 1
    student1.field_access(&env, &mut cache, "age");
    assert_eq!(env.build_count.get(), 1);

    // Access student 2 - should build a new object
    student2.field_access(&env, &mut cache, "age");
    assert_eq!(env.build_count.get(), 2);

    // Access student 1 again - should use cache
    student1.field_access(&env, &mut cache, "age");
    assert_eq!(env.build_count.get(), 2, "Should reuse cached student 1");

    // Access student 2 again - should use cache
    student2.field_access(&env, &mut cache, "age");
    assert_eq!(env.build_count.get(), 2, "Should reuse cached student 2");
}

#[test]
fn test_pretty_print_uses_cache() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );

    let env = TestEnv {
        students,
        build_count: Cell::new(0),
    };

    let mut cache = CachedObjectIdCache::default();
    let student = CachedObjectId::Student(StudentId(1));

    // First pretty_print - should call build()
    let pretty1 = student.pretty_print(&env, &mut cache);
    assert_eq!(env.build_count.get(), 1);

    // Second pretty_print - should use cache
    let pretty2 = student.pretty_print(&env, &mut cache);
    assert_eq!(env.build_count.get(), 1, "pretty_print should use cache");

    assert_eq!(pretty1, pretty2);
}

#[test]
fn test_cache_shared_between_field_access_and_pretty_print() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );

    let env = TestEnv {
        students,
        build_count: Cell::new(0),
    };

    let mut cache = CachedObjectIdCache::default();
    let student = CachedObjectId::Student(StudentId(1));

    // Access field first
    student.field_access(&env, &mut cache, "age");
    assert_eq!(env.build_count.get(), 1);

    // Then pretty_print - should use the same cache
    student.pretty_print(&env, &mut cache);
    assert_eq!(env.build_count.get(), 1, "Cache should be shared");
}

#[test]
fn test_different_cache_instances_are_independent() {
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );

    let env = TestEnv {
        students,
        build_count: Cell::new(0),
    };

    let mut cache1 = CachedObjectIdCache::default();
    let mut cache2 = CachedObjectIdCache::default();
    let student = CachedObjectId::Student(StudentId(1));

    // Use cache1
    student.field_access(&env, &mut cache1, "age");
    assert_eq!(env.build_count.get(), 1);

    // Use cache2 - should build again since it's a different cache
    student.field_access(&env, &mut cache2, "age");
    assert_eq!(env.build_count.get(), 2, "Different cache should rebuild");

    // Use cache1 again - should use its cache
    student.field_access(&env, &mut cache1, "age");
    assert_eq!(env.build_count.get(), 2, "Original cache still valid");
}

#[test]
fn test_custom_cache_name() {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
    #[env(TestEnv)]
    #[cached(MyCustomCache)] // Custom name
    enum CustomObjectId {
        Student(StudentId),
    }

    #[derive(ViewObject, Clone)]
    #[eval_object(CustomObjectId)]
    struct CustomStudent {
        age: i32,
    }

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

    // Test that the custom cache name works
    let mut students = HashMap::new();
    students.insert(
        1,
        StudentData {
            age: 20,
            name: "Alice".to_string(),
        },
    );

    let env = TestEnv {
        students,
        build_count: Cell::new(0),
    };

    let mut cache = MyCustomCache::default(); // Using custom name
    let student = CustomObjectId::Student(StudentId(1));

    let result = student.field_access(&env, &mut cache, "age");
    assert_eq!(result, Some(collo_ml::ExprValue::Int(20)));
}
