//! Property fuzz over the cascade ([`collomatique_state::apply_cascade`] driven
//! by the colloscope resolution map).
//!
//! This is the third property harness, alongside `property_ops.rs` (the gated
//! walk plus undo/redo) and `property_apply_gate.rs` (the gate's own atomicity
//! and honesty). It reuses the same testgen walk, but routes **every** generated
//! op through `annotate` + `apply_cascade` instead of `apply`, so the resolution
//! map is consulted on every op that breaks an invariant.
//!
//! What it asserts:
//!
//! * **no panic** — implicit in the test passing, and the reason this harness
//!   exists. The engine holds the map to its contract with three panics (a fix
//!   rejected as invalid, a fix-created invariant the map then disowns, a fix
//!   that applies as a perfect no-op). With the round fuse gone, those panics
//!   plus the by-hand audit of every arm are what stands between a map bug and a
//!   production hang, until step 6.5 adds the `PartialOrd` in-flight check.
//! * **`Ok` ⇒ honesty** — the landed state is fully valid, the target op is the
//!   *last* entry of the returned [`AggregatedOp`], and replaying the reverses in
//!   reverse order restores the pre-call state exactly.
//! * **`Err` ⇒ atomicity** — the state is bit-identical to before the call.
//!
//! **Coverage guards, because a green run is not by itself evidence.** This
//! harness would pass just as happily if no cascade ever fired — every target
//! landing alone, the map never once consulted. So the run counts the cascades
//! and asserts, across all seeds, that fixes really landed and that a target was
//! really convicted. The conviction counter is deliberately *not* the rejection
//! counter: `Err(InvalidOp)` is the gate bouncing the target before the map is
//! asked anything, and guarding on it would prove nothing about this step.
//!
//! **What the first run measured** (July 29 2026, the whole 50 × 500): 21323
//! targets landed, 1597 of them needing at least one fix, 4592 fix ops in all,
//! the widest single cascade **25 fixes** deep; 1381 targets convicted by the map
//! and 2296 bounced by the gate. No panic, first run.
//!
//! The document size is tracked for one specific reason: the cascade *removes*
//! material where the gate merely rejected, so the walk could in principle gut
//! its own document and end up fuzzing something far smaller than the other two
//! harnesses see. Measured rather than assumed — the same walk driven through the
//! plain gate ends at a mean document size of 61 (min 21), through the cascade at
//! **42 (min 17)**. So the cascade explores a document about two-thirds the size,
//! which is a real difference and not a collapse: no seed ran itself down to
//! anything like empty, and the extra ~2100 landings are exactly the ones the
//! plain gate rejected on invariant grounds. Worth re-measuring before anyone
//! shrinks the configuration, since a smaller document means less material for
//! deep cascades to walk through.
//!
//! On failure the harness prints the seed and the full op log so the sequence
//! replays exactly.

use std::cell::Cell;
use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state::{InMemoryData, apply_cascade};
use collomatique_state_colloscopes::{Data, Error, InnerData};

use harness::RunConfig;

/// Deliberately wide while the migration is in flight (★ user ruling, July 28
/// 2026): a cascade multiplies gate calls, but we would rather catch a map bug
/// and wait a bit than tune the harness down before the map has stopped moving.
/// Below the two existing harnesses, which run 100 × 1000 each. Shrinking is a
/// later decision, to be justified the way `property_ops.rs:32-34` justifies its
/// own — not a knob to reach for the first time the suite feels slow.
const CONFIG: RunConfig = RunConfig {
    seeds: 50,
    ops_per_run: 500,
    invalid_fraction: 0.15,
};

/// The invariant oracle: the whole-model checker must report a fully clean
/// document (no logic errors, no dangling references, no convergence breaks).
fn assert_clean(data: &Data) {
    assert_eq!(
        data.get_inner_data().broken_invariants(),
        Ok(BTreeSet::new()),
        "apply_cascade returned Ok but the state is not fully valid",
    );
}

/// Number of live entities in the document — the degeneration probe.
fn document_size(data: &Data) -> usize {
    data.get_inner_data().params.all_ids().count()
}

