//! Canary: old checked `apply` and the new `try_apply` gate agree (step 5)
//!
//! During the step-5 migration window both apply APIs coexist: consumers move
//! one file at a time from the checked `apply_*` world to the
//! [`Data::try_apply`] gate (snapshot → `force_apply` → `broken_invariants` →
//! rollback), and undo/redo still replays recorded ops through the *old*
//! `apply`. That coexistence is only sound if the two APIs agree op-by-op —
//! but the contract is *one-directional* (see plan_step_5.md §3): everything
//! old `apply` accepts, `try_apply` must also accept with the same resulting
//! state and the same computed reverse. The reverse direction has one known
//! exception, carried over from step 4: old `apply` rejects a harmless *no-op*
//! (e.g. clearing a group-list association on a non-interrogation subject,
//! `SubjectHasNoInterrogation`) that the gate accepts as a perfect no-op. This
//! canary re-verifies that contract continuously, across the whole generated
//! op space, for the entire life of the migration.
//!
//! **Shape.** A validated random walk (the testgen harness, byte-untouched)
//! drives an authoritative [`Data`] through the *old* checked path. For every
//! generated op — both `gen_op`'s valid/invalid walk ops and, every
//! [`PROBE_STRIDE`] committed ops, a `gen_corruption_op` probe — the op is
//! annotated once and then run on two twin clones of the same pre-state: one
//! through old `apply`, one through new `try_apply`. The canary asserts:
//!   * on double-`Ok` — equal resulting state and equal computed reverses
//!     (the replay-interchangeability contract);
//!   * on double-`Err` — both twins bit-identical to the pre-state (old-apply
//!     guard atomicity and new-gate rollback atomicity), and a new-side
//!     `Invariants`/`Logic` error carries a non-empty set;
//!   * on an old-`Ok`/new-`Err` split — a hard failure (new-checker-stricter,
//!     never tolerated);
//!   * on an old-`Err`/new-`Ok` split — tolerated **only** when the new twin is
//!     a perfect no-op (state bit-identical to the pre-state *and* the returned
//!     reverse itself that same no-op, so replay is a no-op either direction);
//!     any state-changing new-`Ok` against an old-`Err` is hidden work and stays
//!     fatal.
//!
//! The corruption-probe interleave is the important part: `gen_op`'s invalid
//! arm produces op-shaped invalidity, while `gen_corruption_op`'s five kinds
//! target exactly the space the stripped guards used to police — where
//! old-vs-new verdicts could plausibly split.
//!
//! **Lifetime.** This canary is *deliberately temporary*. It structurally
//! requires both APIs, and its job (verdict-equivalence evidence) is complete
//! the moment the old API loses authority. It is deleted in the deactivation
//! commit (R1) together with the old differential fuzz — do **not** port it
//! forward. `property_try_apply.rs` carries the randomized coverage of the
//! surviving primitive.
//!
//! On failure the harness prints the seed and the full op log so the sequence
//! replays exactly.

use std::cell::Cell;

use collomatique_testgen_colloscopes::generator::CorruptionKind;
use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{generator, harness};

use collomatique_state::InMemoryData;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{AnnotatedOp, ApplyError, Data, InnerData};

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

