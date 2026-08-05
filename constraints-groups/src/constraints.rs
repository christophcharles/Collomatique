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
    use crate::extras::{V, extra_var};
    use crate::specs::GenerationPlan;
    use crate::specs::tests::student;
    use crate::types::ExtraVarName;
    use crate::vars::tests::plan_of;
    use crate::vars::{GroupListIdx, Var};
    use collomatique_ilp::ConfigData;
    use collomatique_ilp::linexpr::LinExpr;
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
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
            modeler.maximize(*weight, LinExpr::var(var.clone()));
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
        assert_eq!(in_group_0, 2.0);
    }
}
