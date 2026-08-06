//! The stability objective (piece 9 of the roadmap, §2.4).
//!
//! Minimize `w_pairs · Σ SharedPair`: keep the groups stable across lists.
//! `SharedPair` counts a pair once however many lists it shares a group in,
//! and a pair pinned by a kept list is the constant 1, so reusing an
//! existing grouping costs nothing.
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
    for ((a, b), _lists) in co_occurrences(env) {
        expr = expr + weights.w_pairs * LinExpr::var(extra_var(ExtraVarName::SharedPair { a, b }));
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
    use crate::specs::tests::student;
    use crate::vars::tests::plan_of;
    use crate::vars::{GroupListIdx, Var};
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

    fn shared(a: u64, b: u64) -> V {
        extra_var(ExtraVarName::SharedPair {
            a: student(a),
            b: student(b),
        })
    }

    /// The sum of the six `SharedPair` variables over students 1 to 4.
    fn shared_total_of_four(cfg: &ConfigData<InternalVar<Var, ExtraVarName>>) -> f64 {
        [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)]
            .iter()
            .map(|&(a, b)| value(cfg, shared(a, b)))
            .sum()
    }

    #[test]
    fn optimum_reuses_groupings_across_lists() {
        // Two lists over the same four students, with distinct specs (equal
        // ones would have been deduplicated upstream) that both force two
        // groups of two: with 4 students, two groups and max <= 3, the only
        // size split is 2 + 2. The three perfect matchings of {1, 2, 3, 4}
        // have pairwise disjoint pair sets, so equal matchings share 2 pairs
        // and different ones share 4.
        //
        // The places pin list 0 to {1, 2} / {3, 4} and student 1 to group 0
        // of list 1, so list 1's group 0 is {1, x}. With x = 2 (reuse):
        // 2 shared. With x = 3: 4 − 0.5 = 3.5. With x = 4: 4. The optimum
        // reuses, with margin 1.5.
        //
        // The adversary is load-bearing: without it, deleting the pairs term
        // would leave every matching of list 1 optimal and CBC might return
        // the reusing one by chance — a tie, not a red test.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4], (2, 3))]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights::default(),
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                (0.5, in_group(1, 3, 0)),
            ],
        );

        assert_close(shared_total_of_four(&cfg), 2.0);
        assert_close(value(&cfg, shared(1, 2)), 1.0);
        assert_close(value(&cfg, shared(3, 4)), 1.0);
    }

    #[test]
    fn pinned_pairs_make_reuse_free() {
        // One list of four students, sizes 2..=2, with (1, 2) pinned by a
        // kept list: `SharedPair(1, 2)` is then the constant 1, so grouping
        // 1 with 2 creates only one *new* shared pair ({3, 4}) while any
        // other matching creates two.
        //
        // Group 0 = {1, 2}: shared = 1 (pinned) + 1 ({3, 4}) = 2. Group 0 =
        // {1, 3}: 1 + 1 + 1 = 3, minus the 0.5 adversary = 2.5. Group 0 =
        // {1, 4}: 3. The optimum groups the pinned pair — reuse is free, so
        // the alternative's fresh pairs decide.
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        plan.pinned_pairs = [(student(1), student(2))].into_iter().collect();
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights::default(),
            &[place(0, 1, 0), (0.5, in_group(0, 3, 0))],
        );

        assert_close(value(&cfg, in_group(0, 2, 0)), 1.0);
        assert_close(shared_total_of_four(&cfg), 2.0);
    }

    #[test]
    fn explicit_weight_scales_the_pair_term() {
        // The instance of `optimum_reuses_groupings_across_lists`, with the
        // pair weight turned down to 0.1: reuse now costs 0.2 while the
        // adversarial x = 3 costs 0.4 − 0.5 = −0.1, so the optimum stops
        // reusing and list 1 becomes {1, 3} / {2, 4} — four shared pairs
        // instead of two. A build that ignored the passed weight would keep
        // reusing, so this is what pins that `w_pairs` is read.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4], (2, 3))]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights { w_pairs: 0.1 },
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                (0.5, in_group(1, 3, 0)),
            ],
        );

        assert_close(shared_total_of_four(&cfg), 4.0);
        assert_close(value(&cfg, shared(1, 3)), 1.0);
    }
}
