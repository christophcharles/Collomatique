use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::Infallible;
use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, LinExpr, Problem, UsableData, Variable};
use collomatique_ilp_modeler::{
    ConstraintBundle, ConstraintSource, InternalVar, Model, Modeler, Var,
};

use crate::{
    NoObjectiveSolveProgress, SolveProblemOpts, SolveStatus, StopReason, Strategy, StrategyContext,
    StrategyError, StrategyOutcome, VarOrderSerializable,
};

/// Per-run payload for [`IncrementalStrategy`]: an epoch index for (some of) the base
/// variables. The map is keyed by the model's *base* variable type, so entries naming
/// extras or helpers are unrepresentable. A base variable absent from the map is solved
/// in the *final* epoch (`max epoch + 1`). An entry naming a base variable that is not
/// part of the model (a stale name) is ignored.
#[derive(Debug, Clone)]
pub struct IncrementalPayload<B: UsableData> {
    pub epochs: HashMap<B, u32>,
}

// Manual (not derived): `#[derive(Default)]` would wrongly require `B: Default`. An empty epoch map
// puts every base variable in the final epoch, i.e. a single priming solve.
impl<B: UsableData> Default for IncrementalPayload<B> {
    fn default() -> Self {
        IncrementalPayload {
            epochs: HashMap::new(),
        }
    }
}

/// Serializable counterpart of [`IncrementalPayload<B>`]: the epoch of each variable is
/// erased to a column-indexed `Vec<Option<u32>>` against the model's `var_order`
/// (`Some(e)` = epoch `e`; `None` = unlisted → final epoch), so it can cross the IPC barrier.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IncrementalPayloadData {
    pub epochs: Vec<Option<u32>>,
}

impl<B, E> VarOrderSerializable<InternalVar<B, E>> for IncrementalPayload<B>
where
    B: UsableData + Send,
    E: UsableData + Send,
{
    type Data = IncrementalPayloadData;
    type Error = Infallible;
    fn into_data(
        &self,
        var_order: &[InternalVar<B, E>],
    ) -> Result<IncrementalPayloadData, Infallible> {
        // `None` distinguishes "unlisted" (→ final epoch) from "epoch 0"; non-base columns
        // are always `None` (the payload cannot name them).
        let epochs = var_order
            .iter()
            .map(|v| match v {
                InternalVar::Base(b) => self.epochs.get(b).copied(),
                _ => None,
            })
            .collect();
        Ok(IncrementalPayloadData { epochs })
    }
    fn from_data(
        data: &IncrementalPayloadData,
        var_order: &[InternalVar<B, E>],
    ) -> Result<Self, Infallible> {
        let epochs = data
            .epochs
            .iter()
            .zip(var_order)
            .filter_map(|(opt, v)| match (opt, v) {
                (Some(e), InternalVar::Base(b)) => Some((b.clone(), *e)),
                _ => None,
            })
            .collect();
        Ok(IncrementalPayload { epochs })
    }
}

/// Extra-variable name for the per-epoch surrogate model. The original extras are wrapped
/// as [`IncrementalExtra::Inner`]; the L1 penalty produced by objectifying the non-binary
/// anchor constraints gets the reserved [`IncrementalExtra::Penalty`] name.
///
/// This wrapper only ever exists inside [`IncrementalStrategy`]: every solution is reduced
/// back to its base variables before anything leaves the strategy, so the wrapper never
/// reaches the caller.
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
enum IncrementalExtra<E> {
    Inner(E),
    Penalty,
}

