//! Property fuzz over the *resolution map's* half of the cascade contract:
//! every fix it offers lands strictly below the state it was asked about.
//!
//! This is the fourth property harness, alongside `property_ops.rs` (the gated
//! walk plus undo/redo), `property_apply_gate.rs` (the gate's own atomicity and
//! honesty) and `property_cascade.rs` (the engine end to end). It reuses the
//! same testgen walk, but does **not** cascade: it runs the plain gate, and the
//! interesting event is a rejection over broken invariants. At that moment the
//! gate has rolled back, so `data` is unchanged and valid — precisely the state
//! the cascade would consult the map on.
//!
//! **Why this exists next to `property_cascade.rs`.** Commit 5 put the
//! strictly-below check inside the engine, but the engine only ever sees the
//! *canonical first pick* of a break set, and only along the trajectory the
//! cascade happens to walk. This harness asks the map about **every** invariant
//! in every reported set, so it probes arms the engine never reaches. It is
//! also the only systematic exercise of the map's `Some` branches — step 6's
//! §9bis innocent-state tests systematically cover `None`.
//!
//! **Why `force_apply` and not `apply`.** A fix is allowed to land a state that
//! still breaks *other* invariants: those are the mid-cascade states the engine
//! walks through, and the gate would bounce them and hide exactly the
//! comparison this property is about. Prechecks still run, and a precheck
//! failure is a map bug — the engine panics on it, so here it is an `expect`.
//!
//! **Coverage guards, because a green run is not by itself evidence.** The walk
//! would pass just as happily if no generated op ever broke an invariant, or if
//! the map answered `None` every time. Both counters are asserted across all
//! seeds, and they count the specific outcome the test is about rather than a
//! proxy (the step-6 commit-8 lesson).
//!
//! **What it measured** (July 29 2026, green on its first run): across the 50
//! seeds, 3564 landings were rejected over broken invariants and the map
//! answered `Some` 6354 times — so the average rejection carries close to two
//! probed arms, and the property is not resting on a handful of lucky
//! trajectories. Run time 5.0 s, below `property_cascade.rs`'s 7.7 s.
//!
//! On failure the harness prints the seed and the full op log so the sequence
//! replays exactly.

use std::cell::Cell;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state::{ContentOrd, FixOp, Fixable, InMemoryData};
use collomatique_state_colloscopes::{Data, Error, InnerData, Op};

use harness::{OpLog, RunConfig};

/// The house configuration for the step-6-family harnesses: one hardcoded
/// const, no environment variables, no `#[ignore]` tiers. Matches
/// `property_cascade.rs`, whose walk this one mirrors.
const CONFIG: RunConfig = RunConfig {
    seeds: 50,
    ops_per_run: 500,
    invalid_fraction: 0.15,
};

/// Generates the next op and logs it.
fn next_op(
    rng: &mut ChaCha8Rng,
    data: &Data,
    snapshots: &[InnerData],
    log: &mut OpLog,
) -> (&'static str, Op) {
    let (category, op) = generator::gen_op(
        rng,
        data.get_inner_data(),
        snapshots,
        CONFIG.invalid_fraction,
    );
    log.push(category, &op);
    (category, op)
}

/// Occasionally keeps a snapshot of a landed state for the generator to reach
/// for. Kept on the success path, and drawing its coin only there, exactly as
/// the three existing harnesses do — moving it would perturb the RNG
/// trajectory and the runs would no longer be comparable.
fn maybe_snapshot(rng: &mut ChaCha8Rng, data: &Data, snapshots: &mut Vec<InnerData>) {
    if snapshots.len() < 8 && rng.random_bool(0.02) {
        snapshots.push(data.get_inner_data().clone());
    }
}

/// Design doc §8 step 6.5: over generated broken states, every
/// `fix_invariant` answer is `None` or an op whose applied result sits
/// strictly below the pre-fix state — never above, never equivalent.
#[test]
fn every_fix_lands_strictly_below() {
    let probed_fixes = Cell::new(0usize);
    let broken_landings = Cell::new(0usize);

    harness::for_each_seed(
        "every_fix_lands_strictly_below",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let mut snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = next_op(rng, &data, &snapshots, log);
                let (annotated, _) = data.annotate(op);
                match data.apply(&annotated) {
                    Ok(_) => {
                        stats.record(category, true);
                        maybe_snapshot(rng, &data, &mut snapshots);
                    }
                    Err(Error::BrokenInvariants(set)) => {
                        stats.record(category, false);
                        broken_landings.set(broken_landings.get() + 1);
                        // Every member of the set, not just the canonical
                        // first pick: strictly wider than what the engine's
                        // in-loop assertion sees, and the point of having
                        // this property alongside `property_cascade.rs`.
                        for invariant in &set {
                            let Some(fix) = data.fix_invariant(invariant) else {
                                continue;
                            };
                            let mut fixed = data.clone();
                            fixed.force_apply(&fix.to_annotated_op()).expect(
                                "a fix op emitted by the resolution map must \
                                 pass the prechecks",
                            );
                            assert!(
                                fixed.get_inner_data().content_lt(data.get_inner_data()),
                                "fix {fix:?} for {invariant:?} must land \
                                 strictly below the pre-fix state (content_cmp \
                                 = {:?})",
                                fixed.get_inner_data().content_cmp(data.get_inner_data()),
                            );
                            probed_fixes.set(probed_fixes.get() + 1);
                        }
                    }
                    Err(Error::InvalidOp(_)) => stats.record(category, false),
                }
            }
        },
    );

    // Coverage guards (step-6 commit-8 lesson: count the specific outcome the
    // test is about, not a proxy). Without them the walk could go green with
    // the map never once answering `Some`.
    assert!(
        broken_landings.get() > 0,
        "no generated op ever broke an invariant across all seeds",
    );
    assert!(
        probed_fixes.get() > 0,
        "the map never answered Some across all seeds — the strictly-below \
         property was never exercised",
    );

    eprintln!(
        "content-order fuzz: {} rejections over broken invariants, \
         {} map answers probed",
        broken_landings.get(),
        probed_fixes.get(),
    );
}
