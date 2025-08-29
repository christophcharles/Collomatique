use super::*;
#[test]
fn nd_problem_definition() {
    use crate::ilp::linexpr::Expr;

    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["a", "b", "c", "d", "e"])
        .unwrap()
        .add_constraint(
            (2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c") - 3)
                .leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))),
        )
        .unwrap()
        .add_constraint(
            (-Expr::var("a") + Expr::var("b") + 3 * Expr::var("c") + 3)
                .leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))),
        )
        .unwrap()
        .add_constraint(
            (2 * Expr::var("c") - 3 * Expr::var("d") + 4 * Expr::var("e") + 2)
                .eq(&(-1 * Expr::var("e") + Expr::var("c"))),
        )
        .unwrap()
        .build();

    use ndarray::array;

    assert_eq!(
        pb.pb_repr.leq_mat,
        array![[-3, 1, 3, 5, 0], [0, -3, 4, 5, 0]] // We must follow lexicographical order because of BTreeSet
    );
    assert_eq!(pb.pb_repr.eq_mat, array![[0, 0, 1, -3, 5]]);

    assert_eq!(pb.pb_repr.leq_constants, array![3, -3]);
    assert_eq!(pb.pb_repr.eq_constants, array![2]);
}

#[test]
fn test_is_feasable() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["a", "b", "c", "d"])
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

    let nd_problem = &pb.pb_repr;

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(config_0.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_1.cfg_repr.is_feasable(nd_problem), true);
    assert_eq!(config_2.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_3.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_4.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_5.cfg_repr.is_feasable(nd_problem), true);
    assert_eq!(config_6.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_7.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_8.cfg_repr.is_feasable(nd_problem), true);
    assert_eq!(config_9.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_a.cfg_repr.is_feasable(nd_problem), true);
    assert_eq!(config_b.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_c.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_d.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_e.cfg_repr.is_feasable(nd_problem), false);
    assert_eq!(config_f.cfg_repr.is_feasable(nd_problem), false);
}

#[test]
fn test_is_feasable_no_constraint() {
    use crate::ilp::Problem;

    let pb: Problem<String> = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b"])
        .unwrap()
        .build();

    let config_0 = pb.default_config();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["b"]).unwrap();
    let config_3 = pb.config_from(["a", "b"]).unwrap();

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(config_0.cfg_repr.is_feasable(&pb.pb_repr), true);
    assert_eq!(config_1.cfg_repr.is_feasable(&pb.pb_repr), true);
    assert_eq!(config_2.cfg_repr.is_feasable(&pb.pb_repr), true);
    assert_eq!(config_3.cfg_repr.is_feasable(&pb.pb_repr), true);
}

#[test]
fn test_neighbours() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["a", "b", "c", "d"])
        .unwrap()
        .add_constraint((&a + &b).leq(&Expr::constant(1)))
        .unwrap()
        .add_constraint((&c + &d).leq(&Expr::constant(1)))
        .unwrap()
        .add_constraint((&a + &d).eq(&Expr::constant(1)))
        .unwrap()
        .build();

    let config = pb.config_from(["a", "b"]).unwrap();

    let nd_config = config.cfg_repr.clone();

    use crate::ilp::mat_repr::ConfigRepr;
    use std::collections::BTreeSet;
    let neighbours = nd_config
        .neighbours()
        .into_iter()
        .collect::<BTreeSet<NdConfig<String>>>();

    let config_0 = pb.config_from(["b"]).unwrap();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["a", "b", "c"]).unwrap();
    let config_3 = pb.config_from(["a", "b", "d"]).unwrap();

    let nd_config_0 = config_0.cfg_repr.clone();
    let nd_config_1 = config_1.cfg_repr.clone();
    let nd_config_2 = config_2.cfg_repr.clone();
    let nd_config_3 = config_3.cfg_repr.clone();

    let neighbours_expected =
        BTreeSet::from_iter([nd_config_0, nd_config_1, nd_config_2, nd_config_3]);

    assert_eq!(neighbours, neighbours_expected);
}

#[test]
fn nd_config_ord() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");

    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["a", "b", "c"])
        .unwrap()
        .add_constraint((&a + &b).leq(&Expr::constant(1)))
        .unwrap()
        .add_constraint((&c + &b).leq(&Expr::constant(1)))
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

    let nd_config_0 = config_0.cfg_repr.clone();
    let nd_config_1 = config_1.cfg_repr.clone();
    let nd_config_2 = config_2.cfg_repr.clone();
    let nd_config_3 = config_3.cfg_repr.clone();
    let nd_config_4 = config_4.cfg_repr.clone();
    let nd_config_5 = config_5.cfg_repr.clone();
    let nd_config_6 = config_6.cfg_repr.clone();
    let nd_config_7 = config_7.cfg_repr.clone();

    assert_eq!(nd_config_0.cmp(&nd_config_0), std::cmp::Ordering::Equal);
    assert!(nd_config_0 < nd_config_1);
    assert!(nd_config_0 < nd_config_2);
    assert!(nd_config_0 < nd_config_3);
    assert!(nd_config_0 < nd_config_4);
    assert!(nd_config_0 < nd_config_5);
    assert!(nd_config_0 < nd_config_6);
    assert!(nd_config_0 < nd_config_7);

    assert_eq!(nd_config_1.cmp(&nd_config_1), std::cmp::Ordering::Equal);
    assert!(nd_config_1 > nd_config_2);
    assert!(nd_config_1 < nd_config_3);
    assert!(nd_config_1 > nd_config_4);
    assert!(nd_config_1 < nd_config_5);
    assert!(nd_config_1 > nd_config_6);
    assert!(nd_config_1 < nd_config_7);

    assert_eq!(nd_config_2.cmp(&nd_config_2), std::cmp::Ordering::Equal);
    assert!(nd_config_2 < nd_config_3);
    assert!(nd_config_2 > nd_config_4);
    assert!(nd_config_2 < nd_config_5);
    assert!(nd_config_2 < nd_config_6);
    assert!(nd_config_2 < nd_config_7);

    assert_eq!(nd_config_3.cmp(&nd_config_3), std::cmp::Ordering::Equal);
    assert!(nd_config_3 > nd_config_4);
    assert!(nd_config_3 > nd_config_5);
    assert!(nd_config_3 > nd_config_6);
    assert!(nd_config_3 < nd_config_7);

    assert_eq!(nd_config_4.cmp(&nd_config_4), std::cmp::Ordering::Equal);
    assert!(nd_config_4 < nd_config_5);
    assert!(nd_config_4 < nd_config_6);
    assert!(nd_config_4 < nd_config_7);

    assert_eq!(nd_config_5.cmp(&nd_config_5), std::cmp::Ordering::Equal);
    assert!(nd_config_5 > nd_config_6);
    assert!(nd_config_5 < nd_config_7);

    assert_eq!(nd_config_6.cmp(&nd_config_6), std::cmp::Ordering::Equal);
    assert!(nd_config_6 < nd_config_7);

    assert_eq!(nd_config_7.cmp(&nd_config_7), std::cmp::Ordering::Equal);
}

#[test]
fn compute_lhs() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String>::new()
        .add_variables(["a", "b", "c", "d"])
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

    let nd_problem = &pb.pb_repr;

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(
        config_0.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_1.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_2.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_3.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_4.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_5.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_6.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_7.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_8.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_9.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_a.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_b.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_c.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_d.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_e.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_f.cfg_repr.compute_lhs(nd_problem),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
}
