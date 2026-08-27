//! The extra variables of the model: the linearization of the collision
//! objective.
//!
//! The objective is `Σ_s Σ_t P_s(t)²` with
//! `P_s(t) = c_{s→t} + Σ_sites m_s(site) · z_site` (`crate::pairs`), so the
//! model needs the square of a sum of binaries. Expanded, that is a constant,
//! one term per site and one term per *couple* of sites — and since every
//! `m_s` is a constant here, the only non-linear objects are the products
//! `z_i z_j`. Two families cover them:
//!
//! - [`ExtraVarName::Together`] is `z_site` itself, one per (pair, list,
//!   group) the enumeration kept;
//! - [`ExtraVarName::Coincide`] is the product, one per couple of *tiers* of
//!   two different lists. A tier's site binaries are mutually exclusive — one
//!   group per student per list — so their sum is 0/1 and one product variable
//!   stands for the whole block of site couples between two tiers.
//!
//! Declaration is lazy: `Modeler::build` only expands extras that are
//! (transitively) referenced by a constraint or the objective. Here the
//! objective references every declared variable of both families, and a
//! `Coincide` definition references the `Together` variables of its two tiers,
//! which the modeler's dependency graph resolves transitively. So declared,
//! expanded and valuated are the same set — which is what
//! [`group_lists_to_warm_start`](crate::group_lists_to_warm_start) needs, since
//! a configuration naming a variable the model did not expand is refused
//! wholesale.
//!
//! **One-sidedness.** Both families are defined from above only:
//! `Together ≤ x_a`, `Together ≤ x_b`, and `Coincide ≤ Σ Together(tier)` for
//! each of its two tiers. Nothing forces either up. That is the mirror image
//! of the retired `SharedPair` pattern — which was forced *up* under a
//! minimize — and it is sound for the same three reasons, transposed to a
//! maximize:
//!
//! - every declared variable has a **strictly positive** objective coefficient
//!   (the enumeration filters out the mass-0 lists and tiers, F1), so the
//!   maximizing objective pushes each to its upper bound, which is exactly
//!   `x_a · x_b` for a `Together` and the product of its two tier sums for a
//!   `Coincide`;
//! - no constraint references either family, so the defining rows are left out
//!   of the checker problem entirely (`generic/ilp-modeler/src/lib.rs`, the
//!   `for_constraints: true` filter) — the strategies that strip the objective
//!   never even see them;
//! - reported objective values stay exact anyway: every strategy recovers them
//!   through `Model::reconstruction_problem`, which re-optimizes the true
//!   objective with the base values fixed, and under the maximize the
//!   one-sided rows are tight.
//!
//! The only consumer of the crate, gtk4, also filters a solved configuration
//! down to the base variables before reading it
//! (`colloscopes/gtk4/src/editor/group_lists.rs`), so a floating extra value
//! never reaches a group list.

#[cfg(test)]
mod tests;

use crate::pairs::{PairData, cross_tiers};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp::Variable;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_ilp_modeler::bundle::{ExtraEntry, ReifyError};
use collomatique_ilp_modeler::{ExtraVar, IntConstraintBundle, Var as ModelerVar};

pub(crate) type V = ModelerVar<Var, ExtraVarName>;

/// The variable type *inside* an extra-definition closure: [`V`] plus the
/// helper case. The one-sided rows declare no helper — helper ids are not
/// externally addressable, and the warm start has to name every variable of
/// the model — but they still have to be written in this type.
type DefV = ExtraVar<Var, ExtraVarName>;
pub(crate) type MyBundle = IntConstraintBundle<
    'static,
    Var,
    ExtraVarName,
    ConstraintDesc,
    VarEnv,
    ReifyError<Var, ExtraVarName>,
>;

pub(crate) fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

pub(crate) fn extra_var(v: ExtraVarName) -> V {
    ModelerVar::Extra(v)
}

/// One `Together` per site of the enumeration: the pairs that can meet, in the
/// groups they can meet in.
fn build_together(pairs: &PairData) -> MyBundle {
    let mut bundle = MyBundle::new();
    for ((a, b), table) in pairs.pairs() {
        for tier in table {
            let list = tier.list;
            for &group in &tier.groups {
                bundle = bundle
                    .with_extra(
                        ExtraVarName::Together { a, b, list, group },
                        ExtraEntry::new(Variable::binary(), move |_helpers, _ctx, name| {
                            // `tog − x <= 0` for each of the two students:
                            // either one sitting elsewhere holds `tog` at 0,
                            // and nothing pushes it up but the objective,
                            // which pays for it (see the module doc).
                            let tog: IntLinExpr<DefV> = IntLinExpr::var(DefV::Extra(name));
                            let mut rows = Vec::new();
                            for student in [a, b] {
                                let x = IntLinExpr::var(DefV::Base(Var::StudentInGroup {
                                    list,
                                    student,
                                    group,
                                }));
                                rows.push(
                                    (tog.clone() - x)
                                        .leq(&IntLinExpr::constant(0))
                                        .into_constraint(),
                                );
                            }
                            Ok(rows)
                        }),
                    )
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

/// One `Coincide` per couple of tiers of two different lists: the products the
/// expanded square needs.
fn build_coincide(pairs: &PairData) -> MyBundle {
    let mut bundle = MyBundle::new();
    for ((a, b), table) in pairs.pairs() {
        for (first, second) in cross_tiers(table) {
            // The two tier sums the product is bounded by, captured by value:
            // the definition closure outlives this borrow of the enumeration.
            let sides = [
                (first.list, first.groups.clone()),
                (second.list, second.groups.clone()),
            ];
            bundle = bundle
                .with_extra(
                    ExtraVarName::Coincide {
                        a,
                        b,
                        list1: first.list,
                        target1: first.target,
                        list2: second.list,
                        target2: second.target,
                    },
                    ExtraEntry::new(Variable::binary(), move |_helpers, _ctx, name| {
                        // `w − Σ_{g in tier} Together(a, b, l, g) <= 0`, once
                        // per tier: the pair missing either tier holds `w` at
                        // 0. Each sum is 0/1 — one group per list — so the two
                        // rows together cap `w` at the product, which the
                        // maximizing objective then reaches.
                        let w: IntLinExpr<DefV> = IntLinExpr::var(DefV::Extra(name));
                        let mut rows = Vec::new();
                        for (list, groups) in &sides {
                            let mut side: IntLinExpr<DefV> = IntLinExpr::constant(0);
                            for &group in groups {
                                side += IntLinExpr::var(DefV::Extra(ExtraVarName::Together {
                                    a,
                                    b,
                                    list: *list,
                                    group,
                                }));
                            }
                            rows.push(
                                (w.clone() - side)
                                    .leq(&IntLinExpr::constant(0))
                                    .into_constraint(),
                            );
                        }
                        Ok(rows)
                    }),
                )
                .expect("no duplicate extras");
        }
    }
    bundle
}

pub(crate) fn build_extras(pairs: &PairData) -> MyBundle {
    build_together(pairs)
        .merge(build_coincide(pairs))
        .expect("no duplicate extras")
}
