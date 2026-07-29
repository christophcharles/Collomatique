//! Property fuzz over the cascade ([`collomatique_state::apply_cascade`] driven
//! by the colloscope resolution map).
//!
//! This is the third property harness, alongside `property_ops.rs` (the gated
//! walk plus undo/redo) and `property_apply_gate.rs` (the gate's own atomicity
//! and honesty). It reuses the same testgen walk, but routes **every** generated
//! op through `annotate` + `apply_cascade` instead of `apply`, so the resolution
//! map is consulted on every op that breaks an invariant.
//!
//! What both properties assert, through the one shared [`cascade_step`]:
//!
//! * **no panic** — implicit in the test passing, and the reason this harness
//!   exists. The engine holds the map to its contract with three panics (a fix
//!   rejected as invalid, a fix-created invariant the map then disowns, a fix
//!   that applies as a perfect no-op). With the round fuse gone, those panics
//!   plus the by-hand audit of every arm are what stands between a map bug and a
//!   production hang, until step 6.5 adds the `PartialOrd` in-flight check.
//! * **`Ok` ⇒ honesty** — the landed state is fully valid, the target op is the
//!   *last* entry of the returned aggregated op, and replaying the reverses in
//!   reverse order restores the pre-call state exactly.
//! * **`Err` ⇒ atomicity** — the state is bit-identical to before the call.
//!
//! **Two walks, because document size is a coverage dimension.** The cascade
//! *removes* material where the gate merely rejected, so a walk that cascades
//! from the first op keeps tearing down the very structures that make later
//! cascades interesting. The first property starts cascading straight off the
//! bootstrap; the second grows a document through the plain gate first
//! ([`GROW_OPS`] ops) and only then starts cascading, so the map is also
//! exercised against the larger, richer documents the gated walk builds up.
//!
//! **Coverage guards, because a green run is not by itself evidence.** Either
//! property would pass just as happily if no cascade ever fired — every target
//! landing alone, the map never once consulted. So each run counts the cascades
//! and asserts, across all seeds, that fixes really landed and that a target was
//! really convicted. The conviction counter is deliberately *not* the rejection
//! counter: `Err(InvalidOp)` is the gate bouncing the target before the map is
//! asked anything, and guarding on it would prove nothing about this step.
//!
//! **What the two walks measured** (July 29 2026, both green on their first run,
//! no panic). Growing first is not a marginal gain — it is where most of this
//! harness's cascade coverage comes from:
//!
//! | | from bootstrap | grown first |
//! | --- | --- | --- |
//! | targets landed | 21323 | 21250 |
//! | landings needing a fix | 1597 | 2077 |
//! | fix ops in total | 4592 | **7298** |
//! | mean fixes per cascade | 2.9 | **3.5** |
//! | widest single cascade | 25 | **42** |
//! | convicted / refused | 1381 / 2296 | 1521 / 2229 |
//! | document size at handover | 21 | **61** |
//! | document size at the end | 42 (min 17) | 50 (min 20) |
//!
//! So the same number of cascade ops does about **60% more repair work** on the
//! grown document, and the deepest chain the map is driven through nearly
//! doubles. The handover figure also settles that 500 gated ops is enough: it
//! lands on 61, exactly where a pure 500-op gated walk ends up, so the growth
//! curve has flattened by the handover and a longer phase 1 would buy nothing.
//!
//! The end-of-walk sizes are the other half of the picture. The bootstrap walk
//! *climbs* 21 → 42 and the grown walk *decays* 61 → 50, so the cascade has an
//! equilibrium document size somewhere near the mid-40s that both walks approach
//! from opposite sides. That is why the second walk matters: cascading erodes
//! the very structures that make cascades deep, so the only way to exercise the
//! map against a big document is to hand it one already built. It also means the
//! erosion is bounded — no seed ran itself down to anything like empty.
//!
//! On failure the harness prints the seed and the full op log so the sequence
//! replays exactly.

use std::cell::Cell;
use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state::{InMemoryData, apply_cascade};
use collomatique_state_colloscopes::{Data, Error, InnerData, Op};

use harness::{OpLog, RunConfig, RunStats};

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