/// Solve the ILP in staggered *epochs*, progressively enlarging the set of active base
/// variables. Each epoch optimizes only its own *margin* of the true objective, while
/// previous epochs' decisions are held in place by a big-weight L1 anchor to the previous
/// pass's solution (a soft penalty, so they may flex for consistency rather than being
/// hard-fixed). A final reconstruction pass recovers the extra variables and the true
/// objective value on the original model.
///
/// The epoch of each base variable is supplied as the payload; unlisted base variables are
/// solved together in a final epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncrementalStrategy {
    /// Big weight `W` applied to the L1 anchor that ties each epoch to the previous pass's
    /// solution. Larger values make previous decisions stickier relative to the epoch's own
    /// margin of the true objective.
    pub l1_weight: f64,
    /// Absolute tolerance on each epoch's objective gap: stop an epoch's solve as soon as a
    /// feasible incumbent is within this distance of the best bound, instead of proving the
    /// epoch optimal. Mirrors [`FindClosestStrategy::distance_tolerance`](crate::FindClosestStrategy::distance_tolerance).
    /// `0.0` means "solve each epoch to proven optimality".
    pub distance_tolerance: f64,
    /// Time limit for each epoch's solve.
    pub epoch_time_limit: collomatique_time::TimeLimit,
    /// Time limit on each epoch's solve, counted from that epoch's first feasible incumbent
    /// and independent of [`IncrementalStrategy::epoch_time_limit`]: the epoch stops at
    /// whichever deadline comes first. Each epoch is its own solve, so the clock restarts
    /// every epoch. [`TimeLimit::none()`](collomatique_time::TimeLimit::none) disables it; the
    /// default is one minute. The final reconstruction solve is unaffected.
    pub epoch_incumbent_time_limit: collomatique_time::TimeLimit,
    /// Time limit for the final reconstruction solve.
    pub reconstruction_time_limit: collomatique_time::TimeLimit,
    pub disable_logging: bool,
}

impl Default for IncrementalStrategy {
    fn default() -> Self {
        IncrementalStrategy {
            l1_weight: 1000.0,
            distance_tolerance: 10.0,
            epoch_time_limit: collomatique_time::TimeLimit::none(),
            epoch_incumbent_time_limit: collomatique_time::TimeLimit::seconds(
                std::num::NonZeroU32::new(60).expect("60 is non-zero"),
            ),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
            disable_logging: false,
        }
    }
}

/// Wrap a flattened variable of the original model into the per-epoch surrogate's variable
/// space.
fn wrap_var<B, E>(v: &InternalVar<B, E>) -> InternalVar<B, IncrementalExtra<E>>
where
    B: UsableData,
    E: UsableData,
{
    match v {
        InternalVar::Base(b) => InternalVar::Base(b.clone()),
        InternalVar::Extra(e) => InternalVar::Extra(IncrementalExtra::Inner(e.clone())),
        InternalVar::Helper { owner, id } => InternalVar::Helper {
            owner: IncrementalExtra::Inner(owner.clone()),
            id: id.clone(),
        },
    }
}

/// Wrap a constraint source, discarding the original description (the surrogate model is
/// only ever solved for its base-variable values, so descriptions carry no information here).
fn wrap_source<E, C>(src: &ConstraintSource<E, C>) -> ConstraintSource<IncrementalExtra<E>, ()>
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
            extra: IncrementalExtra::Inner(extra.clone()),
            index: *index,
            for_constraints: *for_constraints,
        },
    }
}

/// Rebuild a filtered epoch problem in the surrogate variable space (wrapped extras, unit
/// `()` descriptions). Unlike [`find_closest`](super::find_closest), the objective is kept:
/// it is the epoch's margin of the true objective.
fn wrap_problem<B, E, C>(
    problem: &Problem<InternalVar<B, E>, ConstraintSource<E, C>>,
) -> Result<
    Problem<InternalVar<B, IncrementalExtra<E>>, ConstraintSource<IncrementalExtra<E>, ()>>,
    StrategyError,
>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    problem
        .transmute(wrap_var, wrap_source)
        .map_err(|e| StrategyError::SolveError(format!("failed to wrap epoch problem: {e:?}")))
}

