//! Property tests over generated elementary-op sequences
//!
//! Phase 0 of the state consolidation plan (docs/state_consolidation_plan.md §3):
//! a deterministic, seed-driven safety net exercising `Data::apply` (which
//! computes and returns the reverse of every op) and the undo/redo
//! machinery, using the explicit `InnerData::check_invariants` oracle
//! (which stays valid once Phase 2 demotes the internal panicking check).
//!
//! On failure, the seed and the full op log are printed: re-running the
//! same test binary reproduces the exact same sequence.

mod property_ops {
    pub mod generator;
    pub mod harness;
    pub mod synth;
}

use property_ops::{generator, harness};

use collomatique_state::AppSession;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, InnerData};
use rand::Rng;

use harness::CONFIG;

/// Generates one op, applies it on any manager, records the outcome
macro_rules! gen_and_apply {
    ($manager:expr, $rng:expr, $log:expr, $stats:expr, $snapshots:expr) => {{
        let (category, op) = generator::gen_op(
            $rng,
            $manager.get_data().get_inner_data(),
            $snapshots,
            CONFIG.invalid_fraction,
        );
        $log.push(category, &op);
        let ok = $manager.apply(op, category.to_string()).is_ok();
        $stats.record(category, ok);
        ok
    }};
}

/// Property 1: after every successful op the explicit invariants hold,
/// and every failed op leaves the state exactly unchanged (error atomicity).
#[test]
fn invariants_hold_and_errors_are_atomic() {
    harness::for_each_seed(
        "invariants_hold_and_errors_are_atomic",
        &CONFIG,
        |rng, log, stats| {
            let (mut state, _) = harness::bootstrap(rng);
            let mut snapshots: Vec<InnerData> = vec![state.get_data().get_inner_data().clone()];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = generator::gen_op(
                    rng,
                    state.get_data().get_inner_data(),
                    &snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);
                let before = state.get_data().get_inner_data().clone();

                match state.apply(op, category.to_string()) {
                    Ok(_) => {
                        stats.record(category, true);
                        state
                            .get_data()
                            .get_inner_data()
                            .check_invariants()
                            .expect("invariants must hold after a successful op");
                        if snapshots.len() < 8 && rng.random_bool(0.02) {
                            snapshots.push(state.get_data().get_inner_data().clone());
                        }
                    }
                    Err(_) => {
                        stats.record(category, false);
                        assert!(
                            state.get_data().get_inner_data() == &before,
                            "a failed op must leave the state unchanged",
                        );
                    }
                }
            }
        },
    );
}

/// Property 2: undoing the whole history walks back through every
/// recorded state down to the empty document, and redoing walks forward
/// through the exact same states (annotated ids make redo reproducible).
#[test]
fn undo_all_and_redo_all_round_trip() {
    harness::for_each_seed(
        "undo_all_and_redo_all_round_trip",
        &CONFIG,
        |rng, log, stats| {
            let (mut state, mut snapshots) = harness::bootstrap(rng);
            let mut inner_snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                if gen_and_apply!(state, rng, log, stats, &inner_snapshots) {
                    snapshots.push(state.get_data().clone());
                    if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                        inner_snapshots.push(state.get_data().get_inner_data().clone());
                    }
                }
            }

            for pos in (0..snapshots.len() - 1).rev() {
                state.undo().expect("history should not be depleted yet");
                assert!(
                    state.get_data() == &snapshots[pos],
                    "state diverged while undoing back to history position {pos}",
                );
            }
            assert!(!state.can_undo());
            assert!(state.undo().is_err());

            for (pos, snapshot) in snapshots.iter().enumerate().skip(1) {
                state.redo().expect("redo tail should not be depleted yet");
                assert!(
                    state.get_data() == snapshot,
                    "state diverged while redoing forward to history position {pos}",
                );
            }
            assert!(!state.can_redo());
            assert!(state.redo().is_err());
        },
    );
}

/// Property 3: a random walk of undo / redo / fresh-apply moves always
/// matches a simple model (position pointer + snapshot per position),
/// including the truncation of the redo branch on a fresh apply.
#[test]
fn random_undo_redo_apply_walk() {
    harness::for_each_seed("random_undo_redo_apply_walk", &CONFIG, |rng, log, stats| {
        let (mut state, mut snapshots) = harness::bootstrap(rng);
        let mut pos = snapshots.len() - 1;
        let mut inner_snapshots: Vec<InnerData> = vec![];

        for _ in 0..CONFIG.ops_per_run {
            match rng.random_range(0..3) {
                0 if state.can_undo() => {
                    state.undo().expect("can_undo was just checked");
                    pos -= 1;
                }
                1 if state.can_redo() => {
                    state.redo().expect("can_redo was just checked");
                    pos += 1;
                }
                _ => {
                    if gen_and_apply!(state, rng, log, stats, &inner_snapshots) {
                        snapshots.truncate(pos + 1);
                        snapshots.push(state.get_data().clone());
                        pos += 1;
                        if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                            inner_snapshots.push(state.get_data().get_inner_data().clone());
                        }
                    }
                }
            }

            assert!(
                state.get_data() == &snapshots[pos],
                "state diverged from the model at history position {pos}",
            );
            assert_eq!(state.can_undo(), pos > 0);
            assert_eq!(state.can_redo(), pos < snapshots.len() - 1);
        }
    });
}

