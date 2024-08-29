use super::*;
#[test]
fn sprs_problem_definition() {
    use crate::ilp::linexpr::Expr;

    let pb = crate::ilp::ProblemBuilder::<String, SprsProblem<_>>::new()
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

    let mut trimat = sprs::TriMat::new((2, 5));
    trimat.add_triplet(0, 0, -3);
    trimat.add_triplet(0, 1, 1);
    trimat.add_triplet(0, 2, 3);
    trimat.add_triplet(0, 3, 5);

    trimat.add_triplet(1, 1, -3);
    trimat.add_triplet(1, 2, 4);
    trimat.add_triplet(1, 3, 5);

    assert_eq!(
        pb.pb_repr.leq_mat,
        trimat.to_csr() // We must follow lexicographical order because of BTreeSet
    );

    let mut trimat = sprs::TriMat::new((1, 5));
    trimat.add_triplet(0, 2, 1);
    trimat.add_triplet(0, 3, -3);
    trimat.add_triplet(0, 4, 5);
    assert_eq!(pb.pb_repr.eq_mat, trimat.to_csr());

    let expected = CsVec::new(2, vec![0, 1], vec![3, -3]);
    assert_eq!(pb.pb_repr.leq_constants, expected);

    let expected = CsVec::new(1, vec![0], vec![2]);
    assert_eq!(pb.pb_repr.eq_constants, expected);
}

