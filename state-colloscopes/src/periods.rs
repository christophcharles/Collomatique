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

    /// Finds the position of a period by id and gives the number of the first week
    pub fn find_period_position_and_first_week(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (pos, (period_id, desc)) in self.ordered_period_list.iter().enumerate() {
            if *period_id == id {
                return Some((pos, first_week));
            }
            first_week += desc.len();
        }

        return None;
    }

    /// Finds the position of a period by id and gives the total number of weeks up to and including the
    /// given period
    pub fn find_period_position_and_total_number_of_weeks(
        &self,
        id: PeriodId,
    ) -> Option<(usize, usize)> {
        let mut total_weeks = 0usize;

        for (pos, (period_id, desc)) in self.ordered_period_list.iter().enumerate() {
            total_weeks += desc.len();
            if *period_id == id {
                return Some((pos, total_weeks));
            }
        }

        return None;
    }

    /// Finds a period by id
    pub fn find_period(&self, id: PeriodId) -> Option<&Vec<WeekDesc>> {
        let pos = self.find_period_position(id)?;

        Some(&self.ordered_period_list[pos].1)
    }

    /// Finds the first week number and the length of a period
    pub fn get_first_week_and_length_for_period(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (period_id, desc) in &self.ordered_period_list {
            if *period_id == id {
                return Some((first_week, desc.len()));
            }
            first_week += desc.len();
        }

        return None;
    }
}
