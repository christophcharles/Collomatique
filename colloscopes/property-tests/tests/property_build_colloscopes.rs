//! Fuzz-build regression net over `constraints-colloscopes`
//!
//! Reuses the deterministic fuzzy-walk machinery from
//! `collomatique-testgen-colloscopes` (the same generator/harness that drives
//! the state property tests) to reach many arbitrary-but-valid documents, and
//! at each probe point builds the constraint model. The crate is riddled with
//! `panic`/`expect` invariant assertions ("valid state ⇒ never fires") and the
//! `Err` paths of [`SolveConfig::build_model`] are the same family of internal
//! modeler-inconsistency conditions propagated as `Result`; both count as
//! failures. So the assertion is simply: **building never panics and never
//! returns `Err` on a reachable valid state.**
//!
//! On failure the harness prints the seed and the full op log, so re-running
//! the binary reproduces the exact walk.

use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::colloscope_params::Parameters;

use collomatique_constraints_colloscopes::{
    GroupListRecompute, GroupListSolveData, PeriodSolveData, SolveConfig, build_incremental_epochs,
};

use harness::RunConfig;

/// Much smaller than the state suite: every probe is a full model build, so we
/// trade seeds/ops for a handful of hundred builds. `invalid_fraction: 0.0`
/// because a failed op doesn't change the state (it would just waste walk
/// steps). Tune these three constants against the measured runtime.
const CONFIG: RunConfig = RunConfig {
    seeds: 15,
    ops_per_run: 200,
    invalid_fraction: 0.0,
};

/// Build a model every `BUILD_STRIDE` successful ops (plus on the bootstrap and
/// final states).
const BUILD_STRIDE: usize = 5;

/// Synthesize a random [`SolveConfig`] against the given parameters.
///
/// Mirrors the id-iteration patterns of [`SolveConfig::sanitize`]: periods come
/// from `Periods::period_ids`, group lists from `group_list_map` with prefilled
/// ones skipped (the same contract `sanitize` enforces). A random subset of
/// each carries explicit data; the rest fall back to the build's own defaults.
fn gen_solve_config(rng: &mut ChaCha8Rng, params: &Parameters) -> SolveConfig {
    let mut config = SolveConfig::default();

    config.periods.clear();
    for id in params.periods.period_ids() {
        if rng.random_bool(0.5) {
            config.periods.insert(
                id,
                PeriodSolveData {
                    recompute: rng.random_bool(0.7),
                    use_current_values: rng.random_bool(0.5),
                },
            );
        }
    }

    config.group_lists.clear();
    for (id, group_list) in params.group_lists.group_list_map.iter() {
        if group_list.is_prefilled() {
            // `sanitize` skips prefilled group lists; honor the same contract so
            // we never feed the builder a config it would reject as stale.
            continue;
        }
        if rng.random_bool(0.5) {
            let recompute = if rng.random_bool(0.7) {
                Some(GroupListRecompute {
                    previous_values_as_objective: rng.random_bool(0.5),
                })
            } else {
                None
            };
            config
                .group_lists
                .insert(id, GroupListSolveData { recompute });
        }
    }

    config.objectify_cross_fixed_period = if rng.random_bool(0.3) {
        None
    } else {
        Some(1000.0)
    };
    config.l1_anchor_weight = [1.0, 10.0, 1000.0][rng.random_range(0..3)];

    config
}

/// One probe: synthesize a config (half the time additionally `sanitize`d so
/// both partial-raw and reconciled shapes are exercised) and build the model.
/// The build panicking or returning `Err` is the failure this whole suite
/// exists to catch.
fn build_and_check(rng: &mut ChaCha8Rng, inner: &InnerData) {
    let mut config = gen_solve_config(rng, &inner.params);
    if rng.random_bool(0.5) {
        config = config.sanitize(&inner.params);
    }

    let model = config
        .build_model(inner, &mut |_: &str| {})
        .expect("configured model build must succeed on a valid state");

    // Cheap generic no-panic pass over the built model.
    let _epochs = build_incremental_epochs(&model);
}

/// Along random valid-op walks, building the constraint model must neither
/// panic nor return `Err` for any reachable state.
///
/// **This walk keeps the bootstrap start alone**, where the six others in this
/// package run from five documents (`support/start_points.rs`). The colloscope
/// model grows much faster than the document does: from hogwarts, one seed of
/// this walk took 196.6 s against 1.46 s for all fifteen, so a full run would
/// be about **49 minutes for a single start**. No seed count rescues that —
/// the cost is per probe. The coverage it would buy is bought instead by
/// `constraints-colloscopes/tests/examples_build.rs`, which builds this same
/// model against every file in `examples/`, hogwarts included.
#[test]
fn model_builds_never_panic_along_random_walks() {
    harness::for_each_seed(
        "model_builds_never_panic_along_random_walks",
        &CONFIG,
        |rng, log, stats| {
            let (mut state, _) = harness::bootstrap(rng);
            let mut snapshots: Vec<InnerData> = vec![state.get_data().get_inner_data().clone()];

            // Probe the bootstrap state before any random op.
            build_and_check(rng, state.get_data().get_inner_data());

            let mut since_build = 0usize;
            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = generator::gen_op(
                    rng,
                    state.get_data().get_inner_data(),
                    &snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);

                match state.apply(op, category.to_string()) {
                    Ok(_) => {
                        stats.record(category, true);
                        let inner = state.get_data().get_inner_data();
                        // Re-check the state invariants so a corrupt-state build
                        // failure is attributed to the state layer, not blamed
                        // on the builder. Trivially cheap next to a build.
                        assert_eq!(
                            inner.broken_invariants(),
                            Ok(BTreeSet::new()),
                            "invariants must hold after a successful op",
                        );
                        if snapshots.len() < 8 && rng.random_bool(0.02) {
                            snapshots.push(inner.clone());
                        }
                        since_build += 1;
                        if since_build >= BUILD_STRIDE {
                            since_build = 0;
                            build_and_check(rng, inner);
                        }
                    }
                    Err(_) => {
                        stats.record(category, false);
                    }
                }
            }

            // Probe the final state.
            build_and_check(rng, state.get_data().get_inner_data());
        },
    );
}