#[test]
fn cascade_never_panics_and_is_atomic() {
    // Cross-seed counters (interior mutability: `for_each_seed` takes a `Fn`).
    let landed = Cell::new(0usize); // targets that landed
    let convicted = Cell::new(0usize); // targets the map refused to fix
    let refused = Cell::new(0usize); // targets bounced before the map was asked
    let cascaded = Cell::new(0usize); // landings that needed at least one fix
    let fix_ops = Cell::new(0usize); // fix ops landed in total
    let widest = Cell::new(0usize); // most fixes in a single cascade
    let size_sum = Cell::new(0usize); // end-of-seed document sizes, summed
    let size_min = Cell::new(usize::MAX);

    harness::for_each_seed(
        "cascade_never_panics_and_is_atomic",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            assert_clean(&data);
            let mut inner_snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = generator::gen_op(
                    rng,
                    data.get_inner_data(),
                    &inner_snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);

                // The caller annotates, exactly as production would: the engine
                // works in annotated ops and cannot issue ids itself. `before` is
                // taken after `annotate` to match what the engine snapshots at
                // entry. (The id issuer sits outside `PartialEq for Data`, so
                // the ordering does not change the assertion — it keeps the two
                // snapshots describing the same moment.)
                let (annotated, _new_id) = data.annotate(op);
                let before = data.clone();

                let applied = match apply_cascade(&mut data, annotated.clone()) {
                    Ok(applied) => applied,
                    Err(e) => {
                        stats.record(category, false);
                        // The two rejections are not the same event, and only one
                        // of them exercises this step's code. `InvalidOp` is the
                        // gate bouncing the target before any invariant was
                        // checked, so the map is never asked. `BrokenInvariants`
                        // is the map asked and answering `None` for the target
                        // (`cascade.rs:114-119`) — the branch these fixtures and
                        // this harness are here for.
                        match e {
                            Error::BrokenInvariants(set) => {
                                convicted.set(convicted.get() + 1);
                                assert!(
                                    !set.is_empty(),
                                    "a conviction carries a non-empty break set",
                                );
                            }
                            Error::InvalidOp(_) => refused.set(refused.get() + 1),
                        }
                        assert!(
                            data == before,
                            "a rejected target must leave the state bit-identical",
                        );
                        continue;
                    }
                };
                stats.record(category, true);
                landed.set(landed.get() + 1);

                // Honesty 1: what the cascade lands is fully valid.
                assert_clean(&data);

                // Honesty 2: the target is the last thing that landed, so
                // `.rev()` undoes it first and the history slot reads right.
                assert_eq!(
                    applied.inner().last().map(|step| step.inner()),
                    Some(&annotated),
                    "the target op must be the last entry of the aggregated op",
                );

                let fixes = applied.inner().len() - 1;
                if fixes > 0 {
                    cascaded.set(cascaded.get() + 1);
                    fix_ops.set(fix_ops.get() + fixes);
                    widest.set(widest.get().max(fixes));
                }

                // Honesty 3: the compound undo works stepwise. Replaying the
                // reverses in reverse order walks back through the exact
                // trajectory the cascade came by, so every intermediate state is
                // one the gate already accepted.
                let mut undone = data.clone();
                for step in applied.rev().inner() {
                    undone
                        .force_apply(step.inner())
                        .expect("the reverse of a landed cascade step must apply");
                }
                assert!(
                    undone == before,
                    "replaying the reverses must restore the pre-cascade state",
                );

                if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                    inner_snapshots.push(data.get_inner_data().clone());
                }
            }

            let size = document_size(&data);
            size_sum.set(size_sum.get() + size);
            size_min.set(size_min.get().min(size));
        },
    );

    // --- coverage guards (cross-seed, over the whole run) ---
    assert!(landed.get() > 0, "no target ever landed across all seeds");
    assert!(
        cascaded.get() > 0,
        "no target ever needed a fix across all seeds — the resolution map was \
         never exercised, so a green run here proves nothing",
    );
    assert!(
        convicted.get() > 0,
        "no target was ever convicted across all seeds — the `None if is_target` \
         branch was never reached, so `Err ⇒ unchanged` was only ever asserted \
         about ops the gate bounced before the map was consulted",
    );

    eprintln!(
        "cascade fuzz: {} landed ({} with fixes, {} fix ops, widest {}), \
         {} convicted, {} refused; end-of-seed document size min {} mean {}",
        landed.get(),
        cascaded.get(),
        fix_ops.get(),
        widest.get(),
        convicted.get(),
        refused.get(),
        size_min.get(),
        size_sum.get() / CONFIG.seeds as usize,
    );
}
