use super::*;

#[test]
fn test_config_from_iterator() {
    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["x", "y", "z", "t"])
        .unwrap()
        .build();

    let config = pb.config_from(["x", "y", "z"]).unwrap();

    assert_eq!(config.get("x"), Ok(true));
    assert_eq!(config.get("y"), Ok(true));
    assert_eq!(config.get("z"), Ok(true));
    assert_eq!(config.get("t"), Ok(false));

    assert_eq!(
        config.get("w"),
        Err(Error::InvalidVariable(String::from("w")))
    );
}

#[test]
fn invalid_variable_in_config() {
    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["x", "y", "z", "t"])
        .unwrap()
        .build();

    let config = pb.config_from(["x", "y", "w"]);

    assert_eq!(
        config.err(),
        Some(Error::InvalidVariable(String::from("w")))
    );
}

#[test]
fn test_is_feasable() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variable("a")
        .unwrap()
        .add_variable("b")
        .unwrap()
        .add_variable("c")
        .unwrap()
        .add_variable("d")
        .unwrap()
        .add_constraint((&a + &b).leq(&Expr::constant(1)))
        .unwrap()
        .add_constraint((&c + &d).leq(&Expr::constant(1)))
        .unwrap()
        .add_constraint((&a + &d).eq(&Expr::constant(1)))
        .unwrap()
        .build();

    let config_0 = pb.default_config();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["b"]).unwrap();
    let config_3 = pb.config_from(["a", "b"]).unwrap();
    let config_4 = pb.config_from(["c"]).unwrap();
    let config_5 = pb.config_from(["a", "c"]).unwrap();
    let config_6 = pb.config_from(["b", "c"]).unwrap();
    let config_7 = pb.config_from(["a", "b", "c"]).unwrap();
    let config_8 = pb.config_from(["d"]).unwrap();
    let config_9 = pb.config_from(["a", "d"]).unwrap();
    let config_a = pb.config_from(["b", "d"]).unwrap();
    let config_b = pb.config_from(["a", "b", "d"]).unwrap();
    let config_c = pb.config_from(["c", "d"]).unwrap();
    let config_d = pb.config_from(["a", "c", "d"]).unwrap();
    let config_e = pb.config_from(["b", "c", "d"]).unwrap();
    let config_f = pb.config_from(["a", "b", "c", "d"]).unwrap();

    assert_eq!(config_0.is_feasable(), false);
    assert_eq!(config_1.is_feasable(), true);
    assert_eq!(config_2.is_feasable(), false);
    assert_eq!(config_3.is_feasable(), false);
    assert_eq!(config_4.is_feasable(), false);
    assert_eq!(config_5.is_feasable(), true);
    assert_eq!(config_6.is_feasable(), false);
    assert_eq!(config_7.is_feasable(), false);
    assert_eq!(config_8.is_feasable(), true);
    assert_eq!(config_9.is_feasable(), false);
    assert_eq!(config_a.is_feasable(), true);
    assert_eq!(config_b.is_feasable(), false);
    assert_eq!(config_c.is_feasable(), false);
    assert_eq!(config_d.is_feasable(), false);
    assert_eq!(config_e.is_feasable(), false);
    assert_eq!(config_f.is_feasable(), false);
}

#[test]
fn test_is_feasable_no_constraint() {
    let pb: Problem<String> = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b"])
        .unwrap()
        .build();

    let config_0 = pb.default_config();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["b"]).unwrap();
    let config_3 = pb.config_from(["a", "b"]).unwrap();

    assert_eq!(config_0.is_feasable(), true);
    assert_eq!(config_1.is_feasable(), true);
    assert_eq!(config_2.is_feasable(), true);
    assert_eq!(config_3.is_feasable(), true);
}

#[test]
fn problem_extra_variable() {
    let pb = ProblemBuilder::<String>::new()
        .add_variable("X")
        .unwrap()
        .build();

    assert_eq!(pb.variables, BTreeSet::from([String::from("X")]));
}

#[test]
fn problem_extra_variables() {
    let pb = ProblemBuilder::<String>::new()
        .add_variable("X")
        .unwrap()
        .add_variable("Y")
        .unwrap()
        .add_variables(["Z", "W"])
        .unwrap()
        .build();

    assert_eq!(
        pb.variables,
        BTreeSet::from([
            String::from("X"),
            String::from("Y"),
            String::from("Z"),
            String::from("W")
        ])
    );
}

#[test]
fn problem_undeclared_variable() {
    use crate::ilp::linexpr::Expr;

    let res = ProblemBuilder::<String>::new()
        .add_variable("X")
        .unwrap()
        .add_constraint((Expr::var("X") + Expr::var("Y")).eq(&Expr::constant(1)));

    assert_eq!(
        res.err(),
        Some(ConstraintError::UndeclaredVariable(String::from("Y")))
    );
}

#[test]
fn problem_filter_variable() {
    use crate::ilp::linexpr::Expr;

    let pb1 = ProblemBuilder::<String>::new()
        .add_variables(["T", "S", "X", "Y", "Z", "W"])
        .unwrap()
        .add_constraints([
            (Expr::var("X") + Expr::var("Y")).eq(&Expr::constant(1)),
            (Expr::var("X") + Expr::var("Z")).eq(&Expr::constant(1)),
            (Expr::var("Y") + Expr::var("Z")).leq(&Expr::constant(1)),
            (Expr::var("Y") + Expr::var("W")).leq(&Expr::constant(1)),
            (Expr::var("Z") + Expr::var("W")).eq(&Expr::constant(1)),
        ])
        .unwrap()
        .filter_variables(|v| (*v != String::from("Z")) && (*v != String::from("S")));

    let pb2 = ProblemBuilder::<String>::new()
        .add_variables(["T", "X", "Y", "W"])
        .unwrap()
        .add_constraints([
            (Expr::var("X") + Expr::var("Y")).eq(&Expr::constant(1)),
            (Expr::var("Y") + Expr::var("W")).leq(&Expr::constant(1)),
        ])
        .unwrap();

    assert_eq!(pb1.constraints, pb2.constraints);
    assert_eq!(pb1.variables, pb2.variables);
}
