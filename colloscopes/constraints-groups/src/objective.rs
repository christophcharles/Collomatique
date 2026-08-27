//! The collision objective: **maximize** `Σ_s Σ_t P_s(t)²`.
//!
//! This is the greedy's own objective (`docs/plans/greedy_algorithm.md` §2),
//! restated for a solver — which is what makes "send it to the ILP" a strict
//! refinement of the greedy rather than a different taste. `P_s(t)` is the
//! probability that a grouping decision of `s`, drawn uniformly among all of
//! `s`'s list-uses, points at `t`; squaring rewards *concentration*, so the
//! optimum is the placement where students meet the same partners again
//! instead of a fresh face every week.
//!
//! Everything it needs is enumerated once, by [`PairData`](crate::pairs):
//!
//! - the **constant** `Σ_dir c_dir²`, what the kept lists already score. It
//!   cannot be dropped as a mere offset — a pair that only ever meets in kept
//!   lists appears nowhere else — and it is carried in the objective's
//!   `LinExpr` so that the model's value *equals* the greedy's, rather than
//!   merely ranking placements the same way;
//! - one **linear** term per declared
//!   [`Together`](crate::ExtraVarName::Together), the site binary, at
//!   `Σ_dir (m_dir² + 2·c_dir·m_dir)`;
//! - one **quadratic** term per declared
//!   [`Coincide`](crate::ExtraVarName::Coincide), the tier product, at
//!   `Σ_dir 2·m_dir(l1, τ1)·m_dir(l2, τ2)`.
//!
//! Nothing is approximated and nothing is weighted by hand: the coefficients
//! are the expansion of the square, and the group sizes they divide by are
//! pinned by the model ([`crate::targets`]), which is what makes them
//! constants at all.
//!
//! Every coefficient is strictly positive (the enumeration filters out the
//! mass-0 lists and tiers, F1), which is what pulls the one-sided defining
//! rows of `crate::extras` tight under the maximize.
//!
//! The group count is not optimized: it has a closed form
//! (`VarEnv::group_count`) and is imposed by the model, which also makes it
//! hold under the solve strategies that strip the objective.

#[cfg(test)]
mod tests;

use crate::extras::{MyBundle, extra_var};
use crate::pairs::{PairData, cross_tiers};
use crate::types::ExtraVarName;
use collomatique_ilp::linexpr::LinExpr;

pub(crate) fn build(pairs: &PairData) -> MyBundle {
    // The expression is grown in place: `LinExpr`'s `+` clones the whole
    // coefficient map, which would make building the objective quadratic in
    // the number of extras — and there is one per site and per tier couple.
    let mut expr = LinExpr::constant(pairs.constant_term());

    for ((a, b), table) in pairs.pairs() {
        for tier in table {
            // The mass of a site depends on its group only through the target
            // size, so every site of a tier shares one coefficient.
            let coef = pairs.together_coefficient(a, b, tier.list, tier.target);
            for &group in &tier.groups {
                expr += coef
                    * LinExpr::var(extra_var(ExtraVarName::Together {
                        a,
                        b,
                        list: tier.list,
                        group,
                    }));
            }
        }
        for (first, second) in cross_tiers(table) {
            let coef = pairs.coincide_coefficient(a, b, first, second);
            expr += coef
                * LinExpr::var(extra_var(ExtraVarName::Coincide {
                    a,
                    b,
                    list1: first.list,
                    target1: first.target,
                    list2: second.list,
                    target2: second.target,
                }));
        }
    }

    // Emitted even when the plan declares no extra at all: the constant is a
    // real part of the score, and a model whose objective were absent there
    // would report 0 for a placement the greedy scores above 0.
    MyBundle::new().with_maximize(1.0, expr)
}
