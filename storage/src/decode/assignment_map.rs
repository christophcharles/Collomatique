//! Assignment map submodule
//!
//! This module contains the code for decoding AssignmentMap entries

use super::*;

pub fn decode_entry(
    assignment_map: json::assignment_map::Map,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    for (period_id, _) in &pre_data.periods.ordered_period_list {
        let Some(period_assignments) = assignment_map.period_map.get(period_id) else {
            return Err(DecodeError::InconsistentAssignmentData);
        };

        let period_assignments_data = pre_data.assignments.period_map.get_mut(period_id)
            .expect("pre_data should always be consistent and therefore period assignment should exist for all periods");

        for (subject_id, subject_assignments) in &period_assignments.subject_map {
            let Some(assigned_students) = period_assignments_data.subject_map.get_mut(subject_id)
            else {
                return Err(DecodeError::InconsistentAssignmentData);
            };

            assigned_students.extend(subject_assignments.assigned_students.clone());
        }
    }

    Ok(())
}
