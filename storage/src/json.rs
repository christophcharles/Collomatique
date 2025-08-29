//! Json submodule
//!
//! This module defines the various types matching the JSON representation
//! of [collomatique_state_colloscopes::Data].
//!
//! If a file is correctly formatted, it should normally be representable as
//! a [JsonData].
//!

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonData {
    pub header: Header,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub file_type: FileType,
    pub produced_with_version: Version,
    pub file_content: FileContent,
}

/// Represents a semantic version number
///
/// A semantic version number is structure as MAJOR.MINOR.PATCH
/// as given by th various members
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Version {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    /// Returns the version number of the compiled Collomatique package
    pub fn current() -> Version {
        let major_str = env!("CARGO_PKG_VERSION_MAJOR");
        let minor_str = env!("CARGO_PKG_VERSION_MINOR");
        let patch_str = env!("CARGO_PKG_VERSION_PATCH");

        use std::str::FromStr;
        let major = u32::from_str(major_str).expect("Major version should be a valid u32");
        let minor = u32::from_str(minor_str).expect("Minor version should be a valid u32");
        let patch = u32::from_str(patch_str).expect("Patch number should be a valid u32");

        Version {
            major,
            minor,
            patch,
        }
    }

    pub fn new(major: u32, minor: u32, patch: u32) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Collomatique,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileContent {
    ValidFileContent(ValidFileContent),
    UnknownFileContent(serde_json::Value),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidFileContent {
    Colloscope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub minimum_spec_version: u32,
    pub needed_entry: bool,
    pub content: EntryContent,
}

pub mod assignment_map;
pub mod common;
pub mod period_list;
pub mod student_list;
pub mod subject_list;
pub mod teacher_list;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum EntryContent {
    ValidEntry(ValidEntry),
    UnknownEntry,
}

impl<'de> Deserialize<'de> for EntryContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Box<serde_json::value::RawValue> = Deserialize::deserialize(deserializer)?;

        use serde::de::IntoDeserializer;
        match ValidEntry::deserialize(value.into_deserializer()) {
            Ok(valid_entry) => Ok(EntryContent::ValidEntry(valid_entry)),
            Err(_) => Ok(EntryContent::UnknownEntry),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidEntry {
    StudentList(student_list::List),
    PeriodList(period_list::List),
    SubjectList(subject_list::List),
    TeacherList(teacher_list::List),
    AssignmentMap(assignment_map::Map),
}

pub const CURRENT_SPEC_VERSION: u32 = 1;

impl ValidEntry {
    pub fn minimum_spec_version(&self) -> u32 {
        match self {
            ValidEntry::StudentList(_) => 1,
            ValidEntry::PeriodList(_) => 1,
            ValidEntry::SubjectList(_) => 1,
            ValidEntry::TeacherList(_) => 1,
            ValidEntry::AssignmentMap(_) => 1,
        }
    }

    pub fn needed_entry(&self) -> bool {
        match self {
            ValidEntry::StudentList(_) => true,
            ValidEntry::PeriodList(_) => true,
            ValidEntry::SubjectList(_) => true,
            ValidEntry::TeacherList(_) => true,
            ValidEntry::AssignmentMap(_) => true,
        }
    }
}