/// Whether an epoch incumbent `best_obj` is within `tolerance` of the best bound. Unlike
/// [`find_closest`](super::find_closest), whose objective is purely the (minimized) L1 distance,
/// an epoch objective is `margin + l1_weight·distance` and inherits the original model's sense,
/// which is not necessarily a minimization. The bound and the incumbent always bracket the
/// optimum from opposite sides regardless of sense (a lower bound for minimize, an upper bound for
/// maximize), so the absolute gap `|best_obj - best_bound|` is a sense-agnostic (over-)estimate of
/// how far the incumbent can be from optimal.
fn within_distance_tolerance(best_obj: f64, best_bound: f64, tolerance: f64) -> bool {
    (best_obj - best_bound).abs() <= tolerance
}

/// Add the L1 anchor to `modeler`, tying the anchored base variables to `prev_values` with
/// weight `l1_weight`. Binary variables contribute a direct linear term (`|x - v|` is linear
/// on {0,1}); non-binary ones are objectified into an L1 penalty (the
/// [`IncrementalExtra::Penalty`] extra). Mirrors [`find_closest`](super::find_closest)'s split.
fn add_l1_anchor<'m, B, E>(
    modeler: &mut Modeler<'m, B, IncrementalExtra<E>, (), (), Infallible>,
    prev_values: &HashMap<B, f64>,
    l1_weight: f64,
) -> Result<(), StrategyError>
where
    B: UsableData + 'm,
    E: UsableData + 'm,
{
    let base_kinds: HashMap<B, Variable> = modeler.base_vars().clone();

    let mut binary_expr: LinExpr<Var<B, IncrementalExtra<E>>> = LinExpr::constant(0.0);
    let mut has_binary = false;
    let mut nonbinary: HashMap<B, f64> = HashMap::new();
    for (b, &v) in prev_values {
        let Some(kind) = base_kinds.get(b) else {
            // A previously-solved variable not declared in this epoch's problem cannot
            // happen (S_{k-1} ⊆ S_k), but skip defensively rather than panic.
            continue;
        };
        if kind.is_binary() {
            // minimize (1 - 2v)·x  ==  minimize |x - v| on {0,1}
            binary_expr = binary_expr + (1.0 - 2.0 * v) * LinExpr::var(Var::Base(b.clone()));
            has_binary = true;
        } else {
            nonbinary.insert(b.clone(), v);
        }
    }

    if has_binary {
        modeler.minimize(l1_weight, binary_expr);
    }
    if !nonbinary.is_empty() {
        // alpha = 0 (pure L1); coef = W·count so the averaged penalty (1/n)·Σ scales back
        // to a unit-weight sum, each anchored variable weighted by W (matching the binary
        // terms' weight of W each).
        let count = nonbinary.len() as f64;
        let config = ConfigData::from(nonbinary);
        let bundle: ConstraintBundle<B, IncrementalExtra<E>, (), (), Infallible> =
            ConstraintBundle::from_config_data(&config, |_b, _v| ());
        let objectified = bundle
            .objectify_with_balance_and_coef(IncrementalExtra::Penalty, 0.0, l1_weight * count)
            .map_err(|e| {
                StrategyError::SolveError(format!("failed to objectify anchor bundle: {e:?}"))
            })?
            .into_general();
        modeler.apply_bundle(objectified).map_err(|e| {
            StrategyError::SolveError(format!("failed to apply anchor bundle: {e:?}"))
        })?;
    }
    Ok(())
}

/// A [`StrategyOutcome`] that carries no solution (the run stopped or was infeasible).
fn empty_outcome<V: UsableData>(status: SolveStatus) -> StrategyOutcome<V> {
    StrategyOutcome {
        status,
        objective: None,
        best_bound: None,
        solution: None,
    }
}

#[async_trait]
impl Strategy for IncrementalStrategy {
    type Progress<V: UsableData + Send> = IncrementalProgressData;
    type Payload<B: UsableData + Send, E: UsableData + Send> = IncrementalPayload<B>;

