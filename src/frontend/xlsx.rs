use super::state::{
    GroupListHandle, IncompatHandle, StudentHandle, SubjectGroupHandle, SubjectHandle,
    TeacherHandle,
};
use crate::backend;

use rust_xlsxwriter::*;
use thiserror::Error;

use std::collections::BTreeMap;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error in rust_xlsxwrite crate: {0:?}")]
    XlsxError(#[from] XlsxError),
    #[error("Colloscope is not compatible with the provided database")]
    BadColloscope,
    #[error("Colloscope contains zero weeks")]
    NoWeeks,
    #[error("Colloscope contains too many weeks (max is 65535)")]
    TooManyWeeks,
    #[error("Colloscope contains too many students (max is 2^32 -1)")]
    TooManyStudents,
    #[error("Colloscope is inconsistent: a group number is invalid")]
    InvalidGroupNumber,
}

pub type Result<T> = std::result::Result<T, Error>;

fn sort_with<T, I, K, F>(data: I, mut func: F) -> Result<BTreeMap<K, Vec<T>>>
where
    I: IntoIterator<Item = T>,
    K: PartialOrd + Ord + PartialEq + Eq,
    F: FnMut(&T) -> Result<K>,
{
    let mut output: BTreeMap<K, Vec<T>> = BTreeMap::new();

    for element in data {
        let key = func(&element)?;
        match output.get_mut(&key) {
            Some(array) => {
                array.push(element);
            }
            None => {
                output.insert(key, vec![element]);
            }
        }
    }

    Ok(output)
}

fn merge_if_needed(
    worksheet: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
    string: &str,
    format: &Format,
) -> Result<()> {
    if first_row == last_row && first_col == last_col {
        worksheet.write_with_format(first_row, first_col, string, format)?;
    } else {
        worksheet.merge_range(first_row, first_col, last_row, last_col, string, format)?;
    }

    Ok(())
}

const ROW_COLLOSCOPE_CATEGORIES: u32 = 0;
const ROW_COLLOSCOPE_TITLES: u32 = 1;
const ROW_FIRST_TIME_SLOT: u32 = 2;

const COL_SUBJECT_GROUP: u16 = 0;
const COL_SUBJECT: u16 = 1;
const COL_GROUP_LIST: u16 = 2;
const COL_TEACHER: u16 = 3;
const COL_TEACHER_CONTACT: u16 = 4;
const COL_SLOT: u16 = 5;
const COL_ROOM: u16 = 6;
const COL_FIRST_WEEK: u16 = 7;

