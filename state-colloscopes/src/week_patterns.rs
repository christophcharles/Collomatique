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

impl WeekPattern {
    pub fn add_weeks(&mut self, first_week: usize, week_count: usize) {
        if self.weeks.len() <= first_week {
            return;
        }

        self.weeks
            .splice(first_week..first_week, vec![true; week_count]);
    }

    pub fn remove_weeks(&mut self, first_week: usize, week_count: usize) {
        if self.weeks.len() <= first_week {
            return;
        }

        let last_week = (first_week + week_count).min(self.weeks.len());

        for week in &self.weeks[first_week..last_week] {
            assert!(!*week);
        }

        self.weeks.splice(first_week..last_week, vec![]);
    }

    pub fn can_remove_weeks(&self, first_week: usize, week_count: usize) -> bool {
        if self.weeks.len() <= first_week {
            return true;
        }

        let last_week = (first_week + week_count).min(self.weeks.len());

        for week in &self.weeks[first_week..last_week] {
            if !*week {
                return false;
            }
        }

        true
    }
}
