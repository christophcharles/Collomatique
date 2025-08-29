//! Student list submodule
//!
//! This module contains the code for decoding StudentList entries

use super::*;

pub fn decode_entry(
    student_list: json::student_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    for (id, student) in student_list.map {
        if pre_data
            .student_list
            .insert(
                id,
                collomatique_state_colloscopes::PersonWithContacts {
                    firstname: student.firstname,
                    surname: student.surname,
                    tel: student.telephone,
                    email: student.email,
                },
            )
            .is_some()
        {
            return Err(DecodeError::DuplicatedID);
        }
    }
    Ok(())
}
