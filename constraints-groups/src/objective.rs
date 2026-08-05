//! The stability objective (piece 9 of the roadmap, §2.4).
//!
//! Minimize `w_groups · Σ GroupHasStudents + w_pairs · Σ SharedPair`. The
//! first term uses as few groups as possible; the second keeps groups stable
//! across lists — `SharedPair` counts a pair once however many lists it
//! shares a group in, and a pair pinned by a kept list is the constant 1, so
//! reusing an existing grouping costs nothing.
//!
//! The weights are configurable ([`ObjectiveWeights`], piece 11) and the
//! default is group-dominant (w_groups = 1000, w_pairs = 1): at equal
//! weights the two terms cancel exactly when merging singletons, so the
//! pair term is a tie-breaker among the solutions that already minimize
//! the group count.

use crate::extras::{MyBundle, co_occurrences, extra_var};
use crate::types::ExtraVarName;
use crate::vars::VarEnv;
use collomatique_ilp::linexpr::LinExpr;

/// Default weight of the "use as few groups as possible" term. Group-dominant
/// on purpose (roadmap §5 piece 11): at equal weights, moving a student out
/// of a singleton into a group of size s trades one group against s new
/// pairs — exactly cost-neutral — so the group count was drowned by the pair
/// term. 1000 dominates every plausible instance while keeping the model
/// well scaled; a strict lexicographic weight would have to exceed
/// Σ C(n_i, 2) and would ill-condition the LP relaxation.
const W_GROUPS_DEFAULT: f64 = 1000.0;
/// Default weight of the "share as few pairs as possible" term: a tie-breaker
/// among the solutions that already use as few groups as possible.
const W_PAIRS_DEFAULT: f64 = 1.0;

/// The two weights of the stability objective (§2.4), handed to
/// [`build_model`](crate::build_model) next to the plan. Deliberately not a
/// field of [`GenerationRequest`](crate::GenerationRequest): the request says
/// *what* to rebuild and is consumed by `build_generation_plan`; the weights
/// are read only by the objective builder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveWeights {
    /// Weight of the "use as few groups as possible" term.
    pub w_groups: f64,
    /// Weight of the "share as few pairs as possible" term.
    pub w_pairs: f64,
}

impl Default for ObjectiveWeights {
    fn default() -> Self {
        ObjectiveWeights {
            w_groups: W_GROUPS_DEFAULT,
            w_pairs: W_PAIRS_DEFAULT,
        }
    }
}

