use super::*;
use crate::specs::tests::student;
use crate::vars::tests::plan_with_uses;
use collomatique_ilp::f64_equals;

/// The masses are rationals with small denominators, but they are reached by
/// different routes here and in the enumeration, so every comparison goes
/// through the crate's tolerance rather than `assert_eq!`.
fn assert_close(got: f64, expected: f64) {
    assert!(f64_equals(got, expected), "expected {expected}, got {got}");
}

fn data(plan: &GenerationPlan) -> PairData {
    PairData::new(plan, &VarEnv::new(plan))
}

fn tier(list: usize, target: u32, groups: &[u32]) -> Tier {
    Tier {
        list: GroupListIdx(list),
        target,
        groups: groups.to_vec(),
    }
}

#[test]
fn a_mass_needs_a_partner_and_a_use() {
    // The plain case: two uses of a list, a student taking part in three uses
    // overall, three partners in the group.
    assert_close(pair_mass(2, 3, 4), 2.0 / 9.0);
    // Alone in the group: nobody to put mass on.
    assert_close(pair_mass(2, 3, 1), 0.0);
    // A student whose every list serves nothing: placed, but scoring nothing.
    // The formula would divide by zero here, which is the reason for the
    // guard rather than a mere shortcut.
    assert_close(pair_mass(0, 0, 4), 0.0);
    assert_close(pair_mass(3, 0, 4), 0.0);
}

#[test]
fn n_uses_counts_kept_lists_too() {
    // Student 1 is in the spec (2 uses) and in a kept list serving 3 pairs;
    // student 2's kept list is inert; students 5 and 6 are known through kept
    // lists only.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 2)],
        &[(&[&[1, 5]], 3), (&[&[2, 6]], 0)],
    );
    let data = data(&plan);

    assert_eq!(data.n_uses(student(1)), 5);
    assert_eq!(data.n_uses(student(2)), 2);
    assert_eq!(data.n_uses(student(5)), 3);
    // An inert kept list weighs nothing and does not even make its members
    // part of the universe.
    assert_eq!(data.n_uses(student(6)), 0);
    // And a student the plan never mentions.
    assert_eq!(data.n_uses(student(99)), 0);
}

#[test]
fn tier_tables_collapse_the_sites_by_target() {
    // 7 students in groups of 2 to 3 → targets 3 / 2 / 2: two tiers, the
    // second holding two interchangeable sites.
    let plan = plan_with_uses(&[(&[1, 2, 3, 4, 5, 6, 7], (2, 3), 1)], &[]);
    let data = data(&plan);

    assert_eq!(
        data.tiers(student(1), student(2)),
        &[tier(0, 2, &[1, 2]), tier(0, 3, &[0])],
    );
    // Every pair of the list has the same table — the sites are a property of
    // the list, not of the pair.
    assert_eq!(data.pairs().count(), 7 * 6 / 2);
    for (_pair, table) in data.pairs() {
        assert_eq!(table, data.tiers(student(1), student(2)));
    }
    // A pair is keyed `a < b`, so the reversed lookup finds nothing.
    assert!(data.tiers(student(2), student(1)).is_empty());
}

#[test]
fn massless_lists_and_tiers_are_filtered_out() {
    // A spec covering no (period, subject) pair: mass 0 in both directions,
    // so it contributes no site at all (F1).
    let unused = plan_with_uses(&[(&[1, 2, 3, 4], (2, 2), 0)], &[]);
    assert_eq!(data(&unused).pairs().count(), 0);

    // 3 students in groups of 1 to 2 → targets 2 / 1. The lone seat can never
    // hold a pair, so only the first group is a site.
    let lone_seat = plan_with_uses(&[(&[1, 2, 3], (1, 2), 1)], &[]);
    assert_eq!(
        data(&lone_seat).tiers(student(1), student(2)),
        &[tier(0, 2, &[0])],
    );

    // And a list of nothing but lone seats drops out entirely.
    let all_alone = plan_with_uses(&[(&[1, 2], (1, 1), 1)], &[]);
    assert_eq!(data(&all_alone).pairs().count(), 0);
}

#[test]
fn cross_tiers_skips_the_couples_inside_a_list() {
    let plan = plan_with_uses(
        &[
            (&[1, 2, 3, 4, 5, 6, 7], (2, 3), 1),
            (&[1, 2, 3, 4], (2, 2), 1),
        ],
        &[],
    );
    let data = data(&plan);

    let table = data.tiers(student(1), student(2));
    assert_eq!(
        table,
        &[tier(0, 2, &[1, 2]), tier(0, 3, &[0]), tier(1, 2, &[0, 1])],
    );

    // The two tiers of list 0 are mutually exclusive — one group per list —
    // so their product is identically zero and is never enumerated. What is
    // left is each tier of list 0 against the tier of list 1.
    let couples: Vec<(&Tier, &Tier)> = cross_tiers(table).collect();
    assert_eq!(
        couples,
        vec![(&table[0], &table[2]), (&table[1], &table[2]),],
    );
}

