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
