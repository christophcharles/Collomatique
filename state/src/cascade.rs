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
//!
//! Termination rests on the resolution map's contract, and the engine holds it
//! to it in-flight, on both routes a map bug could hang it. Fixes that land:
//! after every fix, the new state must compare **strictly below** the pre-fix
//! state in the document order ([ContentOrd], step 6.5) — landing equivalent
//! (the old perfect-no-op panic), above, or incomparable panics. Fixes that
//! never land: re-picking the same invariant with no landing in between panics
//! (the no-progress ledger) — state only changes on landings and the map is a
//! pure function of state, so such a re-pick is a cycle, not a repair. The one
//! shape neither check can catch in-flight is a map that answers a failing fix
//! with ever-fresh invented material instead of the state's own; the
//! presence-test frame rule (design doc H.3) is what excludes it, and its
//! material is finite, so a conforming map either lands or repeats a pick.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::history::{AggregatedOp, ReversibleOp};
use crate::partial_order::ContentOrd;
use crate::traits::{ApplyError, InMemoryData};

/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. ([ContentOrd] materializes the document order of the
/// monotonicity contract; the engine checks every fix against it in-flight,
/// and its content equivalence backs the no-op-fix panic — the engine never
/// compares with `==`.)
pub trait Fixable: InMemoryData + ContentOrd {
    /// One repair step for `invariant` on the current state, or `None` when
    /// the current state holds nothing that causes it (the invariant can then
    /// only come from the failing op's own payload — [apply_cascade] rejects
    /// the target op, or panics if a fix op produced the invariant).
    ///
    /// # Contract (the engraved cascade contract — design doc §5)
    ///
    /// States form a **well-founded** partial order — the document order,
    /// materialized by the [ContentOrd] supertrait bound (design doc §8,
    /// step 6.5); the empty document `Default::default()` is a minimal
    /// element. Every returned op must land **strictly below** the current
    /// state in that order: it removes a row or entity, clears an edge, or
    /// rewrites a value minus an element — never creates, and never lands
    /// equivalent. Return `None`, or a strictly-decreasing op. Because the
    /// order is well-founded, this contract bounds the number of fixes that
    /// *land*, and [apply_cascade] asserts it after every one: a fix landing
    /// equivalent, above, or incomparable panics instead of hanging. Fix
    /// chains that never land are bounded by the engine's no-progress ledger,
    /// for any map that (per the presence rule below) only ever names material
    /// present in the state: such a map has finitely many answers on an
    /// unchanged state, so it either lands one or repeats a pick.
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
    // Picks since the last landed op. State only changes on landings and the
    // map is a pure function of (state, invariant), so re-picking the same
    // invariant with no landing in between reproduces the same failing fix
    // forever — a stuck map. Any landing resets the ledger; the legitimate
    // N-round repair path always lands between re-picks.
    let mut picks_since_landing: BTreeSet<T::Invariant> = BTreeSet::new();

    loop {
        let Some(front) = stack.last().cloned() else {
            return Ok(AggregatedOp::new(applied));
        };
        let is_target = stack.len() == 1;

        // Snapshot for the monotonicity check; only fix ops are held to it (a
        // no-op *target* is a legitimate perfect no-op, G.2).
        let before = (!is_target).then(|| data.clone());

        match data.apply(&front) {
            Ok(backward) => {
                if let Some(before) = before {
                    // The in-flight monotonicity check (step 6.5): a fix
                    // must land strictly below the pre-fix state in the
                    // document order. Equivalent = the old no-op panic;
                    // above or incomparable = a growing/sideways map, which
                    // without this check would hang the cascade instead.
                    match (*data).content_cmp(&before) {
                        Some(Ordering::Less) => {}
                        Some(Ordering::Equal) => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} landed equivalent to the \
                             pre-fix state (a perfect no-op — return None when \
                             no material is present)"
                        ),
                        not_below => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} did not land strictly below \
                             the pre-fix state (content_cmp = {not_below:?})"
                        ),
                    }
                }
                picks_since_landing.clear();
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
                if !picks_since_landing.insert(pick.clone()) {
                    panic!(
                        "cascade made no progress: invariant {pick:?} was picked \
                         twice with no fix landing in between (failing op \
                         {front:?}) — the resolution map is stuck in a cycle"
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        EvilMode, EvilQuoteData, QuoteData, QuoteInvalidOp, QuoteInvariant, QuoteOp,
    };
    use std::collections::BTreeSet;

    fn quote_data(students: &[u64], quotes: &[(u64, u64)]) -> QuoteData {
        quote_data_with_notes(students, quotes, &[])
    }

    fn quote_data_with_notes(
        students: &[u64],
        quotes: &[(u64, u64)],
        notes: &[(u64, u64)],
    ) -> QuoteData {
        QuoteData {
            students: students.iter().copied().collect(),
            quotes: quotes.iter().copied().collect(),
            notes: notes.iter().copied().collect(),
        }
    }

    /// The forward op of every landed step, in order.
    fn forward_ops(applied: &AggregatedOp<QuoteOp>) -> Vec<QuoteOp> {
        applied.inner().iter().map(|r| r.inner().clone()).collect()
    }

    // 1. Two quotes by one student; removing the student cascades over two
    //    repair rounds (canonical pick order) to exactly the quote removals
    //    then the student removal.
    #[test]
    fn happy_cascade_repairs_in_canonical_order() {
        let mut data = quote_data(&[1], &[(10, 1), (20, 1)]);
        // The caller annotates the target itself — identity for the toy.
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let applied = apply_cascade(&mut data, target).expect("cascade resolves");

        assert_eq!(
            forward_ops(&applied),
            vec![
                QuoteOp::RemoveQuote(10),
                QuoteOp::RemoveQuote(20),
                QuoteOp::RemoveStudent(1),
            ],
        );
        assert!(data.students.is_empty());
        assert!(data.quotes.is_empty());
    }

    // 2. Replaying the landed steps' backwards, in reverse order, returns the
    //    exact original state (the compound reverse works stepwise, §5).
    #[test]
    fn undo_replays_backwards_to_the_original_state() {
        let original = quote_data(&[1], &[(10, 1), (20, 1)]);
        let mut data = original.clone();
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let applied = apply_cascade(&mut data, target).expect("cascade resolves");
        for rev_op in applied.inner().iter().rev() {
            data.apply(&rev_op.backward).expect("backward op applies");
        }

        assert_eq!(data, original);
    }

    // 3. A target that breaks nothing lands alone.
    #[test]
    fn a_target_that_breaks_nothing_lands_alone() {
        let mut data = quote_data(&[1], &[]);
        let (target, ()) = data.annotate(QuoteOp::AddStudent(2));

        let applied = apply_cascade(&mut data, target).expect("valid op");

        assert_eq!(applied.inner().len(), 1);
        assert!(data.students.contains(&2));
    }

    // 4. An invalid target (precheck failure) is rejected, state untouched,
    //    nothing applied.
    #[test]
    fn an_invalid_target_is_rejected_untouched() {
        let mut data = quote_data(&[1], &[(10, 1)]);
        let before = data.clone();
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(7));

        let err = apply_cascade(&mut data, target).unwrap_err();

        assert_eq!(
            err,
            ApplyError::InvalidOp(QuoteInvalidOp::UnknownStudent(7)),
        );
        assert_eq!(data, before);
    }

    // 5. A self-caused break: the canonical arm sees no such quote row in the
    //    pre-op state, returns None -> the target is rejected with its own
    //    broken-invariant set, state bit-identical, no fix ever applied.
    #[test]
    fn a_self_caused_break_the_map_cannot_fix_is_returned() {
        let mut data = quote_data(&[1], &[]);
        let before = data.clone();
        // Author 2 does not exist; there is no quote 99 to remove -> None.
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 2,
        });

        let err = apply_cascade(&mut data, target).unwrap_err();

        assert_eq!(
            err,
            ApplyError::BrokenInvariants(BTreeSet::from([QuoteInvariant::DanglingQuoteAuthor(99)])),
        );
        assert_eq!(data, before);
    }

    // 6. A no-op fix is a map-contract violation regardless of who requested
    //    it: blind mode returns Some(RemoveQuote(99)) against a quote 99 that
    //    does not exist, so the fix applies as a perfect no-op.
    #[test]
    #[should_panic(expected = "landed equivalent")]
    fn a_no_op_fix_panics() {
        let mut data = EvilQuoteData(quote_data(&[1], &[]), EvilMode::Blind);
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 2,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 7. Mid-cascade restore is real: the evil map destroys innocent quotes
    //    round after round, then says None with the target as the failing op;
    //    the snapshot restore actually runs, so every innocent quote is back
    //    despite a non-empty applied prefix.
    #[test]
    fn a_mid_cascade_failure_restores_every_innocent_change() {
        let original = quote_data(&[1, 2], &[(10, 1), (20, 2), (30, 2)]);
        let mut data = EvilQuoteData(original.clone(), EvilMode::WrongTargetElseNone);
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let err = apply_cascade(&mut data, target).unwrap_err();

        assert_eq!(
            err,
            ApplyError::BrokenInvariants(BTreeSet::from([QuoteInvariant::DanglingQuoteAuthor(10)])),
        );
        // Quotes 20 and 30 were destroyed mid-cascade; the restore brought them
        // back bit-identically.
        assert_eq!(data.0, original);
    }

    // 8. A fix op rejected as invalid is a map bug -> panic.
    #[test]
    #[should_panic(expected = "rejected as invalid")]
    fn a_fix_that_fails_precheck_panics() {
        let mut data = EvilQuoteData(quote_data(&[1], &[(10, 1)]), EvilMode::InvalidFix);
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let _ = apply_cascade(&mut data, target);
    }

    // 9. A fix that breaks a fresh invariant the map then disowns: the failing
    //    op is a fix, not the target, so None -> panic.
    #[test]
    #[should_panic(expected = "unfixable")]
    fn a_none_for_a_fix_created_invariant_panics() {
        let mut data = EvilQuoteData(
            quote_data(&[], &[]),
            EvilMode::CreateThenDisown {
                fresh_quote: 20,
                fresh_author: 6,
            },
        );
        // Target breaks DanglingQuoteAuthor(10); the map "fixes" it by creating
        // a fresh dangling quote 20, then disowns DanglingQuoteAuthor(20).
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 10,
            author: 5,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 10. A fix that grows the state: the map "fixes" the dangling author by
    //     creating the missing student. Before step 6.5 this landed a quiet
    //     creative Ok; the document-order check panics instead.
    #[test]
    #[should_panic(expected = "did not land strictly below")]
    fn a_growing_fix_panics() {
        let mut data = EvilQuoteData(quote_data(&[1], &[]), EvilMode::CreateAuthor { author: 2 });
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 2,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 11. A fix that moves the state sideways: an existing quote is
    //     re-authored to another existing student — nothing removed, nothing
    //     added, the result incomparable with the pre-fix state.
    #[test]
    #[should_panic(expected = "did not land strictly below")]
    fn a_sideways_fix_panics() {
        let mut data = EvilQuoteData(
            quote_data(&[1, 2], &[(10, 1)]),
            EvilMode::ReauthorExisting {
                quote: 10,
                author: 2,
            },
        );
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 3,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 12. A map stuck in a never-landing two-cycle: without the no-progress
    //     ledger this loops forever with no state change; with it, the second
    //     pick of the same invariant panics.
    #[test]
    #[should_panic(expected = "made no progress")]
    fn a_never_landing_fix_cycle_panics() {
        let mut data = EvilQuoteData(
            quote_data(&[1], &[]),
            EvilMode::PingPong {
                a: 10,
                b: 20,
                author: 7,
            },
        );
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 10,
            author: 7,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 13. The remembered-error conviction (D4): a fix consumes the target's
    //     own target, the retried target hits a precheck, and the user is told
    //     what the target kept breaking — not a baffling "unknown quote" for a
    //     quote they can see.
    #[test]
    fn a_fix_consuming_the_targets_target_reports_the_remembered_break() {
        let original = quote_data(&[1], &[(10, 1)]);
        let mut data = original.clone();
        let (target, ()) = data.annotate(QuoteOp::UpdateQuote {
            quote: 10,
            author: 7,
        });

        let err = apply_cascade(&mut data, target).unwrap_err();

        assert_eq!(
            err,
            ApplyError::BrokenInvariants(BTreeSet::from([QuoteInvariant::DanglingQuoteAuthor(10)])),
        );
        assert_eq!(data, original);
    }

    // 14. Depth 3: a fix that itself needs a fix. Every other green cascade
    //     here is a depth-2 alternation, because removing a quote used to
    //     break nothing. The applied list is the depth-first unwinding —
    //     deepest fix first, target last.
    #[test]
    fn a_fix_needing_its_own_fix_unwinds_depth_first() {
        let mut data = quote_data_with_notes(&[1], &[(10, 1)], &[(5, 10)]);
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let applied = apply_cascade(&mut data, target).expect("cascade resolves");

        assert_eq!(
            forward_ops(&applied),
            vec![
                QuoteOp::RemoveNote(5),
                QuoteOp::RemoveQuote(10),
                QuoteOp::RemoveStudent(1),
            ],
        );
        assert!(data.students.is_empty());
        assert!(data.quotes.is_empty());
        assert!(data.notes.is_empty());
    }

    // 15. Test 2's compound reverse, pinned at depth 3: the unwinding is
    //     replayed backwards through three levels back to the exact original.
    #[test]
    fn undo_replays_a_depth_three_cascade_to_the_original_state() {
        let original = quote_data_with_notes(&[1], &[(10, 1)], &[(5, 10)]);
        let mut data = original.clone();
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let applied = apply_cascade(&mut data, target).expect("cascade resolves");
        for rev_op in applied.inner().iter().rev() {
            data.apply(&rev_op.backward).expect("backward op applies");
        }

        assert_eq!(data, original);
    }
}
