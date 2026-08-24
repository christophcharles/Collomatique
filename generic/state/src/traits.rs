//! Traits module
//!
//! This module defines the various traits that an in-memory
//! data representation should have.

/// This trait represents an operation (annotated or not)
pub trait Operation: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq {}

/// This trait represents an action description (to be stored in history)
pub trait Description: Send + Sync + Clone {}

impl<T: Send + Sync + Clone> Description for T {}

/// In memory data trait
///
/// This trait should be implemented by an struct
/// that represents the complete state of a file
/// in memory.
///
/// It is built upon by the state crate to have
/// a consistent modification history, sessions, etc.
pub trait InMemoryData: Clone + Send + Sync + std::fmt::Debug {
    /// Non-annotated type for the operations
    type OriginalOperation: Operation;

    /// Annotated type for the operations
    ///
    /// Possibly this can be the same as [Self::OriginalOperation]
    /// if the original operation is indeed complete.
    type AnnotatedOperation: Operation;

    /// Additionnal information when annotating
    ///
    /// Annotating technically adds informations to an operation
    /// This type should encode relevant info that might be
    /// useful for the operation issuer.
    type NewInfo;

    /// The unresolvable tier of [ApplyError]: bad op input (including op
    /// payloads that would land logically impossible data).
    type InvalidOp: std::error::Error + Send + Sync + Clone;

    /// The resolvable tier of [ApplyError]: one broken invariant. `Ord` is the
    /// canonical order the cascade's deterministic pick relies on.
    type Invariant: Send + Sync + Clone + Ord + std::fmt::Debug + std::fmt::Display;

    /// Annotate an operation
    ///
    /// If [Self::OriginalOperation] and [Self::AnnotatedOperation]
    /// are the same type, it can simply do a no-op and return
    /// directly the original operation.
    ///
    /// In general however, [Self::OriginalOperation] will be a
    /// less complete description operation that should be annotated with ids.
    /// The [InMemoryData] object must then issue ids and complete the type
    /// accordingly.
    ///
    /// Takes `&mut self`: annotation may consume ids from the implementor's
    /// issuer, and the signature declares that side effect.
    fn annotate(
        &mut self,
        op: Self::OriginalOperation,
    ) -> (Self::AnnotatedOperation, Self::NewInfo);

    /// Apply an operation through the apply/check/rollback gate and return its
    /// inverse.
    ///
    /// The inverse operation is computed from the state as it was *before* the
    /// operation was applied. This method forces the operation through and then
    /// checks the resulting state: on any breakage the data is rolled back from
    /// a snapshot. Precheck failures never touch the data; invariant/logic
    /// failures are rolled back. Either way, a failed call leaves the data
    /// strictly unchanged.
    fn apply(
        &mut self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, ApplyError<Self::InvalidOp, Self::Invariant>>;
}

use thiserror::Error;

use crate::history::AggregatedOp;

/// Error surface of the apply/check/rollback gate, shared by every
/// [InMemoryData] implementor.
///
/// Two tiers. [ApplyError::InvalidOp] means the op cannot be made sense of
/// against the current state (bad input: no-clobber, dangling op target, bad
/// anchor — or an op payload that would land logically impossible data). It is
/// never resolvable. [ApplyError::BrokenInvariants] means the op is
/// well-formed but the state does not satisfy what it needs: the payload is
/// the exact set of broken invariants, in the canonical `Ord`. At step 6 this
/// is what the cascade resolves; outside the cascade it is simply an error.
///
/// Either way the failed `apply` left the data strictly unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ApplyError<InvalidOp, Invariant>
where
    InvalidOp: std::error::Error,
    Invariant: std::fmt::Debug + std::fmt::Display + Ord,
{
    /// Bad op input; never resolvable.
    #[error(transparent)]
    InvalidOp(InvalidOp),
    /// The op needs these invariants fixed first; resolvable by the cascade.
    #[error("the operation would break data invariants: {}", format_error_set(.0))]
    BrokenInvariants(std::collections::BTreeSet<Invariant>),
}

/// Itemises a set of errors through each entry's own [std::fmt::Display] so a
/// UI dialog can surface a meaningful message without learning the vocabulary.
/// (Moved up from `state-colloscopes`, which keeps its own private copy for
/// its remaining local enums.)
fn format_error_set<T: std::fmt::Display>(set: &std::collections::BTreeSet<T>) -> String {
    set.iter().map(T::to_string).collect::<Vec<_>>().join("; ")
}

/// Error for [Manager::redo] and [Manager::undo]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HistoryError {
    /// History is depleted
    ///
    /// Happens when trying to undo/redo but there is
    /// no more history to go through.
    #[error("History is depleted - cannot undo/redo")]
    HistoryDepleted,
}