/// Gated ops the second property runs before it starts cascading, to hand the
/// cascade a document the plain walk has had time to build up. Measured, not
/// guessed: at 500 the handover size lands on 61, exactly where a pure 500-op
/// gated walk ends, so the growth curve has flattened by then and a longer
/// phase 1 would only cost time. The handover size stays printed so this
/// remains checkable if the generator changes.
const GROW_OPS: usize = 500;

/// The invariant oracle: the whole-model checker must report a fully clean
/// document (no logic errors, no dangling references, no convergence breaks).
fn assert_clean(data: &Data) {
    assert_eq!(
        data.get_inner_data().broken_invariants(),
        Ok(BTreeSet::new()),
        "apply_cascade returned Ok but the state is not fully valid",
    );
}

/// Number of live entities in the document — the size probe.
fn document_size(data: &Data) -> usize {
    data.get_inner_data().params.all_ids().count()
}

/// Cross-seed tallies for one property. `for_each_seed` takes a `Fn` closure, so
/// these need interior mutability.
#[derive(Default)]
struct Counters {
    landed: Cell<usize>,     // targets that landed
    convicted: Cell<usize>,  // targets the map refused to fix
    refused: Cell<usize>,    // targets bounced before the map was asked
    cascaded: Cell<usize>,   // landings that needed at least one fix
    fix_ops: Cell<usize>,    // fix ops landed in total
    widest: Cell<usize>,     // most fixes in a single cascade
    size_sum: Cell<usize>,   // end-of-seed document sizes, summed
    size_min: Cell<usize>,   // smallest end-of-seed document
    handover: Cell<usize>,   // document sizes when cascading began, summed
    seeds_done: Cell<usize>, // seeds that reached the end
}

impl Counters {
    fn new() -> Self {
        Counters {
            size_min: Cell::new(usize::MAX),
            ..Default::default()
        }
    }

    fn bump(cell: &Cell<usize>, by: usize) {
        cell.set(cell.get() + by);
    }

    /// Closes one seed: records where the document started cascading and where
    /// it ended up.
    fn finish_seed(&self, handover: usize, data: &Data) {
        let size = document_size(data);
        Self::bump(&self.handover, handover);
        Self::bump(&self.size_sum, size);
        Self::bump(&self.seeds_done, 1);
        self.size_min.set(self.size_min.get().min(size));
    }

    /// The guards that make a green run mean something.
    fn assert_covered(&self) {
        assert!(
            self.landed.get() > 0,
            "no target ever landed across all seeds"
        );
        assert!(
            self.cascaded.get() > 0,
            "no target ever needed a fix across all seeds — the resolution map \
             was never exercised, so a green run here proves nothing",
        );
        assert!(
            self.convicted.get() > 0,
            "no target was ever convicted across all seeds — the `None if \
             is_target` branch was never reached, so `Err ⇒ unchanged` was only \
             ever asserted about ops the gate bounced before the map was asked",
        );
    }

    fn report(&self, label: &str) {
        let seeds = self.seeds_done.get().max(1);
        eprintln!(
            "{label}: {} landed ({} with fixes, {} fix ops, widest {}), \
             {} convicted, {} refused; document size — handover mean {}, \
             end mean {} min {}",
            self.landed.get(),
            self.cascaded.get(),
            self.fix_ops.get(),
            self.widest.get(),
            self.convicted.get(),
            self.refused.get(),
            self.handover.get() / seeds,
            self.size_sum.get() / seeds,
            self.size_min.get(),
        );
    }
}

