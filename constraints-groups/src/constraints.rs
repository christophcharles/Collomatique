//! The shape constraints of piece 8, one `pub(super)` builder per family,
//! merged here and applied by `builder.rs` **after** the extras bundle: the
//! constraints reference `StudentInGroup` and `GroupHasStudents`, which must
//! be declared first.

mod students_per_group;

use crate::extras::MyBundle;
use crate::vars::VarEnv;

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    students_per_group::build(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::MyModeler;
    use crate::extras::{V, base_var, extra_var};
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
    fn solve_with_objective(
        plan: &GenerationPlan,
        terms: &[(f64, V)],
    ) -> ConfigData<InternalVar<Var, ExtraVarName>> {
        let env = VarEnv::new(plan);
        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
        modeler
            .apply_bundle(crate::extras::build_extras(&env).into_general())
            .expect("no duplicate extras");
        modeler
            .apply_bundle(build(&env).into_general())
            .expect("no duplicate extras");
        for (weight, var) in terms {
            // A negative weight must go through `minimize`, not through
            // `maximize` with a negative coefficient: as the `Objective`
            // documentation warns, scaling by a negative number also
            // reverses the sense, and adding two objectives of opposite
            // senses subtracts them — so the two flips cancel and
            // `maximize(-1.0, x)` would reward `x` instead of penalizing
            // it.
            if *weight >= 0.0 {
                modeler.maximize(*weight, LinExpr::var(var.clone()));
            } else {
                modeler.minimize(-*weight, LinExpr::var(var.clone()));
            }
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
    /// error (a 1 came back as `0.9999999999999999` here), so every value
    /// comparison of this module goes through [`f64_equals`] — the crate's
    /// own `TOLERANCE` — rather than `assert_eq!`.
    fn assert_close(got: f64, expected: f64) {
        assert!(f64_equals(got, expected), "expected {expected}, got {got}");
    }

    #[test]
    fn max_size_caps_each_group() {
        // 4 students, sizes 1..=2 → 4 slots, max 2. Push all four into
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
                    extra_var(ExtraVarName::StudentInGroup {
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
    fn min_size_binds_only_nonempty_groups() {
        // 4 students, sizes 2..=4 → 2 slots. All four fit in group 0;
        // group 1 stays empty, which the conditional minimum must allow —
        // an unconditional one would forbid it.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 4))]);
        let list = GroupListIdx(0);
        let has_students_1 = extra_var(ExtraVarName::GroupHasStudents { list, group: 1 });

        let cfg = solve_with_objective(
            &plan,
            &[
                place(0, 1, 0),
                place(0, 2, 0),
                place(0, 3, 0),
                place(0, 4, 0),
                // Adversarial: push the emptiness indicator the wrong way.
                (1.0, has_students_1.clone()),
            ],
        );

        for s in [1, 2, 3, 4] {
            assert_close(
                value(
                    &cfg,
                    base_var(Var::StudentGroup {
                        list,
                        student: student(s),
                    }),
                ),
                0.0,
            );
        }
        assert_close(value(&cfg, has_students_1), 0.0);
    }

    #[test]
    fn min_size_forces_a_companion() {
        // 5 students, sizes 2..=4 → 2 slots. Weight 100 sends student 1 to
        // group 1; weight −1 on every other student's presence there makes
        // any companion cost. The minimum of 2 forces exactly one anyway.
        //
        // The maximum of 4 is deliberately loose enough to leave student 1
        // alone in group 1 (the other four fit in group 0), so the
        // companion is forced by the minimum and by nothing else.
        let plan = plan_of(&[(&[1, 2, 3, 4, 5], (2, 4))]);
        let list = GroupListIdx(0);
        let in_group_1 = |s: u64| {
            extra_var(ExtraVarName::StudentInGroup {
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

        assert_close(
            value(
                &cfg,
                base_var(Var::StudentGroup {
                    list,
                    student: student(1),
                }),
            ),
            1.0,
        );
        let count_1: f64 = [1, 2, 3, 4, 5]
            .iter()
            .map(|&s| value(&cfg, in_group_1(s)))
            .sum();
        assert_close(count_1, 2.0);
    }

    #[test]
    fn undersized_spec_is_infeasible() {
        // 2 students, sizes 3..=4: the slot-count clamp of §2.1 gives one
        // (necessarily undersized) slot, both students land there, and
        // 2 >= 3 has no satisfying assignment. Infeasibility is the correct
        // signal — the data genuinely cannot satisfy the policy.
        let plan = plan_of(&[(&[1, 2], (3, 4))]);
        let model = crate::build_model(&plan);
        assert!(
            model
                .solve(&ColloCbcSolver::with_disable_logging(true))
                .is_none()
        );
    }
}
