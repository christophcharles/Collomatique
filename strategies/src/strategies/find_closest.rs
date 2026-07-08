use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, LinExpr, Problem, ProblemBuilder, UsableData, Variable};
use collomatique_ilp_modeler::{
    ConstraintBundle, ConstraintSource, InternalVar, Model, Modeler, Var,
};

use crate::{
    NoObjectiveSolveProgress, SolveProblemOpts, SolveStatus, Strategy, StrategyContext,
    StrategyError, StrategyOutcome, VarOrderSerializable,
};

/// Per-run payload for [`FindClosestStrategy`]: the `target` configuration whose feasible
/// L1-nearest point the strategy searches for.
#[derive(Debug, Clone)]
pub struct FindClosestPayload<V: UsableData> {
    pub target: ConfigData<V>,
}

/// Serializable counterpart of [`FindClosestPayload<V>`]: the target is erased to a
/// column-indexed `Vec<f64>` against the model's `var_order`, so it can cross the IPC barrier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindClosestPayloadData {
    pub target: Vec<f64>,
}

impl<V: UsableData + Send> VarOrderSerializable<V> for FindClosestPayload<V> {
    type Data = FindClosestPayloadData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<FindClosestPayloadData, Infallible> {
        Ok(FindClosestPayloadData {
            target: collomatique_ilp::config_data_to_hint(&self.target, var_order),
        })
    }
    fn from_data(data: &FindClosestPayloadData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(FindClosestPayload {
            target: collomatique_ilp::solution_to_config_data(&data.target, var_order),
        })
    }
}

/// Extra-variable name for the surrogate "closeness" model. The
/// original extras are wrapped as [`ClosestExtra::Inner`]; the L1
/// penalty produced by objectifying the non-binary closeness
/// constraints gets the reserved [`ClosestExtra::Penalty`] name.
///
/// This wrapper only ever exists inside [`FindClosestStrategy`]: the
/// solution is reduced back to its base variables before anything
/// leaves the strategy, so the wrapper never reaches the caller.
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
enum ClosestExtra<E> {
    Inner(E),
    Penalty,
}

/// Find a feasible assignment of the base variables that is as close
/// as possible (L1 distance) to a target config, then reconstruct the
/// extra variables against the original model.
///
/// Mirrors [`NoObjectiveStrategy`](crate::NoObjectiveStrategy): it
/// strips the real objective and solves a cheap surrogate, but here
/// the surrogate *minimizes distance to the payload target* instead of
/// merely checking feasibility. Binary base variables contribute a
/// direct linear term; non-binary ones are objectified into an L1
/// penalty. The target is supplied as the payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindClosestStrategy {
    pub closeness_time_limit_seconds: Option<u32>,
    pub reconstruction_time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
    /// Absolute tolerance on the L1 closeness distance (Phase 2): accept the first feasible
    /// point found within this distance of the closest possible one, instead of solving the
    /// closeness model to proven optimality. `0.0` means "find the exact closest point".
    pub distance_tolerance: f64,
}

/// Whether the closeness incumbent `best_obj` is within `tolerance` of the best bound.
/// The closeness objective is always minimized, so this is a one-sided absolute gap.
fn within_distance_tolerance(best_obj: f64, best_bound: f64, tolerance: f64) -> bool {
    best_obj <= best_bound + tolerance
}

/// Wrap a flattened variable of the original model into the surrogate
/// model's variable space.
fn wrap_var<B, E>(v: &InternalVar<B, E>) -> InternalVar<B, ClosestExtra<E>>
where
    B: UsableData,
    E: UsableData,
{
    match v {
        InternalVar::Base(b) => InternalVar::Base(b.clone()),
        InternalVar::Extra(e) => InternalVar::Extra(ClosestExtra::Inner(e.clone())),
        InternalVar::Helper { owner, id } => InternalVar::Helper {
            owner: ClosestExtra::Inner(owner.clone()),
            id: id.clone(),
        },
    }
}