/// Drives one generated op through the cascade and asserts every property of the
/// outcome. Shared by both walks on purpose: the two differ only in the document
/// they hand the cascade, never in what is checked. Returns whether the target
/// landed.
fn cascade_step(
    data: &mut Data,
    op: Op,
    category: &'static str,
    stats: &mut RunStats,
    c: &Counters,
) -> bool {
    // The caller annotates, exactly as production would: the engine works in
    // annotated ops and cannot issue ids itself. `before` is taken after
    // `annotate` to match what the engine snapshots at entry. (The id issuer sits
    // outside `PartialEq for Data`, so the ordering does not change the
    // assertion — it keeps the two snapshots describing the same moment.)
    let (annotated, _new_id) = data.annotate(op);
    let before = data.clone();

    let applied = match apply_cascade(data, annotated.clone()) {
        Ok(applied) => applied,
        Err(e) => {
            stats.record(category, false);
            // The two rejections are not the same event, and only one of them
            // exercises this step's code. `InvalidOp` is the gate bouncing the
            // target before any invariant was checked, so the map is never asked.
            // `BrokenInvariants` is the map asked and answering `None` for the
            // target (`cascade.rs:114-119`) — the branch this harness is for.
            match e {
                Error::BrokenInvariants(set) => {
                    Counters::bump(&c.convicted, 1);
                    assert!(
                        !set.is_empty(),
                        "a conviction carries a non-empty break set"
                    );
                }
                Error::InvalidOp(_) => Counters::bump(&c.refused, 1),
            }
            assert!(
                *data == before,
                "a rejected target must leave the state bit-identical",
            );
            return false;
        }
    };
    stats.record(category, true);
    Counters::bump(&c.landed, 1);

    // Honesty 1: what the cascade lands is fully valid.
    assert_clean(data);

    // Honesty 2: the target is the last thing that landed, so `.rev()` undoes it
    // first and the history slot reads right.
    assert_eq!(
        applied.inner().last().map(|step| step.inner()),
        Some(&annotated),
        "the target op must be the last entry of the aggregated op",
    );

    let fixes = applied.inner().len() - 1;
    if fixes > 0 {
        Counters::bump(&c.cascaded, 1);
        Counters::bump(&c.fix_ops, fixes);
        c.widest.set(c.widest.get().max(fixes));
    }

    // Honesty 3: the compound undo works stepwise. Replaying the reverses in
    // reverse order walks back through the exact trajectory the cascade came by,
    // so every intermediate state is one the gate already accepted.
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
    true
}

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
/// `property_ops.rs` and `property_apply_gate.rs` do.
fn maybe_snapshot(rng: &mut ChaCha8Rng, data: &Data, snapshots: &mut Vec<InnerData>) {
    if snapshots.len() < 8 && rng.random_bool(0.02) {
        snapshots.push(data.get_inner_data().clone());
    }
}

/// Property 1: cascading from the first op, off the bootstrap document.
#[test]
fn cascade_never_panics_and_is_atomic() {
    let c = Counters::new();

    harness::for_each_seed(
        "cascade_never_panics_and_is_atomic",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            assert_clean(&data);
            let handover = document_size(&data);
            let mut snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = next_op(rng, &data, &snapshots, log);
                if cascade_step(&mut data, op, category, stats, &c) {
                    maybe_snapshot(rng, &data, &mut snapshots);
                }
            }

            c.finish_seed(handover, &data);
        },
    );

    c.assert_covered();
    c.report("cascade fuzz (from bootstrap)");
}

/// Property 2: the same cascade walk, but handed a document the plain gate has
/// already grown. Same assertions, larger and structurally richer input — the
/// gated walk accumulates the slots, group lists, colloscope cells and pairings
/// that a from-the-bootstrap cascade walk keeps tearing back down, and those are
/// the structures with the most invariant sites for the map to work through.
#[test]
fn cascade_on_a_grown_document_never_panics() {
    let c = Counters::new();

    harness::for_each_seed(
        "cascade_on_a_grown_document_never_panics",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            assert_clean(&data);
            let mut snapshots: Vec<InnerData> = vec![];

            // Phase 1 — grow through the plain gate. Invariant-breaking ops are
            // simply rejected here, exactly as in `property_ops.rs`; nothing is
            // repaired and nothing is torn down.
            for _ in 0..GROW_OPS {
                let (category, op) = next_op(rng, &data, &snapshots, log);
                let (annotated, _) = data.annotate(op);
                let landed = data.apply(&annotated).is_ok();
                stats.record(category, landed);
                if landed {
                    maybe_snapshot(rng, &data, &mut snapshots);
                }
            }
            assert_clean(&data);
            let handover = document_size(&data);

            // Phase 2 — cascade from there.
            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = next_op(rng, &data, &snapshots, log);
                if cascade_step(&mut data, op, category, stats, &c) {
                    maybe_snapshot(rng, &data, &mut snapshots);
                }
            }

            c.finish_seed(handover, &data);
        },
    );

    c.assert_covered();
    c.report("cascade fuzz (grown first)");
}
