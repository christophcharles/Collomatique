//! Student list submodule
//!
//! This module contains the code for decoding StudentList entries

use std::collections::BTreeMap;

use collomatique_state_colloscopes::assignments::PeriodAssignmentsExternalData;

use super::*;

pub fn decode_entry(
    period_list: json::period_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    pre_data.periods.first_week = match period_list.first_week {
        Some(date) => {
            let monday_date = collomatique_time::NaiveMondayDate::new(date);
            if monday_date.is_none() {
                return Err(DecodeError::InvalidStartDate);
            }
            monday_date
        }
        None => None,
    };

    if !pre_data.periods.ordered_period_list.is_empty() {
        return Err(DecodeError::PeriodsAlreadyDecoded);
    }
    if !pre_data.subjects.ordered_subject_list.is_empty() {
        return Err(DecodeError::SubjectsDecodedBeforePeriods);
    }
    if !pre_data.assignments.period_map.is_empty() {
        return Err(DecodeError::AssignmentsDecodedBeforePeriods);
    }
    let mut ids = BTreeSet::new();
    for (id, desc) in period_list.ordered_period_list {
        if !ids.insert(id) {
            return Err(DecodeError::DuplicatedID);
        }
        pre_data.periods.ordered_period_list.push((id, desc));
        pre_data.assignments.period_map.insert(
            id,
            PeriodAssignmentsExternalData {
                subject_exclusion_map: BTreeMap::new(),
            },
        );
    }

    Ok(())
}