/// This trait will be implemented by *managers*.
///
/// Managers manage the in memory data and keep it in sync
/// with modification history.
///
/// To be a [Manager], a struct must first implement the sealed
/// trait ManagerInternal which provides *privately* (not accessible
/// from public API) access to internal data and modification history.
///
/// The two main managers are [crate::AppState] and [crate::AppSession].
pub trait Manager: private::ManagerInternal {
    /// Gives read-only access to the internal data
    fn get_data(&self) -> &<Self as private::ManagerInternal>::Data {
        self.get_in_memory_data()
    }

    /// Apply an operation through the apply/check/rollback gate and keep the
    /// modification history consistent.
    ///
    /// Routes the operation through [InMemoryData::apply]. A failed op
    /// leaves the data unchanged and stores nothing in history.
    fn apply(
        &mut self,
        op: <<Self as private::ManagerInternal>::Data as InMemoryData>::OriginalOperation,
        desc: <Self as private::ManagerInternal>::Desc,
    ) -> Result<
        <<Self as private::ManagerInternal>::Data as InMemoryData>::NewInfo,
        ApplyError<
            <<Self as private::ManagerInternal>::Data as InMemoryData>::InvalidOp,
            <<Self as private::ManagerInternal>::Data as InMemoryData>::Invariant,
        >,
    > {
        let (annotated_op, new_info) = self.get_in_memory_data_mut().annotate(op);

        let backward = self.get_in_memory_data_mut().apply(&annotated_op)?;
        let rev_op = crate::history::ReversibleOp {
            forward: annotated_op,
            backward,
        };

        let aggregated_op = crate::history::AggregatedOp::new(vec![rev_op]);
        self.get_modification_history_mut()
            .store(aggregated_op, desc);

        Ok(new_info)
    }

    /// Apply an operation through the cascade (see [crate::cascade::apply_cascade])
    /// and keep the modification history consistent: the whole cascade — the
    /// fixes it had to apply and the operation itself — lands as **one** history
    /// slot, so a single [Manager::undo] takes the document back where it was.
    ///
    /// Returns the annotation's [InMemoryData::NewInfo] and the fixes the
    /// cascade applied, as their [crate::cascade::FixOp] meanings: what the
    /// repairs *did*, never which invariant caused them (that never leaves the
    /// engine). The order is the engine's application order. Each fix comes
    /// with the index, in that same list, of the fix whose failure required it
    /// — `None` when the target itself did; since a fix lands before the one
    /// that needed it, the index always points to a later entry.
    ///
    /// On `Err`, data and history are unchanged — the engine restores its entry
    /// snapshot bit-identically on every failure, and nothing is stored in
    /// history unless the cascade succeeded. The id allocator may however have
    /// advanced: the annotation happens first, and its ids burn on failure,
    /// never to be reused. That is the same contract [Manager::apply] has, and
    /// the same relationship [Manager::undo] has to the allocator (undone ids
    /// stay live because [Manager::redo] can restore them): the allocator is
    /// monotone across the manager's whole life and is not part of rollback.
    fn apply_cascade(
        &mut self,
        op: <<Self as private::ManagerInternal>::Data as InMemoryData>::OriginalOperation,
        desc: <Self as private::ManagerInternal>::Desc,
    ) -> Result<
        (
            <<Self as private::ManagerInternal>::Data as InMemoryData>::NewInfo,
            Vec<(
                <<Self as private::ManagerInternal>::Data as crate::cascade::Fixable>::Fix,
                Option<usize>,
            )>,
        ),
        ApplyError<
            <<Self as private::ManagerInternal>::Data as InMemoryData>::InvalidOp,
            <<Self as private::ManagerInternal>::Data as InMemoryData>::Invariant,
        >,
    >
    where
        <Self as private::ManagerInternal>::Data: crate::cascade::Fixable,
    {
        let (annotated_op, new_info) = self.get_in_memory_data_mut().annotate(op);

        let receipt = crate::cascade::apply_cascade(self.get_in_memory_data_mut(), annotated_op)?;

        let fixes = receipt
            .fixes()
            .iter()
            .map(|landed| (landed.fix().clone(), landed.parent()))
            .collect();

        self.get_modification_history_mut()
            .store(receipt.into_aggregated_op(), desc);

        Ok((new_info, fixes))
    }