#[test]
fn test_is_feasable() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String, SprsProblem<_>>::new()
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

    let sprs_problem = &pb.pb_repr;

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(
        config_0
            .cfg_repr
            .is_feasable(sprs_problem, &config_0.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_1
            .cfg_repr
            .is_feasable(sprs_problem, &config_1.cfg_repr.precompute(sprs_problem)),
        true
    );
    assert_eq!(
        config_2
            .cfg_repr
            .is_feasable(sprs_problem, &config_2.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_3
            .cfg_repr
            .is_feasable(sprs_problem, &config_3.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_4
            .cfg_repr
            .is_feasable(sprs_problem, &config_4.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_5
            .cfg_repr
            .is_feasable(sprs_problem, &config_5.cfg_repr.precompute(sprs_problem)),
        true
    );
    assert_eq!(
        config_6
            .cfg_repr
            .is_feasable(sprs_problem, &config_6.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_7
            .cfg_repr
            .is_feasable(sprs_problem, &config_7.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_8
            .cfg_repr
            .is_feasable(sprs_problem, &config_8.cfg_repr.precompute(sprs_problem)),
        true
    );
    assert_eq!(
        config_9
            .cfg_repr
            .is_feasable(sprs_problem, &config_9.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_a
            .cfg_repr
            .is_feasable(sprs_problem, &config_a.cfg_repr.precompute(sprs_problem)),
        true
    );
    assert_eq!(
        config_b
            .cfg_repr
            .is_feasable(sprs_problem, &config_b.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_c
            .cfg_repr
            .is_feasable(sprs_problem, &config_c.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_d
            .cfg_repr
            .is_feasable(sprs_problem, &config_d.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_e
            .cfg_repr
            .is_feasable(sprs_problem, &config_e.cfg_repr.precompute(sprs_problem)),
        false
    );
    assert_eq!(
        config_f
            .cfg_repr
            .is_feasable(sprs_problem, &config_f.cfg_repr.precompute(sprs_problem)),
        false
    );
}

#[test]
fn test_is_feasable_no_constraint() {
    use crate::ilp::Problem;

    let pb: Problem<String, SprsProblem<_>> = crate::ilp::ProblemBuilder::new()
        .add_variables(["a", "b"])
        .unwrap()
        .build();

    let config_0 = pb.default_config();
    let config_1 = pb.config_from(["a"]).unwrap();
    let config_2 = pb.config_from(["b"]).unwrap();
    let config_3 = pb.config_from(["a", "b"]).unwrap();

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(
        config_0
            .cfg_repr
            .is_feasable(&pb.pb_repr, &config_0.cfg_repr.precompute(&pb.pb_repr)),
        true
    );
    assert_eq!(
        config_1
            .cfg_repr
            .is_feasable(&pb.pb_repr, &config_1.cfg_repr.precompute(&pb.pb_repr)),
        true
    );
    assert_eq!(
        config_2
            .cfg_repr
            .is_feasable(&pb.pb_repr, &config_2.cfg_repr.precompute(&pb.pb_repr)),
        true
    );
    assert_eq!(
        config_3
            .cfg_repr
            .is_feasable(&pb.pb_repr, &config_3.cfg_repr.precompute(&pb.pb_repr)),
        true
    );
}

#[test]
fn sprs_config_ord() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");

    let pb = crate::ilp::ProblemBuilder::<String, SprsProblem<_>>::new()
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

    let sprs_config_0 = config_0.cfg_repr.clone();
    let sprs_config_1 = config_1.cfg_repr.clone();
    let sprs_config_2 = config_2.cfg_repr.clone();
    let sprs_config_3 = config_3.cfg_repr.clone();
    let sprs_config_4 = config_4.cfg_repr.clone();
    let sprs_config_5 = config_5.cfg_repr.clone();
    let sprs_config_6 = config_6.cfg_repr.clone();
    let sprs_config_7 = config_7.cfg_repr.clone();

    assert_eq!(sprs_config_0.cmp(&sprs_config_0), std::cmp::Ordering::Equal);
    assert!(sprs_config_0 < sprs_config_1);
    assert!(sprs_config_0 < sprs_config_2);
    assert!(sprs_config_0 < sprs_config_3);
    assert!(sprs_config_0 < sprs_config_4);
    assert!(sprs_config_0 < sprs_config_5);
    assert!(sprs_config_0 < sprs_config_6);
    assert!(sprs_config_0 < sprs_config_7);

    assert_eq!(sprs_config_1.cmp(&sprs_config_1), std::cmp::Ordering::Equal);
    assert!(sprs_config_1 > sprs_config_2);
    assert!(sprs_config_1 < sprs_config_3);
    assert!(sprs_config_1 > sprs_config_4);
    assert!(sprs_config_1 < sprs_config_5);
    assert!(sprs_config_1 > sprs_config_6);
    assert!(sprs_config_1 < sprs_config_7);

    assert_eq!(sprs_config_2.cmp(&sprs_config_2), std::cmp::Ordering::Equal);
    assert!(sprs_config_2 < sprs_config_3);
    assert!(sprs_config_2 > sprs_config_4);
    assert!(sprs_config_2 < sprs_config_5);
    assert!(sprs_config_2 < sprs_config_6);
    assert!(sprs_config_2 < sprs_config_7);

    assert_eq!(sprs_config_3.cmp(&sprs_config_3), std::cmp::Ordering::Equal);
    assert!(sprs_config_3 > sprs_config_4);
    assert!(sprs_config_3 > sprs_config_5);
    assert!(sprs_config_3 > sprs_config_6);
    assert!(sprs_config_3 < sprs_config_7);

    assert_eq!(sprs_config_4.cmp(&sprs_config_4), std::cmp::Ordering::Equal);
    assert!(sprs_config_4 < sprs_config_5);
    assert!(sprs_config_4 < sprs_config_6);
    assert!(sprs_config_4 < sprs_config_7);

    assert_eq!(sprs_config_5.cmp(&sprs_config_5), std::cmp::Ordering::Equal);
    assert!(sprs_config_5 > sprs_config_6);
    assert!(sprs_config_5 < sprs_config_7);

    assert_eq!(sprs_config_6.cmp(&sprs_config_6), std::cmp::Ordering::Equal);
    assert!(sprs_config_6 < sprs_config_7);

    assert_eq!(sprs_config_7.cmp(&sprs_config_7), std::cmp::Ordering::Equal);
}

#[test]
fn compute_lhs() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String, SprsProblem<_>>::new()
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

    let sprs_problem = &pb.pb_repr;

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(
        config_0
            .cfg_repr
            .compute_lhs(sprs_problem, &config_0.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    println!("{:?}", config_1.cfg_repr);
    assert_eq!(
        config_1
            .cfg_repr
            .compute_lhs(sprs_problem, &config_1.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_2
            .cfg_repr
            .compute_lhs(sprs_problem, &config_2.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_3
            .cfg_repr
            .compute_lhs(sprs_problem, &config_3.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_4
            .cfg_repr
            .compute_lhs(sprs_problem, &config_4.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_5
            .cfg_repr
            .compute_lhs(sprs_problem, &config_5.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_6
            .cfg_repr
            .compute_lhs(sprs_problem, &config_6.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_7
            .cfg_repr
            .compute_lhs(sprs_problem, &config_7.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_8
            .cfg_repr
            .compute_lhs(sprs_problem, &config_8.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_9
            .cfg_repr
            .compute_lhs(sprs_problem, &config_9.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_a
            .cfg_repr
            .compute_lhs(sprs_problem, &config_a.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_b
            .cfg_repr
            .compute_lhs(sprs_problem, &config_b.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_c
            .cfg_repr
            .compute_lhs(sprs_problem, &config_c.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_d
            .cfg_repr
            .compute_lhs(sprs_problem, &config_d.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_e
            .cfg_repr
            .compute_lhs(sprs_problem, &config_e.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_f
            .cfg_repr
            .compute_lhs(sprs_problem, &config_f.cfg_repr.precompute(sprs_problem)),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
}

#[test]
fn update_precomputation() {
    use crate::ilp::linexpr::Expr;

    let a = Expr::<String>::var("a");
    let b = Expr::<String>::var("b");
    let c = Expr::<String>::var("c");
    let d = Expr::<String>::var("d");

    let pb = crate::ilp::ProblemBuilder::<String, SprsProblem<_>>::new()
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
    let _ = config_0.get_precomputation();

    let mut config_1 = config_0.clone();
    config_1.set("a", true).unwrap(); // ["a"]
    let mut config_2 = config_0.clone();
    config_2.set("b", true).unwrap(); // ["b"]
    let mut config_3 = config_1.clone();
    config_3.set("b", true).unwrap(); // ["a", "b"]
    let mut config_4 = config_2.clone();
    config_4.set("b", false).unwrap();
    config_4.set("c", true).unwrap(); // ["c"]
    let mut config_5 = config_4.clone();
    config_5.set("a", true).unwrap(); // ["a","c"]
    let mut config_6 = config_4.clone();
    config_6.set("b", true).unwrap(); // ["b","c"]
    let mut config_7 = config_6.clone();
    config_7.set("a", true).unwrap(); // ["a","b","c"]

    let mut config_8 = config_0.clone();
    config_8.set("d", true).unwrap(); // ["d"]

    let mut config_9 = config_8.clone();
    config_9.set("a", true).unwrap(); // ["a","d"]
    let mut config_a = config_8.clone();
    config_a.set("b", true).unwrap(); // ["b","d"]
    let mut config_b = config_9.clone();
    config_b.set("b", true).unwrap(); // ["a", "b","d"]
    let mut config_c = config_a.clone();
    config_c.set("b", false).unwrap();
    config_c.set("c", true).unwrap(); // ["c","d"]
    let mut config_d = config_c.clone();
    config_d.set("a", true).unwrap(); // ["a","c","d"]
    let mut config_e = config_c.clone();
    config_e.set("b", true).unwrap(); // ["b","c","d"]
    let mut config_f = config_e.clone();
    config_f.set("a", true).unwrap(); // ["a","b","c","d"]

    let sprs_problem = &pb.pb_repr;

    use crate::ilp::mat_repr::ConfigRepr;
    assert_eq!(
        config_0
            .cfg_repr
            .compute_lhs(sprs_problem, &config_0.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    println!("{:?}", config_1.cfg_repr);
    assert_eq!(
        config_1
            .cfg_repr
            .compute_lhs(sprs_problem, &config_1.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_2
            .cfg_repr
            .compute_lhs(sprs_problem, &config_2.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_3
            .cfg_repr
            .compute_lhs(sprs_problem, &config_3.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), -1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_4
            .cfg_repr
            .compute_lhs(sprs_problem, &config_4.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_5
            .cfg_repr
            .compute_lhs(sprs_problem, &config_5.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_6
            .cfg_repr
            .compute_lhs(sprs_problem, &config_6.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), -1),
        ])
    );
    assert_eq!(
        config_7
            .cfg_repr
            .compute_lhs(sprs_problem, &config_7.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_8
            .cfg_repr
            .compute_lhs(sprs_problem, &config_8.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_9
            .cfg_repr
            .compute_lhs(sprs_problem, &config_9.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_a
            .cfg_repr
            .compute_lhs(sprs_problem, &config_a.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_b
            .cfg_repr
            .compute_lhs(sprs_problem, &config_b.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 0),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_c
            .cfg_repr
            .compute_lhs(sprs_problem, &config_c.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), -1),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_d
            .cfg_repr
            .compute_lhs(sprs_problem, &config_d.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
    assert_eq!(
        config_e
            .cfg_repr
            .compute_lhs(sprs_problem, &config_e.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 0),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 0),
        ])
    );
    assert_eq!(
        config_f
            .cfg_repr
            .compute_lhs(sprs_problem, &config_f.get_precomputation()),
        BTreeMap::from([
            ((&a + &b).leq(&Expr::constant(1)), 1),
            ((&c + &d).leq(&Expr::constant(1)), 1),
            ((&a + &d).eq(&Expr::constant(1)), 1),
        ])
    );
}