#[test]
fn kept_lists_enter_as_directed_constants() {
    // Student 2 takes part in four uses, student 1 in three: the very same
    // kept meeting weighs differently in each direction.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 1), (&[2, 3], (2, 2), 3)],
        &[(&[&[1, 2]], 2)],
    );
    let data = data(&plan);

    assert_eq!(data.n_uses(student(1)), 3);
    assert_eq!(data.n_uses(student(2)), 6);
    assert_close(data.kept_constant(student(1), student(2)), 2.0 / 3.0);
    assert_close(data.kept_constant(student(2), student(1)), 1.0 / 3.0);
    // A pair no kept list groups together.
    assert_close(data.kept_constant(student(1), student(3)), 0.0);

    // The constant term is what the solution scores before the solver decides
    // anything: both directions of the one kept pair.
    assert_close(data.constant_term(), 4.0 / 9.0 + 1.0 / 9.0);

    // And the linear coefficient carries the `2·c·m` cross term of the
    // expansion: sharing a group of list 0 takes `P_1(2)` to 1 and `P_2(1)`
    // to 1/2, so the pair's score goes from 5/9 to 5/4.
    assert_close(
        data.together_coefficient(student(1), student(2), GroupListIdx(0), 2),
        5.0 / 4.0 - 5.0 / 9.0,
    );
}

#[test]
fn a_kept_only_pair_is_all_constant() {
    // Students 5 and 6 share nothing but a kept list: they never get a site,
    // yet their mass must still be part of the objective — which is why the
    // constant term cannot be dropped as a mere offset.
    let plan = plan_with_uses(&[(&[1, 2], (2, 2), 1)], &[(&[&[5, 6]], 4)]);
    let data = data(&plan);

    assert!(data.tiers(student(5), student(6)).is_empty());
    assert_eq!(data.n_uses(student(5)), 4);
    // 4 uses, 4 uses in the denominator, one partner: mass 1 each way.
    assert_close(data.constant_term(), 2.0);
}

#[test]
fn the_license_case_coefficients_expand_the_square() {
    // The §2.4 scenario, scaled down as in the greedy tests: eight students,
    // a tutorial in two groups of four and two colles in pairs, one use each.
    // Every student has N = 3, so a colle partner weighs 1/3 and a tutorial
    // mate 1/9.
    let plan = plan_with_uses(
        &[
            (&[1, 2, 3, 4, 5, 6, 7, 8], (4, 4), 1),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2), 1),
        ],
        &[],
    );
    let data = data(&plan);
    let (a, b) = (student(1), student(2));

    assert_close(data.mass(a, GroupListIdx(0), 4), 1.0 / 9.0);
    assert_close(data.mass(a, GroupListIdx(1), 2), 1.0 / 3.0);

    // No kept list, so a site binary is worth `m_a² + m_b²`.
    let table = data.tiers(a, b);
    assert_eq!(
        table,
        &[
            tier(0, 4, &[0, 1]),
            tier(1, 2, &[0, 1, 2, 3]),
            tier(2, 2, &[0, 1, 2, 3])
        ]
    );
    assert_close(
        data.together_coefficient(a, b, GroupListIdx(0), 4),
        2.0 / 81.0,
    );
    assert_close(
        data.together_coefficient(a, b, GroupListIdx(1), 2),
        2.0 / 9.0,
    );

    // A product is worth `2·(m_a·m_a' + m_b·m_b')`: two colles together are
    // worth 4/9, a colle and the tutorial 4/27.
    assert_close(
        data.coincide_coefficient(a, b, &table[1], &table[2]),
        4.0 / 9.0,
    );
    assert_close(
        data.coincide_coefficient(a, b, &table[0], &table[1]),
        4.0 / 27.0,
    );

    // All three lists differ, so every couple of tiers is a product here, and
    // the whole expansion adds up to the score of a pair grouped together
    // everywhere: `P` is 1/9 + 1/3 + 1/3 = 7/9 each way, so 2·(7/9)².
    let linear: f64 = table
        .iter()
        .map(|t| data.together_coefficient(a, b, t.list, t.target))
        .sum();
    let quadratic: f64 = cross_tiers(table)
        .map(|(first, second)| data.coincide_coefficient(a, b, first, second))
        .sum();
    assert_eq!(cross_tiers(table).count(), 3);
    assert_close(data.constant_term(), 0.0);
    assert_close(linear + quadratic, 2.0 * (7.0 / 9.0) * (7.0 / 9.0));
}
