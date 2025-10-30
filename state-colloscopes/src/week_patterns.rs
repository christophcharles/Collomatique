//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::WeekPatternId;

/// Description of the week patterns
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPatterns {
    /// Week patterns
    ///
    /// Each item associate to a single ID a sequence of weeks
    pub week_pattern_map: BTreeMap<WeekPatternId, WeekPattern>,
}

/// Description of a week pattern
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPattern {
    /// Name of the week pattern for identification
    pub name: String,
    /// Weeks the interrogation happens on
    ///
    /// If the Vec is shorter than the total amount of weeks
    /// it is assumed the interrogation happens on all the
    /// remaining weeks.
    ///
    /// If the Vec is longer, the extra weeks are ignored
    /// They are kept in case some one expands again the number of weeks.
    pub weeks: Vec<bool>,
}