    /// Returns the name of the last operation if it exists
    fn get_undo_name(&self) -> Option<&<Self as private::ManagerInternal>::Desc> {
        self.get_modification_history().get_undo_name()
    }

    /// Returns the name of the next operation if it exists
    fn get_redo_name(&self) -> Option<&<Self as private::ManagerInternal>::Desc> {
        self.get_modification_history().get_redo_name()
    }

    /// Checks if it is possible to cancel an operation
    ///
    /// Returns `true` if there is a cancellable operation in history.
    fn can_undo(&self) -> bool {
        self.get_modification_history().can_undo()
    }

    /// Checks if it is possible to redo an operation
    ///
    /// Returns `true` if there is a redoable operation in (future) history.
    fn can_redo(&self) -> bool {
        self.get_modification_history().can_redo()
    }

    /// Returns the last operation in history but does not reverse it
    ///
    /// Similarly to [Manager::undo], this returns the last operation in
    /// history (if it exists). However, it does not reverse it: the app state
    /// is not changed. You can use [Manager::get_undo_name] if you want the
    /// name of the operation.
    fn get_last_op(
        &self,
    ) -> Option<AggregatedOp<<Self::Data as InMemoryData>::AnnotatedOperation>> {
        self.get_modification_history().get_last_op()
    }

    /// Undo previous operation in history
    ///
    /// If no more operation can be undone, fails.
    ///
    /// Panics if there is an error in applying the previous operation
    /// (this means there is a logic error as the previous operation was applied and must be reversible).
    fn undo(
        &mut self,
    ) -> Result<AggregatedOp<<Self::Data as InMemoryData>::AnnotatedOperation>, HistoryError> {
        match self.get_modification_history_mut().undo() {
            Some(aggregated_op) => {
                if let Err(e) = private::update_internal_state_with_aggregated(self, &aggregated_op)
                {
                    panic!(
                        "Data should be consistent as it was automatically build from previous state.\n{}",
                        e
                    );
                }
                Ok(aggregated_op)
            }
            None => Err(HistoryError::HistoryDepleted),
        }
    }

    /// Redo last operation in history
    ///
    /// If no more operation can be redone, fails.
    ///
    /// Panics if there is an error in applying the last operation
    /// (this means there is a logic error as the operation was already previously applied).
    fn redo(
        &mut self,
    ) -> Result<AggregatedOp<<Self::Data as InMemoryData>::AnnotatedOperation>, HistoryError> {
        match self.get_modification_history_mut().redo() {
            Some(aggregated_op) => {
                if let Err(e) = private::update_internal_state_with_aggregated(self, &aggregated_op)
                {
                    panic!(
                        "Data should be consistent as it was automatically build from previous state.\n{}",
                        e
                    );
                }
                Ok(aggregated_op)
            }
            None => Err(HistoryError::HistoryDepleted),
        }
    }

    /// Returns the aggregated history
    ///
    /// See [crate::history::ModificationHistory::build_aggregated_op]
    fn get_aggregated_history(
        &self,
    ) -> crate::history::AggregatedOp<
        <<Self as private::ManagerInternal>::Data as InMemoryData>::AnnotatedOperation,
    > {
        self.get_modification_history().build_aggregated_op()
    }
}

impl<T: private::ManagerInternal> Manager for T {}

pub(crate) mod private {
    use super::*;

