//! Encode submodule
//!
//! This module contains the logic that builds
//! a [json::JsonData] from a [Data].
//!
//! The main function for this is [self::encode]

use super::*;
use json::*;

fn generate_header() -> Header {
    Header {
        file_type: FileType::Collomatique,
        produced_with_version: Version::current(),
        file_content: FileContent::Colloscope,
    }
}

fn generate_student_list(data: &Data) -> student_list::List {
    let orig_student_list = data.get_student_list();

    student_list::List {
        map: orig_student_list
            .into_iter()
            .map(|(id, person)| {
                (
                    id.inner(),
                    common::PersonWithContact {
                        firstname: person.firstname.clone(),
                        surname: person.surname.clone(),
                        telephone: person.tel.clone(),
                        email: person.email.clone(),
                    },
                )
            })
            .collect(),
    }
}

pub fn encode(data: &Data) -> JsonData {
    let header = generate_header();
    let student_list_entry = ValidEntry::StudentList(generate_student_list(data));

    JsonData {
        header,
        entries: vec![Entry {
            minimum_spec_version: student_list_entry.minimum_spec_version(),
            needed_entry: student_list_entry.needed_entry(),
            content: EntryContent::ValidEntry(student_list_entry),
        }],
    }
}
