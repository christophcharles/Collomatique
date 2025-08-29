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
