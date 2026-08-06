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
//! The template families are one-sided too, in the two opposite directions.
//! `CanonicalPair` is capped from above by its defining rows (a template
//! group holding exactly one of the pair forces it to 0) and pushed up only
//! by the objective, which gains by it: a high `t` relaxes the `Deviation`
//! rows, and the affinity term rewards it outright. That reward is the same
//! direction as the relief, so it changes nothing here beyond making the cap
//! tight for every pair — including one the template groups but that meets in
//! no list, which the relief alone would leave undecided. `Deviation` is
//! floored from below by its rows (a meeting of a
//! non-template pair forces it to 1) and pushed down only by the objective,
//! which pays for it. Each therefore lands on its tight value under the
//! minimize, and neither is ever read anywhere else. `Deviation` references
//! `CanonicalPair`, an extra referencing an extra — the lazy expansion
//! follows such chains transitively, so declaring `Deviation` is enough to
//! bring `CanonicalPair` into the built model.
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
use crate::vars::{GroupListIdx, SizeClassIdx, Var, VarEnv};
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

/// The (pair, list) sites that get a [`ExtraVarName::Deviation`] variable, and
/// whose keys are the pairs that get a [`ExtraVarName::CanonicalPair`] one:
/// the co-occurrence sites of [`co_occurrences`] with the size class dropped.
/// A deviation is paid once per list a pair meets in, whatever the class,
/// because the template is a single grouping of everybody — the class only
/// enters through the weight the objective puts on the site.
///
/// A list belongs to exactly one class, so merging the class-keyed entries of
/// a pair never repeats a list. Every pair here also lies inside the template:
/// the template spans the union of the specs' students, so two students who
/// co-occur in a spec both have a template group.
///
/// Empty without a template ([`VarEnv::ghost`]) — the whole family is then
/// absent from the extras and from the objective alike, both of which read
/// this one function.
pub(crate) fn deviation_sites(env: &VarEnv) -> BTreeMap<(StudentId, StudentId), Vec<GroupListIdx>> {
    let mut map: BTreeMap<(StudentId, StudentId), Vec<GroupListIdx>> = BTreeMap::new();
    if env.ghost().is_none() {
        return map;
    }
    for ((a, b, _class), lists) in co_occurrences(env) {
        map.entry((a, b)).or_default().extend(lists);
    }
    for lists in map.values_mut() {
        lists.sort();
    }
    map
}

fn build_canonical_pair(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    let ghost_groups = env.ghost_group_count();
    for (a, b) in deviation_sites(env).into_keys() {
        bundle = bundle
            .with_extra(
                ExtraVarName::CanonicalPair { a, b },
                ExtraEntry::new(Variable::binary(), move |_helpers, _ctx, name| {
                    // Two rows per template group `g`:
                    //   `t + y_a − y_b <= 1` and `t − y_a + y_b <= 1`.
                    // A group holding exactly one of the two makes one of them
                    // read `t + 1 <= 1`, so `t` is capped at 0; a group
                    // holding both, or neither, leaves it free. Every student
                    // sits in exactly one template group, so `t` may reach 1
                    // exactly when the template groups the pair.
                    let canonical: IntLinExpr<DefV> = IntLinExpr::var(DefV::Extra(name));
                    let mut rows = Vec::new();
                    for group in 0..ghost_groups {
                        let ya = IntLinExpr::var(DefV::Base(Var::StudentInGhostGroup {
                            student: a,
                            group,
                        }));
                        let yb = IntLinExpr::var(DefV::Base(Var::StudentInGhostGroup {
                            student: b,
                            group,
                        }));
                        rows.push(
                            (canonical.clone() + ya.clone() - yb.clone())
                                .leq(&IntLinExpr::constant(1))
                                .into_constraint(),
                        );
                        rows.push(
                            (canonical.clone() - ya + yb)
                                .leq(&IntLinExpr::constant(1))
                                .into_constraint(),
                        );
                    }
                    Ok(rows)
                }),
            )
            .expect("no duplicate extras");
    }
    bundle
}

