//! Traits module
//!
//! This module defines the various traits that an in-memory
//! data representation should have.

/// This trait represents an operation (annotated or not)
pub trait Operation: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq {}

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
    fn annotate(
        &mut self,
        op: Self::OriginalOperation,
    ) -> (Self::AnnotatedOperation, Self::NewInfo);

    /// Build the reverse of an operation
    ///
    /// Build the reverse of an operation from the current state.
    /// This function should return the reversed operation of
    /// the operation given as a parameter *if* it was applied to
    /// the current state of the data.
    ///
    /// It can fail as it might be non-sensical to apply the given
    /// operation.
    fn build_rev_with_current_state(
        &self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error>;

    /// Apply an operation to the data
    ///
    /// In case of failure, it can return the error type [Self::Error].
    fn apply(&mut self, op: &Self::AnnotatedOperation) -> std::result::Result<(), Self::Error>;
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
        name: String,
    ) -> Result<
        <<Self as private::ManagerInternal>::Data as InMemoryData>::NewInfo,
        <<Self as private::ManagerInternal>::Data as InMemoryData>::Error,
    > {
        let (annotated_op, new_info) = self.get_in_memory_data_mut().annotate(op);

        let reverse_operation = self
            .get_in_memory_data()
            .build_rev_with_current_state(&annotated_op)?;
        let rev_op = crate::history::ReversibleOp {
            forward: annotated_op,
            backward: reverse_operation,
        };

        self.get_in_memory_data_mut().apply(&rev_op.forward)?;

        let aggregated_op = crate::history::AggregatedOp::new(vec![rev_op]);
        self.get_modification_history_mut()
            .store(aggregated_op, name);

        Ok(new_info)
    }

    /// Returns the name of the last operation if it exists
    fn get_undo_name(&self) -> Option<&str> {
        self.get_modification_history().get_undo_name()
    }

    /// Returns the name of the next operation if it exists
    fn get_redo_name(&self) -> Option<&str> {
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
                    panic!("Data should be consistent as it was automatically build from previous state.\n{}", e);
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
                    panic!("Data should be consistent as it was automatically build from previous state.\n{}", e);
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
    pub fn update_internal_state_with_aggregated<T: ManagerInternal + ?Sized>(
        manager: &mut T,
        aggregated_op: &crate::history::AggregatedOp<<T::Data as InMemoryData>::AnnotatedOperation>,
    ) -> Result<(), <T::Data as InMemoryData>::Error> {
        let ops = aggregated_op.inner();

        let mut error = None;
        let mut count = 0;

        for rev_op in ops {
            let result = manager.get_in_memory_data_mut().apply(&rev_op.forward);

            if let Err(err) = result {
                error = Some(err);
                break;
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

        /// Return a mutable reference to the in-memory data
        fn get_in_memory_data_mut(&mut self) -> &mut Self::Data;
        /// Return a mutable reference to the modification history
        fn get_modification_history_mut(
            &mut self,
        ) -> &mut crate::history::ModificationHistory<
            <Self::Data as InMemoryData>::AnnotatedOperation,
        >;

        /// Return a non-mutable reference to the in-memory data
        fn get_in_memory_data(&self) -> &Self::Data;
        /// Return a non-mutable reference to the modification history
        fn get_modification_history(
            &self,
        ) -> &crate::history::ModificationHistory<<Self::Data as InMemoryData>::AnnotatedOperation>;
    }
}
