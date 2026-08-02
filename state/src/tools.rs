//! Tools module
//!
//! This contains a few tools that are useful for defining specific [crate::InMemoryData].
//!
//! The main tool is [IdIssuerHelper] which helps building an Id issuer
//! for your specific use case.

use std::collections::BTreeSet;

use thiserror::Error;

/// Errors for IDs
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum IdError {
    /// Generating new IDs is not secure: half the usable IDs have been used already.
    ///
    /// This *should* not happen. If this happen, most probably a malicious
    /// file was opened.
    #[error("generating new IDs is not secure, half the usable IDs have been used already")]
    EndOfTheUniverse,
    /// Duplicated ID found
    #[error("duplicated ID found")]
    DuplicatedId,
}

/// Id issuer
///
/// This is a helper struct. It helps generate
/// new, unique ids every time we need one.
#[derive(Debug, Clone)]
pub struct IdIssuerHelper {
    next_available_id: u64,
}

/// Id type
///
/// This types ensures that the ID was
/// correctly generated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootId(u64);

impl RootId {
    /// Returns the value for the ID
    pub fn inner(&self) -> u64 {
        self.0
    }
}

impl IdIssuerHelper {
    /// Create a new IdIssuerHelper
    ///
    /// It takes an iterator on existing ID values
    pub fn new(
        existing_ids: impl Iterator<Item = u64>,
    ) -> std::result::Result<IdIssuerHelper, IdError> {
        let mut ids_found_so_far = BTreeSet::new();
        for id in existing_ids {
            if !ids_found_so_far.insert(id) {
                return Err(IdError::DuplicatedId);
            }
        }

        let next_available_id = match ids_found_so_far.last() {
            None => 0,
            Some(&val) => {
                if val > (u64::MAX >> 1) {
                    return Err(IdError::EndOfTheUniverse);
                } else {
                    val + 1
                }
            }
        };

        Ok(IdIssuerHelper { next_available_id })
    }

    /// Returns internal counter
    ///
    /// This is useful for invariant checks
    pub fn get_internal_counter(&self) -> u64 {
        self.next_available_id
    }

    /// Advance the counter to at least `next_id`
    ///
    /// If the current counter is already at or above `next_id`, this is a no-op.
    /// Returns an error if `next_id` is too large (EndOfTheUniverse).
    pub fn skip_to_id(&mut self, next_id: u64) -> Result<(), IdError> {
        if next_id > (u64::MAX >> 1) {
            return Err(IdError::EndOfTheUniverse);
        }
        if next_id > self.next_available_id {
            self.next_available_id = next_id;
        }
        Ok(())
    }

    /// Generates a new (untyped) id
    ///
    /// This function generates a new ID.
    ///
    /// There are no types for this id and it can
    /// easily be misued
    pub fn get_new_id(&mut self) -> RootId {
        let current_id = self.next_available_id;
        self.next_available_id += 1;
        RootId(current_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_from_empty_starts_at_zero() {
        let issuer = IdIssuerHelper::new(std::iter::empty()).expect("no duplicates");

        assert_eq!(issuer.get_internal_counter(), 0);
    }

    #[test]
    fn new_from_existing_ids_starts_after_the_largest() {
        let issuer = IdIssuerHelper::new([0, 5, 7].into_iter()).expect("no duplicates");

        assert_eq!(issuer.get_internal_counter(), 8);
    }

    #[test]
    fn new_rejects_duplicated_ids() {
        let result = IdIssuerHelper::new([0, 5, 5].into_iter());

        assert_eq!(result.err(), Some(IdError::DuplicatedId));
    }

    #[test]
    fn new_rejects_ids_beyond_half_the_id_space() {
        let result = IdIssuerHelper::new([(u64::MAX >> 1) + 1].into_iter());

        assert_eq!(result.err(), Some(IdError::EndOfTheUniverse));

        // The boundary value itself is still accepted
        let issuer = IdIssuerHelper::new([u64::MAX >> 1].into_iter()).expect("boundary is valid");
        assert_eq!(issuer.get_internal_counter(), (u64::MAX >> 1) + 1);
    }

    #[test]
    fn get_new_id_is_monotonic() {
        let mut issuer = IdIssuerHelper::new(std::iter::empty()).expect("no duplicates");

        assert_eq!(issuer.get_new_id(), RootId(0));
        assert_eq!(issuer.get_new_id(), RootId(1));
        assert_eq!(issuer.get_new_id(), RootId(2));
        assert_eq!(issuer.get_internal_counter(), 3);
    }

    #[test]
    fn skip_to_id_advances_but_never_goes_back() {
        let mut issuer = IdIssuerHelper::new([0, 1, 2].into_iter()).expect("no duplicates");
        assert_eq!(issuer.get_internal_counter(), 3);

        issuer.skip_to_id(10).expect("valid id");
        assert_eq!(issuer.get_internal_counter(), 10);

        // Skipping behind the counter is a no-op
        issuer.skip_to_id(5).expect("valid id");
        assert_eq!(issuer.get_internal_counter(), 10);

        assert_eq!(issuer.get_new_id(), RootId(10));
    }

    #[test]
    fn skip_to_id_rejects_ids_beyond_half_the_id_space() {
        let mut issuer = IdIssuerHelper::new(std::iter::empty()).expect("no duplicates");

        let result = issuer.skip_to_id((u64::MAX >> 1) + 1);

        assert_eq!(result, Err(IdError::EndOfTheUniverse));
        assert_eq!(issuer.get_internal_counter(), 0);
    }
}
