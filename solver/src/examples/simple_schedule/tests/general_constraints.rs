use super::*;

#[test]
fn simple_test_for_generate_at_most_one_course_per_week_for_a_given_group_constraints() {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let result: BTreeSet<_> = SimpleScheduleConstraints::generate_at_most_one_course_per_week_for_a_given_group_constraints(&example_problem)
        .into_iter()
        .collect();

    let v_0_0_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 0,
    });
    let v_0_0_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 1,
    });
    let v_0_0_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 2,
    });
    let v_0_1_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 0,
    });
    let v_0_1_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 1,
    });
    let v_0_1_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 2,
    });
    let v_0_2_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 0,
    });
    let v_0_2_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 1,
    });
    let v_0_2_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 2,
    });
    let v_0_3_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 0,
    });
    let v_0_3_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 1,
    });
    let v_0_3_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 2,
    });

    let v_1_0_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 0,
    });
    let v_1_0_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 1,
    });
    let v_1_0_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 2,
    });
    let v_1_1_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 0,
    });
    let v_1_1_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 1,
    });
    let v_1_1_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 2,
    });
    let v_1_2_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 0,
    });
    let v_1_2_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 1,
    });
    let v_1_2_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    });
    let v_1_3_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 0,
    });
    let v_1_3_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 1,
    });
    let v_1_3_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 2,
    });

    let one = LinExpr::constant(1.0);

    #[rustfmt::skip]
    let expected = BTreeSet::from([
        ((&v_0_0_0 + &v_0_0_1 + &v_0_0_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 0 }),
        ((&v_0_1_0 + &v_0_1_1 + &v_0_1_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 1 }),
        ((&v_0_2_0 + &v_0_2_1 + &v_0_2_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 2 }),
        ((&v_0_3_0 + &v_0_3_1 + &v_0_3_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 3 }),

        ((&v_1_0_0 + &v_1_0_1 + &v_1_0_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 0 }),
        ((&v_1_1_0 + &v_1_1_1 + &v_1_1_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 1 }),
        ((&v_1_2_0 + &v_1_2_1 + &v_1_2_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 2 }),
        ((&v_1_3_0 + &v_1_3_1 + &v_1_3_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 3 }),
    ]);

    assert_eq!(result, expected);
}

#[test]
fn simple_test_for_generate_at_most_one_group_per_course_on_a_given_week_constraints() {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let result: BTreeSet<_> = SimpleScheduleConstraints::generate_at_most_one_group_per_course_on_a_given_week_constraints(&example_problem)
        .into_iter()
        .collect();

    let v_0_0_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 0,
    });
    let v_0_0_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 1,
    });
    let v_0_0_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 2,
    });
    let v_0_1_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 0,
    });
    let v_0_1_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 1,
    });
    let v_0_1_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 2,
    });
    let v_0_2_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 0,
    });
    let v_0_2_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 1,
    });
    let v_0_2_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 2,
    });
    let v_0_3_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 0,
    });
    let v_0_3_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 1,
    });
    let v_0_3_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 2,
    });

    let v_1_0_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 0,
    });
    let v_1_0_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 1,
    });
    let v_1_0_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 2,
    });
    let v_1_1_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 0,
    });
    let v_1_1_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 1,
    });
    let v_1_1_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 2,
    });
    let v_1_2_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 0,
    });
    let v_1_2_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 1,
    });
    let v_1_2_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    });
    let v_1_3_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 0,
    });
    let v_1_3_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 1,
    });
    let v_1_3_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 2,
    });

    let one = LinExpr::constant(1.0);

    #[rustfmt::skip]
    let expected = BTreeSet::from([
        ((&v_0_0_0 + &v_1_0_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 0 }),
        ((&v_0_1_0 + &v_1_1_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 1 }),
        ((&v_0_2_0 + &v_1_2_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 2 }),
        ((&v_0_3_0 + &v_1_3_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 3 }),

        ((&v_0_0_1 + &v_1_0_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 0 }),
        ((&v_0_1_1 + &v_1_1_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 1 }),
        ((&v_0_2_1 + &v_1_2_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 2 }),
        ((&v_0_3_1 + &v_1_3_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 3 }),

        ((&v_0_0_2 + &v_1_0_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 0 }),
        ((&v_0_1_2 + &v_1_1_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 1 }),
        ((&v_0_2_2 + &v_1_2_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 2 }),
        ((&v_0_3_2 + &v_1_3_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 3 }),
    ]);

    assert_eq!(result, expected);
}