    fn name(&self) -> &'static str {
        "incremental"
    }

    fn ui_name(&self) -> &'static str {
        "Incrémental"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        _warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: IncrementalPayload<B>,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        // The model's full base-variable set (every base variable is declared in the built
        // problem, referenced or not).
        let all_base: HashSet<B> = model
            .problem()
            .get_variables()
            .keys()
            .filter_map(|v| match v {
                InternalVar::Base(b) => Some(b.clone()),
                _ => None,
            })
            .collect();

        // Drop entries naming base variables that are not part of the model (stale names keep
        // the documented "ignored" behavior; non-base entries are unrepresentable).
        let mut epoch_of: HashMap<B, u32> = payload
            .epochs
            .into_iter()
            .filter(|(b, _)| all_base.contains(b))
            .collect();

        // Unlisted base variables form the final epoch (`max + 1`).
        let final_epoch = epoch_of.values().copied().max().map_or(0, |m| m + 1);
        for b in &all_base {
            epoch_of.entry(b.clone()).or_insert(final_epoch);
        }

        // Group base variables by epoch, in increasing epoch order; empty epochs simply do
        // not appear.
        let mut by_epoch: BTreeMap<u32, HashSet<B>> = BTreeMap::new();
        for (b, e) in &epoch_of {
            by_epoch.entry(*e).or_default().insert(b.clone());
        }
        let total = by_epoch.len();

        let graph = model.dependency_graph();
        let mut blessed: HashSet<B> = HashSet::new(); // S_k, grows each epoch
        let mut prev_values: Option<HashMap<B, f64>> = None; // most recent solve, over S_{k-1}
        let tolerance = self.distance_tolerance;

        for (seq, (_epoch, e_k)) in by_epoch.into_iter().enumerate() {
            // S_k = S_{k-1} ∪ E_k.
            for b in &e_k {
                blessed.insert(b.clone());
            }
            let s_k = &blessed;

            if !on_progress(IncrementalProgressData::EpochStarted {
                epoch: seq,
                total,
                var_count: e_k.len(),
            }) {
                return Ok(empty_outcome(SolveStatus::Stopped(StopReason::Callback)));
            }

            // 1. Filter the model down to this epoch's sub-problem. Every predicate is keyed
            //    on the base footprint, so they are mutually consistent (no undeclared-var
            //    error). The objective keeps only terms whose footprint is complete under
            //    `S_k` and touches this epoch's new variables `E_k` — the epoch's margin.
            let filtered = model
                .filter(
                    |c, _desc| graph.constraint_footprint(c).is_subset(s_k),
                    |b| s_k.contains(b),
                    |ex| graph.base_footprint(ex).is_subset(s_k),
                    |v| {
                        let fp = graph.var_footprint(v);
                        fp.is_subset(s_k) && !fp.is_disjoint(&e_k)
                    },
                )
                .map_err(|e| {
                    StrategyError::SolveError(format!(
                        "failed to filter epoch {}: {:?}",
                        seq + 1,
                        e
                    ))
                })?;

            // 2. Re-model in the surrogate space, keeping the epoch's margin objective, and
            //    add the L1 anchor to the previous pass's solution (no anchor for epoch 0).
            let wrapped = wrap_problem(&filtered)?;
            let mut modeler: Modeler<B, IncrementalExtra<E>, (), (), Infallible> =
                Modeler::from_model_problem(&wrapped);
            if let Some(prev) = &prev_values {
                add_l1_anchor(&mut modeler, prev, self.l1_weight)?;
            }

            let epoch_model = modeler.build(&()).map_err(|e| {
                StrategyError::SolveError(format!(
                    "failed to build epoch {} model: {:?}",
                    seq + 1,
                    e
                ))
            })?;

            // 3. Solve this epoch.
            let seq_for_progress = seq;
            let outcome = ctx
                .solve_problem_with_echo(
                    epoch_model.problem(),
                    SolveProblemOpts {
                        warm_start: None,
                        time_limit: self.epoch_time_limit,
                        incumbent_time_limit: self.epoch_incumbent_time_limit,
                        disable_logging: self.disable_logging,
                    },
                    &move |p| {
                        let keep_going = on_progress(IncrementalProgressData::EpochSolve {
                            epoch: seq_for_progress,
                            total,
                            var_count: e_k.len(),
                            progress: (&p).into(),
                        });
                        if !keep_going {
                            return false;
                        }
                        // Stop once a feasible incumbent is within tolerance of the best bound.
                        // `best_bound.is_finite()` guards the pre-bound phase (best_bound starts
                        // at ±inf depending on the objective sense).
                        let good_enough = p.incumbent.is_some()
                            && p.best_bound.is_finite()
                            && p.best_obj.is_some_and(|obj| {
                                within_distance_tolerance(obj, p.best_bound, tolerance)
                            });
                        !good_enough
                    },
                    &|line| Some(format!("[epoch {}/{} solver] {}", seq + 1, total, line)),
                )
                .await?;

            let stopped_reason = match &outcome.status {
                SolveStatus::Stopped(reason) => Some(*reason),
                _ => None,
            };
            match outcome.status {
                SolveStatus::Optimal | SolveStatus::Stopped(_) => {
                    let Some(solution) = outcome.solution else {
                        // Stopped without an incumbent: propagate as stopped, keeping the
                        // solver's reason when it gave one.
                        return Ok(empty_outcome(SolveStatus::Stopped(
                            stopped_reason.unwrap_or(StopReason::Callback),
                        )));
                    };
                    // Overwrite `prev_values` wholesale with this solve's entire base
                    // assignment over S_k (flexed previous values included).
                    let base_values: HashMap<B, f64> = solution
                        .get_values()
                        .into_iter()
                        .filter_map(|(v, val)| match v {
                            InternalVar::Base(b) => Some((b, val)),
                            _ => None,
                        })
                        .collect();
                    prev_values = Some(base_values);
                }
                SolveStatus::Infeasible => {
                    // A filtered sub-problem is a subset of the full problem's
                    // constraints/variables, so its infeasibility implies the full problem
                    // is infeasible too.
                    return Ok(empty_outcome(SolveStatus::Infeasible));
                }
                SolveStatus::Error => {
                    return Err(StrategyError::SolveError(format!(
                        "epoch {} solve returned error",
                        seq + 1
                    )));
                }
            }
        }

        // After the final epoch, `prev_values` covers every base variable. Reconstruct the
        // extras and the true objective on the original model.
        // No epoch ran (the problem has no base variables). Fall through to the reconstruction
        // path with an empty base assignment — an empty problem is trivially solved, matching how
        // DefaultStrategy delegates an empty model straight to the solver.
        let base_values = prev_values.unwrap_or_default();

        let recon_problem = model.reconstruction_problem(&base_values).map_err(|e| {
            StrategyError::SolveError(format!("failed to build reconstruction problem: {e}"))
        })?;

        // Announce the cost-computation phase before solving: the reconstruction is forced and may
        // report no progress until it finishes, so this is what flips the UI to "calcul de l'objectif".
        let _ = on_progress(IncrementalProgressData::ReconstructionStarted { total });

        let recon_outcome = ctx
            .solve_problem_with_echo(
                &recon_problem,
                SolveProblemOpts {
                    warm_start: None,
                    time_limit: self.reconstruction_time_limit,
                    incumbent_time_limit: collomatique_time::TimeLimit::none(),
                    disable_logging: self.disable_logging,
                },
                &move |p| {
                    on_progress(IncrementalProgressData::Reconstruction {
                        total,
                        progress: (&p).into(),
                    })
                },
                &|line| Some(format!("[reconstruction solver] {line}")),
            )
            .await?;

        let recon_solution = match recon_outcome.status {
            SolveStatus::Optimal | SolveStatus::Stopped(_) => {
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

        // Terminal event carrying the true cost: the reconstruction's own progress reports no
        // incumbent (the solve is forced), so this is the only reliable carrier of the objective.
        let _ = on_progress(IncrementalProgressData::Done {
            total,
            objective: recon_outcome.objective,
        });

        Ok(StrategyOutcome {
            status: SolveStatus::Optimal,
            objective: recon_outcome.objective,
            best_bound: recon_outcome.best_bound,
            solution: Some(complete_config),
        })
    }
}

