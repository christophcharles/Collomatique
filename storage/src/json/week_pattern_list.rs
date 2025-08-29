//! week patterns submodule
//!
//! This module defines the week patterns entry for the JSON description
//!
use super::*;

use std::collections::BTreeMap;

/// JSON desc of week patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between ids and week patterns
    ///
    /// each week pattern is described by an id (which should not
    /// be duplicate) and a structure [WeekPattern]
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub week_pattern_map: BTreeMap<u64, WeekPattern>,
}

/// JSON desc of a single week pattern
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPattern {
    pub name: String,
    pub weeks: Vec<bool>,
}

impl From<&collomatique_state_colloscopes::week_patterns::WeekPattern> for WeekPattern {
    fn from(value: &collomatique_state_colloscopes::week_patterns::WeekPattern) -> Self {
        WeekPattern {
            name: value.name.clone(),
            weeks: value.weeks.clone(),
        }
    }
}

impl From<WeekPattern> for collomatique_state_colloscopes::week_patterns::WeekPattern {
    fn from(value: WeekPattern) -> Self {
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: value.name,
            weeks: value.weeks,
        }
    }
}
