use super::*;

#[test]
fn sprs_problem_correctly_builds_matrices() {
    use crate::LinExpr;

    let variables = BTreeMap::from([
        (String::from("a"), Variable::binary()),
        (String::from("b"), Variable::integer()),
        (String::from("c"), Variable::binary()),
        (String::from("d"), Variable::binary()),
        (String::from("e"), Variable::continuous().min(0.0)),
    ]);

    let constraints = vec![
        (2 * LinExpr::var("a") - 3 * LinExpr::var("b") + 4 * LinExpr::var("c") - 3)
            .leq(&(2 * LinExpr::var("a") - 5 * LinExpr::var("d"))),
        (-LinExpr::var("a") + LinExpr::var("b") + 3 * LinExpr::var("c") + 3)
            .leq(&(2 * LinExpr::var("a") - 5 * LinExpr::var("d"))),
        (2 * LinExpr::var("c") - 3 * LinExpr::var("d") + 4 * LinExpr::var("e") + 2)
            .eq(&(-1 * LinExpr::var("e") + LinExpr::var("c"))),
    ];

    let pb = SprsProblem::new(&variables, constraints.iter());

    use sprs::{CsMat, CsVec};

    // Expected matrix is :
    //
    // [0., -3., 4., 5., 0.]
    // [-3., 1., 3., 5., 0.]
    // [0., 0., 1., -3., 5.]
    assert_eq!(
        pb.mat,
        CsMat::new(
            (3, 5),
            vec![0, 3, 7, 10],
            vec![1, 2, 3, 0, 1, 2, 3, 2, 3, 4],
            vec![-3., 4., 5., -3., 1., 3., 5., 1., -3., 5.]
        )
    );
    assert_eq!(
        pb.constants,
        CsVec::new(3, vec![0, 1, 2], vec![-3., 3., 2.])
    );
    assert_eq!(
        pb.constraint_symbols,
        vec![EqSymbol::LessThan, EqSymbol::LessThan, EqSymbol::Equals]
    );
    assert_eq!(
        pb.variable_map,
        BTreeMap::from([
            (String::from("a"), 0),
            (String::from("b"), 1),
            (String::from("c"), 2),
            (String::from("d"), 3),
            (String::from("e"), 4),
        ])
    );
}

#[test]
fn sprs_repr_checks_is_feasable_on_simple_example() {
    use crate::LinExpr;

    let variables = BTreeMap::from([
        (String::from("a"), Variable::binary()),
        (String::from("b"), Variable::binary()),
        (String::from("c"), Variable::binary()),
        (String::from("d"), Variable::binary()),
    ]);

    let a = LinExpr::<String>::var("a");
    let b = LinExpr::<String>::var("b");
    let c = LinExpr::<String>::var("c");
    let d = LinExpr::<String>::var("d");

    let one = LinExpr::<String>::constant(1.0);

    let constraints = vec![(&a + &b).leq(&one), (&c + &d).leq(&one), (&a + &d).eq(&one)];

    let pb = SprsProblem::new(&variables, constraints.iter());

    let config_0_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_1_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_2_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_3_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_4_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_5_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_6_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_7_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_8_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_9_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_a_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_b_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_c_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_d_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_e_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_f_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);

    let config_0 = pb.config_from(&config_0_vars);
    let config_1 = pb.config_from(&config_1_vars);
    let config_2 = pb.config_from(&config_2_vars);
    let config_3 = pb.config_from(&config_3_vars);
    let config_4 = pb.config_from(&config_4_vars);
    let config_5 = pb.config_from(&config_5_vars);
    let config_6 = pb.config_from(&config_6_vars);
    let config_7 = pb.config_from(&config_7_vars);
    let config_8 = pb.config_from(&config_8_vars);
    let config_9 = pb.config_from(&config_9_vars);
    let config_a = pb.config_from(&config_a_vars);
    let config_b = pb.config_from(&config_b_vars);
    let config_c = pb.config_from(&config_c_vars);
    let config_d = pb.config_from(&config_d_vars);
    let config_e = pb.config_from(&config_e_vars);
    let config_f = pb.config_from(&config_f_vars);

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
fn sprs_repr_checks_is_feasable_with_no_constraints() {
    let variables = BTreeMap::from([
        (String::from("a"), Variable::binary()),
        (String::from("b"), Variable::binary()),
    ]);

    let constraints = vec![];

    let pb = SprsProblem::new(&variables, constraints.iter());

    let config_0_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_1_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_2_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_3_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
    ]);

    let config_0 = pb.config_from(&config_0_vars);
    let config_1 = pb.config_from(&config_1_vars);
    let config_2 = pb.config_from(&config_2_vars);
    let config_3 = pb.config_from(&config_3_vars);

    assert_eq!(config_0.is_feasable(), true);
    assert_eq!(config_1.is_feasable(), true);
    assert_eq!(config_2.is_feasable(), true);
    assert_eq!(config_3.is_feasable(), true);
}

#[test]
fn sprs_repr_checks_unsatisfied_constraints_on_simple_example() {
    use crate::LinExpr;

    let variables = BTreeMap::from([
        (String::from("a"), Variable::binary()),
        (String::from("b"), Variable::binary()),
        (String::from("c"), Variable::binary()),
        (String::from("d"), Variable::binary()),
    ]);

    let a = LinExpr::<String>::var("a");
    let b = LinExpr::<String>::var("b");
    let c = LinExpr::<String>::var("c");
    let d = LinExpr::<String>::var("d");

    let one = LinExpr::<String>::constant(1.0);

    let constraints = vec![(&a + &b).leq(&one), (&c + &d).leq(&one), (&a + &d).eq(&one)];

    let pb = SprsProblem::new(&variables, constraints.iter());

    let config_vars = BTreeMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);

    let config = pb.config_from(&config_vars);

    assert_eq!(
        config.unsatisfied_constraints(),
        BTreeSet::from([1usize, 2usize])
    );
}
