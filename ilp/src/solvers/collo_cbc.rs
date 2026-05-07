#[cfg(test)]
mod tests;

use std::collections::HashMap;

use super::{
    CallbackSolution, CallbackSolverModel, ProblemRepr, ProgressBounds, ProgressStats, Solver,
    SolverModel, TimeLimitSolution, TimeLimitSolverModel, WarmSolver,
};
use crate::{ConfigData, FeasibleConfig, ObjectiveSense, Problem, UsableData, linexpr::EqSymbol};

pub struct ColloCbcSolver {
    disable_logging: bool,
}

pub struct ColloCbcBuiltModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    model: collo_cbc::Model,
    col_indices: HashMap<V, usize>,
    problem: &'a Problem<V, C, P>,
    disable_logging: bool,
}

pub struct Progress {
    best_objective: f64,
    best_bound: f64,
    nodes: u64,
}

impl ProgressBounds for Progress {
    fn best_bound(&self) -> f64 {
        self.best_bound
    }
    fn best_objective(&self) -> f64 {
        self.best_objective
    }
}

impl ProgressStats for Progress {
    fn nodes(&self) -> u64 {
        self.nodes
    }
}

impl Default for ColloCbcSolver {
    fn default() -> Self {
        ColloCbcSolver::new()
    }
}

impl ColloCbcSolver {
    pub fn new() -> Self {
        ColloCbcSolver {
            disable_logging: true,
        }
    }

    pub fn with_disable_logging(disable_logging: bool) -> Self {
        ColloCbcSolver { disable_logging }
    }

    fn build_model_internal<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>(
        &self,
        problem: &'a Problem<V, C, P>,
        hint: Option<&ConfigData<V>>,
    ) -> ColloCbcBuiltModel<'a, V, C, P> {
        let variables = problem.get_variables();
        let constraints = problem.get_constraints();
        let objective = problem.get_objective();

        let var_order: Vec<V> = variables.keys().cloned().collect();
        let col_indices: HashMap<V, usize> = var_order
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();

        let num_cols = var_order.len() as i32;
        let num_rows = constraints.len() as i32;

        let mut col_lb = Vec::with_capacity(num_cols as usize);
        let mut col_ub = Vec::with_capacity(num_cols as usize);
        let mut obj_coeffs = vec![0.0f64; num_cols as usize];
        let mut is_integer = Vec::with_capacity(num_cols as usize);

        for var in &var_order {
            let desc = &variables[var];
            col_lb.push(desc.get_min().unwrap_or(-f64::INFINITY));
            col_ub.push(desc.get_max().unwrap_or(f64::INFINITY));
            is_integer.push(if desc.is_integer() { 1 } else { 0 });
        }

        for (var, coef) in objective.get_function().coefficients() {
            obj_coeffs[col_indices[var]] = coef;
        }

        let obj_sense = match objective.get_sense() {
            ObjectiveSense::Minimize => 1,
            ObjectiveSense::Maximize => -1,
        };

        // Build column-major sparse constraint matrix.
        // mat_start[j] = index into mat_index/mat_value where column j begins.
        let mut col_entries: Vec<Vec<(i32, f64)>> = vec![vec![]; num_cols as usize];
        let mut row_lb = Vec::with_capacity(num_rows as usize);
        let mut row_ub = Vec::with_capacity(num_rows as usize);

        for (row_idx, (constraint, _desc)) in constraints.iter().enumerate() {
            let rhs = -constraint.get_constant();
            match constraint.get_symbol() {
                EqSymbol::Equals => {
                    row_lb.push(rhs);
                    row_ub.push(rhs);
                }
                EqSymbol::LessThan => {
                    row_lb.push(-f64::INFINITY);
                    row_ub.push(rhs);
                }
            }
            for (var, coef) in constraint.coefficients() {
                let col = col_indices[var];
                col_entries[col].push((row_idx as i32, coef));
            }
        }

        let mut mat_start = Vec::with_capacity(num_cols as usize + 1);
        let mut mat_index = Vec::new();
        let mut mat_value = Vec::new();

        for col_entry in &col_entries {
            mat_start.push(mat_index.len() as i32);
            for &(row, val) in col_entry {
                mat_index.push(row);
                mat_value.push(val);
            }
        }
        mat_start.push(mat_index.len() as i32);

