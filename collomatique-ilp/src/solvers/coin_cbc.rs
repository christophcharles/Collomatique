//! COIN-CBC solver
//!
//! This module implements a solver which uses the
//! [coin_cbc] crate as a backend. This crate is
//! an interface to the COIN-CBC solver which is
//! a quite fast open-source solver.

#[cfg(test)]
mod tests;

use super::{ProblemRepr, Solver, SolverWithTimeLimit, TimeLimitSolution};
use crate::{
    linexpr::EqSymbol, ConfigData, FeasableConfig, ObjectiveSense, Problem, UsableData,
    VariableType,
};

/// Coin-cbc solver
///
/// To create such a solver, use [CbcSolver::new].
#[derive(Debug, Clone)]
pub struct CbcSolver {
    disable_logging: bool,
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> SolverWithTimeLimit<V, C, P> for CbcSolver {
    fn solve_with_time_limit<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
        time_limit_in_seconds: u32,
    ) -> Option<TimeLimitSolution<'a, V, C, P>> {
        self.solve_internal(problem, Some(time_limit_in_seconds))
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Solver<V, C, P> for CbcSolver {
    fn solve<'a>(&self, problem: &'a Problem<V, C, P>) -> Option<FeasableConfig<'a, V, C, P>> {
        self.solve_internal(problem, None).map(|x| x.config)
    }
}

struct CbcModel<V: UsableData> {
    model: coin_cbc::Model,
    cols: std::collections::BTreeMap<V, coin_cbc::Col>,
}

impl Default for CbcSolver {
    fn default() -> Self {
        CbcSolver::new()
    }
}

impl CbcSolver {
    /// Returns a default CBC solver.
    ///
    /// The only real configuration for this solver is
    /// to enable or disable logging.
    ///
    /// By default, logging is disabled. But you can change that
    /// using [CbcSolver::with_disable_logging] rather than this function.
    pub fn new() -> Self {
        CbcSolver {
            disable_logging: true,
        }
    }

    /// Builds a CBC solver.
    ///
    /// By default, logging is disabled for the CBC solver.
    /// You can change it here by passing `false` for the `disable_logging`
    /// argument.
    pub fn with_disable_logging(disable_logging: bool) -> Self {
        CbcSolver { disable_logging }
    }
}

impl CbcSolver {
    fn solve_internal<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        &self,
        problem: &'a Problem<V, C, P>,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<TimeLimitSolution<'a, V, C, P>> {
        // cbc does not seem to shut up even if logging is disabled
        // we block output directly
        let stdout_gag = gag::Gag::stdout();
        // We allow for errors in case this is run in multiple threads
        if !self.disable_logging {
            if let Ok(gag) = stdout_gag {
                drop(gag);
            }
        }

        let mut cbc_model = self.build_model(problem);

        Self::add_objective_func(&mut cbc_model, problem);

        if let Some(time_limit) = time_limit_in_seconds {
            cbc_model.model.set_parameter("timeMode", "elapsed");
            cbc_model
                .model
                .set_parameter("seconds", &time_limit.to_string());
        }

        let sol = cbc_model.model.solve();

        Self::reconstruct_config(problem, &sol, &cbc_model.cols)
    }

    fn build_model<V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        &self,
        problem: &Problem<V, C, P>,
    ) -> CbcModel<V> {
        use coin_cbc::Model;
        use std::collections::BTreeMap;

        let mut model = Model::default();

        let cols: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|(var, desc)| {
                let col = match desc.get_type() {
                    VariableType::Integer => model.add_integer(),
                    VariableType::Continuous => model.add_col(),
                };

                match desc.get_min() {
                    Some(m) => model.set_col_lower(col, m),
                    None => model.set_col_lower(col, -f64::INFINITY),
                }

                match desc.get_max() {
                    Some(m) => model.set_col_upper(col, m),
                    None => model.set_col_upper(col, f64::INFINITY),
                }

                (var.clone(), col)
            })
            .collect();

        for (constraint, _desc) in problem.get_constraints() {
            let row = model.add_row();
            for (v, w) in constraint.coefficients() {
                let col = cols[&v];
                model.set_weight(row, col, w);
            }
            match constraint.get_symbol() {
                EqSymbol::Equals => {
                    model.set_row_equal(row, (-constraint.get_constant()).into());
                }
                EqSymbol::LessThan => {
                    model.set_row_upper(row, (-constraint.get_constant()).into());
                }
            }
        }

        if self.disable_logging {
            model.set_parameter("log", "0");
            model.set_parameter("slog", "0");
        }

        CbcModel { model, cols }
    }

    fn add_objective_func<V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        cbc_model: &mut CbcModel<V>,
        problem: &Problem<V, C, P>,
    ) {
        use coin_cbc::Sense;
        cbc_model
            .model
            .set_obj_sense(match problem.get_objective_sense() {
                ObjectiveSense::Maximize => Sense::Maximize,
                ObjectiveSense::Minimize => Sense::Minimize,
            });

        for (var, coef) in problem.get_objective_function().coefficients() {
            let col = cbc_model.cols[var];

            cbc_model.model.set_obj_coeff(col, coef);
        }
    }

    fn reconstruct_config<'a, 'b, 'c, V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        problem: &'a Problem<V, C, P>,
        sol: &'b coin_cbc::Solution,
        cols: &'c std::collections::BTreeMap<V, coin_cbc::Col>,
    ) -> Option<TimeLimitSolution<'a, V, C, P>> {
        let raw_model = sol.raw();

        let time_limit_reached = (raw_model.status() == coin_cbc::raw::Status::Stopped)
            && (raw_model.secondary_status() == coin_cbc::raw::SecondaryStatus::StoppedOnTime);

        let config_data =
            ConfigData::new().set_iter(cols.iter().map(|(v, col)| (v.clone(), sol.col(*col))));

        let config = problem.build_config(config_data).ok()?;

        let feasable_config = config.into_feasable()?;

        Some(TimeLimitSolution {
            config: feasable_config,
            time_limit_reached,
        })
    }
}