/// Property 4: for every op that applies successfully, applying the
/// reverse computed and returned by `apply` restores the state exactly.
/// This drives `InMemoryData` directly (in the same annotate → apply
/// order as `Manager::apply`) and targets the large `apply_*` family.
#[test]
fn apply_then_apply_rev_is_identity() {
    use collomatique_state::InMemoryData;

    harness::for_each_seed(
        "apply_then_apply_rev_is_identity",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let mut inner_snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = generator::gen_op(
                    rng,
                    data.get_inner_data(),
                    &inner_snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);

                let (annotated, _new_id) = data.annotate(op);
                let before = data.clone();

                let rev = match data.apply(&annotated) {
                    Ok(rev) => rev,
                    Err(_) => {
                        stats.record(category, false);
                        assert!(
                            data == before,
                            "a failed apply must leave the state unchanged",
                        );
                        continue;
                    }
                };
                stats.record(category, true);

                data.apply(&rev)
                    .expect("the reverse of a successfully applied op must apply");
                assert!(
                    data == before,
                    "apply followed by apply(reverse) must be the identity",
                );

                // Advance: the op is known to apply from this state
                let rev2 = data
                    .apply(&annotated)
                    .expect("an op that applied once must apply again after its reverse");
                assert_eq!(
                    rev2, rev,
                    "replaying from the same state must rebuild the same inverse",
                );

                if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                    inner_snapshots.push(data.get_inner_data().clone());
                }
            }
        },
    );
}

/// Property 5: sessions are atomic. Cancelling restores the state at
/// session start and leaves the parent history untouched; committing
/// stores the whole session as exactly one undoable parent slot.
/// Sessions occasionally nest.
#[test]
fn sessions_commit_and_cancel_randomly() {
    harness::for_each_seed(
        "sessions_commit_and_cancel_randomly",
        &CONFIG,
        |rng, log, stats| {
            let (mut state, _) = harness::bootstrap(rng);
            let mut inner_snapshots: Vec<InnerData> = vec![];
            let mut ops_done = 0usize;

            while ops_done < CONFIG.ops_per_run {
                // A few ops directly on the parent state
                for _ in 0..rng.random_range(0..=2) {
                    if gen_and_apply!(state, rng, log, stats, &inner_snapshots)
                        && inner_snapshots.len() < 8
                        && rng.random_bool(0.05)
                    {
                        inner_snapshots.push(state.get_data().get_inner_data().clone());
                    }
                    ops_done += 1;
                }

                let snapshot = state.get_data().clone();
                let undo_name_before: Option<String> = state.get_undo_name().cloned();

                let mut session = AppSession::<_, String>::new(state);
                for _ in 0..rng.random_range(1..=8) {
                    gen_and_apply!(session, rng, log, stats, &inner_snapshots);
                    ops_done += 1;
                    if rng.random_bool(0.1) {
                        let _ = session.undo();
                    }
                }

                // Occasionally nest a session inside the session
                if rng.random_bool(0.15) {
                    let inner_start = session.get_data().clone();
                    let mut inner = AppSession::<_, String>::new(session);
                    for _ in 0..rng.random_range(1..=4) {
                        gen_and_apply!(inner, rng, log, stats, &inner_snapshots);
                        ops_done += 1;
                    }
                    if rng.random_bool(0.5) {
                        session = inner.cancel();
                        assert!(
                            session.get_data() == &inner_start,
                            "cancelling a nested session must restore its start state",
                        );
                    } else {
                        let inner_end = inner.get_data().clone();
                        session = inner.commit("nested session".to_string());
                        assert!(
                            session.get_data() == &inner_end,
                            "committing a nested session must not change the state",
                        );
                    }
                }

                if rng.random_bool(0.5) {
                    state = session.cancel();
                    assert!(
                        state.get_data() == &snapshot,
                        "cancelling a session must restore its start state",
                    );
                    assert_eq!(
                        state.get_undo_name(),
                        undo_name_before.as_ref(),
                        "cancelling a session must leave the parent history untouched",
                    );
                } else {
                    let session_end = session.get_data().clone();
                    state = session.commit("session commit".to_string());
                    assert!(
                        state.get_data() == &session_end,
                        "committing a session must not change the state",
                    );
                    assert_eq!(state.get_undo_name(), Some(&"session commit".to_string()));

                    state
                        .undo()
                        .expect("a committed session must be undoable as one slot");
                    assert!(
                        state.get_data() == &snapshot,
                        "one undo must cancel the whole committed session",
                    );
                    state
                        .redo()
                        .expect("the committed session must be redoable");
                    assert!(
                        state.get_data() == &session_end,
                        "redo must restore the whole committed session",
                    );
                }
            }
        },
    );
}
