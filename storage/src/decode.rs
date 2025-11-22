//! Decode submodule
//!
//! This module contains the logic that builds
//! a [Data] from a [json::JsonData].
//!
//! The main function for this is [self::decode]

use super::*;
use crate::json::*;

/// Error type when decoding the JSON structure
///
/// This error type describes error that happen when interpreting the file content.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("Unknown file type - this might be from a more recent version of Collomatique")]
    UnknownFileType(Version),
    #[error("An unknown entry requires a newer version of Collomatique")]
    UnknownNeededEntry(Version),
    #[error("An entry has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry,
    #[error("An entry is probably ill-formed (and thus not recognized)")]
    ProbablyIllformedEntry,
    #[error("generating new IDs is not secure, half the usable IDs have been used already")]
    EndOfTheUniverse,
    #[error("Duplicated ID")]
    DuplicatedID,
    #[error("InnerDataDump entry should only be used on non-modified inner-data")]
    InnerDataDumpUsedOnModifiedInnerData,
    #[error(transparent)]
    InnerDataError(#[from] collomatique_state_colloscopes::InnerDataError),
}

impl From<collomatique_state_colloscopes::FromInnerDataError> for DecodeError {
    fn from(value: collomatique_state_colloscopes::FromInnerDataError) -> Self {
        use collomatique_state::tools::IdError;
        use collomatique_state_colloscopes::FromInnerDataError;
        match value {
            FromInnerDataError::IdError(id_error) => match id_error {
                IdError::DuplicatedId => DecodeError::DuplicatedID,
                IdError::EndOfTheUniverse => DecodeError::EndOfTheUniverse,
            },
            FromInnerDataError::InnerDataError(inner_data_error) => inner_data_error.into(),
        }
    }
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
        return Err(DecodeError::UnknownFileType(
            header.produced_with_version.clone(),
        ));
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
    version: &Version,
) -> Result<(), DecodeError> {
    for entry in entries {
        match &entry.content {
            EntryContent::UnknownEntry => {
                if entry.minimum_spec_version <= CURRENT_SPEC_VERSION {
                    return Err(DecodeError::ProbablyIllformedEntry);
                }
                if entry.needed_entry {
                    return Err(DecodeError::UnknownNeededEntry(version.clone()));
                }
                caveats.insert(Caveat::UnknownEntries);
            }
            EntryContent::ValidEntry(valid_entry) => {
                if entry.minimum_spec_version != valid_entry.minimum_spec_version() {
                    return Err(DecodeError::MismatchedSpecRequirementInEntry);
                }
                if entry.needed_entry != valid_entry.needed_entry() {
                    return Err(DecodeError::MismatchedSpecRequirementInEntry);
                }
            }
        }
    }
    Ok(())
}

pub fn decode(json_data: JsonData) -> Result<(Data, BTreeSet<Caveat>), DecodeError> {
    let mut caveats = BTreeSet::new();

    check_header(&json_data.header, &mut caveats)?;
    check_entries_consistency(
        &json_data.entries,
        &mut caveats,
        &json_data.header.produced_with_version,
    )?;

    let data = decode_entries(json_data.entries)?;
    Ok((data, caveats))
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct PreData {
    inner_data: collomatique_state_colloscopes::InnerData,
}

mod inner_data_dump;

fn decode_entries(entries: Vec<Entry>) -> Result<Data, DecodeError> {
    let mut pre_data = PreData::default();

    for entry in entries {
        let EntryContent::ValidEntry(valid_entry) = entry.content else {
            continue;
        };

        match valid_entry {
            ValidEntry::InnerDataDump(inner_data) => {
                inner_data_dump::decode_entry(inner_data, &mut pre_data)?;
            }
        }
    }

    let data = Data::from_inner_data(pre_data.inner_data)?;
    Ok(data)
}
