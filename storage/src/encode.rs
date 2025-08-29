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
        file_content: FileContent::ValidFileContent(ValidFileContent::Colloscope),
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

fn generate_period_list(data: &Data) -> period_list::List {
    let orig_periods = data.get_periods();

    period_list::List {
        first_week: orig_periods.first_week.as_ref().map(|x| x.inner().clone()),
        ordered_period_list: orig_periods
            .ordered_period_list
            .iter()
            .map(|(id, desc)| (id.inner(), desc.clone()))
            .collect(),
    }
}

fn generate_subject_list(data: &Data) -> subject_list::List {
    let orig_subjects = data.get_subjects();

    subject_list::List {
        ordered_subject_list: orig_subjects
            .ordered_subject_list
            .iter()
            .map(|(id, desc)| (id.inner(), desc.into()))
            .collect(),
    }
}

pub fn encode(data: &Data) -> JsonData {
    let header = generate_header();
    let student_list_entry = ValidEntry::StudentList(generate_student_list(data));
    let period_list_entry = ValidEntry::PeriodList(generate_period_list(data));
    let subject_list_entry = ValidEntry::SubjectList(generate_subject_list(data));

    JsonData {
        header,
        entries: vec![student_list_entry, period_list_entry, subject_list_entry]
            .into_iter()
            .map(|x| Entry {
                minimum_spec_version: x.minimum_spec_version(),
                needed_entry: x.needed_entry(),
                content: EntryContent::ValidEntry(x),
            })
            .collect(),
    }
}
