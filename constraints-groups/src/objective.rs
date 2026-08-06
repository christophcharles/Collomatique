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
//! The group count used to be a second, dominant term. It no longer is: the
//! minimal count has a closed form (`VarEnv::group_count`) and is imposed by
//! the model instead of optimized, which also makes it hold under the solve
//! strategies that strip the objective.

use crate::extras::{MyBundle, co_occurrences, extra_var};
use crate::types::ExtraVarName;
use crate::vars::VarEnv;
use collomatique_ilp::linexpr::LinExpr;

/// Default weight of the "share as few pairs as possible" term.
const W_PAIRS_DEFAULT: f64 = 1.0;

/// The weights of the stability objective (§2.4), handed to
/// [`build_model`](crate::build_model) next to the plan. Deliberately not a
/// field of [`GenerationRequest`](crate::GenerationRequest): the request says
/// *what* to rebuild and is consumed by `build_generation_plan`; the weights
/// are read only by the objective builder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveWeights {
    /// Weight of the "share as few pairs as possible" term. The only term
    /// of the objective, but the scale still matters: the solve strategies
    /// add terms of their own to it — the incremental strategy's L1 anchor
    /// weighs 1000 per already-solved variable.
    pub w_pairs: f64,
}

impl Default for ObjectiveWeights {
    fn default() -> Self {
        ObjectiveWeights {
            w_pairs: W_PAIRS_DEFAULT,
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
    use crate::vars::{GroupListIdx, SizeClassIdx, Var};
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp::{ConfigData, f64_equals};
    use collomatique_ilp_modeler::{InternalVar, Modeler};

    /// The piece-7/8 harness plus the real objective: apply the extras, the
    /// shape constraints and the piece-9 objective bundle, add the test's
    /// `maximize` terms, build, solve, and return every variable of the
    /// solution. The objective bundle lands first, so the fold keeps
    /// `Minimize` and subtracts each differing-sense term
    /// (`ilp/src/objectives.rs`): a term with weight `w` *rewards* its
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
            // (`ilp/src/objectives.rs:128`).
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
        let group_of = |list: usize, s: u64| -> u32 {
            (0..2)
                .find(|&group| {
                    base.get(Var::StudentInGroup {
                        list: GroupListIdx(list),
                        student: student(s),
                        group,
                    })
                    .expect("every binary of the placement is a base variable")
                    .round() as i64
                        == 1
                })
                .expect("every student sits in a group")
        };

        // Class 0 is 2..=2 (list 0) and class 1 is 2..=3 (list 1): the
        // canonical range is the tighter 2..=2, so a meeting of class 1
        // weighs (2 − 1) / (3 − 1).
        let mut expected_total = 0.0;
        for (class, list, weight) in [(0usize, 0usize, 1.0), (1, 1, 0.5)] {
            for (a, b) in [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)] {
                let together = group_of(list, a) == group_of(list, b);
                let expected = if together { 1.0 } else { 0.0 };
                assert_close(value(&cfg, shared(a, b, class)), expected);
                expected_total += weight * expected;
            }
        }
        // `w_pairs` is 1, so the objective is the class-weighted count.
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
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4, 5, 6], (2, 2))]);
        let cfg = solve_with_adversary(&plan, ObjectiveWeights { w_pairs: 0.1 }, &reuse_places());

        assert_close(shared_total(&cfg, &[1, 2, 3, 4, 5, 6], 0), 5.0);
        assert_close(value(&cfg, shared(1, 3, 0)), 1.0);
    }
}
