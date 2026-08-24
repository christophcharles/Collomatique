//! The stability objective (piece 9 of the roadmap, §2.4).
//!
//! Minimize `w_pairs · Σ class_weight · SharedPair`: keep the groups stable
//! across lists. `SharedPair` counts a pair once however many lists *of its
//! size class* it shares a group in, and a pair pinned by a kept list of that
//! class is the constant 1, so reusing an existing grouping costs nothing.
//!
//! The class weight is what keeps large groups from swamping the sum: a pair
//! meeting in a tutorial group of twenty is worth a fraction of one meeting
//! in a group of three (see `VarEnv::class_weight`). Without it, tutorials —
//! where everyone meets everyone whatever the model does — pre-pay every
//! pair of the class and leave the small groups free to churn.
//!
//! The second term is the *template* one: `w_template · Σ class_weight ·
//! RefGroupInGroup`, which is to say, how many pieces each list breaks each
//! reference group into. Where the step term prices a pair's first meeting,
//! this one prices the shattering of a grouping, so that the lists converge
//! on one grouping instead of merely on few pairs. Both are kept: on data
//! with two genuinely different stable groupings the template term simply has
//! no cheap optimum, and the step term still ranks the alternatives.
//!
//! The reference grouping itself is no longer decided by the solver — it is
//! computed by [`crate::ghost`] and read here as data. That is what lets the
//! term be indexed by reference group rather than by pair, which is both far
//! smaller (`students × lists × groups` rows instead of
//! `pairs × lists × groups`) and a better measure: a list whose groups are
//! *larger* than the reference size may swallow two reference groups whole
//! for the minimum price, which the per-pair form charged for.
//!
//! The group count used to be a second, dominant term. It no longer is: the
//! minimal count has a closed form (`VarEnv::group_count`) and is imposed by
//! the model instead of optimized, which also makes it hold under the solve
//! strategies that strip the objective.

use crate::extras::{MyBundle, co_occurrences, extra_var, scatter_sites};
use crate::types::ExtraVarName;
use crate::vars::VarEnv;
use collomatique_ilp::linexpr::LinExpr;

/// Default weight of the "share as few pairs as possible" term.
const W_PAIRS_DEFAULT: f64 = 1.0;

/// Default weight of one extra piece a list breaks a reference group into: a
/// quarter of a fresh pair. Deliberately well below [`W_PAIRS_DEFAULT`] — the
/// step term still decides which pairs meet at all, and the template term
/// only arbitrates between placements the step term rates alike. Misplacing
/// one student costs about one extra piece, against up to `max − 1` fresh
/// pairs, so the two stay in the balance they were in.
const W_TEMPLATE_DEFAULT: f64 = 0.25;

/// The weights of the stability objective (§2.4), handed to
/// [`build_model`](crate::build_model) next to the plan. Deliberately not a
/// field of [`GenerationRequest`](crate::GenerationRequest): the request says
/// *what* to rebuild and is consumed by `build_generation_plan`; the weights
/// are read only by the objective builder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveWeights {
    /// Weight of the "share as few pairs as possible" term. The scale
    /// matters beyond the ratio to `w_template`: the solve strategies add
    /// terms of their own to the objective — the incremental strategy's L1
    /// anchor weighs 1000 per already-solved variable.
    pub w_pairs: f64,
    /// Weight of one piece a list breaks a reference group into, before the
    /// per-list class weight. The pair term prices a pair's *first* meeting;
    /// this one prices the shattering of the reference grouping, so that ten
    /// lists agreeing beat five and five. 0 makes the reference grouping
    /// free, leaving the pair term alone in charge — though the columns stay
    /// in the model either way: a zero coefficient still counts as a
    /// reference, so the extras are expanded all the same.
    pub w_template: f64,
}

impl Default for ObjectiveWeights {
    fn default() -> Self {
        ObjectiveWeights {
            w_pairs: W_PAIRS_DEFAULT,
            w_template: W_TEMPLATE_DEFAULT,
        }
    }
}

