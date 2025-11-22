//! good-lp solver
//!
//! This module implements a solver which uses the
//! [good_lp] crate as a backend. This crate can use
//! multiple different solvers as a backend
//! and therefore, this multiplies the possiblities for collomatique.

#[cfg(test)]
mod tests;

use super::{ProblemRepr, Solver};
use crate::{linexpr::EqSymbol, ConfigData, FeasableConfig, ObjectiveSense, Problem, UsableData};

/// [good_lp] solver
///
/// To create such a solver, use [GoodSolver::new].
#[derive(Debug, Clone)]
pub struct GoodSolver {}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Solver<V, C, P> for GoodSolver {
    fn solve<'a>(&self, problem: &'a Problem<V, C, P>) -> Option<FeasableConfig<'a, V, C, P>> {
        self.solve_internal(problem)
    }
}

struct GoodModel<V: UsableData> {
    unsolved_problem: good_lp::variable::UnsolvedProblem,
    vars: std::collections::BTreeMap<V, good_lp::Variable>,
}

impl Default for GoodSolver {
    fn default() -> Self {
        GoodSolver::new()
    }
}

impl GoodSolver {
    /// Returns a default [good_lp] solver.
    ///
    /// At this moment, no configuration is allowed.
    /// This will use the lp_solvers feature of [good_lp]
    /// and try various solvers.
    pub fn new() -> Self {
        GoodSolver {}
    }
}

impl GoodSolver {
    fn solve_internal<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        &self,
        problem: &'a Problem<V, C, P>,
    ) -> Option<FeasableConfig<'a, V, C, P>> {
        let good_model = Self::build_model(problem);
        let (sol, vars) = Self::solve_problem(good_model, problem)?;
        Self::reconstruct_config(problem, sol, &vars)
    }

    fn build_model<V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        problem: &Problem<V, C, P>,
    ) -> GoodModel<V> {
        use good_lp::ProblemVariables;
        use std::collections::BTreeMap;

        let mut pb_vars = ProblemVariables::new();
        let vars: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|(var, desc)| {
                let col = pb_vars.add({
                    let mut var_def = good_lp::VariableDefinition::new();

                    if desc.is_integer() {
                        var_def = var_def.integer();
                    }

                    if let Some(m) = desc.get_min() {
                        var_def = var_def.min(m);
                    }

                    if let Some(m) = desc.get_max() {
                        var_def = var_def.max(m);
                    }

                    var_def
                });

                (var.clone(), col)
            })
            .collect();

        let objective = problem.get_objective();

        let mut expr =
            good_lp::Expression::with_capacity(objective.get_function().variables().len());

        for (v, c) in objective.get_function().coefficients() {
            expr.add_mul(c, vars[v]);
        }

        let unsolved_problem = match objective.get_sense() {
            ObjectiveSense::Maximize => pb_vars.maximise(expr),
            ObjectiveSense::Minimize => pb_vars.minimise(expr),
        };

        GoodModel {
            unsolved_problem,
            vars,
        }
    }

    fn solve_problem<V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        good_model: GoodModel<V>,
        problem: &Problem<V, C, P>,
    ) -> Option<(
        Box<dyn good_lp::Solution>,
        std::collections::BTreeMap<V, good_lp::Variable>,
    )> {
        use good_lp::SolverModel;

        let solver = good_lp::solvers::lp_solvers::auto::AllSolvers::new();
        let mut vars_desc = good_model.unsolved_problem.using(good_lp::LpSolver(solver));

        for (c, _desc) in problem.get_constraints() {
            let mut expr = good_lp::Expression::from_other_affine(c.get_constant());

            for (v, c) in c.coefficients() {
                expr.add_mul(c, good_model.vars[v]);
            }

            let constraint = match c.get_symbol() {
                EqSymbol::Equals => expr.eq(0.0),
                EqSymbol::LessThan => expr.leq(0.0),
            };

            vars_desc = vars_desc.with(constraint);
        }

        let solution = vars_desc.solve().ok()?;

        Some((Box::new(solution), good_model.vars))
    }

    fn reconstruct_config<'a, 'b, 'c, V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        problem: &'a Problem<V, C, P>,
        sol: Box<dyn good_lp::Solution>,
        vars: &'c std::collections::BTreeMap<V, good_lp::Variable>,
    ) -> Option<FeasableConfig<'a, V, C, P>> {
        let config_data =
            ConfigData::new().set_iter(vars.iter().map(|(v, var)| (v.clone(), sol.value(*var))));

        let config = problem.build_config(config_data).ok()?;

        config.into_feasable()
    }
}
