//! Decode submodule
//!
//! This module contains the logic that builds
//! a [Data] from a [json::JsonData].
//!
//! The main function for this is [self::decode]

use super::*;

/// Error type when decoding a [json::JsonData]
///
/// This error type describes error that happen when interpreting the file content.
#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("An unknown entry requires a newer version of Collomatique")]
    UnknownNeededEntry,
    #[error("A known entry has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry,
}

/// Caveats type
///
/// A file can be successfully decoded though not all information was
/// decoded successfully. This can happen for instance if we try to
/// open a file from a more recent version of Collomatique that has
/// some extra structures.
///
/// This type enumerates possible caveats that were encountered while decoding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Caveats {
    /// The file was opened but it was created with a newer version
    /// of Collomatique
    CreatedWithNewerVersion(json::Version),
    /// Unknown entries
    ///
    /// Some entries are unknown. They are maarked as unneeded,
    /// so the file can be decoded without them. But some information
    /// might be missing and it is preferable to use a newer version
    /// of Collomatique.
    UnknownEntries,
}

pub fn decode(_json_data: &json::JsonData) -> Result<(Data, Vec<Caveats>), DecodeError> {
    todo!()
}
