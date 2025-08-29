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

pub mod common;
pub mod student_list;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryContent {
    StudentList(student_list::List),
    #[serde(other)]
    UnknownEntry,
}

pub const CURRENT_SPEC_VERSION: u32 = 1;

impl EntryContent {
    pub fn minimum_spec_version(&self) -> u32 {
        match self {
            EntryContent::StudentList(_) => 1,
            EntryContent::UnknownEntry => 1,
        }
    }

    pub fn needed_entry(&self) -> bool {
        match self {
            EntryContent::StudentList(_) => true,
            EntryContent::UnknownEntry => false,
        }
    }
}
