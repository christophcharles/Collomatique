//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{ColloscopeWeekPatternId, Id};

/// Description of the week patterns
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPatterns<WeekPatternId: Id> {
    /// Week patterns
    ///
    /// Each item associate to a single ID a sequence of weeks
    pub week_pattern_map: BTreeMap<WeekPatternId, WeekPattern>,
}

impl<WeekPatternId: Id> Default for WeekPatterns<WeekPatternId> {
    fn default() -> Self {
        WeekPatterns {
            week_pattern_map: BTreeMap::new(),
        }
    }
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

impl<WeekPatternId: Id> WeekPatterns<WeekPatternId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        week_patterns_map: &BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    ) -> Option<WeekPatterns<ColloscopeWeekPatternId>> {
        let mut week_pattern_map = BTreeMap::new();

        for (week_pattern_id, week_pattern) in &self.week_pattern_map {
            let new_id = week_patterns_map.get(week_pattern_id)?;
            week_pattern_map.insert(*new_id, week_pattern.clone());
        }

        Some(WeekPatterns { week_pattern_map })
    }
}
