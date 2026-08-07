#[cfg(test)]
mod tests;

use std::collections::HashMap;

use super::{
    CallbackSolution, CallbackSolverModel, IncumbentInfo, IncumbentTimeLimitSolverModel,
    ProblemRepr, ProgressBounds, ProgressIncumbentData, ProgressIncumbentInfo, ProgressStats,
    Solver, SolverModel, StopReason, TimeLimitSolution, TimeLimitSolverModel, TimeLimitsSolution,
    WarmSolver,
};
use crate::{ConfigData, FeasibleConfig, ObjectiveSense, Problem, UsableData, linexpr::EqSymbol};
use collomatique_time::TimeLimit;

pub struct ColloCbcSolver {
    disable_logging: bool,
}

pub struct ColloCbcBuiltModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    model: collo_cbc::Model,
    col_indices: HashMap<V, usize>,
    problem: &'a Problem<V, C, P>,
}

pub struct Progress<V: UsableData> {
    best_objective: Option<f64>,
    best_bound: f64,
    nodes: u64,
    solutions: u64,
    incumbent: Option<IncumbentInfo>,
    incumbent_config: Option<ConfigData<V>>,
    /// Whether the event being reported is the one that brought
    /// `incumbent_config`. Unlike the fields above, this describes the event
    /// and is not carried forward.
    incumbent_is_fresh: bool,
}

impl<V: UsableData> Progress<V> {
    /// Folds one raw CBC event into the carried state.
    ///
    /// Returns `true` when the event carried an incumbent CBC could not map
    /// back into the problem's own columns.
    ///
    /// A tick is carried, not applied. It means "CBC is alive" and nothing
    /// more: it comes from a nested heuristic sub-MIP whose bound, node count
    /// and incumbent all live in that sub-MIP's own reduced column space, so
    /// none of them can be transmitted. Leaving the last authoritative values
    /// in place keeps what the caller sees coherent. The caller is still
    /// called, so its deadlines still run and its stop request still relays
    /// while such a model holds the solve.
    fn update_from(&mut self, raw: &collo_cbc::Progress, col_indices: &HashMap<V, usize>) -> bool {
        // Freshness describes this event, so it is cleared first and set only
        // where an incumbent actually arrives. Everything else below is state
        // that carries forward.
        self.incumbent_is_fresh = false;

        if raw.event_type == collo_cbc::EventType::Tick {
            return false;
        }

        self.best_bound = raw.best_bound;
        self.nodes = raw.node_count as u64;
        self.solutions = raw.solutions_found as u64;

        match &raw.incumbent {
            collo_cbc::IncumbentEvent::Reconstructed {
                objective,
                solution,
            } => {
                self.best_objective = Some(*objective);
                self.incumbent = Some(IncumbentInfo {
                    objective: *objective,
                    feasible: true,
                });
                self.incumbent_config = Some(
                    ConfigData::new().set_iter(
                        col_indices
                            .iter()
                            .map(|(var, &col)| (var.clone(), solution[col])),
                    ),
                );
                self.incumbent_is_fresh = true;
                false
            }
            // No fresh incumbent this event: keep the last known objective
            // and incumbent (they carry forward through tree-status events).
            collo_cbc::IncumbentEvent::None => false,
            // CBC found an incumbent but it couldn't be reconstructed into
            // original column space. We keep the last good incumbent rather
            // than reporting a bogus one.
            collo_cbc::IncumbentEvent::ReconstructionFailed => true,
        }
    }
}

impl<V: UsableData> ProgressBounds for Progress<V> {
    fn best_bound(&self) -> f64 {
        self.best_bound
    }
    fn best_objective(&self) -> Option<f64> {
        self.best_objective
    }
}

impl<V: UsableData> ProgressStats for Progress<V> {
    fn nodes(&self) -> u64 {
        self.nodes
    }
    fn solutions(&self) -> u64 {
        self.solutions
    }
}

impl<V: UsableData> ProgressIncumbentInfo for Progress<V> {
    fn incumbent_info(&self) -> Option<&IncumbentInfo> {
        self.incumbent.as_ref()
    }
}