/// Runs `annotated` on two twin clones of `before` — one through old checked
/// `apply`, one through new `try_apply` — and asserts the one-directional
/// agreement contract (plan_step_5.md §3).
///
/// Returns the old-path result state on a double-`Ok` (so the caller can
/// advance the authoritative walk through the old path), or `None` otherwise
/// (double-`Err`, or a tolerated old-`Err`/new-`Ok` perfect-no-op split — in
/// both of which the old path did not advance).
fn twin_compare(before: &Data, annotated: &AnnotatedOp, what: &str) -> Option<Data> {
    let mut old_twin = before.clone();
    let mut new_twin = before.clone();

    let old_res = old_twin.apply(annotated);
    let new_res = new_twin.try_apply(annotated);

    match (old_res, new_res) {
        (Ok(old_rev), Ok(new_rev)) => {
            assert!(
                old_twin.get_inner_data() == new_twin.get_inner_data(),
                "double-Ok on {what}: old apply and new try_apply landed different states",
            );
            assert_eq!(
                old_rev, new_rev,
                "double-Ok on {what}: old apply and new try_apply computed different reverses",
            );
            Some(old_twin)
        }
        (Err(_old_err), Err(new_err)) => {
            assert!(
                &old_twin == before,
                "double-Err on {what}: a failed old apply must leave the state unchanged",
            );
            assert!(
                &new_twin == before,
                "double-Err on {what}: a failed new try_apply must roll back to the pre-state",
            );
            match &new_err {
                ApplyError::Invariants(set) => assert!(
                    !set.is_empty(),
                    "double-Err on {what}: new Invariants error carries an empty set",
                ),
                ApplyError::Logic(set) => assert!(
                    !set.is_empty(),
                    "double-Err on {what}: new Logic error carries an empty set",
                ),
                ApplyError::Precheck(_) => {}
            }
            None
        }
        (Ok(old_rev), Err(new_err)) => {
            // The forbidden direction: old apply accepts, new try_apply rejects.
            // That would be a new-checker-stricter regression, never tolerated.
            panic!(
                "verdict split on {what}: old apply accepted but new try_apply rejected \
                 (new-checker-stricter — never tolerated):\n  old reverse   = {old_rev:?}\n  \
                 new try_apply = {new_err:?}",
            );
        }
        (Err(old_err), Ok(new_rev)) => {
            // The known step-4 divergence: old checked apply rejects a harmless
            // no-op (e.g. clearing a group-list association on a
            // non-interrogation subject → `SubjectHasNoInterrogation`) that the
            // gate accepts as a perfect no-op. Tolerated ONLY when the new twin
            // changed nothing AND the returned reverse is itself that same no-op:
            // then replaying this recorded op is a no-op in either direction, so
            // it is sound to store (once replay moves onto `try_apply` in commit
            // 3.0). A state-CHANGING new-`Ok` against an old-`Err` is hidden work
            // — a force copy silently repairing what the stripped guard policed —
            // and stays fatal.
            assert!(
                &new_twin == before,
                "old-Err/new-Ok split on {what} with a state change — hidden work, not a no-op:\n  \
                 old apply = {old_err:?}\n  new try_apply landed a *changed* state",
            );
            let mut rev_twin = before.clone();
            let rev_res = rev_twin.try_apply(&new_rev);
            assert!(
                rev_res.is_ok() && &rev_twin == before,
                "old-Err/new-Ok no-op split on {what} but the returned reverse is not itself a \
                 no-op:\n  reverse = {new_rev:?}\n  reverse result = {rev_res:?}",
            );
            None
        }
    }
}

/// The canary. Walk `Data` directly (like property 4 of `property_ops.rs`),
/// comparing old `apply` against new `try_apply` on every walk op and every
/// [PROBE_STRIDE]-th step a corruption probe, and asserting the two APIs agree.
#[test]
fn old_apply_and_try_apply_agree() {
    // Cross-seed coverage guard: every corruption kind must be exercised, so
    // the probe path cannot silently degenerate. Interior mutability because
    // `for_each_seed` takes a `Fn` closure.
    let attempted: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));

    harness::for_each_seed(
        "old_apply_and_try_apply_agree",
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

                match twin_compare(&before, &annotated, category) {
                    Some(advanced) => {
                        stats.record(category, true);
                        // The walk commits through the old (authoritative) path.
                        data = advanced;
                        if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                            inner_snapshots.push(data.get_inner_data().clone());
                        }
                    }
                    None => {
                        stats.record(category, false);
                        // A failed op leaves the walker untouched (both APIs are
                        // atomic on failure; `data` was never mutated).
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
                let before = data.clone();
                // The probe never persists: `twin_compare` runs on twin clones and
                // `data` is never advanced from it.
                twin_compare(&before, &annotated, kind.label());
            }
        },
    );

    // --- honesty guard (cross-seed) ---
    for kind in CorruptionKind::ALL {
        let i = kind_index(kind);
        assert!(
            attempted[i].get() > 0,
            "corruption kind {kind:?} was never attempted across all seeds",
        );
    }
}
