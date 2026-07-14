use std::collections::HashMap;

use rust_xlsxwriter::{Url, Worksheet};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::group_lists::GroupListFilling;
use collomatique_state_colloscopes::ids::{GroupListId, StudentId};

use crate::Error;
use crate::formats;
use crate::get_group_name;

type GroupListsAndMembers = (
    Vec<(GroupListId, String)>,
    HashMap<(GroupListId, StudentId), i64>,
);

/// Membership of the solver-assigned ("automatic") groups, from the colloscope.
fn automatic_student_groups(
    data: &InnerData,
    student_groups: &mut HashMap<(GroupListId, StudentId), i64>,
) {
    for (gl_id, group_list) in &data.colloscope.group_lists {
        for (student_id, group_number) in &group_list.groups_for_students {
            student_groups.insert((*gl_id, *student_id), *group_number as i64);
        }
    }
}

/// Membership of the prefilled groups, from the parameters (group index = position in the list).
fn prefilled_student_groups(
    data: &InnerData,
    student_groups: &mut HashMap<(GroupListId, StudentId), i64>,
) {
    for (gl_id, group_list) in data.params.group_lists.group_list_map.entries() {
        if let GroupListFilling::Prefilled { groups } = &group_list.filling {
            for (group_index, group) in groups.iter().enumerate() {
                for student_id in &group.students {
                    student_groups.insert((gl_id, *student_id), group_index as i64);
                }
            }
        }
    }
}

fn query_all(data: &InnerData) -> GroupListsAndMembers {
    let group_lists = crate::non_empty_group_lists_by_name(data);

    let mut student_groups = HashMap::new();
    automatic_student_groups(data, &mut student_groups);
    prefilled_student_groups(data, &mut student_groups);

    (group_lists, student_groups)
}

fn query_automatic(data: &InnerData) -> GroupListsAndMembers {
    let mut group_lists: Vec<(GroupListId, String)> = data
        .colloscope
        .group_lists
        .iter()
        .filter(|(_gl_id, group_list)| !group_list.groups_for_students.is_empty())
        .map(|(gl_id, _group_list)| {
            let name = data
                .params
                .group_lists
                .group_list_map
                .get(gl_id)
                .map(|gl| gl.params.name.clone())
                .unwrap_or_default();
            (*gl_id, name)
        })
        .collect();
    group_lists.sort_by(|a, b| a.1.cmp(&b.1));

    let mut student_groups = HashMap::new();
    automatic_student_groups(data, &mut student_groups);

    (group_lists, student_groups)
}

fn query_prefilled(data: &InnerData) -> GroupListsAndMembers {
    let mut group_lists: Vec<(GroupListId, String)> = data
        .params
        .group_lists
        .group_list_map
        .entries()
        .filter(|(_gl_id, gl)| gl.filling.iter_students().next().is_some())
        .map(|(gl_id, gl)| (gl_id, gl.params.name.clone()))
        .collect();
    group_lists.sort_by(|a, b| a.1.cmp(&b.1));

    let mut student_groups = HashMap::new();
    prefilled_student_groups(data, &mut student_groups);

    (group_lists, student_groups)
}

