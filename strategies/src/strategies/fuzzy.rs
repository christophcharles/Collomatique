use std::fmt;

use async_trait::async_trait;
use rand::SeedableRng;
use rand::distr::Distribution;
use rand::rngs::StdRng;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, UsableData, Variable};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::strategies::find_closest::{FindClosestProgressData, FindClosestStrategy};
use crate::{SolveStatus, Strategy, StrategyContext, StrategyError, StrategyOutcome};

/// A diversification move: perturb every base variable of a warm start
/// around its current value (typical displacement `sigma`), then hand
/// the perturbed proposal to [`FindClosestStrategy`] to snap it back to
/// a nearby *feasible* point.
///
/// Repeatedly seeding later strategies from such fuzzed-then-repaired
/// points lets a conductor escape a single basin and explore diverse
/// feasible neighborhoods. A warm start is required (there is nothing to
/// perturb otherwise).
///
/// A single knob, `sigma`, drives everything: every base variable is
/// perturbed, and `sigma` is the typical per-variable distance from the
/// current solution. The perturbation is uniform across variable kinds —
/// a binary variable is just a bounded integer `[0, 1]`, so its flip
/// fraction is a function of `sigma` alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzyStrategy {
    /// Gaussian standard deviation of the per-variable displacement.
    pub sigma: f64,
    /// Seed for the perturbation RNG. `None` => entropy-seeded (a fresh
    /// non-reproducible perturbation each run).
    pub seed: Option<u64>,
    /// The repair step: find the feasible point closest to the perturbed
    /// proposal.
    pub find_closest: FindClosestStrategy,
}

/// Reflect `x` into the closed range `[lo, hi]` by billiard / triangle-wave
/// reflection (a value that walks past a bound bounces back off it, and
/// keeps bouncing until it lands inside). Returns `lo` for a degenerate
/// range (`hi <= lo`). Stays integral when `x`, `lo`, `hi` are integral.
fn reflect(x: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return lo;
    }
    let range = hi - lo;
    // Fold into one period of the triangle wave (length 2·range), then map
    // the second half back down: [0, range] rises, [range, 2·range] falls.
    let t = (x - lo).rem_euclid(2.0 * range);
    if t <= range {
        lo + t
    } else {
        lo + 2.0 * range - t
    }
}

/// Draw a perturbed value for a single variable: sample a Gaussian
/// displacement (`sigma` = std dev), round it for integer variables, add
/// it to `current`, and reflect the result back into the variable's
/// bounds. `sigma` must be finite and non-negative.
fn perturb_value(current: f64, kind: &Variable, sigma: f64, rng: &mut StdRng) -> f64 {
    let normal = Normal::new(0.0, sigma).expect("sigma must be finite and non-negative");
    let mut displacement = normal.sample(rng);
    if kind.is_integer() {
        // Rounded Gaussian: an integer displacement with no parity artifact
        // (works for any sigma, including sigma < 1).
        displacement = displacement.round();
    }
    let x = current + displacement;
    match (kind.get_min(), kind.get_max()) {
        // Bounded on both sides: full triangle-wave reflection. Binary
        // `[0, 1]` falls out of this case (range = 1), flipping iff the
        // integer displacement is odd.
        (Some(lo), Some(hi)) => reflect(x, lo, hi),
        // Semi-bounded: a single reflection off the present bound suffices.
        (Some(lo), None) => {
            if x < lo {
                2.0 * lo - x
            } else {
                x
            }
        }
        (None, Some(hi)) => {
            if x > hi {
                2.0 * hi - x
            } else {
                x
            }
        }
        // Unbounded: use the displacement as-is.
        (None, None) => x,
    }
}

#[async_trait]
impl Strategy for FuzzyStrategy {
    type Progress<V: UsableData + Send> = FuzzyProgressData;

