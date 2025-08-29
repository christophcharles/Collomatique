//! Tools module
//!
//! This contains a few tools that are useful for defining specific [crate::InMemoryData].
//!
//! The main tool is [IdIssuerHelper] which helps building an Id issuer
//! for your specific use case.

use std::sync::atomic::{AtomicU64, Ordering};

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
}

/// Id issuer
///
/// This is a helper struct. It helps generate
/// new, unique ids every time we need one.
#[derive(Debug)]
pub struct IdIssuerHelper {
    next_available_id: AtomicU64,
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
    /// It takes an optional biggest_id_so_far to resume generation
    /// from an existing id.
    pub fn new(biggest_id_so_far: Option<u64>) -> std::result::Result<IdIssuerHelper, IdError> {
        let next_available_id = AtomicU64::new(match biggest_id_so_far {
            None => 0,
            Some(val) => {
                if val > (u64::MAX >> 1) {
                    return Err(IdError::EndOfTheUniverse);
                } else {
                    val + 1
                }
            }
        });

        Ok(IdIssuerHelper { next_available_id })
    }

    /// Generates a new (untyped) id
    ///
    /// This function generates a new ID.
    ///
    /// There are no types for this id and it can
    /// easily be misued
    pub fn get_new_id(&self) -> RootId {
        RootId(self.next_available_id.fetch_add(1, Ordering::Relaxed))
    }
}
