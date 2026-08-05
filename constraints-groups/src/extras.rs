//! The reified extra variables of the model (piece 7 of the roadmap).
//!
//! Declaration is lazy: `Modeler::build` only expands extras that are
//! (transitively) referenced by a constraint or the objective, so until
//! pieces 8-9 reference them, applying this bundle leaves the built model
//! unchanged.
//!
//! Every reification is a full equivalence (roadmap §2.2): several solve
//! strategies strip the objective, so a one-sided implication would let the
//! solver report a wrong indicator value.

use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{GroupListIdx, Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{IntConstraintBundle, Var as ModelerVar};
use collomatique_state_colloscopes::StudentId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type V = ModelerVar<Var, ExtraVarName>;
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

/// The pairs `(a, b)` with `a < b` of a student set, in order. `BTreeSet`
/// iteration is sorted, so taking the members in order and pairing each with
/// its successors guarantees `a < b`.
fn pairs_of(students: &BTreeSet<StudentId>) -> Vec<(StudentId, StudentId)> {
    let members: Vec<StudentId> = students.iter().copied().collect();
    let mut pairs = Vec::new();
    for (i, &a) in members.iter().enumerate() {
        for &b in &members[i + 1..] {
            pairs.push((a, b));
        }
    }
    pairs
}

fn build_student_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        for group in 0..env.slot_count(list) {
            for &student in env.students(list) {
                let var = ExtraVarName::StudentInGroup {
                    list,
                    student,
                    group,
                };
                bundle = bundle
                    .and_reified(var, move || {
                        let expr = IntLinExpr::var(base_var(Var::StudentGroup { list, student }));
                        vec![expr.eq(&IntLinExpr::constant(group as i64))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_group_has_students(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        for group in 0..env.slot_count(list) {
            let students: Vec<StudentId> = env.students(list).iter().copied().collect();
            let var = ExtraVarName::GroupHasStudents { list, group };
            bundle = bundle
                .and_reified(var, move || {
                    let sum: IntLinExpr<V> = students
                        .iter()
                        .map(|&student| {
                            IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                                list,
                                student,
                                group,
                            }))
                        })
                        .sum();
                    vec![sum.geq(&IntLinExpr::constant(1))]
                })
                .expect("no duplicate extras");
        }
    }
    bundle
}

fn build_pair_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        for (a, b) in pairs_of(env.students(list)) {
            for group in 0..env.slot_count(list) {
                let var = ExtraVarName::PairInGroup { a, b, list, group };
                bundle = bundle
                    .and_reified(var, move || {
                        // Both operands are binary, so `x_a + x_b >= 2` is
                        // their AND, reified as a single constraint with no
                        // helper column (cheaper than the two-constraint
                        // form of the sibling crate, and the same full
                        // equivalence). This is by far the largest family,
                        // so the saving matters.
                        let sa = IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                            list,
                            student: a,
                            group,
                        }));
                        let sb = IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                            list,
                            student: b,
                            group,
                        }));
                        vec![(sa + sb).geq(&IntLinExpr::constant(2))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_shared_pair(env: &VarEnv) -> MyBundle {
    // Which pairs co-occur, and in which lists. A pair gets a variable iff
    // this map has an entry for it (roadmap §2.2).
    let mut co_occurrences: BTreeMap<(StudentId, StudentId), Vec<GroupListIdx>> = BTreeMap::new();
    for list in env.lists() {
        for pair in pairs_of(env.students(list)) {
            co_occurrences.entry(pair).or_default().push(list);
        }
    }

    let mut bundle = MyBundle::new();
    for ((a, b), lists) in co_occurrences {
        let var = ExtraVarName::SharedPair { a, b };
        if env.pinned_pairs().contains(&(a, b)) {
            // The pair already shares a group in a kept list, so the OR
            // holds by a constant term: the variable is pinned to 1 (an
            // empty conjunction reifies to `indicator = 1`) and the
            // degenerate `PairInGroup` rows are omitted entirely. This is
            // the extras equivalent of the base-variable fixer, which
            // cannot apply here — the fixer chain is base-only.
            bundle = bundle
                .and_reified(var, move || vec![])
                .expect("no duplicate extras");
        } else {
            let terms: Vec<(GroupListIdx, u32)> = lists
                .iter()
                .map(|&list| (list, env.slot_count(list)))
                .collect();
            bundle = bundle
                .and_reified(var, move || {
                    let sum: IntLinExpr<V> = terms
                        .iter()
                        .flat_map(|&(list, slots)| {
                            (0..slots).map(move |group| {
                                IntLinExpr::var(extra_var(ExtraVarName::PairInGroup {
                                    a,
                                    b,
                                    list,
                                    group,
                                }))
                            })
                        })
                        .sum();
                    vec![sum.geq(&IntLinExpr::constant(1))]
                })
                .expect("no duplicate extras");
        }
    }
    bundle
}

pub(crate) fn build_extras(env: &VarEnv) -> MyBundle {
    let bundle = build_student_in_group(env);
    let bundle = bundle
        .merge(build_group_has_students(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_pair_in_group(env))
        .expect("no duplicate extras");
    bundle
        .merge(build_shared_pair(env))
        .expect("no duplicate extras")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::MyModeler;
    use crate::specs::GenerationPlan;
    use crate::specs::tests::student;
    use crate::vars::tests::plan_of;
    use collomatique_ilp::ConfigData;
    use collomatique_ilp::linexpr::LinExpr;
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp_modeler::{InternalVar, Modeler};

    /// Apply the extras to a fresh modeler, maximize each weighted term,
    /// build (lazily — the objective is what forces the expansion), solve,
    /// and return every variable of the solution, extras included.
    fn solve_with_objective(
        plan: &GenerationPlan,
        terms: &[(f64, V)],
    ) -> ConfigData<InternalVar<Var, ExtraVarName>> {
        let env = VarEnv::new(plan);
        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(build_extras(&env).into_general())
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

    /// A weight-100 term placing `student` in `group` of `list`. Maximizing
    /// a full-equivalence indicator forces the base equality, and 100 is
    /// far above the ±1 adversarial weights, so the placement never bends.
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

    #[test]
    fn declarations_expand_cleanly() {
        // Two overlapping lists: 1 and 2 are in both, 3 and 4 only in the
        // first, 5 and 6 only in the second.
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 5, 6], (2, 2))]);
        plan.pinned_pairs = [(student(1), student(2))].into_iter().collect();
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

        let shared = |a: u64, b: u64| {
            InternalVar::<Var, ExtraVarName>::Extra(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
            })
        };
        // 1 and 2 co-occur in both lists (and are pinned), 1 and 5 only in
        // the second one: both get a variable.
        assert!(vars.contains_key(&shared(1, 2)));
        assert!(vars.contains_key(&shared(1, 5)));
        // 3 and 5 never share a spec, so the pair is not declared at all.
        assert!(!vars.contains_key(&shared(3, 5)));
    }

    #[test]
    fn student_in_group_and_group_has_students() {
        // One list of 3 students with minimum size 1, hence 3 slots.
        let plan = plan_of(&[(&[1, 2, 3], (1, 3))]);
        let list = GroupListIdx(0);

        let in_group_1 = extra_var(ExtraVarName::StudentInGroup {
            list,
            student: student(1),
            group: 1,
        });
        let has_students = |group: u32| extra_var(ExtraVarName::GroupHasStudents { list, group });

        let cfg = solve_with_objective(
            &plan,
            &[
                // Everyone into slot 0.
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 0),
                // Adversarial: push the extras the wrong way. Landing on
                // the semantic value anyway is what tests the equivalence
                // direction the objective does not supply.
                (1.0, in_group_1.clone()),
                (1.0, has_students(1)),
                (-1.0, has_students(0)),
            ],
        );

        for s in [1, 2, 3] {
            assert_eq!(
                value(
                    &cfg,
                    base_var(Var::StudentGroup {
                        list,
                        student: student(s)
                    })
                ),
                0.0
            );
        }
        assert_eq!(value(&cfg, in_group_1), 0.0);
        assert_eq!(value(&cfg, has_students(1)), 0.0);
        assert_eq!(value(&cfg, has_students(0)), 1.0);
    }

    #[test]
    fn pair_in_group_and_shared_pair() {
        // Two lists of 4 students with fixed size 2, hence 2 slots each.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 5, 6], (2, 2))]);

        let pair_in_group = |a: u64, b: u64, list: usize, group: u32| {
            extra_var(ExtraVarName::PairInGroup {
                a: student(a),
                b: student(b),
                list: GroupListIdx(list),
                group,
            })
        };
        let shared = |a: u64, b: u64| {
            extra_var(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
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
                // Adversarial: 1 and 2 do share the first group of list 0,
                // 1 and 3 co-occur in list 0 but never share a group.
                (-1.0, pair_in_group(1, 2, 0, 0)),
                (-1.0, shared(1, 2)),
                (1.0, pair_in_group(1, 3, 0, 0)),
                (1.0, shared(1, 3)),
            ],
        );

        assert_eq!(value(&cfg, pair_in_group(1, 2, 0, 0)), 1.0);
        assert_eq!(value(&cfg, shared(1, 2)), 1.0);
        assert_eq!(value(&cfg, pair_in_group(1, 3, 0, 0)), 0.0);
        assert_eq!(value(&cfg, shared(1, 3)), 0.0);
    }

    #[test]
    fn pinned_pair_is_free_even_when_never_sharing() {
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 5, 6], (2, 2))]);
        plan.pinned_pairs = [(student(1), student(2))].into_iter().collect();

        let shared = |a: u64, b: u64| {
            extra_var(ExtraVarName::SharedPair {
                a: student(a),
                b: student(b),
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
                // Adversarial: push the pinned pair down, and push the
                // non-pinned pairs of the same model the wrong way too.
                // (Every extra an assertion reads must be referenced here:
                // expansion is lazy, so an unreferenced extra is not a
                // variable of the built problem at all.)
                (-1.0, shared(1, 2)),
                (-1.0, shared(1, 5)),
                (-1.0, shared(1, 3)),
                (1.0, shared(3, 4)),
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
    fn single_slot_list_forces_the_chain() {
        // Two students with a minimum size of 3: the §2.1 clamp gives a
        // single slot, so the `StudentGroup` domain is `0..=0` and the
        // placement is forced by the domain alone.
        let plan = plan_of(&[(&[1, 2], (3, 4))]);
        let list = GroupListIdx(0);

        let in_group = extra_var(ExtraVarName::StudentInGroup {
            list,
            student: student(1),
            group: 0,
        });
        let has_students = extra_var(ExtraVarName::GroupHasStudents { list, group: 0 });
        let pair = extra_var(ExtraVarName::PairInGroup {
            a: student(1),
            b: student(2),
            list,
            group: 0,
        });
        let shared = extra_var(ExtraVarName::SharedPair {
            a: student(1),
            b: student(2),
        });

        // Only adversarial terms: every extra is pushed toward 0, and the
        // equivalences must still propagate the forced placement through
        // the whole chain.
        let cfg = solve_with_objective(
            &plan,
            &[
                (-1.0, in_group.clone()),
                (-1.0, has_students.clone()),
                (-1.0, pair.clone()),
                (-1.0, shared.clone()),
            ],
        );

        assert_eq!(value(&cfg, in_group), 1.0);
        assert_eq!(value(&cfg, has_students), 1.0);
        assert_eq!(value(&cfg, pair), 1.0);
        assert_eq!(value(&cfg, shared), 1.0);
    }
}
