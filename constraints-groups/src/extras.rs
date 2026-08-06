//! The extra variables of the model (piece 7 of the roadmap).
//!
//! Declaration is lazy: `Modeler::build` only expands extras that are
//! (transitively) referenced by a constraint or the objective, so a declared
//! extra that nothing references costs nothing in the built model.
//!
//! `SharedPair` is defined by *one-sided* rows — `shared ≥ x_a + x_b − 1`,
//! one per (list, group) the pair could meet in — so the variable is forced
//! up when the pair shares a group and merely left free when it does not.
//! The full equivalence the roadmap (§2.2) asked for would need one AND
//! indicator per (pair, list, group), which was the largest block of the
//! model by far.
//!
//! `RefGroupInGroup` is one-sided in the same direction as `SharedPair` and
//! for the same reasons: `x_s − u ≤ 0` for every member of the reference
//! group that belongs to the list, so any of them sitting in that group of
//! the list forces `u` up, and nothing brings it back down but the objective,
//! which pays for it. Under the minimize each therefore lands on its tight
//! value — "this reference group has a member here" — and the sum over the
//! list's groups is exactly the number of pieces the list breaks the
//! reference group into. The template grouping itself is no longer a
//! variable: it is computed by [`crate::ghost`] and read here as data.
//!
//! One-sidedness is sound because nothing ever reads the variable outside a
//! minimizing objective:
//!
//! - the only consumer of the crate, gtk4, filters a solved configuration
//!   down to the base variables before reading it
//!   (`gtk4/src/editor/group_lists.rs`), so a floating extra value never
//!   reaches a group list;
//! - no constraint references `SharedPair`, so the defining rows are left
//!   out of the checker problem entirely (`ilp-modeler/src/lib.rs`, the
//!   `for_constraints: true` filter) — the strategies that strip the
//!   objective never even see them;
//! - reported objective values stay exact anyway: every strategy recovers
//!   them through `Model::reconstruction_problem`, which re-minimizes the
//!   true objective with the base values fixed, and under a minimize the
//!   one-sided rows are tight.

use crate::specs::pairs_of;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{GroupListIdx, RefGroupIdx, SizeClassIdx, Var, VarEnv};
use collomatique_ilp::Variable;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_ilp_modeler::bundle::{ExtraEntry, ReifyError};
use collomatique_ilp_modeler::{ExtraVar, IntConstraintBundle, Var as ModelerVar};
use collomatique_state_colloscopes::StudentId;
use std::collections::BTreeMap;

pub(crate) type V = ModelerVar<Var, ExtraVarName>;

/// The variable type *inside* an extra-definition closure: [`V`] plus the
/// helper case. The one-sided rows declare no helper, but they still have to
/// be written in this type.
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

/// Which pairs co-occur, in which size class, and in which lists of it. A
/// pair gets a `SharedPair` variable per class this map has an entry for
/// (roadmap §2.2, split per class since the size-class objective). The
/// objective (piece 9) must sum over exactly that set — referencing an
/// undeclared extra is a build error — so both read this one function.
///
/// Classes of maximum size 1 are skipped: their groups hold one student, so
/// no pair can ever meet there and the variable would be vacuously 0.
pub(crate) fn co_occurrences(
    env: &VarEnv,
) -> BTreeMap<(StudentId, StudentId, SizeClassIdx), Vec<GroupListIdx>> {
    let mut map: BTreeMap<(StudentId, StudentId, SizeClassIdx), Vec<GroupListIdx>> =
        BTreeMap::new();
    for list in env.lists() {
        let class = env.class_of(list);
        if env.class_range(class).end().get() == 1 {
            continue;
        }
        for (a, b) in pairs_of(env.students(list)) {
            map.entry((a, b, class)).or_default().push(list);
        }
    }
    map
}

