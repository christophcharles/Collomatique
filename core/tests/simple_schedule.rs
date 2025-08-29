#[test]
fn no_solution_because_too_many_groups() {
    use collomatique_core::{
        examples::simple_schedule::{SimpleScheduleConstraints, SimpleScheduleDesc},
        ProblemBuilder,
    };

    let problem_desc = SimpleScheduleDesc {
        group_count: 3,
        week_count: 2,
        course_count: 1,
    };

    let constraints = SimpleScheduleConstraints {};

    let mut problem_builder =
        ProblemBuilder::<_, _, _>::new(problem_desc).expect("Consistent ILP description");
    let _translator = problem_builder
        .add_constraints(constraints, 1.0)
        .expect("Consistent ILP description");
    let problem = problem_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    let solution = problem.solve(&solver);

    assert!(solution.is_none());
}

#[test]
fn no_solution_because_too_many_courses() {
    use collomatique_core::{
        examples::simple_schedule::{SimpleScheduleConstraints, SimpleScheduleDesc},
        ProblemBuilder,
    };

    let problem_desc = SimpleScheduleDesc {
        group_count: 1,
        week_count: 2,
        course_count: 3,
    };

    let constraints = SimpleScheduleConstraints {};

    let mut problem_builder =
        ProblemBuilder::<_, _, _>::new(problem_desc).expect("Consistent ILP description");
    let _translator = problem_builder
        .add_constraints(constraints, 1.0)
        .expect("Consistent ILP description");
    let problem = problem_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    let solution = problem.solve(&solver);

    assert!(solution.is_none());
}

#[test]
fn solution_with_only_one_group() {
    use collomatique_core::{
        examples::simple_schedule::{SimpleScheduleConstraints, SimpleScheduleDesc},
        ProblemBuilder,
    };
    use std::collections::BTreeSet;

    let problem_desc = SimpleScheduleDesc {
        group_count: 1,
        week_count: 3,
        course_count: 3,
    };

    let constraints = SimpleScheduleConstraints {};

    let mut problem_builder =
        ProblemBuilder::<_, _, _>::new(problem_desc).expect("Consistent ILP description");
    let _translator = problem_builder
        .add_constraints(constraints, 1.0)
        .expect("Consistent ILP description");
    let problem = problem_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    let solution = problem
        .solve(&solver)
        .expect("There should be a solution")
        .into_solution();

    // Solution should be complete
    assert!(solution.is_complete());
    // There should be less than one assignment per cell as the constraints are satisfied
    for week in 0..3 {
        assert!(solution.get_assigned(0, week).unwrap().len() <= 1);
    }

    // Let's check all courses are followed and are followed only once
    let mut attended_courses = BTreeSet::new();
    for week in 0..3 {
        let course_opt = solution.get_assigned(0, week).unwrap().first().copied();

        if let Some(course) = course_opt {
            assert!(!attended_courses.contains(&course));
            attended_courses.insert(course);
        }
    }

    assert_eq!(attended_courses, BTreeSet::from_iter(0..3));
}

#[test]
fn solution_with_only_one_group_and_way_too_many_weeks() {
    use collomatique_core::{
        examples::simple_schedule::{SimpleScheduleConstraints, SimpleScheduleDesc},
        ProblemBuilder,
    };
    use std::collections::BTreeSet;

    let week_count = 100;
    let problem_desc = SimpleScheduleDesc {
        group_count: 1,
        week_count,
        course_count: 3,
    };

    let constraints = SimpleScheduleConstraints {};

    let mut problem_builder =
        ProblemBuilder::<_, _, _>::new(problem_desc).expect("Consistent ILP description");
    let _translator = problem_builder
        .add_constraints(constraints, 1.0)
        .expect("Consistent ILP description");
    let problem = problem_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    let solution = problem
        .solve(&solver)
        .expect("There should be a solution")
        .into_solution();

    // Solution should be complete
    assert!(solution.is_complete());
    // There should be less than one assignment per cell as the constraints are satisfied
    for week in 0..week_count {
        assert!(solution.get_assigned(0, week).unwrap().len() <= 1);
    }

    // Let's check all courses are followed and are followed only once
    let mut attended_courses = BTreeSet::new();
    for week in 0..week_count {
        let course_opt = solution.get_assigned(0, week).unwrap().first().copied();

        if let Some(course) = course_opt {
            assert!(!attended_courses.contains(&course));
            attended_courses.insert(course);
        }
    }

    assert_eq!(attended_courses, BTreeSet::from_iter(0..3));
}

