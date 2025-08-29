//! Teacher list submodule
//!
//! This module contains the code for decoding TeacherList entries

use super::*;

pub fn decode_entry(
    teacher_list: json::teacher_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    assert!(pre_data.teachers.teacher_map.is_empty());

    let mut ids = BTreeSet::new();
    for (id, teacher) in teacher_list.teacher_map {
        if !ids.insert(id) {
            return Err(DecodeError::DuplicatedID);
        }
        pre_data.teachers.teacher_map.insert(id, teacher.into());
    }

    Ok(())
}
