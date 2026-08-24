use super::*;

#[test]
fn sprs_problem_correctly_builds_matrices() {
    use crate::LinExpr;

    let variables = HashMap::from([
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

    // Check dimensions
    assert_eq!(pb.mat.shape(), (3, 5));
    assert_eq!(
        pb.constraint_symbols,
        vec![EqSymbol::LessThan, EqSymbol::LessThan, EqSymbol::Equals]
    );

    // Check variable_map has correct keys
    assert_eq!(pb.variable_map.len(), 5);
    for name in ["a", "b", "c", "d", "e"] {
        assert!(pb.variable_map.contains_key(&String::from(name)));
    }

    // Check matrix coefficients using variable_map indices (order-independent)
    let idx = |name: &str| pb.variable_map[&String::from(name)];
    let mat = pb.mat.to_dense();

    // Row 0: 0a - 3b + 4c + 5d + 0e, constant -3
    assert!(f64_is_zero(mat[(0, idx("a"))]));
    assert!(f64_is_zero(mat[(0, idx("b"))] + 3.0));
    assert!(f64_is_zero(mat[(0, idx("c"))] - 4.0));
    assert!(f64_is_zero(mat[(0, idx("d"))] - 5.0));
    assert!(f64_is_zero(mat[(0, idx("e"))]));

    // Row 1: -3a + 1b + 3c + 5d + 0e, constant 3
    assert!(f64_is_zero(mat[(1, idx("a"))] + 3.0));
    assert!(f64_is_zero(mat[(1, idx("b"))] - 1.0));
    assert!(f64_is_zero(mat[(1, idx("c"))] - 3.0));
    assert!(f64_is_zero(mat[(1, idx("d"))] - 5.0));
    assert!(f64_is_zero(mat[(1, idx("e"))]));

    // Row 2: 0a + 0b + 1c - 3d + 5e, constant 2
    assert!(f64_is_zero(mat[(2, idx("a"))]));
    assert!(f64_is_zero(mat[(2, idx("b"))]));
    assert!(f64_is_zero(mat[(2, idx("c"))] - 1.0));
    assert!(f64_is_zero(mat[(2, idx("d"))] + 3.0));
    assert!(f64_is_zero(mat[(2, idx("e"))] - 5.0));

    // Check constants
    let constants = pb.constants.to_dense();
    assert!(f64_is_zero(constants[0] + 3.0));
    assert!(f64_is_zero(constants[1] - 3.0));
    assert!(f64_is_zero(constants[2] - 2.0));
}

#[test]
fn sprs_repr_checks_is_feasible_on_simple_example() {
    use crate::LinExpr;

    let variables = HashMap::from([
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

    let config_0_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_1_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_2_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_3_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_4_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_5_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_6_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_7_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_8_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_9_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_a_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_b_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(0.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_c_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_d_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_e_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_f_vars = HashMap::from([
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

    assert_eq!(config_0.is_feasible(), false);
    assert_eq!(config_1.is_feasible(), true);
    assert_eq!(config_2.is_feasible(), false);
    assert_eq!(config_3.is_feasible(), false);
    assert_eq!(config_4.is_feasible(), false);
    assert_eq!(config_5.is_feasible(), true);
    assert_eq!(config_6.is_feasible(), false);
    assert_eq!(config_7.is_feasible(), false);
    assert_eq!(config_8.is_feasible(), true);
    assert_eq!(config_9.is_feasible(), false);
    assert_eq!(config_a.is_feasible(), true);
    assert_eq!(config_b.is_feasible(), false);
    assert_eq!(config_c.is_feasible(), false);
    assert_eq!(config_d.is_feasible(), false);
    assert_eq!(config_e.is_feasible(), false);
    assert_eq!(config_f.is_feasible(), false);
}

#[test]
fn sprs_repr_checks_is_feasible_with_no_constraints() {
    let variables = HashMap::from([
        (String::from("a"), Variable::binary()),
        (String::from("b"), Variable::binary()),
    ]);

    let constraints = vec![];

    let pb = SprsProblem::new(&variables, constraints.iter());

    let config_0_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_1_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
    ]);
    let config_2_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(0.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
    ]);
    let config_3_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(1.0)),
    ]);

    let config_0 = pb.config_from(&config_0_vars);
    let config_1 = pb.config_from(&config_1_vars);
    let config_2 = pb.config_from(&config_2_vars);
    let config_3 = pb.config_from(&config_3_vars);

    assert_eq!(config_0.is_feasible(), true);
    assert_eq!(config_1.is_feasible(), true);
    assert_eq!(config_2.is_feasible(), true);
    assert_eq!(config_3.is_feasible(), true);
}

#[test]
fn sprs_repr_checks_unsatisfied_constraints_on_simple_example() {
    use crate::LinExpr;

    let variables = HashMap::from([
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

    let config_vars = HashMap::from([
        (String::from("a"), ordered_float::OrderedFloat(1.0)),
        (String::from("b"), ordered_float::OrderedFloat(0.0)),
        (String::from("c"), ordered_float::OrderedFloat(1.0)),
        (String::from("d"), ordered_float::OrderedFloat(1.0)),
    ]);

    let config = pb.config_from(&config_vars);

    assert_eq!(config.unsatisfied_constraints(), vec![1usize, 2usize]);
}
