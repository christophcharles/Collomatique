use collomatique_state_colloscopes::GroupListId;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;
use rust_xlsxwriter::{Worksheet, XlsxError};

use crate::formats;
use crate::get_group_name;

pub fn build(
    worksheet: &mut Worksheet,
    params: &Parameters,
    colloscope: &Colloscope,
) -> Result<(), XlsxError> {
    // Collect group list IDs that have student assignments
    let group_list_ids: Vec<GroupListId> = colloscope.group_lists.keys().copied().collect();

    if group_list_ids.is_empty() {
        return Ok(());
    }

    let gl_count = group_list_ids.len();

    // -- Row 0: Headers --
    let header_fmt = formats::header();
    let fixed_headers = ["Nom", "Prénom", "Courriel", "Téléphone"];
    for (col, name) in fixed_headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *name, &header_fmt)?;
    }

    for (i, gl_id) in group_list_ids.iter().enumerate() {
        let name = params
            .group_lists
            .group_list_map
            .get(gl_id)
            .map(|gl| gl.params.name.as_str())
            .unwrap_or("");
        worksheet.write_with_format(0, 4 + i as u16, name, &header_fmt)?;
    }

    // -- Collect and sort students --
    let mut students_sorted: Vec<_> = params.students.student_map.iter().collect();
    students_sorted.sort_by(|a, b| {
        (&a.1.desc.surname, &a.1.desc.firstname).cmp(&(&b.1.desc.surname, &b.1.desc.firstname))
    });

    let student_count = students_sorted.len();
    for (row_idx, (student_id, student)) in students_sorted.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        let (top_b, bot_b) = vertical_borders(row_idx, student_count);

        let data_fmt = formats::data_cell(top_b, bot_b, 2, 2);
        worksheet.write_with_format(row, 0, &student.desc.surname, &data_fmt)?;
        worksheet.write_with_format(row, 1, &student.desc.firstname, &data_fmt)?;
        worksheet.write_with_format(
            row,
            2,
            student
                .desc
                .email
                .as_ref()
                .map(|e| e.as_str())
                .unwrap_or(""),
            &data_fmt,
        )?;
        worksheet.write_with_format(
            row,
            3,
            student.desc.tel.as_ref().map(|t| t.as_str()).unwrap_or(""),
            &data_fmt,
        )?;

        for (i, gl_id) in group_list_ids.iter().enumerate() {
            let col = 4 + i as u16;
            let (left_b, right_b) = gl_border(i, gl_count);
            let cell_fmt = formats::data_cell(top_b, bot_b, left_b, right_b);

            let cell_text = colloscope
                .group_lists
                .get(gl_id)
                .and_then(|cgl| cgl.groups_for_students.get(student_id))
                .map(|&group_num| {
                    params
                        .group_lists
                        .group_list_map
                        .get(gl_id)
                        .map(|gl| get_group_name(&gl.params, group_num))
                        .unwrap_or_else(|| (group_num + 1).to_string())
                })
                .unwrap_or_default();

            worksheet.write_with_format(row, col, &cell_text, &cell_fmt)?;
        }
    }

    // Column widths
    worksheet.set_column_width(0, 16)?;
    worksheet.set_column_width(1, 14)?;
    worksheet.set_column_width(2, 24)?;
    worksheet.set_column_width(3, 14)?;
    if gl_count > 0 {
        worksheet.set_column_range_width(4, 4 + gl_count as u16 - 1, 12)?;
    }

    Ok(())
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
