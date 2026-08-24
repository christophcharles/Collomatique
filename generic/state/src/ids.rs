//! IDs module
//!
//! This module defines the generic [Id] trait implemented by
//! all typed ID newtypes used in [crate::InMemoryData] implementations.

/// Trait for typed IDs
///
/// A typed ID is a lightweight, copiable handle (wrapping a `u64`)
/// that identifies an object of a given kind.
pub trait Id:
    Clone
    + Copy
    + std::fmt::Debug
    + Ord
    + PartialOrd
    + Eq
    + PartialEq
    + std::hash::Hash
    + Send
    + Sync
    + 'static
{
    /// Returns the value for the ID
    fn inner(&self) -> u64;
    /// Builds a new ID from u64
    ///
    /// # Safety
    ///
    /// This is unsafe as invariants should be checked first (to avoid duplicated ids)
    ///
    /// `value` should be a valid ID. If not, you might get inconsistent data.
    /// Collomatique assumes consistent data everywhere. Generally, you should not
    /// call this function directly
    unsafe fn new(value: u64) -> Self;
}
