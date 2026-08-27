//! The shape constraints of piece 8, one `pub(super)` builder per family,
//! merged here and applied by `builder.rs` **after** the extras bundle: the
//! constraints reference `StudentInGroup`, which must be declared first.

mod frozen_placements;
mod student_in_one_group;
mod students_per_group;

use crate::extras::MyBundle;
use crate::frozen::FrozenPlacements;
use crate::vars::VarEnv;

pub(crate) fn build(env: &VarEnv, frozen: &FrozenPlacements) -> MyBundle {
    student_in_one_group::build(env)
        .merge(students_per_group::build(env))
        .expect("no duplicate extras")
        .merge(frozen_placements::build(env, frozen))
        .expect("no duplicate extras")
}

/// The "exactly one group per student" family on its own, for the harnesses
/// that place students by hand without the size constraints (`extras.rs`'s
/// and this module's): the base binaries only describe a placement under it.
#[cfg(test)]
pub(crate) fn build_student_in_one_group(env: &VarEnv) -> MyBundle {
    student_in_one_group::build(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::MyModeler;
    use crate::extras::{V, base_var};
    use crate::specs::GenerationPlan;
    use crate::specs::tests::student;
    use crate::types::ExtraVarName;
    use crate::vars::tests::plan_of;
    use crate::vars::{GroupListIdx, Var};
    use collomatique_ilp::linexpr::LinExpr;
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp::{ConfigData, f64_equals};
    use collomatique_ilp_modeler::{InternalVar, Modeler};

    /// The `extras.rs` harness, plus the constraints bundle: apply both,
    /// maximize each weighted term, build (lazily — the objective is what
    /// forces the expansion of an extra no constraint mentions), solve, and
    /// return every variable of the solution.
    ///
    /// Same trap as in `extras.rs`: an extra that only an assertion mentions
    /// is never expanded, so every asserted extra must also carry an
    /// objective weight. Base variables are always present.
    pub(super) fn solve_with_objective(
        plan: &GenerationPlan,
        terms: &[(f64, V)],
    ) -> ConfigData<InternalVar<Var, ExtraVarName>> {
        solve_with_objective_pinned(plan, terms, &FrozenPlacements::default())
    }

    /// [`solve_with_objective`], with seats held fixed as well — so a push
    /// and a pin can be made to fight over the same student.
    pub(super) fn solve_with_objective_pinned(
        plan: &GenerationPlan,
        terms: &[(f64, V)],
        frozen: &FrozenPlacements,
    ) -> ConfigData<InternalVar<Var, ExtraVarName>> {
        let env = VarEnv::new(plan);
        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(crate::extras::build_extras(&env).into_general())
            .expect("no duplicate extras");
        modeler
            .apply_bundle(build(&env, frozen).into_general())
            .expect("no duplicate extras");
        for (weight, var) in terms {
            // The weight goes into the `LinExpr`, before the sense is
            // applied. `maximize`'s own `coef` scales the finished
            // `Objective` instead, and scaling an `Objective` by a negative
            // number reverses its sense too (`generic/ilp/src/objectives.rs:128`),
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

    /// A weight-100 term placing `student` in `group` of `list` — far above
    /// the ±1 adversarial weights, but not above the constraints, which is
    /// the point: here the pushes fight the constraint and must lose.
    pub(super) fn place(list: usize, s: u64, group: u32) -> (f64, V) {
        (
            100.0,
            base_var(Var::StudentInGroup {
                list: GroupListIdx(list),
                student: student(s),
                group,
            }),
        )
    }

    pub(super) fn value(cfg: &ConfigData<InternalVar<Var, ExtraVarName>>, var: V) -> f64 {
        cfg.get(var.clone())
            .unwrap_or_else(|| panic!("{:?} should be part of the solved problem", var))
    }

    /// CBC returns integral variables as floats carrying a tiny numerical
    /// error (a 1 came back as `0.9999999999999999` here), so every value
    /// comparison of this module goes through [`f64_equals`] — the crate's
    /// own `TOLERANCE` — rather than `assert_eq!`.
    pub(super) fn assert_close(got: f64, expected: f64) {
        assert!(f64_equals(got, expected), "expected {expected}, got {got}");
    }

    #[test]
    fn max_size_caps_each_group() {
        // 4 students, sizes 1..=2 → 2 groups, max 2. Push all four into
        // group 0; the cap lets exactly two of the pushes win.
        let plan = plan_of(&[(&[1, 2, 3, 4], (1, 2))]);
        let list = GroupListIdx(0);

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 0),
                place(0, 4, 0),
            ],
        );

        let in_group_0: f64 = [1, 2, 3, 4]
            .iter()
            .map(|&s| {
                value(
                    &cfg,
                    base_var(Var::StudentInGroup {
                        list,
                        student: student(s),
                        group: 0,
                    }),
                )
            })
            .sum();
        assert_close(in_group_0, 2.0);
    }

    #[test]
    fn every_student_sits_in_exactly_one_group() {
        // 4 students, sizes 1..=2 → 2 groups. Student 1 is pushed into
        // *both* groups at once and student 2 out of both: the placement is
        // a matrix of independent binaries now, so "one group per student"
        // is a constraint the pushes can fight — the retired integer domain
        // made it unsayable.
        let plan = plan_of(&[(&[1, 2, 3, 4], (1, 2))]);
        let list = GroupListIdx(0);
        let in_group = |s: u64, group: u32| {
            base_var(Var::StudentInGroup {
                list,
                student: student(s),
                group,
            })
        };

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 0),
                place(0, 1, 1),
                (-1.0, in_group(2, 0)),
                (-1.0, in_group(2, 1)),
            ],
        );

        for s in [1, 2] {
            let count: f64 = (0..2).map(|group| value(&cfg, in_group(s, group))).sum();
            assert_close(count, 1.0);
        }
    }

    #[test]
    fn min_size_binds_every_group() {
        // 9 students, sizes 3..=4 → 3 groups. The maximum alone allows a
        // 4 / 4 / 1 split, which the pushes below ask for; the minimum
        // forbids the thin group, and 3 / 3 / 3 is then the only split
        // left. The group count being exact is what makes the minimum
        // unconditional: no group may be starved for another's benefit.
        let plan = plan_of(&[(&[1, 2, 3, 4, 5, 6, 7, 8, 9], (3, 4))]);
        let list = GroupListIdx(0);
        let in_group = |s: u64, group: u32| {
            base_var(Var::StudentInGroup {
                list,
                student: student(s),
                group,
            })
        };

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 0),
                place(0, 4, 0),
                place(0, 5, 1),
                place(0, 6, 1),
                place(0, 7, 1),
                place(0, 8, 1),
            ],
        );

        for group in 0..3 {
            let count: f64 = (1..=9).map(|s| value(&cfg, in_group(s, group))).sum();
            assert_close(count, 3.0);
        }
    }

    #[test]
    fn min_size_forces_a_companion() {
        // 5 students, sizes 2..=4 → 2 groups. Weight 100 sends student 1 to
        // group 1; weight −1 on every other student's presence there makes
        // any companion cost. The minimum of 2 forces exactly one anyway.
        //
        // The maximum of 4 is deliberately loose enough to leave student 1
        // alone in group 1 (the other four fit in group 0), so the
        // companion is forced by the minimum and by nothing else.
        let plan = plan_of(&[(&[1, 2, 3, 4, 5], (2, 4))]);
        let list = GroupListIdx(0);
        let in_group_1 = |s: u64| {
            base_var(Var::StudentInGroup {
                list,
                student: student(s),
                group: 1,
            })
        };

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 1),
                (-1.0, in_group_1(2)),
                (-1.0, in_group_1(3)),
                (-1.0, in_group_1(4)),
                (-1.0, in_group_1(5)),
            ],
        );

        assert_close(value(&cfg, in_group_1(1)), 1.0);
        let count_1: f64 = [1, 2, 3, 4, 5]
            .iter()
            .map(|&s| value(&cfg, in_group_1(s)))
            .sum();
        assert_close(count_1, 2.0);
    }
}