fn build_main_worksheet_first_line(
    worksheet: &mut Worksheet,
    colloscope: &backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
) -> Result<()> {
    let week_count = colloscope
        .subjects
        .iter()
        .map(|(_subject_handle, subject)| {
            subject
                .time_slots
                .iter()
                .map(|time_slot| {
                    time_slot
                        .group_assignments
                        .iter()
                        .map(|(week, _groups)| week.get() + 1)
                        .max()
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);

    if week_count == 0 {
        return Err(Error::NoWeeks);
    }

    let week_count: u16 = week_count.try_into().map_err(|_| Error::TooManyWeeks)?;

    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);

    worksheet.write_with_format(
        ROW_COLLOSCOPE_TITLES,
        COL_SUBJECT_GROUP,
        "Groupement",
        &format,
    )?;
    worksheet.write_with_format(ROW_COLLOSCOPE_TITLES, COL_SUBJECT, "Matière", &format)?;
    worksheet.write_with_format(ROW_COLLOSCOPE_TITLES, COL_GROUP_LIST, "Liste", &format)?;
    worksheet.write_with_format(ROW_COLLOSCOPE_TITLES, COL_TEACHER, "Colleur", &format)?;
    worksheet.write_with_format(
        ROW_COLLOSCOPE_TITLES,
        COL_TEACHER_CONTACT,
        "Contact",
        &format,
    )?;
    worksheet.write_with_format(ROW_COLLOSCOPE_TITLES, COL_SLOT, "Créneau", &format)?;
    worksheet.write_with_format(ROW_COLLOSCOPE_TITLES, COL_ROOM, "Salle", &format)?;

    merge_if_needed(
        worksheet,
        ROW_COLLOSCOPE_CATEGORIES,
        COL_FIRST_WEEK,
        0,
        COL_FIRST_WEEK + week_count - 1,
        "Semaines",
        &format,
    )?;

    for i in 0..week_count {
        worksheet.write_with_format(ROW_COLLOSCOPE_TITLES, i + COL_FIRST_WEEK, i + 1, &format)?;
    }

    Ok(())
}

fn build_main_worksheet_timeslot(
    worksheet: &mut Worksheet,
    start_line: u32,
    time_slot: backend::ColloscopeTimeSlot<TeacherHandle>,
    group_list: &backend::ColloscopeGroupList<StudentHandle>,
) -> Result<u32> {
    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);

    let slot = format!(
        "{} {:02}h{:02}",
        time_slot.start.day,
        time_slot.start.time.get_hour(),
        time_slot.start.time.get_min()
    );
    worksheet.write_with_format(start_line, COL_SLOT, &slot, &format)?;
    worksheet.write_with_format(start_line, COL_ROOM, &time_slot.room, &format)?;

    for (week, groups) in time_slot.group_assignments {
        let group_names = groups
            .into_iter()
            .map(|group_num| {
                group_list
                    .groups
                    .get(group_num)
                    .ok_or(Error::InvalidGroupNumber)
                    .cloned()
            })
            .collect::<Result<Vec<_>>>()?;

        let column = COL_FIRST_WEEK
            + u16::try_from(week.get()).expect(
                "Week numbers should have already been checked when constructing first line",
            );
        worksheet.write_with_format(start_line, column, group_names.join(","), &format)?;
    }

    Ok(start_line + 1)
}

fn build_main_worksheet_teacher(
    worksheet: &mut Worksheet,
    start_line: u32,
    time_slots: Vec<backend::ColloscopeTimeSlot<TeacherHandle>>,
    group_list: &backend::ColloscopeGroupList<StudentHandle>,
    teacher_handle: TeacherHandle,
    teachers: &BTreeMap<TeacherHandle, backend::Teacher>,
) -> Result<u32> {
    let mut current_line = start_line;
    for time_slot in time_slots {
        current_line =
            build_main_worksheet_timeslot(worksheet, current_line, time_slot, group_list)?;
    }

    let teacher = teachers.get(&teacher_handle).ok_or(Error::BadColloscope)?;
    let name = format!("{} {}", teacher.firstname, teacher.surname,);
    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);
    merge_if_needed(
        worksheet,
        start_line,
        COL_TEACHER,
        current_line - 1,
        COL_TEACHER,
        &name,
        &format,
    )?;
    merge_if_needed(
        worksheet,
        start_line,
        COL_TEACHER_CONTACT,
        current_line - 1,
        COL_TEACHER_CONTACT,
        &teacher.contact,
        &format,
    )?;

    Ok(current_line)
}

fn build_main_worksheet_subject(
    worksheet: &mut Worksheet,
    start_line: u32,
    subject: backend::ColloscopeSubject<TeacherHandle, StudentHandle>,
    subject_handle: SubjectHandle,
    teachers: &BTreeMap<TeacherHandle, backend::Teacher>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
) -> Result<u32> {
    let sorted_time_slots = sort_with(subject.time_slots, |time_slot| Ok(time_slot.teacher_id))?;

    let mut current_line = start_line;
    for (teacher_handle, time_slots) in sorted_time_slots {
        current_line = build_main_worksheet_teacher(
            worksheet,
            current_line,
            time_slots,
            &subject.group_list,
            teacher_handle,
            teachers,
        )?;
    }

    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);

    let subject_name = &subjects
        .get(&subject_handle)
        .ok_or(Error::BadColloscope)?
        .name;
    merge_if_needed(
        worksheet,
        start_line,
        COL_SUBJECT,
        current_line - 1,
        COL_SUBJECT,
        subject_name,
        &format,
    )?;
    merge_if_needed(
        worksheet,
        start_line,
        COL_GROUP_LIST,
        current_line - 1,
        COL_GROUP_LIST,
        &subject.group_list.name,
        &format,
    )?;

    Ok(current_line)
}