/// Wrap a constraint source, discarding the original description (the
/// surrogate model is only ever solved for its base-variable values,
/// so descriptions carry no information here).
fn wrap_source<E, C>(src: &ConstraintSource<E, C>) -> ConstraintSource<ClosestExtra<E>, ()>
where
    E: UsableData,
    C: UsableData,
{
    match src {
        ConstraintSource::User(_) => ConstraintSource::User(()),
        ConstraintSource::DefiningExtra {
            extra,
            index,
            for_constraints,
        } => ConstraintSource::DefiningExtra {
            extra: ClosestExtra::Inner(extra.clone()),
            index: *index,
            for_constraints: *for_constraints,
        },
    }
}

/// Rebuild the model's checker problem in the surrogate variable
/// space (wrapped extras, unit `()` descriptions).
fn wrap_checker_problem<B, E, C>(
    checker: &Problem<InternalVar<B, E>, ConstraintSource<E, C>>,
) -> Result<
    Problem<InternalVar<B, ClosestExtra<E>>, ConstraintSource<ClosestExtra<E>, ()>>,
    StrategyError,
>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    let vars: HashMap<InternalVar<B, ClosestExtra<E>>, Variable> = checker
        .get_variables()
        .iter()
        .map(|(v, kind)| (wrap_var(v), kind.clone()))
        .collect();
    let constraints: Vec<_> = checker
        .get_constraints()
        .iter()
        .map(|(c, src)| (c.transmute(wrap_var), wrap_source(src)))
        .collect();
    let objective = checker.get_objective().transmute(wrap_var);

    ProblemBuilder::new()
        .set_variables(vars)
        .add_constraints(constraints)
        .set_objective(objective)
        .build()
        .map_err(|e| StrategyError::SolveError(format!("failed to wrap checker problem: {e:?}")))
}

#[async_trait]
impl Strategy for FindClosestStrategy {
    type Progress<V: UsableData + Send> = FindClosestProgressData;
    type Payload<V: UsableData + Send> = FindClosestPayload<V>;