        let desc = collo_cbc::ProblemDesc {
            num_cols,
            num_rows,
            obj_sense,
            col_lb,
            col_ub,
            obj_coeffs,
            is_integer,
            mat_start,
            mat_index,
            mat_value,
            row_lb,
            row_ub,
        };

        let mut model = collo_cbc::Model::new();
        model.load_problem(&desc);

        if self.disable_logging {
            model.set_parameter("log", "0");
            model.set_parameter("slog", "0");
        }

        if let Some(hint) = hint {
            let mut values = vec![0.0f64; num_cols as usize];
            for (var, &col) in &col_indices {
                if let Some(v) = hint.get(var.clone()) {
                    values[col] = v;
                }
            }
            model.set_mip_start(&values);
        }

        ColloCbcBuiltModel {
            model,
            col_indices,
            problem,
            disable_logging: self.disable_logging,
        }
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Solver<V, C, P> for ColloCbcSolver {
    type Model<'a>
        = ColloCbcBuiltModel<'a, V, C, P>
    where
        V: 'a,
        C: 'a,
        P: 'a;

    fn build_model<'a>(&self, problem: &'a Problem<V, C, P>) -> Self::Model<'a> {
        self.build_model_internal(problem, None)
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> WarmSolver<V, C, P> for ColloCbcSolver {
    fn build_warm_model<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
        hint: &ConfigData<V>,
    ) -> Self::Model<'a> {
        self.build_model_internal(problem, Some(hint))
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> ColloCbcBuiltModel<'a, V, C, P> {
    fn reconstruct_config(
        &self,
        result: &collo_cbc::SolveResult,
    ) -> Option<FeasibleConfig<'a, V, C, P>> {
        let solution = result.solution.as_ref()?;

        let config_data = ConfigData::new().set_iter(
            self.col_indices
                .iter()
                .map(|(var, &col)| (var.clone(), solution[col])),
        );

        let config = match self.problem.build_config(config_data) {
            Ok(c) => c,
            Err(_) => {
                if result.status == collo_cbc::Status::Optimal {
                    panic!(
                        "CBC reported optimal but build_config failed (missing variables). \
                         This should never happen."
                    );
                }
                return None;
            }
        };

        if !config.is_feasible() && result.status == collo_cbc::Status::Optimal {
            let violated_count = config.blame().len();
            panic!(
                "CBC reported optimal solution but {violated_count} constraint(s) violated. \
                 This indicates a numerical tolerance mismatch (TOLERANCE = {}).",
                crate::TOLERANCE
            );
        }

        config.into_feasible()
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> SolverModel<'a, V, C, P>
    for ColloCbcBuiltModel<'a, V, C, P>
{
    fn solve(self) -> Option<FeasibleConfig<'a, V, C, P>> {
        self.solve_with_callback(|_| true).config
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> TimeLimitSolverModel<'a, V, C, P>
    for ColloCbcBuiltModel<'a, V, C, P>
{
    fn solve_with_time_limit(self, time_limit_in_seconds: u32) -> TimeLimitSolution<'a, V, C, P> {
        let start = std::time::Instant::now();
        let duration = std::time::Duration::from_secs(time_limit_in_seconds as u64);
        let result = self.solve_with_callback(|_| start.elapsed() < duration);
        TimeLimitSolution {
            config: result.config,
            time_limit_reached: result.stopped_by_callback,
        }
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> CallbackSolverModel<'a, V, C, P>
    for ColloCbcBuiltModel<'a, V, C, P>
{
    type Progress = Progress;

    fn solve_with_callback<F>(mut self, mut callback: F) -> CallbackSolution<'a, V, C, P>
    where
        F: FnMut(&Self::Progress) -> bool,
    {
        let stdout_gag = gag::Gag::stdout();
        if !self.disable_logging {
            if let Ok(gag) = stdout_gag {
                drop(gag);
            }
        }

        let mut progress = Progress {
            best_objective: f64::INFINITY,
            best_bound: -f64::INFINITY,
            nodes: 0,
        };

        let result = self.model.solve_with_callback(|raw_progress| {
            progress.best_objective = raw_progress.best_obj;
            progress.best_bound = raw_progress.best_bound;
            progress.nodes = raw_progress.node_count as u64;
            callback(&progress)
        });

        let stopped_by_callback = result.status == collo_cbc::Status::Stopped;

        CallbackSolution {
            config: self.reconstruct_config(&result),
            stopped_by_callback,
        }
    }
}
