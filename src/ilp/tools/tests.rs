use super::*;
#[test]
fn test_mat_repr() {
    use crate::ilp::linexpr::Expr;

    let pb = crate::ilp::ProblemBuilder::new()
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
        .build();

    let mat_repr = MatRepr::new(&pb);

    use ndarray::array;

    assert_eq!(
        mat_repr.leq_mat,
        array![[0, -3, 4, 5, 0], [-3, 1, 3, 5, 0],]
    );
    assert_eq!(mat_repr.eq_mat, array![[0, 0, 1, -3, 5],]);

    assert_eq!(mat_repr.leq_constants, array![-3, 3]);
    assert_eq!(mat_repr.eq_constants, array![2]);
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
        .build();

    let mat_repr = MatRepr::new(&pb);

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

    let cfg_repr_0 = mat_repr.config(&config_0);
    let cfg_repr_1 = mat_repr.config(&config_1);
    let cfg_repr_2 = mat_repr.config(&config_2);
    let cfg_repr_3 = mat_repr.config(&config_3);
    let cfg_repr_4 = mat_repr.config(&config_4);
    let cfg_repr_5 = mat_repr.config(&config_5);
    let cfg_repr_6 = mat_repr.config(&config_6);
    let cfg_repr_7 = mat_repr.config(&config_7);
    let cfg_repr_8 = mat_repr.config(&config_8);
    let cfg_repr_9 = mat_repr.config(&config_9);
    let cfg_repr_a = mat_repr.config(&config_a);
    let cfg_repr_b = mat_repr.config(&config_b);
    let cfg_repr_c = mat_repr.config(&config_c);
    let cfg_repr_d = mat_repr.config(&config_d);
    let cfg_repr_e = mat_repr.config(&config_e);
    let cfg_repr_f = mat_repr.config(&config_f);

    assert_eq!(cfg_repr_0.is_feasable(), false);
    assert_eq!(cfg_repr_1.is_feasable(), true);
    assert_eq!(cfg_repr_2.is_feasable(), false);
    assert_eq!(cfg_repr_3.is_feasable(), false);
    assert_eq!(cfg_repr_4.is_feasable(), false);
    assert_eq!(cfg_repr_5.is_feasable(), true);
    assert_eq!(cfg_repr_6.is_feasable(), false);
    assert_eq!(cfg_repr_7.is_feasable(), false);
    assert_eq!(cfg_repr_8.is_feasable(), true);
    assert_eq!(cfg_repr_9.is_feasable(), false);
    assert_eq!(cfg_repr_a.is_feasable(), true);
    assert_eq!(cfg_repr_b.is_feasable(), false);
    assert_eq!(cfg_repr_c.is_feasable(), false);
    assert_eq!(cfg_repr_d.is_feasable(), false);
    assert_eq!(cfg_repr_e.is_feasable(), false);
    assert_eq!(cfg_repr_f.is_feasable(), false);
}
