//! COIN-CBC solver
//!
//! This module implements a solver which uses the
//! [coin_cbc] crate as a backend. This crate is
//! an interface to the COIN-CBC solver which is
//! a quite fast open-source solver.

#[cfg(test)]
mod tests;

use super::{
    ProblemRepr, Solver, SolverModel, TimeLimitSolution, TimeLimitSolverModel, WarmSolver,
};
use crate::{ConfigData, FeasibleConfig, ObjectiveSense, Problem, UsableData, linexpr::EqSymbol};

/// Coin-cbc solver
///
/// To create such a solver, use [CbcSolver::new].
#[derive(Debug, Clone)]
pub struct CbcSolver {
    disable_logging: bool,
}

/// A CBC model ready to be solved.
///
/// Produced by [CbcSolver::build_model].
pub struct CbcBuiltModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    model: coin_cbc::Model,
    cols: std::collections::HashMap<V, coin_cbc::Col>,
    problem: &'a Problem<V, C, P>,
    disable_logging: bool,
}

impl CbcSolver {
    fn build_model_internal<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        &self,
        problem: &'a Problem<V, C, P>,
        hint: Option<&ConfigData<V>>,
    ) -> CbcBuiltModel<'a, V, C, P> {
        use coin_cbc::Model;
        use std::collections::HashMap;

        let mut model = Model::default();

        let cols: HashMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|(var, desc)| {
                let col = if desc.is_integer() {
                    model.add_integer()
                } else {
                    model.add_col()
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
                let col = cols[v];
                model.set_weight(row, col, w);
            }
            match constraint.get_symbol() {
                EqSymbol::Equals => {
                    model.set_row_equal(row, -constraint.get_constant());
                }
                EqSymbol::LessThan => {
                    model.set_row_upper(row, -constraint.get_constant());
                }
            }
        }

        let objective = problem.get_objective();

        model.set_obj_sense(match objective.get_sense() {
            ObjectiveSense::Maximize => coin_cbc::Sense::Maximize,
            ObjectiveSense::Minimize => coin_cbc::Sense::Minimize,
        });

        for (var, coef) in objective.get_function().coefficients() {
            let col = cols[var];
            model.set_obj_coeff(col, coef);
        }

        if self.disable_logging {
            model.set_parameter("log", "0");
            model.set_parameter("slog", "0");
        }

        if let Some(hint) = hint {
            for (var, col) in &cols {
                if let Some(value) = hint.get(var.clone()) {
                    model.set_col_initial_solution(*col, value);
                }
            }
        }

        CbcBuiltModel {
            model,
            cols,
            problem,
            disable_logging: self.disable_logging,
        }
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Solver<V, C, P> for CbcSolver {
    type Model<'a>
        = CbcBuiltModel<'a, V, C, P>
    where
        V: 'a,
        C: 'a,
        P: 'a;

    fn build_model<'a>(&self, problem: &'a Problem<V, C, P>) -> Self::Model<'a> {
        self.build_model_internal(problem, None)
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> WarmSolver<V, C, P> for CbcSolver {
    fn build_warm_model<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
        hint: &ConfigData<V>,
    ) -> Self::Model<'a> {
        self.build_model_internal(problem, Some(hint))
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> SolverModel<'a, V, C, P>
    for CbcBuiltModel<'a, V, C, P>
{
    fn solve(self) -> Option<FeasibleConfig<'a, V, C, P>> {
        self.solve_internal(None).config
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> TimeLimitSolverModel<'a, V, C, P>
    for CbcBuiltModel<'a, V, C, P>
{
    fn solve_with_time_limit(self, time_limit_in_seconds: u32) -> TimeLimitSolution<'a, V, C, P> {
        self.solve_internal(Some(time_limit_in_seconds))
    }
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

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> CbcBuiltModel<'a, V, C, P> {
    fn solve_internal(
        mut self,
        time_limit_in_seconds: Option<u32>,
    ) -> TimeLimitSolution<'a, V, C, P> {
        // cbc does not seem to shut up even if logging is disabled
        // we block output directly
        let stdout_gag = gag::Gag::stdout();
        // We allow for errors in case this is run in multiple threads
        if !self.disable_logging
            && let Ok(gag) = stdout_gag
        {
            drop(gag);
        }

        if let Some(time_limit) = time_limit_in_seconds {
            self.model.set_parameter("timeMode", "elapsed");
            self.model.set_parameter("seconds", &time_limit.to_string());
        }

        let sol = self.model.solve();

        Self::reconstruct_config(self.problem, &sol, &self.cols)
    }

    fn reconstruct_config(
        problem: &'a Problem<V, C, P>,
        sol: &coin_cbc::Solution,
        cols: &std::collections::HashMap<V, coin_cbc::Col>,
    ) -> TimeLimitSolution<'a, V, C, P> {
        let raw_model = sol.raw();

        let time_limit_reached = (raw_model.status() == coin_cbc::raw::Status::Stopped)
            && (raw_model.secondary_status() == coin_cbc::raw::SecondaryStatus::StoppedOnTime);

        let config_data =
            ConfigData::new().set_iter(cols.iter().map(|(v, col)| (v.clone(), sol.col(*col))));

        let config = match problem.build_config(config_data) {
            Ok(c) => c,
            Err(_) => {
                if raw_model.status() == coin_cbc::raw::Status::Finished
                    && raw_model.secondary_status() == coin_cbc::raw::SecondaryStatus::HasSolution
                {
                    panic!(
                        "CBC reported optimal (Status::Finished) but build_config failed \
                         (missing variables). This should never happen."
                    );
                }
                return TimeLimitSolution {
                    config: None,
                    time_limit_reached,
                };
            }
        };

        if !config.is_feasible()
            && raw_model.status() == coin_cbc::raw::Status::Finished
            && raw_model.secondary_status() == coin_cbc::raw::SecondaryStatus::HasSolution
        {
            let violated_count = config.blame().len();
            panic!(
                "CBC reported optimal solution (Status::Finished) but {violated_count} \
                 constraint(s) violated by the returned values. This indicates a numerical \
                 tolerance mismatch between CBC and our feasibility check \
                 (TOLERANCE = {}).",
                crate::TOLERANCE
            );
        }

        let feasible_config = config.into_feasible();

        TimeLimitSolution {
            config: feasible_config,
            time_limit_reached,
        }
    }
}