/// Serializable progress for [`IncrementalStrategy`]. Epoch indices are sequential over the
/// non-empty epochs actually solved (empty epochs are skipped).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IncrementalProgressData {
    /// A new epoch has started. `epoch` is 0-based over the non-empty epochs; `total` is the
    /// number of non-empty epochs; `var_count` is how many new base variables this epoch adds.
    EpochStarted {
        epoch: usize,
        total: usize,
        var_count: usize,
    },
    /// Progress from solving an epoch's sub-problem.
    EpochSolve {
        epoch: usize,
        total: usize,
        var_count: usize,
        progress: NoObjectiveSolveProgress,
    },
    /// The final reconstruction (extras + true cost) is about to start. Emitted once, before the
    /// reconstruction solve, so the UI can show the cost-computation step even when that solve is
    /// forced and reports no progress until it finishes.
    ReconstructionStarted { total: usize },
    /// Progress from the final reconstruction (extras + true objective).
    Reconstruction {
        total: usize,
        progress: NoObjectiveSolveProgress,
    },
    /// The run has finished: reconstruction is done and `objective` is the true cost on the
    /// original model (`None` only if the solve produced no objective). Emitted once, last.
    /// The reconstruction solve is essentially forced, so its incumbent (and thus the
    /// [`Reconstruction`](Self::Reconstruction) progress `best_obj`) is typically never
    /// reported mid-solve — this terminal event is the only reliable carrier of the cost.
    Done {
        total: usize,
        objective: Option<f64>,
    },
}

