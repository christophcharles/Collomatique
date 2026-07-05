//! good-lp solver
//!
//! This module implements a solver which uses the
//! [good_lp] crate as a backend. This crate can use
//! multiple different solvers as a backend
//! and therefore, this multiplies the possiblities for collomatique.

#[cfg(test)]
mod tests;

use super::{ProblemRepr, Solver, SolverModel};
use crate::{ConfigData, FeasibleConfig, ObjectiveSense, Problem, UsableData, linexpr::EqSymbol};

/// [good_lp] solver
///
/// To create such a solver, use [GoodSolver::new].
#[derive(Debug, Clone)]
pub struct GoodSolver {}

/// A good_lp model ready to be solved.
///
/// Produced by [GoodSolver::build_model].
pub struct GoodBuiltModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    unsolved_problem: good_lp::variable::UnsolvedProblem,
    vars: std::collections::HashMap<V, good_lp::Variable>,
    problem: &'a Problem<V, C, P>,
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Solver<V, C, P> for GoodSolver {
    type Model<'a>
        = GoodBuiltModel<'a, V, C, P>
    where
        V: 'a,
        C: 'a,
        P: 'a;

    fn build_model<'a>(&self, problem: &'a Problem<V, C, P>) -> Self::Model<'a> {
        use good_lp::ProblemVariables;
        use std::collections::HashMap;

        let mut pb_vars = ProblemVariables::new();
        let vars: HashMap<_, _> = problem
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

        GoodBuiltModel {
            unsolved_problem,
            vars,
            problem,
        }
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> SolverModel<'a, V, C, P>
    for GoodBuiltModel<'a, V, C, P>
{
    fn solve(self) -> Option<FeasibleConfig<'a, V, C, P>> {
        use good_lp::Solution as _;
        use good_lp::SolverModel as _;

        let solver = good_lp::solvers::lp_solvers::auto::AllSolvers::new();
        let mut vars_desc = self.unsolved_problem.using(good_lp::LpSolver(solver));

        for (c, _desc) in self.problem.get_constraints() {
            let mut expr = good_lp::Expression::from_other_affine(c.get_constant());

            for (v, c) in c.coefficients() {
                expr.add_mul(c, self.vars[v]);
            }

            let constraint = match c.get_symbol() {
                EqSymbol::Equals => expr.eq(0.0),
                EqSymbol::LessThan => expr.leq(0.0),
            };

            vars_desc = vars_desc.with(constraint);
        }

        let solution = vars_desc.solve().ok()?;

        let config_data = ConfigData::new().set_iter(
            self.vars
                .iter()
                .map(|(v, var)| (v.clone(), solution.value(*var))),
        );

        let config = self.problem.build_config(config_data).ok()?;

        config.into_feasible()
    }
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
