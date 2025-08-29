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
    let orig_students = data.get_students();

    student_list::List {
        student_map: orig_students
            .student_map
            .iter()
            .map(|(id, student)| (id.inner(), student.into()))
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

fn generate_teacher_list(data: &Data) -> teacher_list::List {
    let orig_teachers = data.get_teachers();

    teacher_list::List {
        teacher_map: orig_teachers
            .teacher_map
            .iter()
            .map(|(id, teacher)| (id.inner(), teacher.into()))
            .collect(),
    }
}

fn generate_assignment_map(data: &Data) -> assignment_map::Map {
    let orig_assignments = data.get_assignments();

    assignment_map::Map {
        period_map: orig_assignments
            .period_map
            .iter()
            .map(|(id, period_assignments)| (id.inner(), period_assignments.into()))
            .collect(),
    }
}

pub fn encode(data: &Data) -> JsonData {
    let header = generate_header();
    let period_list_entry = ValidEntry::PeriodList(generate_period_list(data));
    let subject_list_entry = ValidEntry::SubjectList(generate_subject_list(data));
    let teacher_list_entry = ValidEntry::TeacherList(generate_teacher_list(data));
    let student_list_entry = ValidEntry::StudentList(generate_student_list(data));
    let assignment_map_entry = ValidEntry::AssignmentMap(generate_assignment_map(data));

    JsonData {
        header,
        entries: vec![
            period_list_entry,
            subject_list_entry,
            teacher_list_entry,
            student_list_entry,
            assignment_map_entry,
        ]
        .into_iter()
        .map(|x| Entry {
            minimum_spec_version: x.minimum_spec_version(),
            needed_entry: x.needed_entry(),
            content: EntryContent::ValidEntry(x),
        })
        .collect(),
    }
}
