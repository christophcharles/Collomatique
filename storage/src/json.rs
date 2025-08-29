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
    produced_with_version: String,
    file_content: FileContent,
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