    /// Used internally
    ///
    /// Replays an aggregated operation (for [Manager::undo] or [Manager::redo]).
    ///
    /// If the aggregated op fails in the middle of the process, everything is reversed
    /// and the error is returned.
    ///
    /// If the reverse process fails, the function panics.
    pub fn update_internal_state_with_aggregated<T: ManagerInternal>(
        manager: &mut T,
        aggregated_op: &crate::history::AggregatedOp<<T::Data as InMemoryData>::AnnotatedOperation>,
    ) -> Result<
        (),
        ApplyError<<T::Data as InMemoryData>::InvalidOp, <T::Data as InMemoryData>::Invariant>,
    > {
        let ops = aggregated_op.inner();

        let mut error = None;
        let mut count = 0;

        for rev_op in ops {
            match manager.get_in_memory_data_mut().apply(&rev_op.forward) {
                Ok(inverse) => {
                    debug_assert_eq!(
                        inverse, rev_op.backward,
                        "stored backward op is inconsistent with the inverse recomputed on replay"
                    );
                }
                Err(err) => {
                    error = Some(err);
                    break;
                }
            }

            count += 1;
        }

        let Some(err) = error else {
            return Ok(());
        };

        let skip_size = ops.len() - count;
        for rev_op in ops.iter().rev().skip(skip_size) {
            let result = manager.get_in_memory_data_mut().apply(&rev_op.backward);

            if let Err(e) = result {
                panic!(
                    r#"Failed to reverse failed aggregated operations.
    Initial failed op: {:?}
    Initial error: {:?}
    Problematic op to reverse: {:?}
    Error in reversing: {:?}"#,
                    ops[count], err, rev_op, e,
                );
            }
        }

        Err(err)
    }

    /// Sealed trait
    ///
    /// [ManagerInternal] is a prerequirement for the [Manager] trait.
    ///
    /// The trait is sealed so that the functions defined here are *private*.
    /// Thus, we can protect from mutable access to the underlying data and
    /// history.
    pub trait ManagerInternal: Send + Sync + Clone {
        /// Type of the underlying data
        type Data: InMemoryData;
        type Desc: Description;

        /// Return a mutable reference to the in-memory data
        fn get_in_memory_data_mut(&mut self) -> &mut Self::Data;
        /// Return a mutable reference to the modification history
        fn get_modification_history_mut(
            &mut self,
        ) -> &mut crate::history::ModificationHistory<
            <Self::Data as InMemoryData>::AnnotatedOperation,
            Self::Desc,
        >;

        /// Return a non-mutable reference to the in-memory data
        fn get_in_memory_data(&self) -> &Self::Data;
        /// Return a non-mutable reference to the modification history
        fn get_modification_history(
            &self,
        ) -> &crate::history::ModificationHistory<
            <Self::Data as InMemoryData>::AnnotatedOperation,
            Self::Desc,
        >;
    }
}

#[cfg(test)]
mod tests {
    use super::private::ManagerInternal;
    use super::*;
    use crate::history::{AggregatedOp, ReversibleOp};
    use crate::state::AppState;
    use crate::test_utils::{
        FakeData, FakeError, FakeOp, QuoteData, QuoteFix, QuoteInvariant, QuoteOp, rev_set,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn new_state(value: i64) -> AppState<FakeData, &'static str> {
        AppState::new(FakeData::new(value))
    }

