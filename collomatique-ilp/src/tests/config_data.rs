use super::*;

fn build_test_problem() -> Problem<String, String> {
    let x11 = LinExpr::<String>::var("x11"); // Group X has course 1 on week 1
    let x12 = LinExpr::<String>::var("x12"); // Group X has course 1 on week 2
    let x21 = LinExpr::<String>::var("x21"); // Group X has course 2 on week 1
    let x22 = LinExpr::<String>::var("x22"); // Group X has course 2 on week 2
    let y11 = LinExpr::<String>::var("y11"); // Group Y has course 1 on week 1
    let y12 = LinExpr::<String>::var("y12"); // Group Y has course 1 on week 2
    let y21 = LinExpr::<String>::var("y21"); // Group Y has course 2 on week 1
    let y22 = LinExpr::<String>::var("y22"); // Group Y has course 2 on week 2
    let one = LinExpr::<String>::constant(1.0); // Constant for easier writing of constraints
    let pb = ProblemBuilder::<String, String>::new()
        .set_variables([
            ("x11", Variable::binary()),
            ("x12", Variable::binary()),
            ("x21", Variable::binary()),
            ("x22", Variable::binary()),
        ])
        .set_variables([
            ("y11", Variable::binary()),
            ("y12", Variable::binary()),
            ("y21", Variable::binary()),
            ("y22", Variable::binary()),
        ])
        // Both class should not attend a course at the same time
        .add_constraints([
            (
                (&x11 + &y11).leq(&one),
                "At most one group in course 1 on week 1",
            ),
            (
                (&x12 + &y12).leq(&one),
                "At most one group in course 1 on week 2",
            ),
            (
                (&x21 + &y21).leq(&one),
                "At most one group in course 2 on week 1",
            ),
            (
                (&x22 + &y22).leq(&one),
                "At most one group in course 2 on week 2",
            ),
        ])
        // Each class should not attend more than one course at a given time
        .add_constraints([
            (
                (&x11 + &x21).leq(&one),
                "At most one course for group X on week 1",
            ),
            (
                (&x12 + &x22).leq(&one),
                "At most one course for group X on week 2",
            ),
            (
                (&y11 + &y21).leq(&one),
                "At most one course for group Y on week 1",
            ),
            (
                (&y12 + &y22).leq(&one),
                "At most one course for group Y on week 2",
            ),
        ])
        // Each class must complete each course exactly once
        .add_constraints([
            (
                (&x11 + &x12).eq(&one),
                "Group X should have course 1 exactly once",
            ),
            (
                (&x21 + &x22).eq(&one),
                "Group X should have course 2 exactly once",
            ),
            (
                (&y11 + &y12).eq(&one),
                "Group Y should have course 1 exactly once",
            ),
            (
                (&y21 + &y22).eq(&one),
                "Group Y should have course 2 exactly once",
            ),
        ])
        // Objective function : prefer group X in course 1 on week 1
        .set_objective_function(x11.clone(), ObjectiveSense::Maximize)
        .build()
        .unwrap();

    pb
}

#[test]
fn check_config_data_var_detects_missing_variables() {
    let pb = build_test_problem();

    let config_data = ConfigData::new()
        .set("x11", 0.0)
        .set("x12", 0.0)
        .set_iter([("y12", 1.0), ("y22", 0.0)]);

    let report = pb.check_config_data_variables(&config_data);

    assert_eq!(
        report.missing_variables,
        BTreeSet::from([
            String::from("x21"),
            String::from("x22"),
            String::from("y11"),
            String::from("y21"),
        ])
    );
}

#[test]
fn check_config_data_var_detects_excess_variables() {
    let pb = build_test_problem();

    let config_data = ConfigData::new()
        .set("x11", 0.0)
        .set("x12", 0.0)
        .set("x21", 0.0)
        .set("x22", 1.0)
        .set_iter([("y11", 1.0), ("y12", 0.0), ("y21", 1.0), ("y22", 0.0)])
        .set("z", 0.0)
        .set("w", 0.5)
        .set("t", 1.0);

    let report = pb.check_config_data_variables(&config_data);

    assert_eq!(
        report.excess_variables,
        BTreeSet::from([String::from("z"), String::from("w"), String::from("t")])
    );
}

#[test]
fn check_config_data_var_detects_non_conforming_variables() {
    let pb = build_test_problem();

    let config_data = ConfigData::new()
        .set("x11", 0.0)
        .set("x12", 2.0)
        .set("x21", 0.3)
        .set("x22", 1.0)
        .set_iter([("y11", 1.0), ("y12", 0.5), ("y21", 1.0), ("y22", 0.0)]);

    let report = pb.check_config_data_variables(&config_data);

    assert_eq!(
        report.non_conforming_variables,
        BTreeSet::from([String::from("x21"), String::from("y12")])
    );
}

#[test]
fn check_config_data_var_works_on_valid_config() {
    let pb = build_test_problem();

    let config_data = ConfigData::new()
        .set("x11", 0.0)
        .set("x12", 0.0)
        .set("x21", 0.0)
        .set("x22", 1.0)
        .set_iter([("y11", 1.0), ("y12", 0.0), ("y21", 1.0), ("y22", 0.0)]);

    let report = pb.check_config_data_variables(&config_data);

    assert!(report.is_empty());
}

#[test]
fn build_config_works_on_valid_config() {
    let pb = build_test_problem();

    let config_data = ConfigData::new()
        .set("x11", 0.0)
        .set("x12", 0.0)
        .set("x21", 0.0)
        .set("x22", 1.0)
        .set_iter([("y11", 1.0), ("y12", 0.0), ("y21", 1.0), ("y22", 0.0)]);

    let config = pb
        .build_config(config_data)
        .expect("Config should be valid");

    assert_eq!(
        config.get_values(),
        BTreeMap::from([
            (String::from("x11"), 0.0),
            (String::from("x12"), 0.0),
            (String::from("x21"), 0.0),
            (String::from("x22"), 1.0),
            (String::from("y11"), 1.0),
            (String::from("y12"), 0.0),
            (String::from("y21"), 1.0),
            (String::from("y22"), 0.0),
        ])
    )
}