fn build_internal(
    worksheet: &mut Worksheet,
    data: &InnerData,
    global: &crate::GlobalConfig,
    show_emails: bool,
    show_tel: bool,
    group_lists: &[(GroupListId, String)],
    student_groups: &HashMap<(GroupListId, StudentId), i64>,
) -> Result<usize, Error> {
    let bg = global.background_color.to_xlsx();
    let stripe = global
        .stripes_color
        .as_ref()
        .map(|c| c.to_xlsx())
        .unwrap_or(bg);

    let gl_count = group_lists.len();

    // -- Row 0: Headers --
    let nom_hdr = formats::header_cell(2, 1, bg);
    let prenom_hdr = formats::header_cell(1, 2, bg);
    worksheet.write_with_format(0, 0, "Nom", &nom_hdr)?;
    worksheet.write_with_format(0, 1, "Prénom", &prenom_hdr)?;
    let mut c = 2u16;
    if show_emails {
        let hdr = formats::header(bg);
        worksheet.write_with_format(0, c, "Courriel", &hdr)?;
        c += 1;
    }
    if show_tel {
        let hdr = formats::header(bg);
        worksheet.write_with_format(0, c, "Téléphone", &hdr)?;
        c += 1;
    }
    let gl_start_col = c;

    let gl_ids: Vec<GroupListId> = group_lists.iter().map(|(id, _)| *id).collect();

    for (i, (_, gl_name)) in group_lists.iter().enumerate() {
        let (left_b, right_b) = gl_border(i, gl_count);
        let gl_hdr = formats::header_cell(left_b, right_b, bg);
        worksheet.write_with_format(0, gl_start_col + i as u16, gl_name, &gl_hdr)?;
    }

    // Students sorted by name
    let mut students: Vec<_> = data
        .params
        .students
        .student_map
        .entries()
        .map(|(student_id, student)| (student_id, &student.desc))
        .collect();
    students.sort_by(|a, b| (&a.1.surname, &a.1.firstname).cmp(&(&b.1.surname, &b.1.firstname)));

    // Group names: group_list_id -> Vec<String>
    let group_names_map: HashMap<GroupListId, Vec<String>> = data
        .params
        .group_lists
        .group_list_map
        .entries()
        .map(|(gl_id, gl)| (gl_id, crate::group_names_vec(gl)))
        .collect();

    let student_count = students.len();
    for (row_idx, (student_id, desc)) in students.iter().enumerate() {
        let surname = &desc.surname;
        let firstname = &desc.firstname;
        let email = desc
            .email
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_default();
        let tel = desc.tel.as_ref().map(|t| t.to_string()).unwrap_or_default();

        let row = (row_idx + 1) as u32;
        let (top_b, bot_b) = vertical_borders(row_idx, student_count);
        let row_bg = if row_idx % 2 == 0 { stripe } else { bg };

        let nom_fmt = formats::data_cell(top_b, bot_b, 2, 1, row_bg);
        let prenom_fmt = formats::data_cell(top_b, bot_b, 1, 2, row_bg);
        worksheet.write_with_format(row, 0, surname, &nom_fmt)?;
        worksheet.write_with_format(row, 1, firstname, &prenom_fmt)?;
        let mut c = 2u16;
        if show_emails {
            let data_fmt = formats::data_cell(top_b, bot_b, 2, 2, row_bg);
            if email.is_empty() {
                worksheet.write_with_format(row, c, "", &data_fmt)?;
            } else {
                let url = Url::new(format!("mailto:{email}")).set_text(&email);
                worksheet.write_url_with_format(row, c, url, &data_fmt)?;
            }
            c += 1;
        }
        if show_tel {
            let data_fmt = formats::data_cell(top_b, bot_b, 2, 2, row_bg);
            worksheet.write_with_format(row, c, &tel, &data_fmt)?;
            c += 1;
        }
        let _ = c;

        for (i, gl_id) in gl_ids.iter().enumerate() {
            let col = gl_start_col + i as u16;
            let (left_b, right_b) = gl_border(i, gl_count);
            let cell_fmt = formats::data_cell(top_b, bot_b, left_b, right_b, row_bg);

            let cell_text = student_groups
                .get(&(*gl_id, *student_id))
                .map(|&group_idx| {
                    group_names_map
                        .get(gl_id)
                        .map(|names| get_group_name(names, group_idx))
                        .unwrap_or_else(|| (group_idx + 1).to_string())
                })
                .unwrap_or_default();

            worksheet.write_with_format(row, col, &cell_text, &cell_fmt)?;
        }
    }

    // Column widths
    worksheet.set_column_width(0, 16)?;
    worksheet.set_column_width(1, 14)?;
    let mut c = 2u16;
    if show_emails {
        worksheet.set_column_width(c, 24)?;
        c += 1;
    }
    if show_tel {
        worksheet.set_column_width(c, 14)?;
        c += 1;
    }
    if gl_count > 0 {
        worksheet.set_column_range_width(c, c + gl_count as u16 - 1, 12)?;
    }

    Ok(gl_count)
}

pub fn build_all(
    worksheet: &mut Worksheet,
    data: &InnerData,
    global: &crate::GlobalConfig,
    show_emails: bool,
    show_tel: bool,
) -> Result<usize, Error> {
    let (group_lists, student_groups) = query_all(data);
    build_internal(
        worksheet,
        data,
        global,
        show_emails,
        show_tel,
        &group_lists,
        &student_groups,
    )
}

pub fn build_automatic(
    worksheet: &mut Worksheet,
    data: &InnerData,
    global: &crate::GlobalConfig,
    show_emails: bool,
    show_tel: bool,
) -> Result<usize, Error> {
    let (group_lists, student_groups) = query_automatic(data);
    build_internal(
        worksheet,
        data,
        global,
        show_emails,
        show_tel,
        &group_lists,
        &student_groups,
    )
}

pub fn build_prefilled(
    worksheet: &mut Worksheet,
    data: &InnerData,
    global: &crate::GlobalConfig,
    show_emails: bool,
    show_tel: bool,
) -> Result<usize, Error> {
    let (group_lists, student_groups) = query_prefilled(data);
    build_internal(
        worksheet,
        data,
        global,
        show_emails,
        show_tel,
        &group_lists,
        &student_groups,
    )
}

fn vertical_borders(row_idx: usize, count: usize) -> (u8, u8) {
    let is_first = row_idx == 0;
    let is_last = row_idx == count - 1;
    match (is_first, is_last) {
        (true, true) => (2, 2),
        (true, false) => (2, 1),
        (false, true) => (1, 2),
        (false, false) => (1, 1),
    }
}

fn gl_border(index: usize, count: usize) -> (u8, u8) {
    if count == 1 {
        (2, 2)
    } else if index == 0 {
        (2, 1)
    } else if index == count - 1 {
        (1, 2)
    } else {
        (1, 1)
    }
}
