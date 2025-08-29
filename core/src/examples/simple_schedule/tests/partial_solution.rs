//! Tests for SimpleSchedulePartialSolution

use super::*;
use std::collections::BTreeSet;

fn example_partial_solution() -> SimpleSchedulePartialSolution {
    SimpleSchedulePartialSolution {
        week_count: 4,
        assigned_courses: vec![
            // Group 1
            BTreeSet::from([0, 2]),
            BTreeSet::from([1]),
            BTreeSet::from([2]),
            BTreeSet::from([]),
            // Group 2
            BTreeSet::from([1]),
            BTreeSet::from([]),
            BTreeSet::from([0, 2]),
            BTreeSet::from([]),
            // Group 3
            BTreeSet::from([2]),
            BTreeSet::from([0]),
            BTreeSet::from([]),
            BTreeSet::from([]),
        ],
        unassigned_courses: vec![
            // Group 1
            BTreeSet::from([1]),
            BTreeSet::from([2]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 2
            BTreeSet::from([0, 2]),
            BTreeSet::from([]),
            BTreeSet::from([1]),
            BTreeSet::from([0, 1, 2]),
            // Group 3
            BTreeSet::from([]),
            BTreeSet::from([1, 2]),
            BTreeSet::from([0, 1, 2]),
            BTreeSet::from([0, 2]),
        ],
    }
}

#[test]
fn check_compute_index_with_invalid_week() {
    let partial_solution = example_partial_solution();

    assert_eq!(partial_solution.compute_index(1, 4), None);
}

#[test]
fn check_compute_index_with_invalid_group() {
    let partial_solution = example_partial_solution();

    assert_eq!(partial_solution.compute_index(3, 2), None);
}

#[test]
fn check_compute_index_with_both_invalid_week_and_invalid_group() {
    let partial_solution = example_partial_solution();

    assert_eq!(partial_solution.compute_index(3, 4), None);
}

#[test]
fn check_compute_index_with_valid_data() {
    let partial_solution = example_partial_solution();

    assert_eq!(partial_solution.compute_index(1, 2), Some(6));
}

#[test]
fn check_is_complete_on_clearly_not_complete() {
    let partial_solution = example_partial_solution();

    assert_eq!(partial_solution.is_complete(), false);
}

#[test]
fn check_is_complete_on_nearly_complete() {
    let partial_solution = SimpleSchedulePartialSolution {
        week_count: 4,
        assigned_courses: vec![
            // Group 1
            BTreeSet::from([0, 2]),
            BTreeSet::from([1]),
            BTreeSet::from([2]),
            BTreeSet::from([]),
            // Group 2
            BTreeSet::from([1]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 3
            BTreeSet::from([2]),
            BTreeSet::from([0]),
            BTreeSet::from([]),
            BTreeSet::from([]),
        ],
        unassigned_courses: vec![
            // Group 1
            BTreeSet::from([]),
            BTreeSet::from([2]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 2
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 3
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
        ],
    };

    assert_eq!(partial_solution.is_complete(), false);
}

#[test]
fn check_is_complete_on_actually_complete() {
    let partial_solution = SimpleSchedulePartialSolution {
        week_count: 4,
        assigned_courses: vec![
            // Group 1
            BTreeSet::from([0, 2]),
            BTreeSet::from([1]),
            BTreeSet::from([2]),
            BTreeSet::from([]),
            // Group 2
            BTreeSet::from([1]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 3
            BTreeSet::from([2]),
            BTreeSet::from([0]),
            BTreeSet::from([]),
            BTreeSet::from([]),
        ],
        unassigned_courses: vec![
            // Group 1
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 2
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            // Group 3
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
            BTreeSet::from([]),
        ],
    };

    assert_eq!(partial_solution.is_complete(), true);
}

#[test]
fn simple_get_assigned_test() {
    let partial_solution = example_partial_solution();

    assert_eq!(
        partial_solution.get_assigned(1, 2).cloned(),
        Some(BTreeSet::from([0, 2]))
    );
}

#[test]
fn simple_get_unassigned_test() {
    let partial_solution = example_partial_solution();

    assert_eq!(
        partial_solution.get_unassigned(1, 2).cloned(),
        Some(BTreeSet::from([1]))
    );
}
