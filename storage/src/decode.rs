//! Decode submodule
//!
//! This module contains the logic that builds
//! a [Data] from a [json::JsonData].
//!
//! The main function for this is [self::decode]

use crate::json::*;

use super::*;

/// Error type when decoding a [json::JsonData]
///
/// This error type describes error that happen when interpreting the file content.
#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("Unknown file type - this might be from a more recent version of Collomatique")]
    UnknownFileType,
    #[error("An unknown entry requires a newer version of Collomatique")]
    UnknownNeededEntry,
    #[error("An entry has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry,
    #[error("An entry of type {0:?} is duplicated")]
    DuplicatedEntry(EntryTag),
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
pub enum Caveat {
    /// The file was opened but it was created with a newer version
    /// of Collomatique
    CreatedWithNewerVersion(Version),
    /// Unknown entries
    ///
    /// Some entries are unknown. They are maarked as unneeded,
    /// so the file can be decoded without them. But some information
    /// might be missing and it is preferable to use a newer version
    /// of Collomatique.
    UnknownEntries,
}

fn check_header(header: &Header, caveats: &mut BTreeSet<Caveat>) -> Result<(), DecodeError> {
    if let FileContent::UnknownFileContent(_value) = &header.file_content {
        return Err(DecodeError::UnknownFileType);
    }
    if header.produced_with_version > Version::current() {
        caveats.insert(Caveat::CreatedWithNewerVersion(
            header.produced_with_version.clone(),
        ));
    }
    Ok(())
}

fn check_entries_consistency(
    entries: &[Entry],
    caveats: &mut BTreeSet<Caveat>,
) -> Result<(), DecodeError> {
    let mut entries_found_so_far = BTreeSet::new();

    for entry in entries {
        match &entry.content {
            EntryContent::ValidEntry(valid_entry) => {
                if entry.minimum_spec_version != valid_entry.minimum_spec_version() {
                    return Err(DecodeError::MismatchedSpecRequirementInEntry);
                }
                if entry.needed_entry != valid_entry.needed_entry() {
                    return Err(DecodeError::MismatchedSpecRequirementInEntry);
                }
                let tag = EntryTag::from(valid_entry);
                if !entries_found_so_far.insert(tag) {
                    return Err(DecodeError::DuplicatedEntry(tag));
                }
            }
            EntryContent::UnknownEntry(_) => {
                if entry.minimum_spec_version <= CURRENT_SPEC_VERSION {
                    return Err(DecodeError::MismatchedSpecRequirementInEntry);
                }
                if entry.needed_entry {
                    return Err(DecodeError::UnknownNeededEntry);
                }
                caveats.insert(Caveat::UnknownEntries);
            }
        }
    }
    Ok(())
}

pub fn decode(json_data: &JsonData) -> Result<(Data, BTreeSet<Caveat>), DecodeError> {
    let mut caveats = BTreeSet::new();

    check_header(&json_data.header, &mut caveats)?;
    check_entries_consistency(&json_data.entries, &mut caveats)?;

    let data = decode_entries(&json_data.entries)?;
    Ok((data, caveats))
}

fn decode_entries(_entries: &[Entry]) -> Result<Data, DecodeError> {
    todo!()
}

/// Type of entries that can be found in a file
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryTag {
    StudentList,
}

impl From<&ValidEntry> for EntryTag {
    fn from(value: &ValidEntry) -> Self {
        match value {
            ValidEntry::StudentList(_) => EntryTag::StudentList,
        }
    }
}
