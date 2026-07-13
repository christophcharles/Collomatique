//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{IncompatId, SlotId, WeekPatternId};

/// Description of the week patterns
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPatterns {
    /// Week patterns
    ///
    /// Each item associate to a single ID a sequence of weeks
    pub week_pattern_map: BTreeMap<WeekPatternId, WeekPattern>,
}

impl WeekPatterns {
    pub(crate) fn get_pattern(&self, week_pattern_id: WeekPatternId) -> Vec<bool> {
        self.week_pattern_map
            .get(&week_pattern_id)
            .expect("Week pattern id must be valid for get_pattern")
            .weeks
            .clone()
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

impl WeekPattern {
    pub fn add_weeks(&mut self, first_week: usize, week_count: usize) {
        assert!(self.weeks.len() >= first_week);

        self.weeks
            .splice(first_week..first_week, vec![true; week_count]);
    }

    pub fn clean_weeks(&mut self, first_week: usize, week_count: usize) {
        assert!(self.weeks.len() > first_week);

        let last_week = first_week + week_count;
        assert!(self.weeks.len() >= last_week);

        for week in &mut self.weeks[first_week..last_week] {
            *week = true;
        }
    }

    pub fn remove_weeks(&mut self, first_week: usize, week_count: usize) {
        assert!(self.weeks.len() > first_week);

        let last_week = first_week + week_count;
        assert!(self.weeks.len() >= last_week);

        for week in &self.weeks[first_week..last_week] {
            assert!(*week);
        }

        self.weeks.splice(first_week..last_week, vec![]);
    }

    pub fn can_remove_weeks(&self, first_week: usize, week_count: usize) -> bool {
        assert!(self.weeks.len() > first_week);

        let last_week = first_week + week_count;
        assert!(self.weeks.len() >= last_week);

        for week in &self.weeks[first_week..last_week] {
            if !*week {
                return false;
            }
        }

        true
    }
}

/// Errors for week pattern operations
///
/// These errors can be returned when trying to modify [crate::Data] with a week pattern op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPatternError {
    /// A week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// The week pattern id already exists
    #[error("week pattern id ({0:?}) already exists")]
    WeekPatternIdAlreadyExists(WeekPatternId),

    /// The week pattern is referenced by a slot
    #[error("week pattern id ({0:?}) is referenced by a slot ({1:?})")]
    WeekPatternStillHasAssociatedSlots(WeekPatternId, SlotId),

    /// The week pattern is referenced by a schedule incompatibility
    #[error("week pattern id ({0:?}) is referenced by an incompat ({1:?})")]
    WeekPatternStillHasAssociatedIncompat(WeekPatternId, IncompatId),

    /// The week pattern does not have the right length
    #[error("week pattern does not have the right length")]
    BadWeekPatternLength,

    /// The slot in colloscope is incompatible with the new week pattern
    #[error("slot {0:?} in colloscope is not compatible with the new week pattern")]
    NotCompatibleSlotInColloscope(SlotId),
}