impl<V: UsableData> ProgressIncumbentData<V> for Progress<V> {
    fn incumbent_data(&self) -> Option<&ConfigData<V>> {
        self.incumbent_config.as_ref()
    }
    fn incumbent_is_fresh(&self) -> bool {
        self.incumbent_is_fresh
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
            model.set_log_level(0);
        } else {
            model.set_parameter("log", "1");
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
            Err(check) => {
                if result.status == collo_cbc::Status::Optimal {
                    panic!(
                        "CBC reported optimal but build_config failed. \
                         missing={}, excess={}, non_conforming={}. \
                         This should never happen.",
                        check.missing_variables.len(),
                        check.excess_variables.len(),
                        check.non_conforming_variables.len(),
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
        self.solve_with_time_limit(TimeLimit::none()).config
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> TimeLimitSolverModel<'a, V, C, P>
    for ColloCbcBuiltModel<'a, V, C, P>
{
    fn solve_with_time_limit(self, time_limit: TimeLimit) -> TimeLimitSolution<'a, V, C, P> {
        match time_limit.duration() {
            Some(duration) => {
                let start = std::time::Instant::now();
                let result = self.solve_with_callback(|_| start.elapsed() < duration);
                TimeLimitSolution {
                    config: result.config,
                    time_limit_reached: result.stopped_by_callback,
                }
            }
            None => TimeLimitSolution {
                config: self.solve_with_callback(|_| true).config,
                time_limit_reached: false,
            },
        }
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> CallbackSolverModel<'a, V, C, P>
    for ColloCbcBuiltModel<'a, V, C, P>
{
    type Progress = Progress<V>;

    fn solve_with_callback<F>(mut self, mut callback: F) -> CallbackSolution<'a, V, C, P>
    where
        F: FnMut(&Self::Progress) -> bool,
    {
        let mut progress = Progress {
            best_objective: None,
            best_bound: -f64::INFINITY,
            nodes: 0,
            solutions: 0,
            incumbent: None,
            incumbent_config: None,
            incumbent_is_fresh: false,
        };

        let col_indices = &self.col_indices;
        // Every failure in one solve has the same cause, and CBC re-reports the
        // same unmappable incumbent on each event that carries it, so saying it
        // once is the whole of the diagnostic value.
        let mut reported_failure = false;
        let result = self.model.solve_with_callback(|raw_progress| {
            if progress.update_from(raw_progress, col_indices) && !reported_failure {
                reported_failure = true;
                eprintln!(
                    "collo_cbc: an incumbent could not be mapped back into the problem's \
                     own columns and was skipped. This is expected once CBC restarts its \
                     search (see docs/todos/todo_subtree_incumbent_reconstruction.md); the \
                     final solution is unaffected. Reported once per solve."
                );
            }
            callback(&progress)
        });

        let stopped_by_callback = result.status == collo_cbc::Status::Stopped;

        CallbackSolution {
            config: self.reconstruct_config(&result),
            stopped_by_callback,
        }
    }
}

/// Tracks the two solve deadlines: the global one (armed at construction) and
/// the incumbent one (armed on the first incumbent).
///
/// See [IncumbentTimeLimitSolverModel] for the min-composition contract.
struct SolveDeadlines {
    global_deadline: Option<std::time::Instant>,
    incumbent_duration: Option<std::time::Duration>,
    incumbent_deadline: Option<std::time::Instant>,
}

impl SolveDeadlines {
    fn new(
        time_limit: Option<std::time::Duration>,
        incumbent_time_limit: Option<std::time::Duration>,
    ) -> Self {
        SolveDeadlines {
            global_deadline: time_limit.map(|d| std::time::Instant::now() + d),
            incumbent_duration: incumbent_time_limit,
            incumbent_deadline: None,
        }
    }

    /// Call on every progress event. `Some` once a deadline has passed.
    ///
    /// The global deadline is checked first, so it wins when both have passed —
    /// that is the `min` of the two.
    fn check(&mut self, has_incumbent: bool) -> Option<StopReason> {
        let now = std::time::Instant::now();

        if self.incumbent_deadline.is_none() && has_incumbent {
            self.incumbent_deadline = self.incumbent_duration.map(|d| now + d);
        }

        if self.global_deadline.is_some_and(|d| now >= d) {
            return Some(StopReason::TimeLimit);
        }
        if self.incumbent_deadline.is_some_and(|d| now >= d) {
            return Some(StopReason::IncumbentTimeLimit);
        }

        None
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> IncumbentTimeLimitSolverModel<'a, V, C, P>
    for ColloCbcBuiltModel<'a, V, C, P>
{
    fn solve_with_time_limits<F>(
        self,
        time_limit: TimeLimit,
        incumbent_time_limit: TimeLimit,
        mut callback: F,
    ) -> TimeLimitsSolution<'a, V, C, P>
    where
        F: FnMut(&Self::Progress) -> bool,
    {
        let mut deadlines =
            SolveDeadlines::new(time_limit.duration(), incumbent_time_limit.duration());

        // Only ever set on the event that stops the solve: the deadlines are
        // checked last, and their `Some` immediately returns `false` below.
        let mut limit_stop = None;

        let result = self.solve_with_callback(|progress| {
            // The caller comes first, so a stop it asked for is attributed to
            // it rather than to a deadline that passed on the same event.
            if !callback(progress) {
                return false;
            }

            limit_stop = deadlines.check(progress.incumbent_info().is_some());
            limit_stop.is_none()
        });

        TimeLimitsSolution {
            config: result.config,
            stopped: match (result.stopped_by_callback, limit_stop) {
                // One of our deadlines fired.
                (true, Some(reason)) => Some(reason),
                // The caller stopped the solve.
                (true, None) => Some(StopReason::Callback),
                // The solve ran to completion.
                (false, _) => None,
            },
        }
    }
}
