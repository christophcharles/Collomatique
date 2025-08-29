use super::*;
#[test]
fn test_mat_repr() {
    use crate::ilp::linexpr::Expr;

    let pb = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b", "c", "d", "e"])
        .add(
            (2 * Expr::var("a") - 3 * Expr::var("b") + 4 * Expr::var("c") - 3)
                .leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))),
        )
        .add(
            (-Expr::var("a") + Expr::var("b") + 3 * Expr::var("c") + 3)
                .leq(&(2 * Expr::var("a") - 5 * Expr::var("d"))),
        )
        .add(
            (2 * Expr::var("c") - 3 * Expr::var("d") + 4 * Expr::var("e") + 2)
                .eq(&(-1 * Expr::var("e") + Expr::var("c"))),
        )
        .build()
        .unwrap();

    use ndarray::array;

    assert_eq!(
        pb.mat_repr.leq_mat,
        array![[0, -3, 4, 5, 0], [-3, 1, 3, 5, 0],]
    );
    assert_eq!(pb.mat_repr.eq_mat, array![[0, 0, 1, -3, 5],]);

    assert_eq!(pb.mat_repr.leq_constants, array![-3, 3]);
    assert_eq!(pb.mat_repr.eq_constants, array![2]);
}

#[test]
fn test_is_feasable() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");
    let d = Expr::var("d");

    let pb = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b", "c", "d"])
        .add((&a + &b).leq(&Expr::constant(1)))
        .add((&c + &d).leq(&Expr::constant(1)))
        .add((&a + &d).eq(&Expr::constant(1)))
        .build()
        .unwrap();

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

    let mat_repr = &pb.mat_repr;

    assert_eq!(config_0.repr.is_feasable(mat_repr), false);
    assert_eq!(config_1.repr.is_feasable(mat_repr), true);
    assert_eq!(config_2.repr.is_feasable(mat_repr), false);
    assert_eq!(config_3.repr.is_feasable(mat_repr), false);
    assert_eq!(config_4.repr.is_feasable(mat_repr), false);
    assert_eq!(config_5.repr.is_feasable(mat_repr), true);
    assert_eq!(config_6.repr.is_feasable(mat_repr), false);
    assert_eq!(config_7.repr.is_feasable(mat_repr), false);
    assert_eq!(config_8.repr.is_feasable(mat_repr), true);
    assert_eq!(config_9.repr.is_feasable(mat_repr), false);
    assert_eq!(config_a.repr.is_feasable(mat_repr), true);
    assert_eq!(config_b.repr.is_feasable(mat_repr), false);
    assert_eq!(config_c.repr.is_feasable(mat_repr), false);
    assert_eq!(config_d.repr.is_feasable(mat_repr), false);
    assert_eq!(config_e.repr.is_feasable(mat_repr), false);
    assert_eq!(config_f.repr.is_feasable(mat_repr), false);
}

#[test]
fn test_is_feasable_no_constraint() {
    let pb = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b"])
        .build()
        .unwrap();

    let config_0 = pb.default_config();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["b"]).unwrap();
    let config_3 = pb.config_from(["a", "b"]).unwrap();

    assert_eq!(config_0.repr.is_feasable(&pb.mat_repr), true);
    assert_eq!(config_1.repr.is_feasable(&pb.mat_repr), true);
    assert_eq!(config_2.repr.is_feasable(&pb.mat_repr), true);
    assert_eq!(config_3.repr.is_feasable(&pb.mat_repr), true);
}

#[test]
fn test_neighbours() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");
    let d = Expr::var("d");

    let pb = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b", "c", "d"])
        .add((&a + &b).leq(&Expr::constant(1)))
        .add((&c + &d).leq(&Expr::constant(1)))
        .add((&a + &d).eq(&Expr::constant(1)))
        .build()
        .unwrap();

    let config = pb.config_from(["a", "b"]).unwrap();

    let cfg_repr = config.repr.clone();

    let neighbours = cfg_repr
        .neighbours()
        .into_iter()
        .collect::<BTreeSet<ConfigRepr>>();

    let config_0 = pb.config_from(["b"]).unwrap();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["a", "b", "c"]).unwrap();
    let config_3 = pb.config_from(["a", "b", "d"]).unwrap();

    let cfg_repr_0 = config_0.repr.clone();
    let cfg_repr_1 = config_1.repr.clone();
    let cfg_repr_2 = config_2.repr.clone();
    let cfg_repr_3 = config_3.repr.clone();

    let neighbours_expected = BTreeSet::from_iter([cfg_repr_0, cfg_repr_1, cfg_repr_2, cfg_repr_3]);

    assert_eq!(neighbours, neighbours_expected);
}

