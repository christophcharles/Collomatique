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
/// It is built upon by the core crate to have
/// a consistent modification history, sessions, etc.
pub trait InMemoryData: Send + Sync + std::fmt::Debug {
    /// Non-annotated type for the operations
    type OriginalOperation: Operation;

    /// Annotated type for the operations
    ///
    /// Possibly this can be the same as [Self::OriginalOperation]
    /// if the original operation is indeed complete.
    type AnnotatedOperation: Operation;

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
    fn annotate(&self, op: Self::OriginalOperation) -> Self::AnnotatedOperation;

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
        op: Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error>;

    /// Apply an operation to the data
    ///
    /// In case of failure, it can return the error type [Self::Error].
    fn apply(&mut self, op: Self::AnnotatedOperation) -> std::result::Result<(), Self::Error>;
}
