//! Fuzz-build regression net over `constraints-groups`
//!
//! Reuses the deterministic fuzzy-walk machinery from
//! `collomatique-testgen-colloscopes` (the same generator/harness that drives
//! the state property tests) to reach many arbitrary-but-valid documents, and
//! at each probe point draws a random *request* against that document, builds
//! the plan and the model, and converts a random in-domain assignment back
//! into group lists. Every layer is riddled with `panic`/`expect` invariant
//! assertions ("valid state ⇒ never fires"), and `build_generation_plan`
//! returning `Err` on a request drawn from valid state is the same family of
//! internal inconsistency. So the assertion is simply: **the whole round trip
//! never panics and never returns `Err` on a reachable valid state.**
//!
//! On failure the harness prints the seed and the full op log, so re-running
//! the binary reproduces the exact walk.

use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::InnerData;

use collomatique_constraints_groups::{
    FrozenPlacements, GroupListIdx, ObjectiveWeights, Var, build_generation_plan,
    build_group_lists, build_model, vars::VarEnv,
};
use collomatique_ilp::ConfigData;

use harness::RunConfig;

// Shared with `property_greedy_groups`: both walks must draw their requests
// the same way (see the module for why it lives outside this file).
#[path = "support/generation_request.rs"]
mod generation_request;
use generation_request::gen_generation_request;

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

/// One probe: synthesize a request, plan it, build the model, and convert a
/// random in-domain assignment back into group lists.
fn build_and_check(rng: &mut ChaCha8Rng, inner: &InnerData) {
    let request = gen_generation_request(rng, &inner.params);
    let plan = build_generation_plan(&inner.params, &request)
        .expect("a request drawn from valid state must produce a plan");

    // Every requested pair is covered by exactly one spec, or skipped.
    for pair in &request.rebuild {
        let covering = plan
            .specs
            .iter()
            .filter(|(_, covered)| covered.contains(pair))
            .count();
        assert!(
            (covering == 1) != plan.skipped.contains(pair),
            "pair must be covered exactly once xor skipped",
        );
    }

    // The (trivial) model must build without panicking.
    let model = build_model(
        &plan,
        ObjectiveWeights::default(),
        &FrozenPlacements::default(),
    );
    let _ = model.stats();

    let env = VarEnv::new(&plan);

    // A random placement must convert into structurally valid lists
    // (`GroupList::new` inside `build_group_lists` panics otherwise), with
    // every student placed and no more groups than slots. The placement is
    // random but keeps every student in exactly one group: the conversion
    // reads a solved assignment matrix, and "exactly one" is a constraint
    // of the model rather than a shape the conversion re-derives.
    let mut config = ConfigData::new();
    for (i, (spec, _covered)) in plan.specs.iter().enumerate() {
        let list = GroupListIdx(i);
        let group_count = env.group_count(list);
        for &student in spec.students() {
            let slot = rng.random_range(0..group_count);
            for group in 0..group_count {
                config = config.set(
                    Var::StudentInGroup {
                        list,
                        student,
                        group,
                    },
                    if group == slot { 1.0 } else { 0.0 },
                );
            }
        }
    }

    let names: Vec<String> = (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect();
    let lists = build_group_lists(&plan, &names, &config);
    assert_eq!(lists.len(), plan.specs.len());
    for (i, ((group_list, _covered), (spec, _))) in lists.iter().zip(plan.specs.iter()).enumerate()
    {
        assert_eq!(
            group_list.filling().iter_students().count(),
            spec.students().len(),
            "every student must be placed exactly once",
        );
        assert!(
            group_list.params().group_names.len() <= env.group_count(GroupListIdx(i)) as usize,
            "compaction can only reduce the group count",
        );
    }
}

/// Along random valid-op walks, the plan/model/conversion round trip must
/// neither panic nor return `Err` for any reachable state.
///
/// **This walk keeps the bootstrap start alone**, where the six others in this
/// package run from five documents (`support/start_points.rs`). Two reasons,
/// both measured. A model build costs far more on a big document than a state
/// op does: from hogwarts this walk goes from 0.17 s to 44.9 s, a factor of
/// **264**, and it would become the largest line in the fuzz suite for a walk
/// that is not what the start points were added for. And a big *real* document
/// is already covered here by cheaper means —
/// `constraints-groups/tests/examples_build.rs` builds this same model against
/// every file in `examples/`, hogwarts included, at one build per example
/// instead of thousands.
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
