//! Json submodule
//!
//! This module defines the various types matching the JSON representation
//! of [collomatique_state_colloscopes::InnerData].
//!
//! Reading goes through [RawJsonData], whose entry payloads stay raw so
//! that the spec-version check and the block-name tolerance rules can run
//! before payload interpretation. Writing goes through [Spec2Document].
//!

use serde::{Deserialize, Serialize};

/// Raw envelope used for the spec-version check
///
/// The entry payloads are kept unparsed (as [serde_json::value::RawValue]) so
/// that a file can be routed to the right decoding pipeline — legacy (spec 1)
/// or spec 2 — based only on the declared `minimum_spec_version` values,
/// before any payload interpretation happens.
///
/// The envelope structs are records in the sense of the spec (§2-§3):
/// every field is always present and an unknown field makes the document
/// invalid, hence `deny_unknown_fields` on each of them. (It is
/// compatible with the raw `content` payload: that is a named field, not
/// a `flatten`.)
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawJsonData {
    pub header: Header,
    pub entries: Vec<RawEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawEntry {
    pub minimum_spec_version: u32,
    pub needed_entry: bool,
    pub content: Box<serde_json::value::RawValue>,
}

/// Serialize-only envelope for spec-2 documents
///
/// Reading goes through [RawJsonData] instead: the tolerance rules for
/// unknown block names require keeping the entry payloads raw.
#[derive(Debug, Serialize)]
pub struct Spec2Document {
    pub header: Header,
    pub entries: Vec<Spec2Entry>,
}

#[derive(Debug, Serialize)]
pub struct Spec2Entry {
    pub minimum_spec_version: u32,
    pub needed_entry: bool,
    /// External tagging emits the spec encoding: an object with exactly
    /// one key, the block name
    pub content: crate::format::Block,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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

/// The `file_type` discriminant
///
/// An unrecognized value parses into [FileType::UnknownFileType] rather
/// than failing serde, so that the header check can report it as an
/// unknown file type instead of a generic malformed-JSON error.
/// Serialization is transparent: a [ValidFileType] emits exactly its own
/// encoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileType {
    ValidFileType(ValidFileType),
    UnknownFileType(serde_json::Value),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidFileType {
    Collomatique,
}

/// The `file_content` discriminant, same shape as [FileType]
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

pub const CURRENT_SPEC_VERSION: u32 = 2;
