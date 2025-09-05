//! Student list submodule
//!
//! This module contains the code for decoding StudentList entries

use super::*;

pub fn decode_entry(
    student_list: json::student_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    if !pre_data.main_params.students.student_map.is_empty() {
        return Err(DecodeError::StudentsAlreadyDecoded);
    }

    let mut ids = BTreeSet::new();
    for (id, student) in student_list.student_map {
        if !ids.insert(id) {
            return Err(DecodeError::DuplicatedID);
        }
        pre_data
            .main_params
            .students
            .student_map
            .insert(id, student.into());
    }

    Ok(())
}
