//! Slot list submodule
//!
//! This module contains the code for decoding SlotList entries

use super::*;

pub fn decode_entry(
    slot_list: json::slot_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    let mut ids = BTreeSet::new();

    for (subject_id, subject_slots) in &slot_list.subject_map {
        let Some(subject) = pre_data.subjects.find_subject(*subject_id) else {
            return Err(DecodeError::InconsistentSlotsData);
        };
        if subject.parameters.interrogation_parameters.is_none() {
            return Err(DecodeError::InconsistentSlotsData);
        }

        let pre_subject_slots = pre_data
            .slots
            .subject_map
            .get_mut(subject_id)
            .expect("Subject id should be valid at this point");

        if !pre_subject_slots.ordered_slots.is_empty() {
            return Err(DecodeError::InconsistentSlotsData);
        }

        for (id, slot) in &subject_slots.ordered_slot_list {
            if !ids.insert(id) {
                return Err(DecodeError::DuplicatedID);
            }
            pre_subject_slots
                .ordered_slots
                .push((*id, slot.clone().into()));
        }
    }

    Ok(())
}
