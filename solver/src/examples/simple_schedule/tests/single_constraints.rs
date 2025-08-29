//! Tests for the construction of single (general) constraints

use super::*;

#[test]
fn simple_test_for_generate_at_most_one_course_per_week_for_a_given_group_constraint_for_specific_group_and_week(
) {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let (c, d) = SimpleScheduleConstraints::generate_at_most_one_course_per_week_for_a_given_group_constraint_for_specific_group_and_week(&example_problem, 1, 2);

    let lhs = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 0,
    }) + LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 1,
    }) + LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    });
    let rhs = LinExpr::constant(1.0);

    let expected_c = lhs.leq(&rhs);

    assert_eq!(c, expected_c);
    assert_eq!(
        d,
        SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 2 }
    );
}

#[test]
fn simple_test_for_generate_at_most_one_group_per_course_on_a_given_week_constraint_for_specific_week_and_course(
) {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let (c, d) = SimpleScheduleConstraints::generate_at_most_one_group_per_course_on_a_given_week_constraint_for_specific_week_and_course(&example_problem, 2, 2);

    let lhs = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 2,
    }) + LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    });
    let rhs = LinExpr::constant(1.0);

    let expected_c = lhs.leq(&rhs);

    assert_eq!(c, expected_c);
    assert_eq!(
        d,
        SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 2 }
    );
}

#[test]
fn simple_test_for_generate_each_group_should_attend_each_course_exactly_once_constraint_for_specific_group_and_course(
) {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let (c, d) = SimpleScheduleConstraints::generate_each_group_should_attend_each_course_exactly_once_constraint_for_specific_group_and_course(&example_problem, 1, 2);

    let lhs = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 2,
    }) + LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 2,
    }) + LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    }) + LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 2,
    });
    let rhs = LinExpr::constant(1.0);

    let expected_c = lhs.eq(&rhs);

    assert_eq!(c, expected_c);
    assert_eq!(
        d,
        SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce {
            course: 2,
            group: 1
        }
    );
}
