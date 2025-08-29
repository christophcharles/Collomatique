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

fn generate_week_pattern_list(data: &Data) -> week_pattern_list::List {
    let orig_week_patterns = data.get_week_patterns();

    week_pattern_list::List {
        week_pattern_map: orig_week_patterns
            .week_pattern_map
            .iter()
            .map(|(id, week_pattern)| (id.inner(), week_pattern.into()))
            .collect(),
    }
}

fn generate_slot_list(data: &Data) -> slot_list::List {
    let orig_slots = data.get_slots();

    slot_list::List {
        subject_map: orig_slots
            .subject_map
            .iter()
            .map(|(subject_id, subject_slots)| {
                (
                    subject_id.inner(),
                    slot_list::SubjectSlots {
                        ordered_slot_list: subject_slots
                            .ordered_slots
                            .iter()
                            .map(|(slot_id, slot)| (slot_id.inner(), slot.into()))
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

fn generate_incompat_list(data: &Data) -> incompat_list::List {
    let orig_incompats = data.get_incompats();

    incompat_list::List {
        incompat_map: orig_incompats
            .incompat_map
            .iter()
            .map(|(incompat_id, incompat)| (incompat_id.inner(), incompat.into()))
            .collect(),
    }
}

fn generate_group_list_list(data: &Data) -> group_list_list::List {
    let orig_group_lists = data.get_group_lists();

    group_list_list::List {
        group_list_map: orig_group_lists
            .group_list_map
            .iter()
            .map(|(group_list_id, group_list)| (group_list_id.inner(), group_list.into()))
            .collect(),
        subjects_associations: orig_group_lists
            .subjects_associations
            .iter()
            .map(|(period_id, subject_map)| {
                (
                    period_id.inner(),
                    subject_map
                        .iter()
                        .map(|(subject_id, group_list_id)| {
                            (subject_id.inner(), group_list_id.inner())
                        })
                        .collect(),
                )
            })
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
    let week_pattern_list_entry = ValidEntry::WeekPatternList(generate_week_pattern_list(data));
    let slot_list_entry = ValidEntry::SlotList(generate_slot_list(data));
    let incompat_list_entry = ValidEntry::IncompatList(generate_incompat_list(data));
    let group_list_list_entry = ValidEntry::GroupListList(generate_group_list_list(data));

    JsonData {
        header,
        entries: vec![
            period_list_entry,
            subject_list_entry,
            teacher_list_entry,
            student_list_entry,
            assignment_map_entry,
            week_pattern_list_entry,
            slot_list_entry,
            incompat_list_entry,
            group_list_list_entry,
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