/// The (list, reference group) sites that get a piece count: those whose
/// intersection is non-empty. A reference group no member of the list belongs
/// to would be a vacuous 0 in every group of the list and is not declared.
///
/// Lists whose groups hold a single student are skipped. Their piece count is
/// the same whatever the model does — one piece per member — so the whole
/// block would be a constant, and their size class is the one
/// [`VarEnv::class_weight`] has no meaningful value for.
///
/// Empty without a template ([`VarEnv::ghost`]), since there is then no
/// reference group at all — so the whole family, and the objective term that
/// reads it, self-gate on this one function.
pub(crate) fn scatter_sites(env: &VarEnv) -> Vec<(GroupListIdx, RefGroupIdx)> {
    let mut sites = Vec::new();
    for list in env.lists() {
        if env.max_size(list) == 1 {
            continue;
        }
        let students = env.students(list);
        for ref_group in env.ref_groups() {
            if env
                .ref_group(ref_group)
                .iter()
                .any(|s| students.contains(s))
            {
                sites.push((list, ref_group));
            }
        }
    }
    sites
}

fn build_ref_group_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (list, ref_group) in scatter_sites(env) {
        // Only the members the list actually holds — reference groups span
        // the union of the specs, so they routinely straddle several lists.
        // A row for an outsider would be harmless (the name is stale, hence
        // fixed to 0, so the row reads `0 <= u`), just pointless.
        let members: Vec<StudentId> = env
            .ref_group(ref_group)
            .iter()
            .copied()
            .filter(|s| env.students(list).contains(s))
            .collect();
        for group in 0..env.group_count(list) {
            let members = members.clone();
            bundle = bundle
                .with_extra(
                    ExtraVarName::RefGroupInGroup {
                        list,
                        ref_group,
                        group,
                    },
                    ExtraEntry::new(Variable::binary(), move |_helpers, _ctx, name| {
                        // `x_s − u <= 0` for every member of the reference
                        // group that belongs to this list: any of them
                        // sitting in this group forces `u` up. Nothing pushes
                        // it back down but the objective, which pays for it —
                        // the same one-sidedness as `SharedPair`, and sound
                        // for the same reasons.
                        let piece: IntLinExpr<DefV> = IntLinExpr::var(DefV::Extra(name));
                        let mut rows = Vec::new();
                        for &s in &members {
                            let x = IntLinExpr::var(DefV::Base(Var::StudentInGroup {
                                list,
                                student: s,
                                group,
                            }));
                            rows.push(
                                (x - piece.clone())
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

fn build_shared_pair(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for ((a, b, class), lists) in co_occurrences(env) {
        let var = ExtraVarName::SharedPair { a, b, class };
        if env.pinned_pairs(class).contains(&(a, b)) {
            // The pair already shares a group in a kept list of this class,
            // so it costs nothing there whatever the model does: the
            // variable is pinned to 1
            // (an empty conjunction reifies to `indicator = 1`) and no
            // defining row is emitted at all. This is the extras equivalent
            // of the base-variable fixer, which cannot apply here — the
            // fixer chain is base-only.
            bundle = bundle
                .and_reified(var, move || vec![])
                .expect("no duplicate extras");
        } else {
            let terms: Vec<(GroupListIdx, u32)> = lists
                .iter()
                .map(|&list| (list, env.group_count(list)))
                .collect();
            bundle = bundle
                .with_extra(
                    var,
                    ExtraEntry::new(Variable::binary(), move |_helpers, _ctx, name| {
                        // `x_a + x_b − shared <= 1` for every (list, group)
                        // the pair could meet in: the pair sharing a group
                        // forces `shared` up, and nothing forces it back
                        // down but the objective (see the module doc).
                        let shared: IntLinExpr<DefV> = IntLinExpr::var(DefV::Extra(name));
                        let mut rows = Vec::new();
                        for &(list, groups) in &terms {
                            for group in 0..groups {
                                let xa = IntLinExpr::var(DefV::Base(Var::StudentInGroup {
                                    list,
                                    student: a,
                                    group,
                                }));
                                let xb = IntLinExpr::var(DefV::Base(Var::StudentInGroup {
                                    list,
                                    student: b,
                                    group,
                                }));
                                rows.push(
                                    (xa + xb - shared.clone())
                                        .leq(&IntLinExpr::constant(1))
                                        .into_constraint(),
                                );
                            }
                        }
                        Ok(rows)
                    }),
                )
                .expect("no duplicate extras");
        }
    }
    bundle
}

pub(crate) fn build_extras(env: &VarEnv) -> MyBundle {
    build_shared_pair(env)
        .merge(build_ref_group_in_group(env))
        .expect("no duplicate extras")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::MyModeler;
    use crate::specs::GenerationPlan;
    use crate::specs::tests::{range, set, student};
    use crate::vars::tests::plan_of;
    use collomatique_ilp::ConfigData;
    use collomatique_ilp::linexpr::LinExpr;
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp_modeler::{InternalVar, Modeler};

    /// Apply the extras to a fresh modeler, maximize each weighted term,
    /// build (lazily — the objective is what forces the expansion), solve,
    /// and return every variable of the solution, extras included.
    ///
    /// The "exactly one group per student" family comes along: the base
    /// binaries only mean "the placement of the students" under it, and it
    /// used to be the domain of the retired integer base variable. The size
    /// constraints stay out — this harness places students by hand.
    fn solve_with_objective(
        plan: &GenerationPlan,
        terms: &[(f64, V)],
    ) -> ConfigData<InternalVar<Var, ExtraVarName>> {
        let env = VarEnv::new(plan);
        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(build_extras(&env).into_general())
            .expect("no duplicate extras");
        modeler
            .apply_bundle(crate::constraints::build_student_in_one_group(&env).into_general())
            .expect("no duplicate extras");
        for (weight, var) in terms {
            // The weight goes into the `LinExpr`, before the sense is
            // applied. `maximize`'s own `coef` scales the finished
            // `Objective` instead, and scaling an `Objective` by a negative
            // number reverses its sense too (`ilp/src/objectives.rs:128`),
            // so a negative weight there would reward the term rather than
            // penalize it.
            modeler.maximize(1.0, *weight * LinExpr::var(var.clone()));
        }
        let model = modeler.build(&env).expect("build should succeed");
        let solution = model
            .solve(&ColloCbcSolver::with_disable_logging(true))
            .expect("model should be solvable");
        solution.get_complete_data()
    }

    /// A weight-100 term placing `student` in `group` of `list` — the base
    /// binary itself, at a weight far above the ±1 adversarial ones, so the
    /// placement never bends.
    fn place(list: usize, s: u64, group: u32) -> (f64, V) {
        (
            100.0,
            base_var(Var::StudentInGroup {
                list: GroupListIdx(list),
                student: student(s),
                group,
            }),
        )
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

    fn value(cfg: &ConfigData<InternalVar<Var, ExtraVarName>>, var: V) -> f64 {
        cfg.get(var.clone())
            .unwrap_or_else(|| panic!("{:?} should be part of the solved problem", var))
    }

    #[test]
    fn declarations_expand_cleanly() {
        // Two overlapping lists of the same size class: 1 and 2 are in both,
        // 3 and 4 only in the first, 5 and 6 only in the second.
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 5, 6], (2, 2))]);
        plan.pinned_pairs = [(
            range(2, 2),
            [(student(1), student(2))].into_iter().collect(),
        )]
        .into_iter()
        .collect();
        let env = VarEnv::new(&plan);

        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(build_extras(&env).into_general())
            .expect("no duplicate extras");

        // `build_full` force-expands every declared extra, so any
        // definition referencing a name that was never declared (an index
        // mismatch between the families, say), any cycle, and any
        // duplicate declaration surfaces here in one sweep.
        let model = modeler
            .build_full(&env)
            .expect("every declared extra should expand");
        let vars = model.problem().get_variables();

        let shared = |a: u64, b: u64, class: usize| {
            InternalVar::<Var, ExtraVarName>::Extra(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
                class: SizeClassIdx(class),
            })
        };
        // 1 and 2 co-occur in both lists (and are pinned), 1 and 5 only in
        // the second one: both get a variable of the single class.
        assert!(vars.contains_key(&shared(1, 2, 0)));
        assert!(vars.contains_key(&shared(1, 5, 0)));
        // 3 and 5 never share a spec, so the pair is not declared at all.
        assert!(!vars.contains_key(&shared(3, 5, 0)));
    }

    #[test]
    fn a_pair_gets_one_variable_per_size_class() {
        // The same four students grouped two ways — by pairs and by threes —
        // plus a class of singletons, where nobody can meet anybody.
        let plan = plan_of(&[
            (&[1, 2, 3, 4], (1, 1)),
            (&[1, 2, 3, 4], (2, 2)),
            (&[1, 2, 3, 4], (3, 4)),
        ]);
        let env = VarEnv::new(&plan);

        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(build_extras(&env).into_general())
            .expect("no duplicate extras");
        let model = modeler
            .build_full(&env)
            .expect("every declared extra should expand");
        let vars = model.problem().get_variables();

        let shared = |a: u64, b: u64, class: usize| {
            InternalVar::<Var, ExtraVarName>::Extra(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
                class: SizeClassIdx(class),
            })
        };
        // Classes are the sorted distinct ranges: 1..=1, 2..=2, 3..=4. The
        // pair meets in the last two and is a separate variable in each, so
        // meeting in one never pays for the other.
        assert!(vars.contains_key(&shared(1, 2, 1)));
        assert!(vars.contains_key(&shared(1, 2, 2)));
        // Groups of one hold no pair at all: the variable would be a
        // vacuous 0 and is not declared.
        assert!(!vars.contains_key(&shared(1, 2, 0)));
    }

    #[test]
    fn a_shared_group_forces_the_pair_up() {
        // Two lists of 4 students with fixed size 2, hence 2 groups each.
        //
        // Only the ≥ direction can be tested by an adversary: the rows are
        // one-sided, so pushing a *non*-sharing pair up would simply
        // succeed. That the value comes back down to 0 when the pair does
        // not share is a property of the minimizing objective, pinned by
        // `objective.rs`'s tests instead.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 5, 6], (2, 2))]);

        // Both lists have the range 2..=2, hence the single class 0.
        let shared = |a: u64, b: u64| {
            extra_var(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
                class: SizeClassIdx(0),
            })
        };

        let cfg = solve_with_objective(
            &plan,
            &[
                // List 0: {1, 2} and {3, 4}. List 1: {1, 5} and {2, 6}.
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                place(1, 5, 0),
                place(1, 2, 1),
                place(1, 6, 1),
                // Adversarial: 1 and 2 share the first group of list 0 and
                // 1 and 5 the first group of list 1, in both cases against
                // the push. 1 and 6 co-occur but never share a group, so
                // nothing holds them up and the same push wins there —
                // which is what tells the two directions apart.
                (-1.0, shared(1, 2)),
                (-1.0, shared(1, 5)),
                (-1.0, shared(1, 6)),
            ],
        );

        assert_eq!(value(&cfg, shared(1, 2)), 1.0);
        assert_eq!(value(&cfg, shared(1, 5)), 1.0);
        assert_eq!(value(&cfg, shared(1, 6)), 0.0);
    }

    #[test]
    fn pinned_pair_is_free_even_when_never_sharing() {
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 5, 6], (2, 2))]);
        plan.pinned_pairs = [(
            range(2, 2),
            [(student(1), student(2))].into_iter().collect(),
        )]
        .into_iter()
        .collect();

        let shared = |a: u64, b: u64| {
            extra_var(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
                class: SizeClassIdx(0),
            })
        };

        let cfg = solve_with_objective(
            &plan,
            &[
                // 1 and 2 are separated in *both* lists: list 0 is
                // {1, 3} / {2, 4} and list 1 is {1, 5} / {2, 6}.
                place(0, 1, 0),
                place(0, 3, 0),
                place(0, 2, 1),
                place(0, 4, 1),
                place(1, 1, 0),
                place(1, 5, 0),
                place(1, 2, 1),
                place(1, 6, 1),
                // Every pair is pushed down. The pinned one and the two
                // sharing ones must resist; 3 and 4 are separated, so the
                // push wins there and the pin is shown to be a property of
                // the pair rather than of the whole family. (Every extra an
                // assertion reads must be referenced here: expansion is
                // lazy, so an unreferenced extra is not a variable of the
                // built problem at all.)
                (-1.0, shared(1, 2)),
                (-1.0, shared(1, 5)),
                (-1.0, shared(1, 3)),
                (-1.0, shared(3, 4)),
            ],
        );

        // The pin holds against both the placement and the objective:
        // grouping such a pair again costs nothing, which is the whole
        // point of pinning (roadmap §2.2).
        assert_eq!(value(&cfg, shared(1, 2)), 1.0);
        // A non-pinned pair of the same model still follows the placement.
        assert_eq!(value(&cfg, shared(1, 5)), 1.0);
        assert_eq!(value(&cfg, shared(1, 3)), 1.0);
        assert_eq!(value(&cfg, shared(3, 4)), 0.0);
    }

    #[test]
    fn a_member_in_a_group_forces_the_piece_up() {
        // Six students at 2..=2, so the list has three groups and the
        // computed template three reference groups — {1, 2}, {3, 4} and
        // {5, 6}, since nothing in this plan tells the pairs apart and the
        // clustering breaks ties by student id.
        //
        // The list is placed as {1, 2} / {3, 5} / {4, 6}: reference group 0
        // stays whole in one group, while reference groups 1 and 2 are each
        // cut in half.
        //
        // Only the ≥ direction can be tested by an adversary, mirroring
        // `SharedPair`: the rows are one-sided, so every piece is pushed
        // *down* — the direction the objective pushes it in — and whichever
        // comes back at 1 was held up by its defining rows alone.
        let plan = plan_of(&[(&[1, 2, 3, 4, 5, 6], (2, 2))]);

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 5, 1),
                place(0, 4, 2),
                place(0, 6, 2),
                (-1.0, piece(0, 0, 0)),
                (-1.0, piece(0, 0, 1)),
                (-1.0, piece(0, 1, 1)),
                (-1.0, piece(0, 1, 2)),
            ],
        );

        // Reference group 0 is whole in the list's group 0: one piece there,
        // and nothing holds it up anywhere else.
        assert_eq!(value(&cfg, piece(0, 0, 0)), 1.0);
        assert_eq!(value(&cfg, piece(0, 0, 1)), 0.0);
        // Reference group 1 is {3, 4}, and the list separates them: both
        // indicators are held up, which is the two-piece count the objective
        // charges for.
        assert_eq!(value(&cfg, piece(0, 1, 1)), 1.0);
        assert_eq!(value(&cfg, piece(0, 1, 2)), 1.0);
    }

    #[test]
    fn a_reference_group_is_counted_over_the_members_the_list_holds() {
        // Reference groups span the union of the specs, so one routinely
        // straddles several lists. Here the two lists overlap in student 4
        // alone, and the clustering — which follows the pair (4, ·) links
        // and breaks the rest by id — makes the reference groups
        // {1, 2, 4}, {3, 5} and {6, 7}.
        //
        // List 1 is {4, 5, 6, 7}, so it holds one member of reference group
        // 0 and one of reference group 1. Placed as {4, 5} / {6, 7}, it
        // keeps every reference group it touches in a single piece — the
        // members it does *not* hold are none of its business.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[4, 5, 6, 7], (2, 3))]);
        assert_eq!(
            plan.ghost
                .as_ref()
                .expect("this plan has a template")
                .groups(),
            [set(&[1, 2, 4]), set(&[3, 5]), set(&[6, 7])],
        );

        let cfg = solve_with_objective(
            &plan,
            &[
                place(1, 4, 0),
                place(1, 5, 0),
                place(1, 6, 1),
                place(1, 7, 1),
                (-1.0, piece(1, 0, 0)),
                (-1.0, piece(1, 0, 1)),
                (-1.0, piece(1, 1, 0)),
            ],
        );

        // Student 4 is the only member of reference group 0 that list 1
        // holds, and it sits in group 0.
        assert_eq!(value(&cfg, piece(1, 0, 0)), 1.0);
        assert_eq!(value(&cfg, piece(1, 0, 1)), 0.0);
        // Same for student 5 and reference group 1: one piece, not two,
        // even though student 3 is elsewhere entirely.
        assert_eq!(value(&cfg, piece(1, 1, 0)), 1.0);
    }

    #[test]
    fn a_reference_group_outside_the_list_is_not_declared() {
        // Two disjoint lists of the same range. The template spans their
        // union — eight students in four reference groups of two, which the
        // tie-breaking clustering makes {1, 2}, {3, 4}, {5, 6}, {7, 8} — so
        // each list meets exactly two of them.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[5, 6, 7, 8], (2, 2))]);
        let env = VarEnv::new(&plan);

        assert_eq!(
            scatter_sites(&env),
            vec![
                (GroupListIdx(0), RefGroupIdx(0)),
                (GroupListIdx(0), RefGroupIdx(1)),
                (GroupListIdx(1), RefGroupIdx(2)),
                (GroupListIdx(1), RefGroupIdx(3)),
            ],
        );
    }

    #[test]
    fn a_plan_without_a_template_declares_no_template_extras() {
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        plan.ghost = None;
        let env = VarEnv::new(&plan);
        assert!(scatter_sites(&env).is_empty());

        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(build_extras(&env).into_general())
            .expect("no duplicate extras");
        let model = modeler
            .build_full(&env)
            .expect("every declared extra should expand");
        let vars = model.problem().get_variables();

        // The `SharedPair` family is unaffected — only the template ones go.
        assert!(vars.contains_key(&InternalVar::<Var, ExtraVarName>::Extra(
            ExtraVarName::SharedPair {
                a: student(1),
                b: student(2),
                class: SizeClassIdx(0),
            }
        )));
        assert!(
            vars.keys()
                .all(|v| !matches!(v, InternalVar::Extra(ExtraVarName::RefGroupInGroup { .. })))
        );
    }

    #[test]
    fn single_group_list_forces_the_shared_pair() {
        // Three students in groups of 3 to 4: ceil(3 / 4) is a single
        // group, so the only term of each student's "exactly one group" row
        // is `group 0` and the placement is forced by that row alone.
        let plan = plan_of(&[(&[1, 2, 3], (3, 4))]);
        let list = GroupListIdx(0);

        let in_group = base_var(Var::StudentInGroup {
            list,
            student: student(1),
            group: 0,
        });
        let shared = extra_var(ExtraVarName::SharedPair {
            a: student(1),
            b: student(2),
            class: SizeClassIdx(0),
        });

        // Only adversarial terms: the placement and the pair are both
        // pushed toward 0, and the forced placement must still propagate
        // through the defining row.
        let cfg = solve_with_objective(&plan, &[(-1.0, in_group.clone()), (-1.0, shared.clone())]);

        assert_eq!(value(&cfg, in_group), 1.0);
        assert_eq!(value(&cfg, shared), 1.0);
    }
}
