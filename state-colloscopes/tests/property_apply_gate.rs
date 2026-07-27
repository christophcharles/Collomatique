//! Property fuzz over the apply/check/rollback gate ([`Data::apply`]).
//!
//! This is the step-5 successor of `differential_force_apply.rs`. The old file
//! *differential-fuzzed* `force_apply` against the two checkers to earn trust in
//! the new checker; that job is done, and the old checker retired with step 5.
//! What survives is the randomized coverage of the exact primitive production
//! now runs on: the gate `apply` = snapshot + `force_apply` +
//! `broken_invariants` + rollback. This file re-expresses the same walk-and-probe
//! shape as *properties of the gate alone*:
//!
//! * **atomicity** — every `Err` arm (precheck, logic, invariants) leaves the
//!   state bit-identical to before the op, and carries a non-empty error set on
//!   the two rolled-back arms;
//! * **honesty** — every `Ok` landing is fully valid (`broken_invariants()` is
//!   `Ok(∅)`), and its returned reverse restores the pre-state exactly;
//! * **coverage** — every [`CorruptionKind`] is attempted, each corrupting kind
//!   is rejected at least once, and `ForceLogic` reaches the
//!   [`InvalidOp::Logic`] tier at least once (the external-data route the
//!   sealing left standing).
//!
//! **Fuzz shape — depth-1 probes off a validated walk.** A validated random walk
//! (the testgen harness, byte-untouched) is interrupted every [`PROBE_STRIDE`]
//! successful ops by a corruption probe: snapshot the state, run *one* op through
//! [`Data::apply`], assert the gate properties, then restore the snapshot and
//! resume. In production `apply` only ever runs on a consistent state, so
//! `{valid state} + {one gated op}` is the exact target distribution.
//!
//! On failure the harness prints the seed and the full op log so the sequence
//! replays exactly.

use std::cell::Cell;
use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::generator::CorruptionKind;
use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{generator, harness};

use collomatique_state::InMemoryData;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, Error, InnerData, InvalidOp};

use harness::RunConfig;

/// House scale, matching `property_ops.rs`.
const CONFIG: RunConfig = RunConfig {
    seeds: 100,
    ops_per_run: 1000,
    invalid_fraction: 0.15,
};

/// One corruption probe every this many *successful* walk ops.
const PROBE_STRIDE: usize = 10;

/// Index of `kind` in [`CorruptionKind::ALL`], for the per-kind counters.
fn kind_index(kind: CorruptionKind) -> usize {
    CorruptionKind::ALL
        .iter()
        .position(|k| *k == kind)
        .expect("every kind is in ALL")
}

/// Walk `Data` through the gate (like property 4 of `property_ops.rs`), probing
/// `apply` every [`PROBE_STRIDE`] ops and asserting the gate's atomicity and
/// honesty on the resulting (usually rejected) op.
#[test]
fn apply_gate_is_atomic_and_honest() {
    // Cross-seed honesty counters (interior mutability: `for_each_seed` takes a
    // `Fn` closure).
    let landed = Cell::new(0usize); // probes that returned Ok
    let rejected = Cell::new(0usize); // probes that returned Err (rolled back)
    let attempted: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));
    let rejected_by_kind: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));
    let logic_seen = Cell::new(0usize);

    harness::for_each_seed(
        "apply_gate_is_atomic_and_honest",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let mut inner_snapshots: Vec<InnerData> = vec![];
            let mut since_probe = 0usize;

            for _ in 0..CONFIG.ops_per_run {
                // --- validated walk op through the gate (feeds category coverage) ---
                let (category, op) = generator::gen_op(
                    rng,
                    data.get_inner_data(),
                    &inner_snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);
                let (annotated, _) = data.annotate(op);
                let before = data.clone();

                match data.apply(&annotated) {
                    Ok(_) => {
                        stats.record(category, true);
                        if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                            inner_snapshots.push(data.get_inner_data().clone());
                        }
                    }
                    Err(_) => {
                        stats.record(category, false);
                        assert!(
                            data == before,
                            "a failed walk apply must leave the state unchanged",
                        );
                        continue;
                    }
                }

                since_probe += 1;
                if since_probe < PROBE_STRIDE {
                    continue;
                }
                since_probe = 0;

                // --- corruption probe off the current (valid) state ---
                let (kind, op) = generator::gen_corruption_op(rng, data.get_inner_data());
                log.push(kind.label(), &op);
                let i = kind_index(kind);
                attempted[i].set(attempted[i].get() + 1);

                let (annotated, _) = data.annotate(op);
                // Snapshot after annotate (like the walk's `before`): the clone
                // carries the already-advanced id issuer, matching production
                // rollback (which restores the issuer too).
                let snapshot = data.clone();
                match data.apply(&annotated) {
                    Err(e) => {
                        rejected.set(rejected.get() + 1);
                        rejected_by_kind[i].set(rejected_by_kind[i].get() + 1);
                        // Atomicity: every error arm rolls back to bit-identical.
                        assert!(
                            data == snapshot,
                            "a rejected apply must leave the state unchanged",
                        );
                        match e {
                            // Precheck bounced before any mutation.
                            Error::InvalidOp(InvalidOp::Precheck(_)) => {}
                            Error::InvalidOp(InvalidOp::Logic(set)) => {
                                logic_seen.set(logic_seen.get() + 1);
                                assert!(!set.is_empty(), "a Logic error carries a non-empty set");
                            }
                            Error::BrokenInvariants(set) => {
                                assert!(
                                    !set.is_empty(),
                                    "an Invariants error carries a non-empty set",
                                );
                            }
                        }
                    }
                    Ok(reverse) => {
                        landed.set(landed.get() + 1);
                        // Honesty: a landing the gate accepts really is fully valid.
                        assert_eq!(
                            data.get_inner_data().broken_invariants(),
                            Ok(BTreeSet::new()),
                            "apply returned Ok but the state is not fully valid",
                        );
                        // The returned reverse restores the pre-state exactly (the
                        // clean-landing reverse pin, carried over from step 4).
                        let mut redo = data.clone();
                        redo.force_apply(&reverse)
                            .expect("reverse of a gated op must apply");
                        assert!(
                            redo.get_inner_data() == snapshot.get_inner_data(),
                            "reverse of a gated op must restore the pre-state",
                        );
                        // ForceValid needs no special arm: without the old checker
                        // there is no "hidden repair" to detect here (the gate only
                        // ever lands fully-valid states, asserted just above). A
                        // valid landing is honest whether it changed state or was a
                        // perfect no-op. (The migration-window canary guarded
                        // force-path drift until it retired with the old world
                        // at R1.)
                    }
                }

                // Production rollback semantics: the probe never persists.
                data = snapshot;
            }
        },
    );

    // --- honesty guards (cross-seed, over the whole run) ---
    assert!(
        landed.get() > 0,
        "no corruption probe ever landed a valid state"
    );
    assert!(rejected.get() > 0, "no corruption probe was ever rejected");

    for kind in CorruptionKind::ALL {
        let i = kind_index(kind);
        assert!(
            attempted[i].get() > 0,
            "corruption kind {kind:?} was never attempted across all seeds",
        );
        if kind.corrupting() {
            assert!(
                rejected_by_kind[i].get() > 0,
                "corrupting kind {kind:?} was never rejected across all seeds",
            );
        }
    }

    assert!(
        logic_seen.get() > 0,
        "no ForceLogic probe ever reached the InvalidOp::Logic tier across all seeds",
    );
}
