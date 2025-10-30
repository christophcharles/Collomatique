//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use serde::{Deserialize, Serialize};

use crate::ids::PeriodId;

/// Description of the periods
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Periods {
    /// Start date for the colloscope
    ///
    /// The date might not be set but of course, this will hinder
    /// the eventual pretty output
    pub first_week: Option<collomatique_time::NaiveMondayDate>,

    /// Ordered list of periods
    ///
    /// This field gives the relative order of the different
    /// periods identified by their ids
    ///
    /// For each period, we get also a list of boolean
    /// Each boolean represents a week. If it is true
    /// there is an interrogation on the given week
    /// otherwise there isn't.
    pub ordered_period_list: Vec<(PeriodId, Vec<WeekDesc>)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekDesc {
    pub interrogations: bool,
    pub annotation: Option<non_empty_string::NonEmptyString>,
}

impl Default for WeekDesc {
    fn default() -> Self {
        WeekDesc {
            interrogations: true,
            annotation: None,
        }
    }
}

impl WeekDesc {
    pub fn new(interrogations: bool) -> WeekDesc {
        WeekDesc {
            interrogations,
            annotation: None,
        }
    }
}

impl Periods {
    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list
            .iter()
            .position(|(current_id, _desc)| *current_id == id)
    }

    /// Finds a period by id
    pub fn find_period(&self, id: PeriodId) -> Option<&Vec<WeekDesc>> {
        let pos = self.find_period_position(id)?;

        Some(&self.ordered_period_list[pos].1)
    }
}
