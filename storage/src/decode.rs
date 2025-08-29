//! Decode submodule
//!
//! This module contains the logic that builds
//! a [Data] from a [json::JsonData].
//!
//! The main function for this is [self::decode]

use super::*;
use crate::json::*;

/// Error type when decoding a [json::JsonData]
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
    #[error("An entry of type {0:?} is duplicated")]
    DuplicatedEntry(EntryTag),
    #[error("Duplicated ID found in file")]
    DuplicatedID,
    #[error("generating new IDs is not secure, half the usable IDs have been used already")]
    EndOfTheUniverse,
    #[error("start date for periods is invalid (should be a monday)")]
    InvalidStartDate,
    #[error("Invalid ID found in file")]
    InvalidId,
    #[error("Periods were already decoded from a previous block")]
    PeriodsAlreadyDecoded,
    #[error("Students were already decoded from a previous block")]
    StudentsAlreadyDecoded,
    #[error("Subjects were already decoded from a previous block")]
    SubjectsAlreadyDecoded,
    #[error("Teachers were already decoded from a previous block")]
    TeachersAlreadyDecoded,
    #[error("Student assignments data is inconsistent")]
    InconsistentAssignmentData,
    #[error("Subjects were decoded before periods")]
    SubjectsDecodedBeforePeriods,
    #[error("Assignments were decoded before periods")]
    AssignmentsDecodedBeforePeriods,
}

impl From<collomatique_state_colloscopes::FromDataError> for DecodeError {
    fn from(value: collomatique_state_colloscopes::FromDataError) -> Self {
        use collomatique_state::tools::IdError;
        use collomatique_state_colloscopes::FromDataError;
        match value {
            FromDataError::IdError(id_error) => match id_error {
                IdError::DuplicatedId => DecodeError::DuplicatedID,
                IdError::EndOfTheUniverse => DecodeError::EndOfTheUniverse,
                IdError::InvalidId => DecodeError::InvalidId,
            },
            FromDataError::InconsistentAssignments => DecodeError::InconsistentAssignmentData,
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
    let mut entries_found_so_far = BTreeSet::new();

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
                let tag = EntryTag::from(valid_entry);
                if !entries_found_so_far.insert(tag) {
                    return Err(DecodeError::DuplicatedEntry(tag));
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
    periods: collomatique_state_colloscopes::periods::PeriodsExternalData,
    subjects: collomatique_state_colloscopes::subjects::SubjectsExternalData,
    teachers: collomatique_state_colloscopes::teachers::TeachersExternalData,
    students: collomatique_state_colloscopes::students::StudentsExternalData,
    assignments: collomatique_state_colloscopes::assignments::AssignmentsExternalData,
}

mod period_list;
mod student_list;
mod subject_list;
mod teacher_list;

fn decode_entries(entries: Vec<Entry>) -> Result<Data, DecodeError> {
    let mut pre_data = PreData::default();

    for entry in entries {
        let EntryContent::ValidEntry(valid_entry) = entry.content else {
            continue;
        };

        match valid_entry {
            ValidEntry::StudentList(student_list) => {
                student_list::decode_entry(student_list, &mut pre_data)?;
            }
            ValidEntry::PeriodList(period_list) => {
                period_list::decode_entry(period_list, &mut pre_data)?;
            }
            ValidEntry::SubjectList(subject_list) => {
                subject_list::decode_entry(subject_list, &mut pre_data)?;
            }
            ValidEntry::TeacherList(teacher_list) => {
                teacher_list::decode_entry(teacher_list, &mut pre_data)?;
            }
        }
    }

    let data = Data::from_data(
        pre_data.periods,
        pre_data.subjects,
        pre_data.teachers,
        pre_data.students,
        pre_data.assignments,
    )?;
    Ok(data)
}

/// Type of entries that can be found in a file
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryTag {
    StudentList,
    PeriodList,
    SubjectList,
    TeacherList,
}

impl From<&ValidEntry> for EntryTag {
    fn from(value: &ValidEntry) -> Self {
        match value {
            ValidEntry::StudentList(_) => EntryTag::StudentList,
            ValidEntry::PeriodList(_) => EntryTag::PeriodList,
            ValidEntry::SubjectList(_) => EntryTag::SubjectList,
            ValidEntry::TeacherList(_) => EntryTag::TeacherList,
        }
    }
}
