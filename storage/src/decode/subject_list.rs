//! Subject list submodule
//!
//! This module contains the code for decoding SubjectList entries

use super::*;

pub fn decode_entry(
    subject_list: json::subject_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    if !pre_data
        .main_params
        .subjects
        .ordered_subject_list
        .is_empty()
    {
        return Err(DecodeError::SubjectsAlreadyDecoded);
    }
    if !pre_data.main_params.slots.subject_map.is_empty() {
        return Err(DecodeError::SlotsDecodedBeforeSubjects);
    }

    let mut ids = BTreeSet::new();
    for (id, subject) in subject_list.ordered_subject_list {
        if !ids.insert(id) {
            return Err(DecodeError::DuplicatedID);
        }

        for (period_id, _) in &pre_data.main_params.periods.ordered_period_list {
            if subject.excluded_periods.contains(period_id) {
                continue;
            }
            let period_assignment = pre_data
                .main_params
                .assignments
                .period_map
                .get_mut(period_id)
                .expect("Period ids should be consistent even in pre_data");

            if period_assignment.subject_map.contains_key(&id) {
                panic!("Subject {} should not be present in pre_data", id);
            }

            period_assignment.subject_map.insert(id, BTreeSet::new());
        }

        if subject.parameters.interrogation_parameters.is_some() {
            pre_data.main_params.slots.subject_map.insert(
                id,
                collomatique_state_colloscopes::slots::SubjectSlotsExternalData {
                    ordered_slots: vec![],
                },
            );
        }

        pre_data
            .main_params
            .subjects
            .ordered_subject_list
            .push((id, subject.into()));
    }

    Ok(())
}
