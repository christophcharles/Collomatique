//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ids::{ColloscopePeriodId, Id};

/// Description of the periods
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Periods<PeriodId: Id> {
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
    pub ordered_period_list: Vec<(PeriodId, Vec<bool>)>,
}

impl<PeriodId: Id> Default for Periods<PeriodId> {
    fn default() -> Self {
        Periods {
            first_week: None,
            ordered_period_list: vec![],
        }
    }
}

impl<PeriodId: Id> Periods<PeriodId> {
    /// Builds a period from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// However the only check is needed is that there are no duplicate ids
    pub(crate) unsafe fn from_external_data(external_data: PeriodsExternalData) -> Self {
        Periods {
            first_week: external_data.first_week,
            ordered_period_list: external_data
                .ordered_period_list
                .into_iter()
                .map(|(id, data)| (unsafe { PeriodId::new(id) }, data))
                .collect(),
        }
    }

    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list
            .iter()
            .position(|(current_id, _desc)| *current_id == id)
    }

    /// Finds a period by id
    pub fn find_period(&self, id: PeriodId) -> Option<&Vec<bool>> {
        let pos = self.find_period_position(id)?;

        Some(&self.ordered_period_list[pos].1)
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
    ) -> Option<Periods<ColloscopePeriodId>> {
        let mut ordered_period_list = vec![];

        for (period_id, period) in &self.ordered_period_list {
            let new_id = periods_map.get(period_id)?;
            ordered_period_list.push((*new_id, period.clone()));
        }

        Some(Periods {
            first_week: self.first_week.clone(),
            ordered_period_list,
        })
    }
}

/// Description of the periods but unchecked
///
/// This structure is an unchecked equivalent of [Periods].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PeriodsExternalData {
    /// Start date for the colloscope
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
    pub ordered_period_list: Vec<(u64, Vec<bool>)>,
}