#[test]
fn solution_with_two_groups() {
    use collomatique_core::{
        examples::simple_schedule::{SimpleScheduleConstraints, SimpleScheduleDesc},
        ProblemBuilder,
    };
    use std::collections::BTreeSet;

    let problem_desc = SimpleScheduleDesc {
        group_count: 2,
        week_count: 3,
        course_count: 3,
    };

    let constraints = SimpleScheduleConstraints {};

    let mut problem_builder =
        ProblemBuilder::<_, _, _>::new(problem_desc).expect("Consistent ILP description");
    let _translator = problem_builder
        .add_constraints(constraints, 1.0)
        .expect("Consistent ILP description");
    let problem = problem_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    let solution = problem
        .solve(&solver)
        .expect("There should be a solution")
        .into_solution();

    // Solution should be complete
    assert!(solution.is_complete());
    // There should be less than one assignment per cell as the constraints are satisfied
    for group in 0..2 {
        for week in 0..3 {
            assert!(solution.get_assigned(group, week).unwrap().len() <= 1);
        }
    }

    // Let's check all courses are followed and are followed only once
    // Let's also check the two groups do not attend the same course on the same week
    let mut attended_courses = vec![BTreeSet::new(); 2];
    for week in 0..3 {
        let course0_opt = solution.get_assigned(0, week).unwrap().first().copied();
        let course1_opt = solution.get_assigned(1, week).unwrap().first().copied();

        match (course0_opt, course1_opt) {
            (Some(course0), Some(course1)) => {
                assert!(!attended_courses[0].contains(&course0));
                assert!(!attended_courses[1].contains(&course1));

                assert!(course0 != course1);

                attended_courses[0].insert(course0);
                attended_courses[1].insert(course1);
            }
            (Some(course0), None) => {
                assert!(!attended_courses[0].contains(&course0));
                attended_courses[0].insert(course0);
            }
            (None, Some(course1)) => {
                assert!(!attended_courses[1].contains(&course1));
                attended_courses[1].insert(course1);
            }
            (None, None) => {}
        }
    }

    assert_eq!(attended_courses[0], BTreeSet::from_iter(0..3));
    assert_eq!(attended_courses[1], BTreeSet::from_iter(0..3));
}

#[test]
fn solution_with_two_groups_and_way_too_many_weeks() {
    use collomatique_core::{
        examples::simple_schedule::{SimpleScheduleConstraints, SimpleScheduleDesc},
        ProblemBuilder,
    };
    use std::collections::BTreeSet;

    let week_count = 100;
    let problem_desc = SimpleScheduleDesc {
        group_count: 2,
        week_count,
        course_count: 3,
    };

    let constraints = SimpleScheduleConstraints {};

    let mut problem_builder =
        ProblemBuilder::<_, _, _>::new(problem_desc).expect("Consistent ILP description");
    let _translator = problem_builder
        .add_constraints(constraints, 1.0)
        .expect("Consistent ILP description");
    let problem = problem_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    let solution = problem
        .solve(&solver)
        .expect("There should be a solution")
        .into_solution();

    // Solution should be complete
    assert!(solution.is_complete());
    // There should be less than one assignment per cell as the constraints are satisfied
    for week in 0..week_count {
        assert!(solution.get_assigned(0, week).unwrap().len() <= 1);
    }

    // Let's check all courses are followed and are followed only once
    // Let's also check the two groups do not attend the same course on the same week
    let mut attended_courses = vec![BTreeSet::new(); 2];
    for week in 0..week_count {
        let course0_opt = solution.get_assigned(0, week).unwrap().first().copied();
        let course1_opt = solution.get_assigned(1, week).unwrap().first().copied();

        match (course0_opt, course1_opt) {
            (Some(course0), Some(course1)) => {
                assert!(!attended_courses[0].contains(&course0));
                assert!(!attended_courses[1].contains(&course1));

                assert!(course0 != course1);

                attended_courses[0].insert(course0);
                attended_courses[1].insert(course1);
            }
            (Some(course0), None) => {
                assert!(!attended_courses[0].contains(&course0));
                attended_courses[0].insert(course0);
            }
            (None, Some(course1)) => {
                assert!(!attended_courses[1].contains(&course1));
                attended_courses[1].insert(course1);
            }
            (None, None) => {}
        }
    }

    assert_eq!(attended_courses[0], BTreeSet::from_iter(0..3));
    assert_eq!(attended_courses[1], BTreeSet::from_iter(0..3));
}