pub(crate) fn build(env: &VarEnv, weights: ObjectiveWeights) -> MyBundle {
    let mut expr = LinExpr::constant(0.0);
    let mut has_terms = false;

    // Exactly the declared `SharedPair` set (see `co_occurrences`). Pinned
    // pairs are included: their variables are the constant 1, so they only
    // shift the objective by a constant — and keeping the sum uniform over
    // every declared variable is what makes "regrouping a kept pair is free"
    // literally true of the objective function.
    for ((a, b, class), _lists) in co_occurrences(env) {
        let weight = weights.w_pairs * env.class_weight(class);
        expr = expr + weight * LinExpr::var(extra_var(ExtraVarName::SharedPair { a, b, class }));
        has_terms = true;
    }

    // The template term: how many pieces each list breaks each reference
    // group into. Exactly the declared `RefGroupInGroup` set (see
    // `scatter_sites`), which is empty when the plan has no template.
    //
    // Its minimum is one piece per site, so the sum carries a constant
    // offset — harmless under an argmin, and the same kind of constant the
    // pinned pairs already contribute.
    //
    // The class weight enters per *list*, not per class: the reference
    // grouping is one grouping of everybody, and a list is only asked to
    // follow it as tightly as its own groups are tight. A list of a single
    // group is at one piece everywhere whatever the model does, and
    // contributes nothing but that constant.
    for (list, ref_group) in scatter_sites(env) {
        let weight = weights.w_template * env.class_weight(env.class_of(list));
        for group in 0..env.group_count(list) {
            expr = expr
                + weight
                    * LinExpr::var(extra_var(ExtraVarName::RefGroupInGroup {
                        list,
                        ref_group,
                        group,
                    }));
            has_terms = true;
        }
    }

    if has_terms {
        MyBundle::new().with_minimize(1.0, expr)
    } else {
        MyBundle::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::MyModeler;
    use crate::extras::{V, base_var};
    use crate::specs::GenerationPlan;
    use crate::specs::tests::{range, student};
    use crate::vars::tests::plan_of;
    use crate::vars::{GroupListIdx, RefGroupIdx, SizeClassIdx, Var};
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp::{ConfigData, f64_equals};
    use collomatique_ilp_modeler::{InternalVar, Modeler};
    use collomatique_state_colloscopes::StudentId;

    /// The piece-7/8 harness plus the real objective: apply the extras, the
    /// shape constraints and the piece-9 objective bundle, add the test's
    /// `maximize` terms, build, solve, and return every variable of the
    /// solution. The objective bundle lands first, so the fold keeps
    /// `Minimize` and subtracts each differing-sense term
    /// (`generic/ilp/src/objectives.rs`): a term with weight `w` *rewards* its
    /// variable by `w` against the real objective.
    fn solve_with_adversary(
        plan: &GenerationPlan,
        weights: ObjectiveWeights,
        terms: &[(f64, V)],
    ) -> ConfigData<InternalVar<Var, ExtraVarName>> {
        let env = VarEnv::new(plan);
        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(crate::extras::build_extras(&env).into_general())
            .expect("no duplicate extras");
        modeler
            .apply_bundle(crate::constraints::build(&env).into_general())
            .expect("no duplicate extras");
        modeler
            .apply_bundle(build(&env, weights).into_general())
            .expect("no duplicate extras");
        for (weight, var) in terms {
            // The weight goes into the `LinExpr`; a negative `coef` on
            // `maximize` would reverse the sense instead
            // (`generic/ilp/src/objectives.rs:128`).
            modeler.maximize(1.0, *weight * LinExpr::var(var.clone()));
        }
        let model = modeler.build(&env).expect("build should succeed");
        let solution = model
            .solve(&ColloCbcSolver::with_disable_logging(true))
            .expect("model should be solvable");
        solution.get_complete_data()
    }

    /// The base binary "`s` sits in `group` of `list`".
    fn in_group(list: usize, s: u64, group: u32) -> V {
        base_var(Var::StudentInGroup {
            list: GroupListIdx(list),
            student: student(s),
            group,
        })
    }

    /// A weight-100 term placing `student` in `group` of `list`. 100 dwarfs
    /// both the real objective at the default weight (at most ~10 on these
    /// instances) and the ±0.5 adversaries, so a placement never bends.
    fn place(list: usize, s: u64, group: u32) -> (f64, V) {
        (100.0, in_group(list, s, group))
    }

    fn value(cfg: &ConfigData<InternalVar<Var, ExtraVarName>>, var: V) -> f64 {
        cfg.get(var.clone())
            .unwrap_or_else(|| panic!("{:?} should be part of the solved problem", var))
    }

    /// CBC returns integral variables as floats carrying a tiny numerical
    /// error, so every value comparison of this module goes through
    /// [`f64_equals`] rather than `assert_eq!`.
    fn assert_close(got: f64, expected: f64) {
        assert!(f64_equals(got, expected), "expected {expected}, got {got}");
    }

    fn shared(a: u64, b: u64, class: usize) -> V {
        extra_var(ExtraVarName::SharedPair {
            a: student(a),
            b: student(b),
            class: SizeClassIdx(class),
        })
    }

    /// The piece indicator "reference group `ref_group` has a member in group
    /// `group` of `list`".
    fn piece(list: usize, ref_group: usize, group: u32) -> V {
        extra_var(ExtraVarName::RefGroupInGroup {
            list: GroupListIdx(list),
            ref_group: RefGroupIdx(ref_group),
            group,
        })
    }

    /// How many pieces `list` breaks reference group `ref_group` into, read
    /// off the indicators: the template term of the objective is the weighted
    /// sum of exactly these counts.
    fn pieces(
        cfg: &ConfigData<InternalVar<Var, ExtraVarName>>,
        list: usize,
        ref_group: usize,
        groups: u32,
    ) -> f64 {
        (0..groups)
            .map(|g| value(cfg, piece(list, ref_group, g)))
            .sum()
    }

    /// The sum of the `SharedPair` variables of one class over the given
    /// students, one term per pair.
    fn shared_total(
        cfg: &ConfigData<InternalVar<Var, ExtraVarName>>,
        students: &[u64],
        class: usize,
    ) -> f64 {
        let mut total = 0.0;
        for (i, &a) in students.iter().enumerate() {
            for &b in &students[i + 1..] {
                total += value(cfg, shared(a, b, class));
            }
        }
        total
    }

    /// The instance of the reuse tests: two lists of the same size class
    /// (2..=2, so one class weighing 1) over nested student sets — the same
    /// range with different students, which is what makes two specs of one
    /// class possible at all (equal specs are deduplicated upstream).
    ///
    /// List 0 holds 1 to 4 in two groups, list 1 holds 1 to 6 in three. The
    /// places pin list 0 to {1, 2} / {3, 4}, list 1's group 2 to {5, 6} and
    /// student 1 to group 0, so list 1's group 0 is {1, x}. With x = 2
    /// (reuse) the shared pairs are {1, 2}, {3, 4}, {5, 6} — 3. With x = 3
    /// they are those plus {1, 3} and {2, 4} — 5, minus the 0.5 adversary.
    /// With x = 4: 5. The optimum reuses, with margin 1.5.
    ///
    /// The adversary is load-bearing: without it, deleting the pairs term
    /// would leave every matching of list 1 optimal and CBC might return the
    /// reusing one by chance — a tie, not a red test.
    fn reuse_places() -> Vec<(f64, V)> {
        vec![
            place(0, 1, 0),
            place(0, 2, 0),
            place(0, 3, 1),
            place(0, 4, 1),
            place(1, 5, 2),
            place(1, 6, 2),
            place(1, 1, 0),
            (0.5, in_group(1, 3, 0)),
        ]
    }

    #[test]
    fn optimum_reuses_groupings_across_lists() {
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4, 5, 6], (2, 2))]);
        let cfg = solve_with_adversary(&plan, ObjectiveWeights::default(), &reuse_places());

        assert_close(shared_total(&cfg, &[1, 2, 3, 4, 5, 6], 0), 3.0);
        assert_close(value(&cfg, shared(1, 2, 0)), 1.0);
        assert_close(value(&cfg, shared(3, 4, 0)), 1.0);
    }

    #[test]
    fn a_tutorial_does_not_free_the_small_groups() {
        // The regression the size classes exist for. On top of the reuse
        // instance, a tutorial list takes all six students at once: its
        // single group of 6 makes every one of the 15 pairs share a group,
        // whatever the model does.
        //
        // With one flat `SharedPair` per pair, that alone would set every
        // variable of the instance to 1: the objective becomes a constant,
        // the small lists are free to churn and the 0.5 adversary decides —
        // list 1's group 0 comes out {1, 3}. Split per size class, the
        // tutorial only pays its own class (a constant 15 × 0.2, since a
        // meeting among six weighs (2 − 1) / (6 − 1) against the canonical
        // 2..=2), and the small class still prefers reuse by 1.5.
        let plan = plan_of(&[
            (&[1, 2, 3, 4], (2, 2)),
            (&[1, 2, 3, 4, 5, 6], (2, 2)),
            (&[1, 2, 3, 4, 5, 6], (6, 6)),
        ]);
        let cfg = solve_with_adversary(&plan, ObjectiveWeights::default(), &reuse_places());

        // The small class is unaffected by the tutorial: same optimum as
        // `optimum_reuses_groupings_across_lists`.
        assert_close(value(&cfg, in_group(1, 2, 0)), 1.0);
        assert_close(shared_total(&cfg, &[1, 2, 3, 4, 5, 6], 0), 3.0);
        assert_close(value(&cfg, shared(1, 2, 0)), 1.0);
        assert_close(value(&cfg, shared(3, 4, 0)), 1.0);
        // And the tutorial class is saturated, as the single group forces.
        assert_close(shared_total(&cfg, &[1, 2, 3, 4, 5, 6], 1), 15.0);
    }

    #[test]
    fn pinned_pairs_make_reuse_free() {
        // One list of four students, sizes 2..=2, with (1, 2) pinned by a
        // kept list of that same range: `SharedPair(1, 2)` is then the
        // constant 1, so grouping 1 with 2 creates only one *new* shared
        // pair ({3, 4}) while any other matching creates two.
        //
        // Group 0 = {1, 2}: shared = 1 (pinned) + 1 ({3, 4}) = 2. Group 0 =
        // {1, 3}: 1 + 1 + 1 = 3, minus the 0.5 adversary = 2.5. Group 0 =
        // {1, 4}: 3. The optimum groups the pinned pair — reuse is free, so
        // the alternative's fresh pairs decide.
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        plan.pinned_pairs = [(
            range(2, 2),
            [(student(1), student(2))].into_iter().collect(),
        )]
        .into_iter()
        .collect();
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights::default(),
            &[place(0, 1, 0), (0.5, in_group(0, 3, 0))],
        );

        assert_close(value(&cfg, in_group(0, 2, 0)), 1.0);
        assert_close(shared_total(&cfg, &[1, 2, 3, 4], 0), 2.0);
    }

    #[test]
    fn the_template_makes_the_lists_agree_with_each_other() {
        // The regression the template exists for. Four students and three
        // lists that all split them two by two: ranges 2..=3, 1..=2 and
        // 2..=2, distinct so the specs do not deduplicate, and each forcing
        // two groups of exactly two. The vote ties three ways at four
        // students apiece and breaks toward the tightest range, so the
        // canonical size is 2..=2 and the template is two groups of two.
        //
        // Lists 0 and 1 are placed to {1, 2} / {3, 4}; list 2 is free, with
        // a 0.5 reward for putting student 3 next to student 1 instead.
        //
        // `w_pairs` is 0, so the step term decides nothing: every pair of
        // the four already meets somewhere, and going along with the
        // adversary makes no *new* pair meet that the step term could
        // charge for. That is exactly the blind spot — with the pair term
        // alone the reward wins and list 2 comes out {1, 3} / {2, 4}.
        //
        // The template term sees it. The reference grouping is computed from
        // the specs — nothing here tells the pairs apart, so the clustering
        // breaks ties by student id and gives {1, 2} and {3, 4} — and list 2
        // disagreeing with it shatters both reference groups: 4 pieces
        // instead of 2, at class weight 1, against a reward of 0.5. So list
        // 2 falls in line.
        let plan = plan_of(&[
            (&[1, 2, 3, 4], (2, 3)),
            (&[1, 2, 3, 4], (1, 2)),
            (&[1, 2, 3, 4], (2, 2)),
        ]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights {
                w_pairs: 0.0,
                w_template: 1.0,
            },
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                place(1, 2, 0),
                place(1, 3, 1),
                place(1, 4, 1),
                place(2, 1, 0),
                (0.5, in_group(2, 3, 0)),
            ],
        );

        // The alignment itself: student 2, not student 3, joins student 1.
        assert_close(value(&cfg, in_group(2, 2, 0)), 1.0);
        assert_close(value(&cfg, in_group(2, 3, 0)), 0.0);

        // Every list reuses the reference grouping, so no reference group is
        // in more than one piece anywhere.
        for list in 0..3 {
            for ref_group in 0..2 {
                assert_close(pieces(&cfg, list, ref_group, 2), 1.0);
            }
        }
    }

    #[test]
    fn a_list_may_merge_two_reference_groups_for_free() {
        // A list whose groups are *larger* than the reference size. Eight
        // students, one list of pairs and one of fours: the vote ties at
        // eight students apiece and breaks toward the tighter range, so the
        // reference groups are {1, 2}, {3, 4}, {5, 6} and {7, 8}.
        //
        // List 1 holds two groups of four, so it cannot follow the reference
        // grouping — it can only swallow two reference groups whole per
        // group, which is the best any list of that shape could do. That must
        // cost the minimum: one piece per (list, reference group) site.
        //
        // Student 1 is pinned to group 0 and student 4 to group 1, so
        // grabbing student 3 into group 0 (the 0.5 adversary) necessarily
        // cuts {3, 4} in half — and drags a fourth student out of some other
        // reference group with it. That is 6 pieces against 4, and list 1's
        // class weight is (2 − 1) / (4 − 1), so at `w_template` 3 each piece
        // costs exactly 1: 2 against a reward of 0.5.
        let plan = plan_of(&[
            (&[1, 2, 3, 4, 5, 6, 7, 8], (2, 2)),
            (&[1, 2, 3, 4, 5, 6, 7, 8], (4, 4)),
        ]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights {
                w_pairs: 0.0,
                w_template: 3.0,
            },
            &[place(1, 1, 0), place(1, 4, 1), (0.5, in_group(1, 3, 0))],
        );

        // The adversary loses: student 3 stays with student 4.
        assert_close(value(&cfg, in_group(1, 3, 0)), 0.0);
        // And the merge is free — every reference group survives whole, in
        // the big list as much as in the small one.
        for ref_group in 0..4 {
            assert_close(pieces(&cfg, 0, ref_group, 4), 1.0);
            assert_close(pieces(&cfg, 1, ref_group, 2), 1.0);
        }
    }

    #[test]
    fn reconstruction_recovers_exact_pair_values() {
        // The load-bearing assumption behind the one-sided `SharedPair`
        // rows (see the `extras` module doc). Under a stripped objective a
        // pair variable may float upward, so this solves the *checker*
        // problem — no objective at all — and then hands its base values
        // back through `Model::solution_from_data`, which is the
        // reconstruction path every strategy uses to report an objective.
        //
        // The checker returns any feasible placement, so nothing here is
        // hardcoded: each pair is checked against the placement it actually
        // got, read off the base binaries.
        //
        // Two classes here — 2..=2 and 2..=3, one list each, the tie
        // electing the tighter one as canonical — so the check also covers
        // the class weights entering the reported objective.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4], (2, 3))]);
        let model = crate::build_model(&plan, ObjectiveWeights::default());
        let solver = ColloCbcSolver::with_disable_logging(true);

        let base = model
            .solve_checker(&solver)
            .expect("the checker problem must be feasible")
            .get_data();
        let solution = model
            .solution_from_data(&base, &solver)
            .expect("reconstruction should succeed");
        let cfg = solution.get_complete_data();

        // Both specs hold 4 students with a maximum of at least 2, so both
        // lists have ceil(4 / max) = 2 groups.
        let group_of = |list: usize, s: StudentId| -> u32 {
            (0..2)
                .find(|&group| {
                    base.get(Var::StudentInGroup {
                        list: GroupListIdx(list),
                        student: s,
                        group,
                    })
                    .expect("every binary of the placement is a base variable")
                    .round() as i64
                        == 1
                })
                .expect("every student sits in a group")
        };

        // The template is plan data now, and the canonical range is that same
        // 2..=2, so it holds ceil(4 / 2) = 2 reference groups.
        let ghost = plan.ghost.as_ref().expect("this plan has a template");
        assert_eq!(ghost.groups().len(), 2);

        // Class 0 is 2..=2 (list 0) and class 1 is 2..=3 (list 1): the
        // canonical range is the tighter 2..=2, so a meeting of class 1
        // weighs (2 − 1) / (3 − 1).
        let weights = ObjectiveWeights::default();
        let mut expected_total = 0.0;
        for (a, b) in [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)] {
            for (class, list, weight) in [(0usize, 0usize, 1.0), (1, 1, 0.5)] {
                let together = group_of(list, student(a)) == group_of(list, student(b));
                let expected = if together { 1.0 } else { 0.0 };
                assert_close(value(&cfg, shared(a, b, class)), expected);
                expected_total += weights.w_pairs * weight * expected;
            }
        }

        // The template term: one indicator per (list, reference group, group
        // of that list), tight against the placement, and the class weight
        // enters per list.
        for (list, weight) in [(0usize, 1.0), (1usize, 0.5)] {
            for (ref_group, members) in ghost.groups().iter().enumerate() {
                for group in 0..2 {
                    let occupied = members.iter().any(|&s| group_of(list, s) == group);
                    let expected = if occupied { 1.0 } else { 0.0 };
                    assert_close(value(&cfg, piece(list, ref_group, group)), expected);
                    expected_total += weights.w_template * weight * expected;
                }
            }
        }
        assert_close(solution.eval(), expected_total);
    }

    #[test]
    fn explicit_weight_scales_the_pair_term() {
        // The instance of `optimum_reuses_groupings_across_lists`, with the
        // pair weight turned down to 0.1: reuse now costs 0.3 while the
        // adversarial x = 3 costs 0.5 − 0.5 = 0, so the optimum stops
        // reusing and list 1 becomes {1, 3} / {2, 4} / {5, 6} — five shared
        // pairs instead of three. A build that ignored the passed weight
        // would keep reusing, so this is what pins that `w_pairs` is read.
        //
        // The template term is switched off: it prefers reuse too, and at
        // its default weight it would decide the instance on its own, which
        // would say nothing about `w_pairs`.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4, 5, 6], (2, 2))]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights {
                w_pairs: 0.1,
                w_template: 0.0,
            },
            &reuse_places(),
        );

        assert_close(shared_total(&cfg, &[1, 2, 3, 4, 5, 6], 0), 5.0);
        assert_close(value(&cfg, shared(1, 3, 0)), 1.0);
    }

    #[test]
    fn explicit_weight_scales_the_template_term() {
        // The instance of `the_template_makes_the_lists_agree_with_each_other`
        // with the template weight turned down to 0.1: shattering both
        // reference groups now costs 2 extra pieces × 0.1 = 0.2 while the
        // adversary pays 0.5, so list 2 stops falling in line and comes out
        // {1, 3} / {2, 4}. A build that ignored the passed weight would keep
        // it aligned, so this is what pins that `w_template` is read.
        //
        // The pair weight is switched off for the same reason it is there:
        // at its default it would decide the instance on its own.
        let plan = plan_of(&[
            (&[1, 2, 3, 4], (2, 3)),
            (&[1, 2, 3, 4], (1, 2)),
            (&[1, 2, 3, 4], (2, 2)),
        ]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights {
                w_pairs: 0.0,
                w_template: 0.1,
            },
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                place(1, 2, 0),
                place(1, 3, 1),
                place(1, 4, 1),
                place(2, 1, 0),
                (0.5, in_group(2, 3, 0)),
            ],
        );

        assert_close(value(&cfg, in_group(2, 3, 0)), 1.0);
        // Both reference groups are cut in two by list 2 — the shattering
        // the weight has stopped paying to avoid.
        assert_close(pieces(&cfg, 2, 0, 2), 2.0);
        assert_close(pieces(&cfg, 2, 1, 2), 2.0);
    }
}
