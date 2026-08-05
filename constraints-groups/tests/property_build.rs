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

use std::collections::{BTreeMap, BTreeSet};

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::colloscope_params::Parameters;

use collomatique_constraints_groups::{
    GenerationRequest, GroupListIdx, ObjectiveWeights, Var, build_generation_plan,
    build_group_lists, build_incremental_epochs, build_model, vars::VarEnv,
};
use collomatique_ilp::ConfigData;

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

/// Random valid request drawn from the current state: any assigned
/// (period, subject) pair whose subject has interrogations may be rebuilt,
/// any prefilled list may be kept.
fn gen_generation_request(rng: &mut ChaCha8Rng, params: &Parameters) -> GenerationRequest {
    let mut rebuild = BTreeSet::new();
    for (period, subject, _students) in params.assignments.iter() {
        let has_interrogations = params
            .subjects
            .find_subject(subject)
            .is_some_and(|s| s.parameters.interrogation_parameters.is_some());
        if has_interrogations && rng.random_bool(0.5) {
            rebuild.insert((period, subject));
        }
    }

    let mut kept_lists = BTreeSet::new();
    for (id, list) in params.group_lists.group_list_map.iter() {
        if list.is_prefilled() && rng.random_bool(0.5) {
            kept_lists.insert(id);
        }
    }

    GenerationRequest {
        rebuild,
        kept_lists,
    }
}

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
    let model = build_model(&plan, ObjectiveWeights::default());
    let _ = model.stats();

    // The epoch map (pieces 10 + 12) must name exactly one entry per base
    // variable and give every variable of a spec the same epoch. Piece 12
    // splits each inclusion level into its connected components, so §2.6's
    // recurrence no longer holds as an equality; instead the probe checks
    // the properties that define the refined numbering, against inclusion
    // heights recomputed here by naive fixpoint iteration (well-founded on
    // strict inclusion, hence a unique solution — and an implementation
    // independent of the production ascending-size pass):
    //   (a) heights ascend with the epochs: a strictly lower height means
    //       a strictly smaller epoch (this subsumes "a strictly included
    //       spec solves strictly earlier");
    //   (b) two specs of the same height that share a student share an
    //       epoch;
    //   (c) an epoch holds specs of a single height, connected through
    //       shared students — epochs are components, never coarser;
    //   (d) inside a height, epochs run the smaller blocks (fewer
    //       distinct students) first;
    //   (e) epoch numbers are contiguous from 0.
    let epochs = build_incremental_epochs(&plan);
    let var_count: usize = plan.specs.iter().map(|(s, _)| s.students.len()).sum();
    assert_eq!(epochs.len(), var_count, "one epoch entry per base variable");
    let spec_epoch = |i: usize| {
        let spec = &plan.specs[i].0;
        let student = *spec
            .students
            .first()
            .expect("a spec always has registered students");
        epochs[&Var::StudentGroup {
            list: GroupListIdx(i),
            student,
        }]
    };
    for (i, (spec, _covered)) in plan.specs.iter().enumerate() {
        for &student in &spec.students {
            assert_eq!(
                epochs[&Var::StudentGroup {
                    list: GroupListIdx(i),
                    student,
                }],
                spec_epoch(i),
                "all variables of a spec share its epoch",
            );
        }
    }

    let n = plan.specs.len();
    let strict_subset = |j: usize, i: usize| {
        let (s, t) = (&plan.specs[j].0.students, &plan.specs[i].0.students);
        s.len() < t.len() && s.is_subset(t)
    };
    let intersects = |i: usize, j: usize| {
        !plan.specs[i]
            .0
            .students
            .is_disjoint(&plan.specs[j].0.students)
    };
    let mut heights = vec![0u32; n];
    loop {
        let mut changed = false;
        for i in 0..n {
            let h = (0..n)
                .filter(|&j| strict_subset(j, i))
                .map(|j| heights[j] + 1)
                .max()
                .unwrap_or(0);
            if heights[i] != h {
                heights[i] = h;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for i in 0..n {
        for j in 0..n {
            if heights[i] < heights[j] {
                assert!(
                    spec_epoch(i) < spec_epoch(j),
                    "heights ascend with the epochs",
                );
            }
            if i < j && heights[i] == heights[j] && intersects(i, j) {
                assert_eq!(
                    spec_epoch(i),
                    spec_epoch(j),
                    "same-height specs sharing a student share an epoch",
                );
            }
        }
    }

    let mut groups: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        groups.entry(spec_epoch(i)).or_default().push(i);
    }
    let epoch_values: Vec<u32> = groups.keys().copied().collect();
    let contiguous: Vec<u32> = (0..groups.len() as u32).collect();
    assert_eq!(
        epoch_values, contiguous,
        "epoch numbers are contiguous from 0",
    );

    let mut prev: Option<(u32, usize)> = None; // previous epoch's (height, block size)
    for members in groups.values() {
        let h = heights[members[0]];
        for &i in members {
            assert_eq!(heights[i], h, "an epoch holds specs of a single height");
        }
        // Connectivity through shared students, by saturation from the
        // first member.
        let mut reached = vec![false; members.len()];
        reached[0] = true;
        loop {
            let mut changed = false;
            for a in 0..members.len() {
                if !reached[a]
                    && (0..members.len()).any(|b| reached[b] && intersects(members[a], members[b]))
                {
                    reached[a] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        assert!(
            reached.iter().all(|&r| r),
            "an epoch's specs are connected through shared students",
        );

        let union: BTreeSet<_> = members
            .iter()
            .flat_map(|&i| plan.specs[i].0.students.iter().copied())
            .collect();
        if let Some((prev_h, prev_size)) = prev {
            if prev_h == h {
                assert!(
                    prev_size <= union.len(),
                    "smaller blocks first inside a height",
                );
            }
        }
        prev = Some((h, union.len()));
    }

    // A random in-domain assignment must convert into structurally valid
    // lists (`GroupList::new` inside `build_group_lists` panics otherwise),
    // with every student placed and no more groups than slots.
    let env = VarEnv::new(&plan);
    let mut config = ConfigData::new();
    for (i, (spec, _covered)) in plan.specs.iter().enumerate() {
        let list = GroupListIdx(i);
        let slot_count = env.slot_count(list);
        for &student in &spec.students {
            let slot = rng.random_range(0..slot_count);
            config = config.set(Var::StudentGroup { list, student }, slot as f64);
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
            spec.students.len(),
            "every student must be placed exactly once",
        );
        assert!(
            group_list.params().group_names.len() <= env.slot_count(GroupListIdx(i)) as usize,
            "compaction can only reduce the group count",
        );
    }
}

/// Along random valid-op walks, the plan/model/conversion round trip must
/// neither panic nor return `Err` for any reachable state.
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