#[test]
fn simple_test_for_generate_each_group_should_attend_each_course_exactly_once_constraints() {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let result: BTreeSet<_> = SimpleScheduleConstraints::generate_each_group_should_attend_each_course_exactly_once_constraints(&example_problem)
        .into_iter()
        .collect();

    let v_0_0_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 0,
    });
    let v_0_0_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 1,
    });
    let v_0_0_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 2,
    });
    let v_0_1_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 0,
    });
    let v_0_1_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 1,
    });
    let v_0_1_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 2,
    });
    let v_0_2_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 0,
    });
    let v_0_2_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 1,
    });
    let v_0_2_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 2,
    });
    let v_0_3_0 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 0,
    });
    let v_0_3_1 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 1,
    });
    let v_0_3_2 = LinExpr::var(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 2,
    });

    let v_1_0_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 0,
    });
    let v_1_0_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 1,
    });
    let v_1_0_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 2,
    });
    let v_1_1_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 0,
    });
    let v_1_1_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 1,
    });
    let v_1_1_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 2,
    });
    let v_1_2_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 0,
    });
    let v_1_2_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 1,
    });
    let v_1_2_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    });
    let v_1_3_0 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 0,
    });
    let v_1_3_1 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 1,
    });
    let v_1_3_2 = LinExpr::var(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 2,
    });

    let one = LinExpr::constant(1.0);

    #[rustfmt::skip]
    let expected = BTreeSet::from([
        ((&v_0_0_0 + &v_0_1_0 + &v_0_2_0 + &v_0_3_0).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 0, group: 0 }),
        ((&v_0_0_1 + &v_0_1_1 + &v_0_2_1 + &v_0_3_1).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 1, group: 0 }),
        ((&v_0_0_2 + &v_0_1_2 + &v_0_2_2 + &v_0_3_2).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 2, group: 0 }),
        
        ((&v_1_0_0 + &v_1_1_0 + &v_1_2_0 + &v_1_3_0).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 0, group: 1 }),
        ((&v_1_0_1 + &v_1_1_1 + &v_1_2_1 + &v_1_3_1).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 1, group: 1 }),
        ((&v_1_0_2 + &v_1_1_2 + &v_1_2_2 + &v_1_3_2).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 2, group: 1 }),
    ]);

    assert_eq!(result, expected);
}

#[test]
fn simple_test_for_general_constraints() {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    let constraints = SimpleScheduleConstraints {};

    use crate::ProblemConstraints;

    let result: BTreeSet<_> = constraints
        .general_constraints(&example_problem)
        .into_iter()
        .collect();

    let v_0_0_0 =
        LinExpr::<ExtraVariable<_, _, _>>::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
            group: 0,
            week: 0,
            course: 0,
        }));
    let v_0_0_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 1,
    }));
    let v_0_0_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 0,
        course: 2,
    }));
    let v_0_1_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 0,
    }));
    let v_0_1_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 1,
    }));
    let v_0_1_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 1,
        course: 2,
    }));
    let v_0_2_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 0,
    }));
    let v_0_2_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 1,
    }));
    let v_0_2_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 2,
        course: 2,
    }));
    let v_0_3_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 0,
    }));
    let v_0_3_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 1,
    }));
    let v_0_3_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 0,
        week: 3,
        course: 2,
    }));

    let v_1_0_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 0,
    }));
    let v_1_0_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 1,
    }));
    let v_1_0_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 0,
        course: 2,
    }));
    let v_1_1_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 0,
    }));
    let v_1_1_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 1,
    }));
    let v_1_1_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 1,
        course: 2,
    }));
    let v_1_2_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 0,
    }));
    let v_1_2_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 1,
    }));
    let v_1_2_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 2,
        course: 2,
    }));
    let v_1_3_0 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 0,
    }));
    let v_1_3_1 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 1,
    }));
    let v_1_3_2 = LinExpr::var(ExtraVariable::BaseMain(SimpleScheduleVariable {
        group: 1,
        week: 3,
        course: 2,
    }));

    let one = LinExpr::constant(1.0);

    #[rustfmt::skip]
    let expected = BTreeSet::from([
        ((&v_0_0_0 + &v_0_0_1 + &v_0_0_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 0 }),
        ((&v_0_1_0 + &v_0_1_1 + &v_0_1_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 1 }),
        ((&v_0_2_0 + &v_0_2_1 + &v_0_2_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 2 }),
        ((&v_0_3_0 + &v_0_3_1 + &v_0_3_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 0, week: 3 }),

        ((&v_1_0_0 + &v_1_0_1 + &v_1_0_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 0 }),
        ((&v_1_1_0 + &v_1_1_1 + &v_1_1_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 1 }),
        ((&v_1_2_0 + &v_1_2_1 + &v_1_2_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 2 }),
        ((&v_1_3_0 + &v_1_3_1 + &v_1_3_2).leq(&one), SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group: 1, week: 3 }),

        ((&v_0_0_0 + &v_1_0_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 0 }),
        ((&v_0_1_0 + &v_1_1_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 1 }),
        ((&v_0_2_0 + &v_1_2_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 2 }),
        ((&v_0_3_0 + &v_1_3_0).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 0, week: 3 }),

        ((&v_0_0_1 + &v_1_0_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 0 }),
        ((&v_0_1_1 + &v_1_1_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 1 }),
        ((&v_0_2_1 + &v_1_2_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 2 }),
        ((&v_0_3_1 + &v_1_3_1).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 1, week: 3 }),

        ((&v_0_0_2 + &v_1_0_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 0 }),
        ((&v_0_1_2 + &v_1_1_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 1 }),
        ((&v_0_2_2 + &v_1_2_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 2 }),
        ((&v_0_3_2 + &v_1_3_2).leq(&one), SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course: 2, week: 3 }),

        ((&v_0_0_0 + &v_0_1_0 + &v_0_2_0 + &v_0_3_0).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 0, group: 0 }),
        ((&v_0_0_1 + &v_0_1_1 + &v_0_2_1 + &v_0_3_1).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 1, group: 0 }),
        ((&v_0_0_2 + &v_0_1_2 + &v_0_2_2 + &v_0_3_2).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 2, group: 0 }),
        
        ((&v_1_0_0 + &v_1_1_0 + &v_1_2_0 + &v_1_3_0).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 0, group: 1 }),
        ((&v_1_0_1 + &v_1_1_1 + &v_1_2_1 + &v_1_3_1).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 1, group: 1 }),
        ((&v_1_0_2 + &v_1_1_2 + &v_1_2_2 + &v_1_3_2).eq(&one), SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course: 2, group: 1 }),
    ]);

    assert_eq!(result, expected);
}
