//! Differential fuzz: `force_apply` + two-checker agreement (step 4)
//!
//! Step 2 delivered the precise new checker ([InnerData::broken_invariants])
//! and step 3 certified the old first-error checker
//! ([InnerData::check_invariants]) as the complete reference oracle. This test
//! earns trust in the new checker by *differential fuzzing*: it force-applies
//! single ops that land in *invalid* states and asserts the two checkers agree
//! via the stage-7 three-part differential ([invariants::assert_differential]).
//!
//! **Fuzz shape — depth-1 probes off a validated walk.** A validated random
//! walk (the existing testgen harness, byte-untouched) is interrupted every
//! [PROBE_STRIDE] successful ops by a corruption probe: snapshot the state,
//! force *one* op via [Data::force_apply], run the differential, then restore
//! the snapshot and resume. In the step-5 architecture `force_apply` only ever
//! runs on a *consistent* state, so `{valid state} + {one forced op}` is the
//! exact target distribution.
//!
//! On failure the harness prints the seed and the full op log (walk + probe
//! entries) so the sequence replays exactly.

use std::cell::Cell;

use collomatique_testgen_colloscopes::generator::CorruptionKind;
use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{generator, harness};

use collomatique_state::InMemoryData;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, InnerData, invariants};

use harness::RunConfig;

/// House scale, matching `property_ops.rs`.
const CONFIG: RunConfig = RunConfig {
    seeds: 100,
    ops_per_run: 1000,
    invalid_fraction: 0.15,
};

/// One corruption probe every this many *successful* walk ops.
const PROBE_STRIDE: usize = 10;

/// Index of `kind` in [CorruptionKind::ALL], for the per-kind counters.
fn kind_index(kind: CorruptionKind) -> usize {
    CorruptionKind::ALL
        .iter()
        .position(|k| *k == kind)
        .expect("every kind is in ALL")
}

/// The differential fuzz. Walk `Data` directly (like property 4 of
/// `property_ops.rs`), probing `force_apply` every [PROBE_STRIDE] ops and
/// asserting the two checkers agree on the resulting (usually broken) state.
#[test]
fn force_apply_agrees_with_old_checker() {
    // Honesty counters, accumulated cross-seed so they stay stable regardless
    // of how any single seed's probes fall. Interior mutability because
    // `for_each_seed` takes a `Fn` closure.
    let landed = Cell::new(0usize);
    let broken = Cell::new(0usize);
    let attempted: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));
    let broken_by_kind: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));

    harness::for_each_seed(
        "force_apply_agrees_with_old_checker",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let mut inner_snapshots: Vec<InnerData> = vec![];
            let mut since_probe = 0usize;

            for _ in 0..CONFIG.ops_per_run {
                // --- validated walk op (feeds the harness category coverage) ---
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
                let snapshot = data.clone();
                let (kind, op) = generator::gen_corruption_op(rng, data.get_inner_data());
                log.push(kind.label(), &op);
                let i = kind_index(kind);
                attempted[i].set(attempted[i].get() + 1);

                let (annotated, _) = data.annotate(op);
                match data.force_apply(&annotated) {
                    Err(_) => {
                        // Bounced off a kept carve-out guard: state untouched.
                        assert!(
                            data == snapshot,
                            "a failed force_apply must leave the state unchanged",
                        );
                    }
                    Ok(reverse) => {
                        // The payoff: the two checkers must agree on this state.
                        invariants::assert_differential(data.get_inner_data());

                        let is_broken = data.get_inner_data().check_invariants().is_err();
                        landed.set(landed.get() + 1);
                        if is_broken {
                            broken.set(broken.get() + 1);
                            broken_by_kind[i].set(broken_by_kind[i].get() + 1);
                        } else {
                            // Clean landing: the reverse feeds history in step 5,
                            // so pin that it restores the pre-state exactly.
                            let mut redo = data.clone();
                            redo.force_apply(&reverse)
                                .expect("reverse of a clean forced op must apply");
                            assert!(
                                redo.get_inner_data() == snapshot.get_inner_data(),
                                "reverse of a clean forced op must restore the pre-state",
                            );
                        }

                        if kind == CorruptionKind::ForceValid {
                            // Standing anti-drift pin: on a valid op the thin
                            // copy must match the checked original exactly, in
                            // both the resulting state and the computed reverse.
                            let mut checked = snapshot.clone();
                            let checked_rev = checked
                                .apply(&annotated)
                                .expect("a ForceValid op must pass checked apply");
                            assert!(
                                checked.get_inner_data() == data.get_inner_data(),
                                "forced apply diverged from checked apply on a valid op",
                            );
                            assert!(
                                checked_rev == reverse,
                                "forced reverse diverged from checked reverse on a valid op",
                            );
                        }
                    }
                }

                // Production rollback semantics: the probe never persists.
                data = snapshot;
            }
        },
    );

    // --- honesty guards (cross-seed, over the whole run) ---
    let total_landed = landed.get();
    let total_broken = broken.get();
    assert!(total_landed > 0, "no corruption probe ever landed");
    assert!(
        total_broken * 4 >= total_landed,
        "expected >=25% of landed probes to be broken (old checker Err); \
         got {total_broken}/{total_landed}",
    );

    for kind in CorruptionKind::ALL {
        let i = kind_index(kind);
        assert!(
            attempted[i].get() > 0,
            "corruption kind {kind:?} was never attempted across all seeds",
        );
        if kind.corrupting() {
            assert!(
                broken_by_kind[i].get() > 0,
                "corrupting kind {kind:?} never landed a broken state across all seeds",
            );
        }
    }
}