fn build_main_worksheet_subject_group(
    worksheet: &mut Worksheet,
    start_line: u32,
    selected_subjects: Vec<(
        SubjectHandle,
        backend::ColloscopeSubject<TeacherHandle, StudentHandle>,
    )>,
    subject_group_handle: SubjectGroupHandle,
    teachers: &BTreeMap<TeacherHandle, backend::Teacher>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
    subject_groups: &BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
) -> Result<u32> {
    let mut current_line = start_line;
    for (subject_handle, subject) in selected_subjects {
        current_line = build_main_worksheet_subject(
            worksheet,
            current_line,
            subject,
            subject_handle,
            teachers,
            subjects,
        )?;
    }

    let name = &subject_groups
        .get(&subject_group_handle)
        .ok_or(Error::BadColloscope)?
        .name;
    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);
    merge_if_needed(
        worksheet,
        start_line,
        COL_SUBJECT_GROUP,
        current_line - 1,
        COL_SUBJECT_GROUP,
        name,
        &format,
    )?;

    Ok(current_line + 1)
}

fn build_main_worksheet(
    worksheet: &mut Worksheet,
    colloscope: &backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    teachers: &BTreeMap<TeacherHandle, backend::Teacher>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
    subject_groups: &BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
) -> Result<()> {
    worksheet.set_name("Colloscope")?;

    build_main_worksheet_first_line(worksheet, colloscope)?;

    let sorted_subjects = sort_with(colloscope.subjects.clone(), |(subject_id, _subject)| {
        subjects
            .get(subject_id)
            .map(|s| s.subject_group_id)
            .ok_or(Error::BadColloscope)
    })?;

    let mut start_line = ROW_FIRST_TIME_SLOT;
    for (subject_group_handle, selected_subjects) in sorted_subjects {
        start_line = build_main_worksheet_subject_group(
            worksheet,
            start_line,
            selected_subjects,
            subject_group_handle,
            teachers,
            subjects,
            subject_groups,
        )?;
    }

    worksheet.autofit();

    Ok(())
}

const COL_FIRSTNAME: u16 = 0;
const COL_SURNAME: u16 = 1;
const COL_EMAIL: u16 = 2;
const COL_PHONE: u16 = 3;
const COL_FIRST_LIST: u16 = 4;

const ROW_SUBJECT_GROUP_NAME: u32 = 0;
const ROW_SUBJECT_NAME: u32 = 1;
const ROW_LIST_NAME: u32 = 2;
const ROW_STUDENT_TITLES: u32 = 2;
const ROW_FIRST_STUDENT: u32 = 3;

fn build_groups_worksheet_first_columns(
    worksheet: &mut Worksheet,
    students: &BTreeMap<StudentHandle, backend::Student>,
) -> Result<BTreeMap<StudentHandle, u32>> {
    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);

    worksheet.write_with_format(ROW_STUDENT_TITLES, COL_FIRSTNAME, "Prénom", &format)?;
    worksheet.write_with_format(ROW_STUDENT_TITLES, COL_SURNAME, "Nom", &format)?;
    worksheet.write_with_format(ROW_STUDENT_TITLES, COL_EMAIL, "Courriel", &format)?;
    worksheet.write_with_format(ROW_STUDENT_TITLES, COL_PHONE, "Téléphone", &format)?;

    let mut line_map = BTreeMap::new();

    for (i, (student_handle, student)) in students.into_iter().enumerate() {
        let line = ROW_FIRST_STUDENT + u32::try_from(i).map_err(|_| Error::TooManyStudents)?;
        line_map.insert(*student_handle, line);

        worksheet.write_with_format(line, COL_FIRSTNAME, &student.firstname, &format)?;
        worksheet.write_with_format(line, COL_SURNAME, &student.surname, &format)?;
        worksheet.write_with_format(
            line,
            COL_EMAIL,
            &student.email.clone().unwrap_or_default(),
            &format,
        )?;
        worksheet.write_with_format(
            line,
            COL_PHONE,
            &student.phone.clone().unwrap_or_default(),
            &format,
        )?;
    }

    Ok(line_map)
}

