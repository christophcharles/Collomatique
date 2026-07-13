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

    /// Error type for when [Self::apply] fails.
    type Error: std::error::Error + Send + Sync + Clone;

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
    fn annotate(&self, op: Self::OriginalOperation) -> (Self::AnnotatedOperation, Self::NewInfo);

    /// Apply an operation to the data and return its inverse
    ///
    /// The inverse operation is computed from the state as it was
    /// *before* the operation was applied (the old value is captured
    /// while it is still in hand).
    ///
    /// In case of failure, the data must be left strictly unchanged
    /// (validate first, mutate after) and the error type [Self::Error]
    /// is returned.
    fn apply(
        &mut self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error>;
}

use thiserror::Error;

use crate::history::AggregatedOp;

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

    /// Apply an operation and keep the modification history consistent
    fn apply(
        &mut self,
        op: <<Self as private::ManagerInternal>::Data as InMemoryData>::OriginalOperation,
        desc: <Self as private::ManagerInternal>::Desc,
    ) -> Result<
        <<Self as private::ManagerInternal>::Data as InMemoryData>::NewInfo,
        <<Self as private::ManagerInternal>::Data as InMemoryData>::Error,
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
    ) -> Result<(), <T::Data as InMemoryData>::Error> {
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
    use crate::test_utils::{FakeData, FakeError, FakeOp, rev_set};

    fn new_state(value: i64) -> AppState<FakeData, &'static str> {
        AppState::new(FakeData::new(value))
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
            Err(FakeError::ValueMismatch {
                expected: 5,
                found: 1
            })
        );
        assert_eq!(state.get_data().value, 0);
    }

    #[test]
    // The rollback "Failed to reverse" panic remains as the release-mode
    // safety net: in debug builds the canary below fires first during the
    // forward replay, so it has no direct debug-mode coverage.
    #[should_panic(expected = "stored backward op is inconsistent")]
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
    fn apply_failing_leaves_state_untouched() {
        let mut state = new_state(0);

        let result = state.apply(FakeOp::Fail, "never happens");

        assert_eq!(result, Err(FakeError::ApplyFailed));
        assert_eq!(state.get_data().value, 0);
        assert!(!state.can_undo());
        assert_eq!(state.get_last_op(), None);
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
}