    fn name(&self) -> &'static str {
        "fuzzy"
    }

    fn ui_name(&self) -> &'static str {
        "Perturbation aléatoire"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        // A warm start is mandatory: without a current point there is
        // nothing to perturb.
        let warm_start = warm_start
            .ok_or_else(|| StrategyError::SolveError("fuzzy requires a warm start".into()))?;

        // `Normal::new` rejects a negative or non-finite std dev; catch it
        // up front so `perturb_value`'s `expect` never fires.
        if !(self.sigma.is_finite() && self.sigma >= 0.0) {
            return Err(StrategyError::SolveError(format!(
                "fuzzy requires a finite, non-negative sigma, got {}",
                self.sigma
            )));
        }

        let warm_values = warm_start.get_values();
        let var_kinds = model.checker_problem().get_variables();

        // Collect the base variables present in the warm start together
        // with their kind and current value. The RNG stream is consumed in
        // a canonical order (variables sorted by their Debug form) so that a
        // fixed `seed` yields the same proposal regardless of HashMap
        // iteration order, which is not stable across runs.
        let mut base_vars: Vec<(InternalVar<B, E>, Variable, f64)> = warm_values
            .iter()
            .filter(|(v, _)| matches!(v, InternalVar::Base(_)))
            .filter_map(|(v, &current)| var_kinds.get(v).map(|k| (v.clone(), k.clone(), current)))
            .collect();
        base_vars.sort_by_key(|(v, _, _)| format!("{v:?}"));

        let mut rng = match self.seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_os_rng(),
        };

        // Perturb every base variable, accumulating the L1 distance
        // (Σ |new − current|) and counting how many actually changed.
        let mut perturbed_values = warm_values.clone();
        let total = base_vars.len();
        let mut perturbed = 0usize;
        let mut l1_distance = 0.0f64;
        for (var, kind, current) in &base_vars {
            let new_value = perturb_value(*current, kind, self.sigma, &mut rng);
            l1_distance += (new_value - current).abs();
            if new_value != *current {
                perturbed += 1;
            }
            perturbed_values.insert(var.clone(), new_value);
        }
        let proposal = ConfigData::from(perturbed_values);

        // Signal the perturbed proposal before the (potentially long) repair.
        if !on_progress(FuzzyProgressData::Perturbed {
            perturbed,
            total,
            l1_distance,
        }) {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped,
                objective: None,
                best_bound: None,
                solution: None,
            });
        }

        // Repair: find the feasible point closest to the perturbed proposal.
        self.find_closest
            .run_with_callback(ctx, model, Some(proposal), &|p| {
                on_progress(FuzzyProgressData::FindClosest(p))
            })
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FuzzyProgressData {
    /// The warm start has been perturbed. `perturbed` of `total` base
    /// variables actually changed value; `l1_distance` is the literal
    /// `Σ |new − current|` over all base variables.
    Perturbed {
        perturbed: usize,
        total: usize,
        l1_distance: f64,
    },
    /// Progress from the repair step (find the closest feasible point).
    FindClosest(FindClosestProgressData),
}