impl fmt::Display for IncrementalProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IncrementalProgressData::EpochStarted {
                epoch,
                total,
                var_count,
            } => write!(
                f,
                "Epoch {}/{} starting ({var_count} new variables)...",
                epoch + 1,
                total
            ),
            IncrementalProgressData::EpochSolve {
                epoch,
                total,
                var_count: _,
                progress,
            } => {
                write!(
                    f,
                    "[epoch {}/{} solver progress] {progress}",
                    epoch + 1,
                    total
                )
            }
            IncrementalProgressData::ReconstructionStarted { total: _ } => {
                write!(f, "Reconstruction starting (computing cost)...")
            }
            IncrementalProgressData::Reconstruction { total: _, progress } => {
                write!(f, "[reconstruction solver progress] {progress}")
            }
            IncrementalProgressData::Done {
                total: _,
                objective,
            } => {
                write!(f, "Done (objective ")?;
                match objective {
                    Some(obj) => write!(f, "{obj:.4}")?,
                    None => write!(f, "—")?,
                }
                write!(f, ")")
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
    use collomatique_ilp::{Objective, ObjectiveSense};
    use collomatique_ilp_modeler::ExtraVar;
    use collomatique_ilp_modeler::model_desc::ModelDesc;

    use crate::{
        RawSolveOutcome, SolveBackend, SolveConfig, SolveProgressData, StrategyContext,
        StrategyKind, StrategyPayloadData, StrategyProgressData,
    };

    /// A backend that actually solves each problem it is handed with CBC, so every epoch
    /// solve and the reconstruction solve get a correct answer. Emits one progress event
    /// per solve so ordering can be asserted.
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
            unimplemented!("incremental tests never spawn subprocesses")
        }
    }

    fn strategy() -> IncrementalStrategy {
        IncrementalStrategy {
            l1_weight: 1000.0,
            // Solve each epoch to proven optimality so assertions are stable.
            distance_tolerance: 0.0,
            epoch_time_limit: collomatique_time::TimeLimit::none(),
            epoch_incumbent_time_limit: collomatique_time::TimeLimit::none(),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
            disable_logging: true,
        }
    }

    /// The payload's epoch map is aligned to `var_order` on the way out and reconstructed on
    /// the way in, distinguishing "unlisted" (`None`) from "epoch 0". The payload is keyed by
    /// the base variable, so a non-base column of the order always encodes as `None`.
    #[test]
    fn payload_round_trips_against_var_order() {
        let var_order: Vec<InternalVar<usize, usize>> = vec![
            InternalVar::Base(0),
            InternalVar::Extra(7),
            InternalVar::Base(1),
            InternalVar::Base(2),
        ];
        let epochs: HashMap<usize, u32> = HashMap::from([(0usize, 0u32), (2usize, 1u32)]);
        let payload = IncrementalPayload {
            epochs: epochs.clone(),
        };

        let data = VarOrderSerializable::into_data(&payload, &var_order).unwrap();
        assert_eq!(
            data,
            IncrementalPayloadData {
                epochs: vec![Some(0), None, None, Some(1)]
            }
        );

        let restored = <IncrementalPayload<usize> as VarOrderSerializable<
            InternalVar<usize, usize>,
        >>::from_data(&data, &var_order)
        .unwrap();
        assert_eq!(restored.epochs, epochs);
    }

    /// Two binary base variables solved across two epochs, with an extra tying them together
    /// so reconstruction is exercised. x0 is solved first (maximize x0 → 1), then x1 is added
    /// under `x0 + x1 <= 1` with x0 anchored to 1, forcing x1 = 0.
    #[tokio::test]
    async fn two_epoch_binary_end_to_end() {
        let vars: HashMap<usize, Variable> =
            [(0, Variable::binary()), (1, Variable::binary())].into();
        let mut modeler: Modeler<usize, usize, (), (), ()> = Modeler::new(vars);
        // extra e = x0 + x1.
        modeler
            .declare_extra(100usize, Variable::integer(), |_f, _ctx, e| {
                Ok(vec![
                    LinExpr::var(ExtraVar::Extra(e))
                        .eq(&(LinExpr::var(ExtraVar::Base(0)) + LinExpr::var(ExtraVar::Base(1)))),
                ])
            })
            .unwrap();
        // user constraint on the extra: e <= 1.
        modeler.add_constraint(
            LinExpr::var(Var::Extra(100)).leq(&LinExpr::constant(1.0)),
            (),
        );
        modeler.add_objective(
            1.0,
            Objective::new(
                LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1)),
                ObjectiveSense::Maximize,
            ),
        );
        let model = modeler.build(&()).unwrap();

        let payload = IncrementalPayload {
            epochs: HashMap::from([(0usize, 0u32), (1usize, 1u32)]),
        };

        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let events: Mutex<Vec<IncrementalProgressData>> = Mutex::new(Vec::new());
        let outcome = strategy()
            .run_with_callback(&ctx, &model, None, payload, &|p| {
                events.lock().unwrap().push(p);
                true
            })
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        let sol = outcome.solution.unwrap();
        let x0 = sol.get(InternalVar::Base(0usize)).unwrap();
        let x1 = sol.get(InternalVar::Base(1usize)).unwrap();
        assert_eq!(x0, 1.0, "x0 solved first, maximized then anchored to 1");
        assert_eq!(x1, 0.0, "x1 forced to 0 by x0 + x1 <= 1");
        // The extra is reconstructed: e = x0 + x1 = 1.
        assert_eq!(sol.get(InternalVar::Extra(100usize)).unwrap(), 1.0);
        assert_eq!(outcome.objective, Some(1.0));

        // Progress visited epoch 0, epoch 1 (started), then reconstruction, in that order.
        let events = events.into_inner().unwrap();
        let epoch0 = events.iter().position(|e| {
            matches!(
                e,
                IncrementalProgressData::EpochStarted {
                    epoch: 0,
                    total: 2,
                    ..
                }
            )
        });
        let epoch1 = events.iter().position(|e| {
            matches!(
                e,
                IncrementalProgressData::EpochStarted {
                    epoch: 1,
                    total: 2,
                    ..
                }
            )
        });
        let recon = events
            .iter()
            .position(|e| matches!(e, IncrementalProgressData::Reconstruction { .. }));
        let (epoch0, epoch1, recon) = (
            epoch0.expect("epoch 0 started"),
            epoch1.expect("epoch 1 started"),
            recon.expect("reconstruction ran"),
        );
        assert!(
            epoch0 < epoch1 && epoch1 < recon,
            "epochs then reconstruction"
        );
    }

    /// A non-binary (integer, [0, 5]) base variable anchored across epochs exercises the
    /// bundle + objectify path. x0 is maximized to 5 in epoch 0, then anchored while x1 is
    /// added under `x0 + x1 <= 4`; the big anchor weight pulls x0 as close to 5 as feasible
    /// (x0 = 4).
    #[tokio::test]
    async fn nonbinary_anchor_uses_objectify() {
        let vars: HashMap<usize, Variable> = [
            (0, Variable::integer().min(0.0).max(5.0)),
            (1, Variable::integer().min(0.0).max(5.0)),
        ]
        .into();
        let mut modeler: Modeler<usize, usize, (), (), ()> = Modeler::new(vars);
        modeler.add_constraint(
            (LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1))).leq(&LinExpr::constant(4.0)),
            (),
        );
        modeler.add_objective(
            1.0,
            Objective::new(
                LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1)),
                ObjectiveSense::Maximize,
            ),
        );
        let model = modeler.build(&()).unwrap();

        let payload = IncrementalPayload {
            epochs: HashMap::from([(0usize, 0u32), (1usize, 1u32)]),
        };

        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let outcome = strategy()
            .run_with_callback(&ctx, &model, None, payload, &|_| true)
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        let sol = outcome.solution.unwrap();
        let x0 = sol.get(InternalVar::Base(0usize)).unwrap();
        let x1 = sol.get(InternalVar::Base(1usize)).unwrap();
        assert_eq!(
            x0, 4.0,
            "anchor pulls x0 to the closest feasible value to 5"
        );
        assert_eq!(x1, 0.0);
    }

    /// An empty epoch index is skipped, and a payload entry naming a base variable that is not
    /// part of the model (a stale name) is ignored. Here x0 → epoch 0 and x1 → epoch 2 (epoch 1
    /// is empty), plus a bogus variable at epoch 0; the run visits exactly two epochs.
    #[tokio::test]
    async fn empty_epoch_skipped_and_unknown_var_ignored() {
        let vars: HashMap<usize, Variable> =
            [(0, Variable::binary()), (1, Variable::binary())].into();
        let mut modeler: Modeler<usize, usize, (), (), ()> = Modeler::new(vars);
        modeler.add_constraint(
            (LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1))).leq(&LinExpr::constant(1.0)),
            (),
        );
        modeler.add_objective(
            1.0,
            Objective::new(
                LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1)),
                ObjectiveSense::Maximize,
            ),
        );
        let model = modeler.build(&()).unwrap();

        let payload = IncrementalPayload {
            epochs: HashMap::from([
                (0usize, 0u32),
                (1usize, 2u32),
                (99usize, 0u32), // not a base variable of the model
            ]),
        };

        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let events: Mutex<Vec<IncrementalProgressData>> = Mutex::new(Vec::new());
        let outcome = strategy()
            .run_with_callback(&ctx, &model, None, payload, &|p| {
                events.lock().unwrap().push(p);
                true
            })
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);

        // Exactly two non-empty epochs were started (epoch 1 was empty, the bogus var ignored).
        let started: Vec<usize> = events
            .into_inner()
            .unwrap()
            .into_iter()
            .filter_map(|e| match e {
                IncrementalProgressData::EpochStarted { epoch, total, .. } => {
                    assert_eq!(total, 2, "two non-empty epochs");
                    Some(epoch)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            started,
            vec![0, 1],
            "sequential indices over non-empty epochs"
        );
    }
}
