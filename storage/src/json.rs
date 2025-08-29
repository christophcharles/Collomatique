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
    header: Header,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    file_type: FileType,
    produced_with_version: Version,
    file_content: FileContent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
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
pub enum FileContent {
    Colloscope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {}
