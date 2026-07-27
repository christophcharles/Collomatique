//! The cascade: apply an op; when it fails on broken invariants, fix the
//! smallest one and retry — depth-first, discovering breakage through failure
//! (design doc §5). Every landed op passed the full apply/check/rollback gate,
//! so no invalid state ever escapes a single elementary `apply`.
//!
//! The engine works entirely in **annotated** ops (design doc D6): the caller
//! annotates the target itself and keeps its `NewInfo`, then hands the
//! [InMemoryData::AnnotatedOperation] to [apply_cascade]; fixes arrive already
//! annotated from the map (which holds only `&self` and so cannot issue ids).
//! On success the return value is the history-ready [AggregatedOp] — the target
//! is always its last entry, and `.rev()` is the compound undo. On failure the
//! data is restored bit-identically from an entry snapshot (id issuer included),
//! so `Err ⇒ unchanged` holds literally; the collected backward ops are never
//! replayed.

use std::collections::BTreeSet;

use crate::history::{AggregatedOp, ReversibleOp};
use crate::traits::{ApplyError, InMemoryData};

/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. (`PartialEq` backs the engine's no-op-fix panic.)
pub trait Fixable: InMemoryData + PartialEq {
    /// One repair step for `invariant` on the current state, or `None` when
    /// the current state holds nothing that causes it (the invariant can then
    /// only come from the failing op's own payload — [apply_cascade] rejects
    /// the target op, or panics if a fix op produced the invariant).
    ///
    /// # Contract (the engraved cascade contract — design doc §5)
    ///
    /// States form a partial order with a universal minimal element:
    /// `Default::default()`, the empty document. Every returned op must land
    /// **strictly below** the current state in that order: it removes a row
    /// or entity, clears an edge, or rewrites a value minus an element —
    /// never creates, and never lands equivalent. Return `None`, or a
    /// strictly-decreasing op; an op that applies as a perfect no-op is a
    /// contract violation, and the engine panics on it. The order is
    /// well-founded, so this contract is the cascade's termination proof —
    /// a map that *grows* the state makes the cascade loop forever (step 6.5
    /// adds a `PartialOrd`-based in-flight check for exactly that).
    ///
    /// The return type is the *annotated* op on purpose: with only `&self`,
    /// an implementation cannot reach the id issuer, so a fix physically
    /// cannot carry a fresh id — the signature leans the same way the
    /// contract does.
    ///
    /// Total: every representable invariant has an arm; no wildcard match.
    /// One step per call: the engine retries the failing op and asks again,
    /// so an invariant needing N removals is repaired over N rounds, each arm
    /// call seeing the then-current state. An arm decides by checking the
    /// **presence of the material it would remove**, never by re-evaluating
    /// the invariant's predicate (which may depend on the failing op's
    /// payload).
    fn fix_invariant(&self, invariant: &Self::Invariant) -> Option<Self::AnnotatedOperation>;
}

/// Apply `target`, resolving broken invariants by cascading fixes.
///
/// See the module docs and [Fixable] for the full contract. Returns the
/// history-ready [AggregatedOp] of every op that landed (target last) on
/// success; on failure restores the entry snapshot and returns the target's
/// most informative error (design doc D4).
pub fn apply_cascade<T: Fixable>(
    data: &mut T,
    target: T::AnnotatedOperation,
) -> Result<AggregatedOp<T::AnnotatedOperation>, ApplyError<T::InvalidOp, T::Invariant>> {
    // Failure = "*data = snapshot": bit-identical restore, id issuer included.
    let snapshot = data.clone();
    let mut stack: Vec<T::AnnotatedOperation> = vec![target];
    let mut applied: Vec<ReversibleOp<T::AnnotatedOperation>> = Vec::new();
    // The target's most recent BrokenInvariants set: the informative error
    // when the target is convicted mid-cascade (D4 — the SlotOverflowsDay trace).
    let mut last_target_break: Option<BTreeSet<T::Invariant>> = None;

    loop {
        let Some(front) = stack.last().cloned() else {
            return Ok(AggregatedOp::new(applied));
        };
        let is_target = stack.len() == 1;

        // Snapshot for the no-op-fix panic; only fix ops are held to it (a
        // no-op *target* is a legitimate perfect no-op, G.2).
        let before = (!is_target).then(|| data.clone());

        match data.apply(&front) {
            Ok(backward) => {
                if let Some(before) = before
                    && *data == before
                {
                    panic!(
                        "resolution map violated the strict-monotonicity \
                         contract: fix {front:?} applied as a perfect no-op \
                         (return None when no material is present)"
                    );
                }
                stack.pop();
                applied.push(ReversibleOp {
                    forward: front,
                    backward,
                });
            }
            Err(ApplyError::BrokenInvariants(set)) => {
                let pick = set
                    .first()
                    .expect("a BrokenInvariants error carries a non-empty set")
                    .clone();
                if is_target {
                    last_target_break = Some(set);
                }
                match data.fix_invariant(&pick) {
                    Some(fix) => stack.push(fix),
                    // None: nothing in the state causes `pick` — the failing
                    // op's own payload does.
                    None if is_target => {
                        *data = snapshot;
                        return Err(ApplyError::BrokenInvariants(
                            last_target_break.expect("just stored for the target"),
                        ));
                    }
                    None => panic!(
                        "resolution map declared {pick:?} unfixable, yet a cascade \
                         fix op produced it: {front:?}"
                    ),
                }
            }
            Err(ApplyError::InvalidOp(e)) => {
                if is_target {
                    *data = snapshot;
                    // Mid-cascade, a fix consumed the target's own target; the
                    // informative error is what the target kept breaking.
                    return Err(match last_target_break {
                        Some(set) => ApplyError::BrokenInvariants(set),
                        None => ApplyError::InvalidOp(e),
                    });
                }
                panic!("cascade fix op {front:?} was rejected as invalid: {e}");
            }
        }
    }
}
