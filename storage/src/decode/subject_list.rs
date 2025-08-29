//! Subject list submodule
//!
//! This module contains the code for decoding SubjectList entries

use super::*;

pub fn decode_entry(
    subject_list: json::subject_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    assert!(pre_data.subjects.ordered_subject_list.is_empty());

    let mut ids = BTreeSet::new();
    for (id, subject) in subject_list.ordered_subject_list {
        if !ids.insert(id) {
            return Err(DecodeError::DuplicatedID);
        }
        pre_data
            .subjects
            .ordered_subject_list
            .push((id, subject.into()));
    }

    Ok(())
}
