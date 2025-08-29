//! Student_list submodule
//!
//! This module defines the student list entry for the JSON description
//!
use super::*;

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    pub map: BTreeMap<u64, common::PersonWithContact>,
}