    /// A state over the cascade toy: the only [InMemoryData] here that is
    /// [crate::cascade::Fixable], hence the only one `apply_cascade` accepts.
    fn quote_state(students: &[u64], quotes: &[(u64, u64)]) -> AppState<QuoteData, &'static str> {
        AppState::new(QuoteData {
            students: students.iter().copied().collect(),
            quotes: quotes.iter().copied().collect(),
            notes: BTreeMap::new(),
        })
    }

    #[test]
    fn update_with_aggregated_applies_all_ops_in_order() {
        let mut state = new_state(0);
        let aggregated = AggregatedOp::new(vec![rev_set(0, 1), rev_set(1, 5)]);

        let result = private::update_internal_state_with_aggregated(&mut state, &aggregated);

        assert_eq!(result, Ok(()));
        assert_eq!(state.get_data().value, 5);
    }

    #[test]
    fn update_with_aggregated_rolls_back_applied_prefix_on_failure() {
        let mut state = new_state(0);
        // Second op expects value 5 but will find 1: it fails mid-aggregate
        let aggregated = AggregatedOp::new(vec![rev_set(0, 1), rev_set(5, 9)]);

        let result = private::update_internal_state_with_aggregated(&mut state, &aggregated);

        assert_eq!(
            result,
            Err(ApplyError::InvalidOp(FakeError::ValueMismatch {
                expected: 5,
                found: 1
            }))
        );
        assert_eq!(state.get_data().value, 0);
    }

    #[test]
    // Which panic fires depends on the profile: in debug builds the canary
    // fires first during the forward replay, in release builds (debug
    // assertions off) the canary is gone and the rollback safety net fires
    // instead. Both are expected, hence the two `cfg_attr`s.
    #[cfg_attr(
        debug_assertions,
        should_panic(expected = "stored backward op is inconsistent")
    )]
    #[cfg_attr(
        not(debug_assertions),
        should_panic(expected = "Failed to reverse failed aggregated operations")
    )]
    fn replay_panics_if_stored_backward_is_inconsistent() {
        let mut state = new_state(0);
        let broken_backward = ReversibleOp {
            forward: FakeOp::Set { old: 0, new: 1 },
            // Wrong backward op: the true inverse is Set { old: 1, new: 0 }
            backward: FakeOp::Set { old: 42, new: 0 },
        };
        let aggregated = AggregatedOp::new(vec![broken_backward, rev_set(5, 9)]);

        let _ = private::update_internal_state_with_aggregated(&mut state, &aggregated);
    }

    #[test]
    fn apply_changes_data_and_stores_history() {
        let mut state = new_state(0);

        let result = state.apply(FakeOp::Set { old: 0, new: 1 }, "set to 1");

        assert_eq!(result, Ok(()));
        assert_eq!(state.get_data().value, 1);
        assert!(state.can_undo());
        assert!(!state.can_redo());
        assert_eq!(state.get_undo_name(), Some(&"set to 1"));
    }

    #[test]
    fn apply_failing_leaves_state_untouched_and_stores_nothing() {
        let mut state = new_state(0);

        let result = state.apply(FakeOp::Fail, "never happens");

        assert_eq!(result, Err(ApplyError::InvalidOp(FakeError::ApplyFailed)));
        assert_eq!(state.get_data().value, 0);
        assert!(!state.can_undo());
        assert_eq!(state.get_last_op(), None);
    }

    #[test]
    // History written by `apply` must replay correctly through undo/redo,
    // which go through `update_internal_state_with_aggregated` — itself now on
    // `apply` (commit 3.0), so the ops it recorded replay through the same
    // gate that accepted them.
    fn apply_history_replays_through_undo_redo() {
        let mut state = new_state(0);
        state
            .apply(FakeOp::Set { old: 0, new: 1 }, "set to 1")
            .expect("valid op");

        state.undo().expect("one op to undo");
        assert_eq!(state.get_data().value, 0);

        state.redo().expect("one op to redo");
        assert_eq!(state.get_data().value, 1);
    }

    #[test]
    fn undo_and_redo_on_empty_history_fail_with_history_depleted() {
        let mut state = new_state(0);

        assert_eq!(state.undo(), Err(HistoryError::HistoryDepleted));
        assert_eq!(state.redo(), Err(HistoryError::HistoryDepleted));
    }

    #[test]
    fn undo_restores_previous_state_and_redo_reapplies() {
        let mut state = new_state(0);
        state
            .apply(FakeOp::Set { old: 0, new: 1 }, "set to 1")
            .expect("valid op");
        state
            .apply(FakeOp::Set { old: 1, new: 2 }, "set to 2")
            .expect("valid op");

        state.undo().expect("one op to undo");
        assert_eq!(state.get_data().value, 1);
        state.undo().expect("one op to undo");
        assert_eq!(state.get_data().value, 0);

        state.redo().expect("one op to redo");
        assert_eq!(state.get_data().value, 1);
        state.redo().expect("one op to redo");
        assert_eq!(state.get_data().value, 2);
    }

    #[test]
    #[should_panic(expected = "Data should be consistent")]
    fn undo_panics_if_data_was_corrupted_behind_historys_back() {
        let mut state = new_state(0);
        state
            .apply(FakeOp::Set { old: 0, new: 1 }, "set to 1")
            .expect("valid op");

        // Corrupt the data without going through the history
        state.get_in_memory_data_mut().value = 999;

        let _ = state.undo();
    }

    #[test]
    fn get_aggregated_history_flattens_applied_ops() {
        let mut state = new_state(0);
        state
            .apply(FakeOp::Set { old: 0, new: 1 }, "set to 1")
            .expect("valid op");
        state
            .apply(FakeOp::Set { old: 1, new: 2 }, "set to 2")
            .expect("valid op");
        state
            .apply(FakeOp::Set { old: 2, new: 3 }, "set to 3")
            .expect("valid op");
        state.undo().expect("one op to undo");

        let aggregated = state.get_aggregated_history();

        assert_eq!(aggregated.inner(), &vec![rev_set(0, 1), rev_set(1, 2)]);
    }

    // Removing a student whose two quotes must go first: the whole cascade —
    // both fixes and the target — becomes a single history slot, and the fixes
    // come back as the `Fix` values the map answered.
    #[test]
    fn apply_cascade_lands_fixes_and_target_in_one_history_slot() {
        let original = quote_state(&[1], &[(10, 1), (20, 1)]);
        let mut state = original.clone();

        let (new_info, fixes) = state
            .apply_cascade(QuoteOp::RemoveStudent(1), "remove student 1")
            .expect("the cascade resolves");

        // The toy annotates by identity, so its `NewInfo` is `()`: what this
        // pins is the plumbing (the value handed back is the annotation's),
        // not an interesting id.
        assert_eq!(new_info, ());
        // Both quote removals are repairs the target itself needed, so neither
        // has a parent fix.
        assert_eq!(
            fixes,
            vec![
                (QuoteFix::RemoveQuote(10), None),
                (QuoteFix::RemoveQuote(20), None),
            ],
        );
        assert!(state.get_data().students.is_empty());
        assert!(state.get_data().quotes.is_empty());

        // One slot, holding the fixes then the target — not three slots.
        assert_eq!(state.get_undo_name(), Some(&"remove student 1"));
        let slot = state.get_last_op().expect("one stored slot");
        assert_eq!(
            slot.inner()
                .iter()
                .map(|r| r.inner().clone())
                .collect::<Vec<_>>(),
            vec![
                QuoteOp::RemoveQuote(10),
                QuoteOp::RemoveQuote(20),
                QuoteOp::RemoveStudent(1),
            ],
        );

        // And it undoes in one step, back to the exact starting document.
        state.undo().expect("one op to undo");
        assert_eq!(state.get_data(), original.get_data());
        assert!(!state.can_undo());
    }

    // An op that breaks nothing goes through the same door and reports no fix:
    // the receipt's target lands alone, as a one-entry slot.
    #[test]
    fn apply_cascade_of_a_lone_op_reports_no_fix() {
        let mut state = quote_state(&[1], &[]);

        let (_new_info, fixes) = state
            .apply_cascade(QuoteOp::AddStudent(2), "add student 2")
            .expect("valid op");

        assert_eq!(fixes, vec![]);
        assert!(state.get_data().students.contains(&2));
        let slot = state.get_last_op().expect("one stored slot");
        assert_eq!(
            slot.inner()
                .iter()
                .map(|r| r.inner().clone())
                .collect::<Vec<_>>(),
            vec![QuoteOp::AddStudent(2)],
        );
    }

    // The `Err` contract: a target the map cannot fix (its break is caused by
    // the op's own payload, not by anything in the state) leaves both data and
    // history exactly as they were. The id allocator is deliberately outside
    // this assertion — the annotation ran, and its ids burn.
    #[test]
    fn apply_cascade_convicted_leaves_data_and_history_unchanged() {
        let mut state = quote_state(&[1], &[]);
        state
            .apply_cascade(QuoteOp::AddStudent(3), "add student 3")
            .expect("valid op");
        let data_before = state.get_data().clone();
        let history_before = state.get_aggregated_history();
        let last_op_before = state.get_last_op();

        let err = state
            .apply_cascade(
                QuoteOp::SetQuote {
                    quote: 99,
                    author: 2,
                },
                "never happens",
            )
            .unwrap_err();

        assert_eq!(
            err,
            ApplyError::BrokenInvariants(BTreeSet::from([QuoteInvariant::DanglingQuoteAuthor(99)])),
        );
        assert_eq!(state.get_data(), &data_before);
        assert_eq!(state.get_aggregated_history(), history_before);
        // The flattened history above cannot see a *spurious empty* slot (it
        // contributes no op); the slot itself, and the name it would carry,
        // can — hence both.
        assert_eq!(state.get_last_op(), last_op_before);
        assert_eq!(state.get_undo_name(), Some(&"add student 3"));
        assert!(!state.can_redo());
    }
}