#[test]
fn test_cfg_repr_ord() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");

    let pb = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b", "c"])
        .add((&a + &b).leq(&Expr::constant(1)))
        .add((&c + &b).leq(&Expr::constant(1)))
        .build()
        .unwrap();

    let config_0 = pb.default_config();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["b"]).unwrap();
    let config_3 = pb.config_from(["a", "b"]).unwrap();
    let config_4 = pb.config_from(["c"]).unwrap();
    let config_5 = pb.config_from(["a", "c"]).unwrap();
    let config_6 = pb.config_from(["b", "c"]).unwrap();
    let config_7 = pb.config_from(["a", "b", "c"]).unwrap();

    let cfg_repr_0 = config_0.repr.clone();
    let cfg_repr_1 = config_1.repr.clone();
    let cfg_repr_2 = config_2.repr.clone();
    let cfg_repr_3 = config_3.repr.clone();
    let cfg_repr_4 = config_4.repr.clone();
    let cfg_repr_5 = config_5.repr.clone();
    let cfg_repr_6 = config_6.repr.clone();
    let cfg_repr_7 = config_7.repr.clone();

    assert_eq!(cfg_repr_0.cmp(&cfg_repr_0), std::cmp::Ordering::Equal);
    assert!(cfg_repr_0 < cfg_repr_1);
    assert!(cfg_repr_0 < cfg_repr_2);
    assert!(cfg_repr_0 < cfg_repr_3);
    assert!(cfg_repr_0 < cfg_repr_4);
    assert!(cfg_repr_0 < cfg_repr_5);
    assert!(cfg_repr_0 < cfg_repr_6);
    assert!(cfg_repr_0 < cfg_repr_7);

    assert_eq!(cfg_repr_1.cmp(&cfg_repr_1), std::cmp::Ordering::Equal);
    assert!(cfg_repr_1 > cfg_repr_2);
    assert!(cfg_repr_1 < cfg_repr_3);
    assert!(cfg_repr_1 > cfg_repr_4);
    assert!(cfg_repr_1 < cfg_repr_5);
    assert!(cfg_repr_1 > cfg_repr_6);
    assert!(cfg_repr_1 < cfg_repr_7);

    assert_eq!(cfg_repr_2.cmp(&cfg_repr_2), std::cmp::Ordering::Equal);
    assert!(cfg_repr_2 < cfg_repr_3);
    assert!(cfg_repr_2 > cfg_repr_4);
    assert!(cfg_repr_2 < cfg_repr_5);
    assert!(cfg_repr_2 < cfg_repr_6);
    assert!(cfg_repr_2 < cfg_repr_7);

    assert_eq!(cfg_repr_3.cmp(&cfg_repr_3), std::cmp::Ordering::Equal);
    assert!(cfg_repr_3 > cfg_repr_4);
    assert!(cfg_repr_3 > cfg_repr_5);
    assert!(cfg_repr_3 > cfg_repr_6);
    assert!(cfg_repr_3 < cfg_repr_7);

    assert_eq!(cfg_repr_4.cmp(&cfg_repr_4), std::cmp::Ordering::Equal);
    assert!(cfg_repr_4 < cfg_repr_5);
    assert!(cfg_repr_4 < cfg_repr_6);
    assert!(cfg_repr_4 < cfg_repr_7);

    assert_eq!(cfg_repr_5.cmp(&cfg_repr_5), std::cmp::Ordering::Equal);
    assert!(cfg_repr_5 > cfg_repr_6);
    assert!(cfg_repr_5 < cfg_repr_7);

    assert_eq!(cfg_repr_6.cmp(&cfg_repr_6), std::cmp::Ordering::Equal);
    assert!(cfg_repr_6 < cfg_repr_7);

    assert_eq!(cfg_repr_7.cmp(&cfg_repr_7), std::cmp::Ordering::Equal);
}