impl fmt::Display for FuzzyProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FuzzyProgressData::Perturbed {
                perturbed,
                total,
                l1_distance,
            } => write!(
                f,
                "Perturbed {perturbed}/{total} base variables (L1 distance = {l1_distance:.4}), repairing..."
            ),
            FuzzyProgressData::FindClosest(p) => write!(f, "[repair] {p}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use collomatique_ilp::solvers::{Solver, SolverModel, collo_cbc::ColloCbcSolver};
    use collomatique_ilp::{LinExpr, ProblemDesc};
    use collomatique_ilp_modeler::model_desc::ModelDesc;
    use collomatique_ilp_modeler::{Modeler, Var};

    use crate::{
        RawSolveOutcome, SolveBackend, SolveConfig, SolveProgressData, StrategyKind,
        StrategyProgressData,
    };

    // ----- pure-helper unit tests -----

    #[test]
    fn reflect_within_range_is_identity() {
        assert_eq!(reflect(5.0, 1.0, 7.0), 5.0);
        assert_eq!(reflect(1.0, 1.0, 7.0), 1.0);
        assert_eq!(reflect(7.0, 1.0, 7.0), 7.0);
    }

    #[test]
    fn reflect_bounces_off_upper_bound() {
        // range [1, 7], walk from 5 by +3 => 8, reflect back to 6.
        assert_eq!(reflect(8.0, 1.0, 7.0), 6.0);
    }

    #[test]
    fn reflect_bounces_off_lower_bound() {
        // range [1, 7], walk from 2 by -3 => -1, reflect to 3.
        assert_eq!(reflect(-1.0, 1.0, 7.0), 3.0);
    }

    #[test]
    fn reflect_wraps_large_displacement() {
        // range [0, 1] (a binary): parity of the integer displacement
        // decides the landing point.
        assert_eq!(reflect(4.0, 0.0, 1.0), 0.0); // even => back to start
        assert_eq!(reflect(5.0, 0.0, 1.0), 1.0); // odd  => flipped
        assert_eq!(reflect(-3.0, 0.0, 1.0), 1.0); // odd  => flipped
    }

    #[test]
    fn reflect_degenerate_range_returns_lo() {
        assert_eq!(reflect(3.0, 2.0, 2.0), 2.0);
    }

    #[test]
    fn perturb_binary_flips_iff_odd_displacement() {
        // With a fixed rng we can't force a specific displacement, but a
        // binary always stays in {0, 1} whatever the draw.
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..100 {
            let v = perturb_value(0.0, &Variable::binary(), 3.0, &mut rng);
            assert!(
                v == 0.0 || v == 1.0,
                "binary must stay in {{0, 1}}, got {v}"
            );
        }
    }

    #[test]
    fn perturb_integer_stays_integral_and_in_bounds() {
        let mut rng = StdRng::seed_from_u64(2);
        let kind = Variable::integer().min(1.0).max(7.0);
        for _ in 0..200 {
            let v = perturb_value(4.0, &kind, 5.0, &mut rng);
            assert_eq!(v, v.round(), "integer var must stay integral, got {v}");
            assert!((1.0..=7.0).contains(&v), "must stay in [1, 7], got {v}");
        }
    }

    #[test]
    fn perturb_continuous_bounded_stays_in_range() {
        let mut rng = StdRng::seed_from_u64(3);
        let kind = Variable::continuous().min(0.0).max(10.0);
        for _ in 0..200 {
            let v = perturb_value(5.0, &kind, 4.0, &mut rng);
            assert!((0.0..=10.0).contains(&v), "must stay in [0, 10], got {v}");
        }
    }

    #[test]
    fn perturb_zero_sigma_is_identity() {
        let mut rng = StdRng::seed_from_u64(4);
        assert_eq!(perturb_value(3.0, &Variable::integer(), 0.0, &mut rng), 3.0);
        assert_eq!(
            perturb_value(2.5, &Variable::continuous(), 0.0, &mut rng),
            2.5
        );
    }

    // ----- strategy-level tests -----

    /// A backend that actually solves each problem it is handed with CBC, so
    /// the repair's closeness solve and reconstruction solve each get a
    /// correct answer. Emits one progress event per solve.
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
            _on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            unimplemented!("fuzzy tests never spawn subprocesses")
        }
    }

    fn find_closest() -> FindClosestStrategy {
        FindClosestStrategy {
            closeness_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: true,
            // Solve to the exact closest point so the end-to-end assertions are stable.
            distance_tolerance: 0.0,
        }
    }

    fn strategy(sigma: f64, seed: Option<u64>) -> FuzzyStrategy {
        FuzzyStrategy {
            sigma,
            seed,
            find_closest: find_closest(),
        }
    }

    /// Build a model of `n` free binary base variables (no constraints), so
    /// the repair leaves the perturbed proposal untouched.
    fn free_binary_model(n: usize) -> Model<usize, (), ()> {
        let vars: HashMap<usize, Variable> = (0..n).map(|i| (i, Variable::binary())).collect();
        let modeler: Modeler<usize, (), (), (), ()> = Modeler::new(vars);
        modeler.build(&()).unwrap()
    }

    fn all_zero_warm_start(n: usize) -> ConfigData<InternalVar<usize, ()>> {
        ConfigData::from(
            (0..n)
                .map(|i| (InternalVar::Base(i), 0.0))
                .collect::<HashMap<_, _>>(),
        )
    }

    /// Run the strategy on a free-binary model with an all-zero warm start,
    /// returning the reported `(perturbed_count, l1_distance)`.
    async fn perturbed_report(n: usize, sigma: f64, seed: u64) -> (usize, f64) {
        let model = free_binary_model(n);
        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let captured: Mutex<Option<(usize, f64)>> = Mutex::new(None);
        strategy(sigma, Some(seed))
            .run_with_callback(&ctx, &model, Some(all_zero_warm_start(n)), &|p| {
                if let FuzzyProgressData::Perturbed {
                    perturbed,
                    l1_distance,
                    ..
                } = p
                {
                    *captured.lock().unwrap() = Some((perturbed, l1_distance));
                }
                true
            })
            .await
            .unwrap();
        captured.into_inner().unwrap().unwrap()
    }

    /// Run the strategy on a free-binary model with an all-zero warm start,
    /// returning the repaired base-variable values (a free model leaves the
    /// perturbed proposal untouched).
    async fn perturbed_solution(n: usize, sigma: f64, seed: u64) -> Vec<f64> {
        let model = free_binary_model(n);
        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let outcome = strategy(sigma, Some(seed))
            .run_with_callback(&ctx, &model, Some(all_zero_warm_start(n)), &|_| true)
            .await
            .unwrap();
        let sol = outcome.solution.unwrap();
        (0..n)
            .map(|i| sol.get(InternalVar::Base(i)).unwrap())
            .collect()
    }

    /// Same seed => identical perturbed proposal (reproducible count + L1).
    #[tokio::test]
    async fn same_seed_is_deterministic() {
        assert_eq!(
            perturbed_report(30, 2.0, 42).await,
            perturbed_report(30, 2.0, 42).await
        );
    }

    /// Different seeds generally give different perturbations.
    #[tokio::test]
    async fn different_seeds_differ() {
        assert_ne!(
            perturbed_solution(40, 2.0, 1).await,
            perturbed_solution(40, 2.0, 2).await
        );
    }

    /// A larger sigma perturbs more binary variables (bigger flip fraction),
    /// so its L1 distance from an all-zero start is larger.
    #[tokio::test]
    async fn larger_sigma_perturbs_more() {
        let (_, small) = perturbed_report(200, 0.2, 7).await;
        let (_, large) = perturbed_report(200, 3.0, 7).await;
        assert!(small < large, "a larger sigma should flip more binaries");
    }

    /// End-to-end: the fuzzed proposal is repaired to a feasible complete
    /// solution, and a `Perturbed` event fires before any `FindClosest` one.
    #[tokio::test]
    async fn end_to_end_repairs_to_feasible() {
        // Two binaries under a + b <= 1.
        let vars: HashMap<usize, Variable> =
            [(0, Variable::binary()), (1, Variable::binary())].into();
        let mut modeler: Modeler<usize, (), (), (), ()> = Modeler::new(vars);
        modeler.add_constraint(
            (LinExpr::var(Var::Base(0)) + LinExpr::var(Var::Base(1))).leq(&LinExpr::constant(1.0)),
            (),
        );
        let model = modeler.build(&()).unwrap();

        let warm = ConfigData::from(HashMap::from([
            (InternalVar::Base(0usize), 0.0),
            (InternalVar::Base(1usize), 0.0),
        ]));

        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let events: Mutex<Vec<FuzzyProgressData>> = Mutex::new(Vec::new());
        let outcome = strategy(3.0, Some(1))
            .run_with_callback(&ctx, &model, Some(warm), &|p| {
                events.lock().unwrap().push(p);
                true
            })
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        let sol = outcome.solution.unwrap();
        let a = sol.get(InternalVar::Base(0usize)).unwrap();
        let b = sol.get(InternalVar::Base(1usize)).unwrap();
        assert!(a + b <= 1.0 + 1e-9, "repair must respect a + b <= 1");

        let events = events.into_inner().unwrap();
        let perturbed_idx = events
            .iter()
            .position(|e| matches!(e, FuzzyProgressData::Perturbed { .. }))
            .expect("a Perturbed event is emitted");
        let find_closest_idx = events
            .iter()
            .position(|e| matches!(e, FuzzyProgressData::FindClosest(_)))
            .expect("a FindClosest event is emitted");
        assert!(
            perturbed_idx < find_closest_idx,
            "perturbation precedes the repair"
        );
    }

    /// Without a warm start there is nothing to perturb, so the strategy errors.
    #[tokio::test]
    async fn missing_warm_start_errors() {
        let model = free_binary_model(1);
        let ctx = StrategyContext::new(Arc::new(RealBackend));
        let err = strategy(1.0, Some(0))
            .run_with_callback(&ctx, &model, None, &|_| true)
            .await
            .unwrap_err();
        match err {
            StrategyError::SolveError(msg) => assert!(msg.contains("warm start")),
            other => panic!("expected SolveError, got {other:?}"),
        }
    }
}
