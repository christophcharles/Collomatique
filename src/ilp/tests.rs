use super::*;

#[test]
fn test_config_from_iterator() {
    let config = Config::from_iter(["x", "y", "z"]);

    assert_eq!(
        config.variables,
        BTreeSet::from_iter([String::from("x"), String::from("y"), String::from("z"),])
    );
}

#[test]
fn test_is_feasable() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");
    let d = Expr::var("d");

    let pb = crate::ilp::ProblemBuilder::new()
        .add((&a + &b).leq(&Expr::constant(1)))
        .add((&c + &d).leq(&Expr::constant(1)))
        .add((&a + &d).eq(&Expr::constant(1)))
        .build()
        .unwrap();

    let config_0 = Config::from_iter::<[&str; 0]>([]);
    let config_1 = Config::from_iter(["a"]);
    let config_2 = Config::from_iter(["b"]);
    let config_3 = Config::from_iter(["a", "b"]);
    let config_4 = Config::from_iter(["c"]);
    let config_5 = Config::from_iter(["a", "c"]);
    let config_6 = Config::from_iter(["b", "c"]);
    let config_7 = Config::from_iter(["a", "b", "c"]);
    let config_8 = Config::from_iter(["d"]);
    let config_9 = Config::from_iter(["a", "d"]);
    let config_a = Config::from_iter(["b", "d"]);
    let config_b = Config::from_iter(["a", "b", "d"]);
    let config_c = Config::from_iter(["c", "d"]);
    let config_d = Config::from_iter(["a", "c", "d"]);
    let config_e = Config::from_iter(["b", "c", "d"]);
    let config_f = Config::from_iter(["a", "b", "c", "d"]);

    assert_eq!(pb.is_feasable(&config_0), false);
    assert_eq!(pb.is_feasable(&config_1), true);
    assert_eq!(pb.is_feasable(&config_2), false);
    assert_eq!(pb.is_feasable(&config_3), false);
    assert_eq!(pb.is_feasable(&config_4), false);
    assert_eq!(pb.is_feasable(&config_5), true);
    assert_eq!(pb.is_feasable(&config_6), false);
    assert_eq!(pb.is_feasable(&config_7), false);
    assert_eq!(pb.is_feasable(&config_8), true);
    assert_eq!(pb.is_feasable(&config_9), false);
    assert_eq!(pb.is_feasable(&config_a), true);
    assert_eq!(pb.is_feasable(&config_b), false);
    assert_eq!(pb.is_feasable(&config_c), false);
    assert_eq!(pb.is_feasable(&config_d), false);
    assert_eq!(pb.is_feasable(&config_e), false);
    assert_eq!(pb.is_feasable(&config_f), false);
}

#[test]
fn test_is_feasable_no_constraint() {
    let pb = crate::ilp::ProblemBuilder::new().build().unwrap();

    let config_0 = Config::from_iter::<[&str; 0]>([]);
    let config_1 = Config::from_iter(["a"]);
    let config_2 = Config::from_iter(["b"]);
    let config_3 = Config::from_iter(["a", "b"]);

    assert_eq!(pb.is_feasable(&config_0), true);
    assert_eq!(pb.is_feasable(&config_1), true);
    assert_eq!(pb.is_feasable(&config_2), true);
    assert_eq!(pb.is_feasable(&config_3), true);
}

#[test]
fn problem_extra_variable() {
    let pb = ProblemBuilder::new().add_variable("X").build().unwrap();

    assert_eq!(pb.variables, BTreeSet::from([String::from("X")]));
}
#[test]
fn problem_extra_variables() {
    let pb = ProblemBuilder::new()
        .add_variable("X")
        .add_variable("Y")
        .add_variables([String::from("Z"), String::from("W")])
        .build()
        .unwrap();

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
