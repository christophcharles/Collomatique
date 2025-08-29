use super::*;

#[test]
fn test_partial_solution_to_configuration() {
    let problem = SimpleScheduleBase {
        group_count: 3,
        week_count: 4,
        course_count: 3,
    };

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
    };

    let config_data = ConfigData::new();

    // Group 1 - week 1
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 0,
            },
            1.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 0,
            },
            1.0,
        );

    // Group 1 - week 2
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 1,
            },
            1.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 1,
            },
            0.0,
        );

    // Group 1 - week 3
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 2,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 2,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 2,
            },
            1.0,
        );

    // Group 1 - week 4
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 3,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 3,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 3,
            },
            0.0,
        );

    // Group 2 - week 1
    let config_data = config_data.set(
        SimpleScheduleVariable {
            group: 1,
            course: 1,
            week: 0,
        },
        1.0,
    );

    // Group 2 - week 2
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 1,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 1,
                week: 1,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 1,
            },
            0.0,
        );

    // Group 2 - week 3
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 2,
            },
            1.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 2,
            },
            1.0,
        );

    // Group 2 - week 4
    let config_data = config_data;

    // Group 3 - week 1
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 2,
                course: 0,
                week: 0,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 2,
                course: 1,
                week: 0,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 2,
                course: 2,
                week: 0,
            },
            1.0,
        );

    // Group 3 - week 2
    let config_data = config_data.set(
        SimpleScheduleVariable {
            group: 2,
            course: 0,
            week: 1,
        },
        1.0,
    );

    // Group 3 - week 3
    let config_data = config_data;

    // Group 3 - week 4
    let config_data = config_data.set(
        SimpleScheduleVariable {
            group: 2,
            course: 1,
            week: 3,
        },
        0.0,
    );

    let computed_config_data = problem.partial_solution_to_configuration(&partial_solution);

    assert_eq!(Some(config_data), computed_config_data);
}

#[test]
fn test_configuration_to_partial_solution() {
    let problem = SimpleScheduleBase {
        group_count: 3,
        week_count: 4,
        course_count: 3,
    };

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
    };

    let config_data = ConfigData::new();

    // Group 1 - week 1
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 0,
            },
            1.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 0,
            },
            1.0,
        );

    // Group 1 - week 2
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 1,
            },
            1.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 1,
            },
            0.0,
        );

    // Group 1 - week 3
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 2,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 2,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 2,
            },
            1.0,
        );

    // Group 1 - week 4
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 3,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 3,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 3,
            },
            0.0,
        );

    // Group 2 - week 1
    let config_data = config_data.set(
        SimpleScheduleVariable {
            group: 1,
            course: 1,
            week: 0,
        },
        1.0,
    );

    // Group 2 - week 2
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 1,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 1,
                week: 1,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 1,
            },
            0.0,
        );

    // Group 2 - week 3
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 2,
            },
            1.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 2,
            },
            1.0,
        );

    // Group 2 - week 4
    let config_data = config_data;

    // Group 3 - week 1
    let config_data = config_data
        .set(
            SimpleScheduleVariable {
                group: 2,
                course: 0,
                week: 0,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 2,
                course: 1,
                week: 0,
            },
            0.0,
        )
        .set(
            SimpleScheduleVariable {
                group: 2,
                course: 2,
                week: 0,
            },
            1.0,
        );

    // Group 3 - week 2
    let config_data = config_data.set(
        SimpleScheduleVariable {
            group: 2,
            course: 0,
            week: 1,
        },
        1.0,
    );

    // Group 3 - week 3
    let config_data = config_data;

    // Group 3 - week 4
    let config_data = config_data.set(
        SimpleScheduleVariable {
            group: 2,
            course: 1,
            week: 3,
        },
        0.0,
    );

    let computed_partial_solution = problem.configuration_to_partial_solution(&config_data);

    assert_eq!(partial_solution, computed_partial_solution);
}