pub(crate) fn build(env: &VarEnv, weights: ObjectiveWeights) -> MyBundle {
    let mut expr = LinExpr::constant(0.0);
    let mut has_terms = false;

    for list in env.lists() {
        for group in 0..env.slot_count(list) {
            expr = expr
                + weights.w_groups
                    * LinExpr::var(extra_var(ExtraVarName::GroupHasStudents { list, group }));
            has_terms = true;
        }
    }

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

    /// The equal weights the piece-9 tests were written against: their
    /// comment arithmetic (and `place()`'s weight-100 scale) assumes 1/1.
    const EQUAL: ObjectiveWeights = ObjectiveWeights {
        w_groups: 1.0,
        w_pairs: 1.0,
    };

    /// A weight-100 term placing `student` in `group` of `list`. 100 dwarfs
    /// both the real objective at the explicit `EQUAL` weights the piece-9
    /// tests pass (at most ~10 on these instances) and the ±0.5 adversaries,
    /// so a placement never bends. Not for use with the group-dominant
    /// default, which 100 does not dwarf.
    fn place(list: usize, s: u64, group: u32) -> (f64, V) {
        (
            100.0,
            extra_var(ExtraVarName::StudentInGroup {
                list: GroupListIdx(list),
                student: student(s),
                group,
            }),
        )
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

    fn group_of(cfg: &ConfigData<InternalVar<Var, ExtraVarName>>, list: usize, s: u64) -> f64 {
        value(
            cfg,
            base_var(Var::StudentGroup {
                list: GroupListIdx(list),
                student: student(s),
            }),
        )
    }

    #[test]
    fn optimum_reuses_groupings_across_lists() {
        // Two lists over the same four students, with distinct specs (equal
        // ones would have been deduplicated upstream) that both force two
        // groups of two: with 4 students, min 2 and max <= 3, the only size
        // split is 2 + 2. The three perfect matchings of {1, 2, 3, 4} have
        // pairwise disjoint pair sets, so equal matchings share 2 pairs and
        // different ones share 4. The groups term is the constant 4 (every
        // slot must be non-empty), so the pairs term alone decides.
        //
        // The places pin list 0 to {1, 2} / {3, 4} and student 1 to group 0
        // of list 1, so list 1's group 0 is {1, x}. With x = 2 (reuse):
        // 4 groups + 2 shared = 6. With x = 3: 4 + 4 − 0.5 = 7.5. With
        // x = 4: 8. The optimum reuses, with margin 1.5.
        //
        // The adversary is load-bearing: without it, deleting the pairs term
        // would leave every matching of list 1 optimal and CBC might return
        // the reusing one by chance — a tie, not a red test.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4], (2, 3))]);
        let cfg = solve_with_adversary(
            &plan,
            EQUAL,
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                (
                    0.5,
                    extra_var(ExtraVarName::StudentInGroup {
                        list: GroupListIdx(1),
                        student: student(3),
                        group: 0,
                    }),
                ),
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
        // Groups is the constant 2. Group 0 = {1, 2}: shared = 1 (pinned)
        // + 1 ({3, 4}) = 2, total 4. Group 0 = {1, 3}: 1 + 1 + 1 = 3, total
        // 5 − 0.5 = 4.5. Group 0 = {1, 4}: 5. The optimum groups the pinned
        // pair — reuse is free, so the alternative's fresh pairs decide.
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        plan.pinned_pairs = [(student(1), student(2))].into_iter().collect();
        let cfg = solve_with_adversary(
            &plan,
            EQUAL,
            &[
                place(0, 1, 0),
                (
                    0.5,
                    extra_var(ExtraVarName::StudentInGroup {
                        list: GroupListIdx(0),
                        student: student(3),
                        group: 0,
                    }),
                ),
            ],
        );

        assert_close(group_of(&cfg, 0, 2), 0.0);
        assert_close(shared_total_of_four(&cfg), 2.0);
    }

    #[test]
    fn groups_term_pulls_toward_fewer_groups() {
        // Neither prescribed test above can catch the deletion of the groups
        // term: in both, the group count is forced by the size constraints,
        // so the term is a constant. And with equal weights the term is
        // subtle — moving a student from a singleton group into a group of
        // size s trades one group for s new pairs, so merging two singletons
        // is exactly cost-neutral. The clean isolation is a *pinned* pair:
        // the pairs term then contributes a constant, and only the groups
        // term distinguishes merging from splitting.
        //
        // Two students, sizes 1..=2 (hence 2 slots), pair (1, 2) pinned, so
        // shared is the constant 1. Together in group 0: 1 group + 1 = 2.
        // Split with 2 in group 1: 2 + 1 − 0.5 = 2.5. Split the other way:
        // 3. The optimum is together. No `place` is needed: ascending fill
        // forbids occupying group 1 while group 0 is empty, so "together"
        // can only mean "both in group 0".
        let mut plan = plan_of(&[(&[1, 2], (1, 2))]);
        plan.pinned_pairs = [(student(1), student(2))].into_iter().collect();
        let cfg = solve_with_adversary(
            &plan,
            EQUAL,
            &[(
                0.5,
                extra_var(ExtraVarName::StudentInGroup {
                    list: GroupListIdx(0),
                    student: student(2),
                    group: 1,
                }),
            )],
        );

        assert_close(group_of(&cfg, 0, 1), 0.0);
        assert_close(group_of(&cfg, 0, 2), 0.0);
    }

    #[test]
    fn default_weights_prefer_fewer_groups_over_fewer_pairs() {
        // The regression piece 11 exists to fix: at the old equal weights,
        // merging two singletons was exactly cost-neutral, so the 0.5
        // adversary toward splitting would win. Two students, sizes 1..=2
        // (hence 2 slots), nothing pinned. Together in group 0: 1 group + 1
        // pair = 1000 + 1 = 1001. Split with 2 in group 1: 2 groups + 0
        // pairs − 0.5 = 1999.5. The optimum merges — at w_groups = 1 it
        // would split (2 vs 1.5). No `place` is needed: ascending fill
        // forbids occupying group 1 while group 0 is empty, so "together"
        // can only mean "both in group 0".
        let plan = plan_of(&[(&[1, 2], (1, 2))]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights::default(),
            &[(
                0.5,
                extra_var(ExtraVarName::StudentInGroup {
                    list: GroupListIdx(0),
                    student: student(2),
                    group: 1,
                }),
            )],
        );

        assert_close(group_of(&cfg, 0, 1), 0.0);
        assert_close(group_of(&cfg, 0, 2), 0.0);
        assert_close(value(&cfg, shared(1, 2)), 1.0);
    }

    #[test]
    fn explicit_weights_override_the_default() {
        // Pair-dominant weights on the same instance, with the adversary
        // rewarding the shared pair (+0.5 toward merging). Together:
        // 1 group + 1000 · 1 pair − 0.5 = 1000.5. Split: 2 groups + 0 = 2.
        // The optimum splits — and a build that ignores the passed weights
        // merges instead: hardcoded 1000/1 gives 1000.5 vs 2000, hardcoded
        // w_pairs = 1 gives 1.5 vs 2. Which student lands in which group is
        // a tie between the two splits, so the assertions are on `SharedPair`
        // and `GroupHasStudents`, not on placements.
        let plan = plan_of(&[(&[1, 2], (1, 2))]);
        let cfg = solve_with_adversary(
            &plan,
            ObjectiveWeights {
                w_groups: 1.0,
                w_pairs: 1000.0,
            },
            &[(0.5, shared(1, 2))],
        );

        assert_close(value(&cfg, shared(1, 2)), 0.0);
        assert_close(
            value(
                &cfg,
                extra_var(ExtraVarName::GroupHasStudents {
                    list: GroupListIdx(0),
                    group: 1,
                }),
            ),
            1.0,
        );
    }
}