fn build_deviation(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for ((a, b), lists) in deviation_sites(env) {
        for list in lists {
            let groups = env.group_count(list);
            bundle = bundle
                .with_extra(
                    ExtraVarName::Deviation { a, b, list },
                    ExtraEntry::new(Variable::binary(), move |_helpers, _ctx, name| {
                        // `x_a + x_b − t − p <= 1` for every group of the
                        // list: meeting there while the pair is not a
                        // template pair reads `2 − 0 − p <= 1` and forces `p`
                        // up. A template pair (`t = 1`) is excused, and so is
                        // a group where they do not meet.
                        let deviation: IntLinExpr<DefV> = IntLinExpr::var(DefV::Extra(name));
                        let canonical: IntLinExpr<DefV> =
                            IntLinExpr::var(DefV::Extra(ExtraVarName::CanonicalPair { a, b }));
                        let mut rows = Vec::new();
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
                                (xa + xb - canonical.clone() - deviation.clone())
                                    .leq(&IntLinExpr::constant(1))
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
        .merge(build_canonical_pair(env))
        .expect("no duplicate extras")
        .merge(build_deviation(env))
        .expect("no duplicate extras")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::MyModeler;
    use crate::specs::GenerationPlan;
    use crate::specs::tests::{range, student};
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

    /// The same, for the template grouping. The harness leaves the size
    /// constraints out, so the template — like every list here — is placed
    /// entirely by these pushes, under its "exactly one group" rows alone.
    fn place_ghost(s: u64, group: u32) -> (f64, V) {
        (
            100.0,
            base_var(Var::StudentInGhostGroup {
                student: student(s),
                group,
            }),
        )
    }

    fn canonical(a: u64, b: u64) -> V {
        extra_var(ExtraVarName::CanonicalPair {
            a: student(a),
            b: student(b),
        })
    }

    fn deviation(a: u64, b: u64, list: usize) -> V {
        extra_var(ExtraVarName::Deviation {
            a: student(a),
            b: student(b),
            list: GroupListIdx(list),
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
    fn the_template_caps_the_canonical_pair() {
        // Four students at 2..=2, so the template has two groups. Placed by
        // hand as {1, 3} / {2, 4}.
        //
        // Only the ≤ direction can be tested by an adversary, mirroring
        // `SharedPair`: the rows are one-sided the other way round, so a pair
        // the template *does* group is simply free to rise. Both cases show
        // up here — the separated pair must fall against its push, the
        // grouped one must follow it.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);

        let cfg = solve_with_objective(
            &plan,
            &[
                place_ghost(1, 0),
                place_ghost(3, 0),
                place_ghost(2, 1),
                place_ghost(4, 1),
                // Both pushed *up*, which is the direction the objective
                // pushes them in the real model.
                (1.0, canonical(1, 2)),
                (1.0, canonical(1, 3)),
            ],
        );

        assert_eq!(value(&cfg, canonical(1, 2)), 0.0);
        assert_eq!(value(&cfg, canonical(1, 3)), 1.0);
    }

    #[test]
    fn a_non_canonical_meeting_forces_the_deviation_up() {
        // Six students at 2..=2: three groups in the list and three in the
        // template. The list is placed as {1, 2} / {3, 4} / {5, 6} and the
        // template as {1, 2} / {3, 5} / {4, 6}, so the three cases the
        // deviation must tell apart are all present at once.
        //
        // Every deviation is pushed *down*, the direction the objective
        // pushes it in: whichever one comes back at 1 was held up by its
        // defining rows and by nothing else.
        let plan = plan_of(&[(&[1, 2, 3, 4, 5, 6], (2, 2))]);

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 1),
                place(0, 4, 1),
                place(0, 5, 2),
                place(0, 6, 2),
                place_ghost(1, 0),
                place_ghost(2, 0),
                place_ghost(3, 1),
                place_ghost(5, 1),
                place_ghost(4, 2),
                place_ghost(6, 2),
                (-1.0, deviation(1, 2, 0)),
                (-1.0, deviation(3, 4, 0)),
                (-1.0, deviation(3, 5, 0)),
            ],
        );

        // 1 and 2 meet in the list *and* in the template: the meeting
        // follows the template, so nothing holds the deviation up.
        assert_eq!(value(&cfg, deviation(1, 2, 0)), 0.0);
        // 3 and 4 meet in the list while the template separates them: this
        // is the deviation, and the push cannot bring it down.
        assert_eq!(value(&cfg, deviation(3, 4, 0)), 1.0);
        // 3 and 5 are a template pair the list does not group. No meeting,
        // no deviation — a pair may differ from the template for free as
        // long as it is the list that stays apart.
        assert_eq!(value(&cfg, deviation(3, 5, 0)), 0.0);
    }

    #[test]
    fn a_plan_without_a_template_declares_no_template_extras() {
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        plan.ghost = None;
        let env = VarEnv::new(&plan);
        assert!(deviation_sites(&env).is_empty());

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
        assert!(vars.keys().all(|v| !matches!(
            v,
            InternalVar::Extra(ExtraVarName::CanonicalPair { .. })
                | InternalVar::Extra(ExtraVarName::Deviation { .. })
        )));
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
