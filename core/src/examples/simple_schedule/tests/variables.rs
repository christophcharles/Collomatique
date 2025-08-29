use super::*;

#[test]
fn test_main_variables() {
    let example_problem = SimpleScheduleDesc {
        course_count: 3,
        week_count: 4,
        group_count: 2,
    };

    use BaseConstraints;

    let variables = example_problem.main_variables();

    let expected_variables = BTreeMap::from([
        (
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 0,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 1,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 2,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 0,
                week: 3,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 0,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 1,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 2,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 1,
                week: 3,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 0,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 1,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 2,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 0,
                course: 2,
                week: 3,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 0,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 1,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 2,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 0,
                week: 3,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 1,
                week: 0,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 1,
                week: 1,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 1,
                week: 2,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 1,
                week: 3,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 0,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 1,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 2,
            },
            Variable::binary(),
        ),
        (
            SimpleScheduleVariable {
                group: 1,
                course: 2,
                week: 3,
            },
            Variable::binary(),
        ),
    ]);

    assert_eq!(variables, expected_variables);
}