    fn name(&self) -> &'static str {
        "find-closest"
    }

    fn ui_name(&self) -> &'static str {
        "Solution la plus proche"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        _warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: FindClosestPayload<InternalVar<B, E>>,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        // The target comes from the payload (specific to this run); `warm_start` is a
        // genuine, currently-unused optional hint.
        let target_base_values: HashMap<B, f64> = payload
            .target
            .get_values()
            .into_iter()
            .filter_map(|(v, val)| match v {
                InternalVar::Base(b) => Some((b, val)),
                _ => None,
            })
            .collect();

        // Phase 1: build the surrogate "closeness" model. Rebuild a
        // modeler from the checker problem, drop its (trivial)
        // objective, and add the L1 distance to the warm start.
        let wrapped = wrap_checker_problem(model.checker_problem())?;
        let mut modeler: Modeler<B, ClosestExtra<E>, (), (), Infallible> =
            Modeler::from_model_problem(&wrapped);
        modeler.clear_objectives();

        // Base variables (with kinds) are exactly the checker
        // problem's base variables — the checker problem always lists
        // all of them, even unconstrained ones.
        let base_kinds: Vec<(B, Variable)> = modeler
            .base_vars()
            .iter()
            .map(|(b, kind)| (b.clone(), kind.clone()))
            .collect();

        // Binary base variables get a direct linear term (`|x - v|`
        // is linear on {0,1}); non-binary ones are collected for
        // objectification into an L1 penalty.
        let mut binary_expr: LinExpr<Var<B, ClosestExtra<E>>> = LinExpr::constant(0.0);
        let mut has_binary = false;
        let mut nonbinary: HashMap<B, f64> = HashMap::new();
        for (b, kind) in base_kinds {
            let Some(&v) = target_base_values.get(&b) else {
                continue;
            };
            if kind.is_binary() {
                // minimize (1 - 2v)·x  ==  minimize |x - v| on {0,1}
                binary_expr = binary_expr + (1.0 - 2.0 * v) * LinExpr::var(Var::Base(b));
                has_binary = true;
            } else {
                nonbinary.insert(b, v);
            }
        }
        if has_binary {
            modeler.minimize(1.0, binary_expr);
        }
        if !nonbinary.is_empty() {
            // alpha = 0 (pure L1), coef = count so the averaged
            // penalty (1/n)·Σ scales back to a unit-weight sum,
            // matching the binary terms' weight of 1 each.
            let count = nonbinary.len() as f64;
            let config = ConfigData::from(nonbinary);
            let bundle: ConstraintBundle<B, ClosestExtra<E>, (), (), Infallible> =
                ConstraintBundle::from_config_data(&config, |_b, _v| ());
            let objectified = bundle
                .objectify_with_balance_and_coef(ClosestExtra::Penalty, 0.0, count)
                .map_err(|e| {
                    StrategyError::SolveError(format!(
                        "failed to objectify closeness bundle: {e:?}"
                    ))
                })?
                .into_general();
            modeler.apply_bundle(objectified).map_err(|e| {
                StrategyError::SolveError(format!("failed to apply closeness bundle: {e:?}"))
            })?;
        }

        let closest_model = modeler.build(&()).map_err(|e| {
            StrategyError::SolveError(format!("failed to build closeness model: {e:?}"))
        })?;

        // The model is ready — this can take a while, so signal it.
        if !on_progress(FindClosestProgressData::ModelReady) {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped,
                objective: None,
                best_bound: None,
                solution: None,
            });
        }

        // Phase 2: solve the surrogate model for the closest feasible
        // base assignment.
        let tolerance = self.distance_tolerance;
        let closeness_outcome = ctx
            .solve_problem_with_echo(
                closest_model.problem(),
                SolveProblemOpts {
                    warm_start: None,
                    time_limit_seconds: self.closeness_time_limit_seconds,
                    disable_logging: self.disable_logging,
                },
                &|p| {
                    let keep_going =
                        on_progress(FindClosestProgressData::ClosenessSolve((&p).into()));
                    if !keep_going {
                        return false;
                    }
                    // Stop once a feasible incumbent is within tolerance of the best
                    // bound. `best_bound.is_finite()` guards the pre-bound phase (best_bound
                    // starts at -inf for a minimize solve).
                    let good_enough = p.incumbent.is_some()
                        && p.best_bound.is_finite()
                        && p.best_obj.is_some_and(|obj| {
                            within_distance_tolerance(obj, p.best_bound, tolerance)
                        });
                    !good_enough
                },
                &|line| Some(format!("[closeness solver] {line}")),
            )
            .await?;

        match closeness_outcome.status {
            SolveStatus::Infeasible => {
                return Ok(StrategyOutcome {
                    status: SolveStatus::Infeasible,
                    objective: None,
                    best_bound: None,
                    solution: None,
                });
            }
            SolveStatus::Error => {
                return Err(StrategyError::SolveError(
                    "closeness solve returned error".into(),
                ));
            }
            SolveStatus::Stopped => {
                // A stop is either external (cancel / time limit) or our own tolerance
                // cutoff. Carry on only if the outcome actually holds a within-tolerance
                // closest point; otherwise bail as before.
                let good_enough = match (closeness_outcome.objective, closeness_outcome.best_bound)
                {
                    (Some(obj), Some(bound)) => within_distance_tolerance(obj, bound, tolerance),
                    _ => false,
                };
                if !(good_enough && closeness_outcome.solution.is_some()) {
                    return Ok(StrategyOutcome {
                        status: SolveStatus::Stopped,
                        objective: None,
                        best_bound: None,
                        solution: None,
                    });
                }
                // else: we hold a within-tolerance closest point — carry on to reconstruction.
            }
            SolveStatus::Optimal => {}
        }

        let closeness_solution = closeness_outcome.solution.ok_or_else(|| {
            StrategyError::SolveError("closeness solve optimal but no solution returned".into())
        })?;

        let should_continue = on_progress(FindClosestProgressData::ClosestFound);
        if !should_continue {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped,
                objective: None,
                best_bound: None,
                solution: None,
            });
        }

        // Phase 3: reconstruct the extra variables (and the real
        // objective) against the *original* model, fixing the base
        // values we just found.
        let base_values: HashMap<B, f64> = closeness_solution
            .get_values()
            .into_iter()
            .filter_map(|(v, val)| match v {
                InternalVar::Base(b) => Some((b, val)),
                _ => None,
            })
            .collect();

        let recon_problem = model.reconstruction_problem(&base_values).map_err(|e| {
            StrategyError::SolveError(format!("failed to build reconstruction problem: {e}"))
        })?;

        let recon_outcome = ctx
            .solve_problem_with_echo(
                &recon_problem,
                SolveProblemOpts {
                    warm_start: None,
                    time_limit_seconds: self.reconstruction_time_limit_seconds,
                    disable_logging: self.disable_logging,
                },
                &move |p| {
                    on_progress(FindClosestProgressData::ObjectiveReconstruction(
                        (&p).into(),
                    ))
                },
                &|line| Some(format!("[reconstruction solver] {line}")),
            )
            .await?;

        let recon_solution = match recon_outcome.status {
            SolveStatus::Optimal | SolveStatus::Stopped => {
                recon_outcome.solution.ok_or_else(|| {
                    StrategyError::SolveError("reconstruction produced no solution".into())
                })?
            }
            SolveStatus::Infeasible => {
                return Err(StrategyError::SolveError(
                    "reconstruction problem is infeasible".into(),
                ));
            }
            SolveStatus::Error => {
                return Err(StrategyError::SolveError(
                    "reconstruction solve returned error".into(),
                ));
            }
        };

        // Combine base + reconstruction into a complete solution.
        let mut complete_values: HashMap<InternalVar<B, E>, f64> = base_values
            .into_iter()
            .map(|(b, v)| (InternalVar::Base(b), v))
            .collect();
        complete_values.extend(recon_solution.get_values());
        let complete_config = ConfigData::from(complete_values);

        Ok(StrategyOutcome {
            status: SolveStatus::Optimal,
            objective: recon_outcome.objective,
            best_bound: recon_outcome.best_bound,
            solution: Some(complete_config),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FindClosestProgressData {
    /// The surrogate closeness model has been assembled and is about
    /// to be solved. Emitted once, since modeling can take a while.
    ModelReady,
    /// Progress from solving the surrogate closeness model.
    ClosenessSolve(NoObjectiveSolveProgress),
    /// A solution has been found. We still need to evaluate its objective value
    ClosestFound,
    /// Progress from reconstructing the extra variables and objective.
    ObjectiveReconstruction(NoObjectiveSolveProgress),
}

impl fmt::Display for FindClosestProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FindClosestProgressData::ModelReady => {
                write!(f, "Closeness model ready, solving...")
            }
            FindClosestProgressData::ClosenessSolve(p) => {
                write!(f, "[closeness solver progress] {p}")
            }
            FindClosestProgressData::ClosestFound => {
                write!(
                    f,
                    "Closest solution found. Starting rebuilding its objective value..."
                )
            }
            FindClosestProgressData::ObjectiveReconstruction(p) => {
                write!(f, "[reconstruction solver progress] {p}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use collomatique_ilp::ProblemDesc;
    use collomatique_ilp::solvers::{Solver, SolverModel, collo_cbc::ColloCbcSolver};
    use collomatique_ilp_modeler::model_desc::ModelDesc;

    use crate::{
        RawSolveOutcome, SolveBackend, SolveConfig, SolveProgressData, StrategyContext,
        StrategyKind, StrategyPayloadData, StrategyProgressData,
    };

    /// A backend that actually solves each problem it is handed with CBC, so the
    /// closeness solve and the reconstruction solve each get a correct answer (a
    /// single canned outcome could not serve both). Emits one progress event per
    /// solve so ordering can be asserted.
    struct RealBackend;

    #[async_trait]
    impl SolveBackend for RealBackend {
        async fn solve_with_progress(
            &self,
            desc: &ProblemDesc,
            _opts: SolveConfig,
            on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            on_progress(SolveProgressData {
                best_obj: None,
                best_bound: 0.0,
                node_count: 0,
                solutions_found: 0,
                incumbent: None,
            });

            let n = desc.variables.len();
            let problem = collomatique_ilp::ProblemBuilder::<usize, ()>::from_desc(desc.clone())
                .build()
                .expect("valid problem desc");
            match ColloCbcSolver::new().build_model(&problem).solve() {
                Some(cfg) => {
                    let solution: Vec<f64> = (0..n).map(|i| cfg.get(i).unwrap_or(0.0)).collect();
                    let obj = cfg.eval();
                    Ok(RawSolveOutcome {
                        status: SolveStatus::Optimal,
                        objective: Some(obj),
                        best_bound: Some(obj),
                        solution: Some(solution),
                    })
                }
                None => Ok(RawSolveOutcome {
                    status: SolveStatus::Infeasible,
                    objective: None,
                    best_bound: None,
                    solution: None,
                }),
            }
        }

        async fn run_strategy_subprocess(
            &self,
            _model_desc: &ModelDesc,
            _strategy: &StrategyKind,
            _warm_start: Option<Vec<f64>>,
            _payload: StrategyPayloadData,
            _on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            unimplemented!("find_closest tests never spawn subprocesses")
        }
    }

    fn strategy() -> FindClosestStrategy {
        FindClosestStrategy {
            closeness_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: true,
            // Solve to the exact closest point so distance-based assertions are stable.
            distance_tolerance: 0.0,
        }
    }

    #[test]
    fn within_distance_tolerance_is_a_one_sided_absolute_gap() {
        // bound = 1000, tolerance = 10: accept up to 1010, reject beyond.
        assert!(within_distance_tolerance(1010.0, 1000.0, 10.0));
        assert!(!within_distance_tolerance(1011.0, 1000.0, 10.0));
        assert!(within_distance_tolerance(1000.0, 1000.0, 10.0));
        // Zero tolerance: accept only when the incumbent reaches the bound.
        assert!(within_distance_tolerance(1000.0, 1000.0, 0.0));
        assert!(!within_distance_tolerance(1001.0, 1000.0, 0.0));
    }

    /// Binary base variables, warm start (1, 1) is infeasible under a + b <= 1.
    /// The closest feasible point sits at L1 distance 1: exactly one of a, b is 1.
    #[tokio::test]
    async fn binary_closest_to_infeasible_warm_start() {
        let vars: HashMap<usize, Variable> =
            [(0, Variable::binary()), (1, Variable::binary())].into();
        let mut modeler: Modeler<usize, (), (), (), ()> = Modeler::new(vars);
        modeler.add_constraint(
            (LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1))).leq(&LinExpr::constant(1.0)),
            (),
        );
        let model = modeler.build(&()).unwrap();

        let warm = ConfigData::from(HashMap::from([
            (InternalVar::Base(0usize), 1.0),
            (InternalVar::Base(1usize), 1.0),
        ]));

        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let events: Mutex<Vec<FindClosestProgressData>> = Mutex::new(Vec::new());
        let outcome = strategy()
            .run_with_callback(
                &ctx,
                &model,
                None,
                FindClosestPayload { target: warm },
                &|p| {
                    events.lock().unwrap().push(p);
                    true
                },
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        let sol = outcome.solution.unwrap();
        let a = sol.get(InternalVar::Base(0usize)).unwrap();
        let b = sol.get(InternalVar::Base(1usize)).unwrap();
        assert!(a + b <= 1.0 + 1e-9, "must respect a + b <= 1");
        assert_eq!(
            a + b,
            1.0,
            "closest feasible point sets exactly one of a, b"
        );

        // ModelReady fires first, before any solve progress.
        let events = events.into_inner().unwrap();
        assert_eq!(events.first(), Some(&FindClosestProgressData::ModelReady));
        let ready_idx = events
            .iter()
            .position(|e| matches!(e, FindClosestProgressData::ModelReady))
            .unwrap();
        let closeness_idx = events
            .iter()
            .position(|e| matches!(e, FindClosestProgressData::ClosenessSolve(_)))
            .expect("a closeness solve progress event is emitted");
        assert!(ready_idx < closeness_idx, "model-ready precedes solving");
    }

    /// A non-binary (integer, [0, 5]) base variable exercises the bundle +
    /// objectify path. Warm start x = 5 is infeasible under x <= 3; the closest
    /// feasible value is 3.
    #[tokio::test]
    async fn nonbinary_closest_uses_objectify() {
        let vars: HashMap<usize, Variable> = [(0, Variable::integer().min(0.0).max(5.0))].into();
        let mut modeler: Modeler<usize, (), (), (), ()> = Modeler::new(vars);
        modeler.add_constraint(LinExpr::var(Var::Base(0)).leq(&LinExpr::constant(3.0)), ());
        let model = modeler.build(&()).unwrap();

        let warm = ConfigData::from(HashMap::from([(InternalVar::Base(0usize), 5.0)]));

        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let outcome = strategy()
            .run_with_callback(
                &ctx,
                &model,
                None,
                FindClosestPayload { target: warm },
                &|_| true,
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        let sol = outcome.solution.unwrap();
        assert_eq!(sol.get(InternalVar::Base(0usize)).unwrap(), 3.0);
    }
}