fn build_groups_worksheet_subject(
    worksheet: &mut Worksheet,
    start_col: u16,
    subject: backend::ColloscopeSubject<TeacherHandle, StudentHandle>,
    subject_handle: SubjectHandle,
    student_line_map: &BTreeMap<StudentHandle, u32>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
) -> Result<u16> {
    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);

    let subject_name = &subjects
        .get(&subject_handle)
        .ok_or(Error::BadColloscope)?
        .name;
    worksheet.write_with_format(ROW_SUBJECT_NAME, start_col, subject_name, &format)?;
    worksheet.write_with_format(ROW_LIST_NAME, start_col, &subject.group_list.name, &format)?;

    for (&student_handle, &group_num) in &subject.group_list.students_mapping {
        let group_name = subject
            .group_list
            .groups
            .get(group_num)
            .ok_or(Error::InvalidGroupNumber)?;
        let line = student_line_map
            .get(&student_handle)
            .expect("student_line_map should be a complete map");

        worksheet.write_with_format(*line, start_col, group_name, &format)?;
    }

    Ok(start_col + 1)
}

fn build_groups_worksheet_subject_group(
    worksheet: &mut Worksheet,
    start_col: u16,
    selected_subjects: Vec<(
        SubjectHandle,
        backend::ColloscopeSubject<TeacherHandle, StudentHandle>,
    )>,
    subject_group_handle: SubjectGroupHandle,
    student_line_map: &BTreeMap<StudentHandle, u32>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
    subject_groups: &BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
) -> Result<u16> {
    let mut current_col = start_col;
    for (subject_handle, subject) in selected_subjects {
        current_col = build_groups_worksheet_subject(
            worksheet,
            current_col,
            subject,
            subject_handle,
            student_line_map,
            subjects,
        )?;
    }

    let name = &subject_groups
        .get(&subject_group_handle)
        .ok_or(Error::BadColloscope)?
        .name;
    let format = Format::new()
        .set_align(FormatAlign::VerticalCenter)
        .set_align(FormatAlign::Center);
    merge_if_needed(
        worksheet,
        ROW_SUBJECT_GROUP_NAME,
        start_col,
        ROW_SUBJECT_GROUP_NAME,
        current_col - 1,
        name,
        &format,
    )?;

    Ok(current_col + 1)
}

fn build_groups_worksheet(
    worksheet: &mut Worksheet,
    colloscope: &backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
    subject_groups: &BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
    students: &BTreeMap<StudentHandle, backend::Student>,
) -> Result<()> {
    worksheet.set_name("Groupes")?;

    let student_line_map = build_groups_worksheet_first_columns(worksheet, students)?;

    let sorted_subjects = sort_with(colloscope.subjects.clone(), |(subject_id, _subject)| {
        subjects
            .get(subject_id)
            .map(|s| s.subject_group_id)
            .ok_or(Error::BadColloscope)
    })?;

    let mut start_col = COL_FIRST_LIST;
    for (subject_group_handle, selected_subjects) in sorted_subjects {
        start_col = build_groups_worksheet_subject_group(
            worksheet,
            start_col,
            selected_subjects,
            subject_group_handle,
            &student_line_map,
            subjects,
            subject_groups,
        )?;
    }

    worksheet.autofit();

    Ok(())
}

pub fn export_colloscope_to_xlsx(
    colloscope: &backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    teachers: &BTreeMap<TeacherHandle, backend::Teacher>,
    subjects: &BTreeMap<
        SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    >,
    subject_groups: &BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
    students: &BTreeMap<StudentHandle, backend::Student>,
    file: &std::path::Path,
) -> Result<()> {
    let mut workbook = Workbook::new();

    build_main_worksheet(
        workbook.add_worksheet(),
        colloscope,
        teachers,
        subjects,
        subject_groups,
    )?;
    build_groups_worksheet(
        workbook.add_worksheet(),
        colloscope,
        subjects,
        subject_groups,
        students,
    )?;

    workbook.save(file)?;

    Ok(())
}
