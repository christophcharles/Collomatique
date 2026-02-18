use std::collections::HashMap;

use rust_xlsxwriter::Worksheet;
use sqlx::{Row, SqlitePool};

use crate::Error;
use crate::formats;
use crate::get_group_name;

pub async fn build(worksheet: &mut Worksheet, pool: &SqlitePool) -> Result<(), Error> {
    // 1. Group lists that have student assignments
    let group_lists = sqlx::query(
        "SELECT DISTINCT cgls.group_list_id, gl.name \
         FROM colloscope_group_list_students cgls \
         JOIN group_lists gl ON gl.id = cgls.group_list_id \
         ORDER BY cgls.group_list_id",
    )
    .fetch_all(pool)
    .await?;

    if group_lists.is_empty() {
        return Ok(());
    }

    let gl_count = group_lists.len();

    // -- Row 0: Headers --
    let header_fmt = formats::header();
    let fixed_headers = ["Nom", "Prénom", "Courriel", "Téléphone"];
    for (col, name) in fixed_headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *name, &header_fmt)?;
    }

    let gl_ids: Vec<i64> = group_lists.iter().map(|r| r.get(0)).collect();

    for (i, gl_row) in group_lists.iter().enumerate() {
        let gl_name: String = gl_row.get(1);
        worksheet.write_with_format(0, 4 + i as u16, &gl_name, &header_fmt)?;
    }

    // 2. Students sorted by name
    let students = sqlx::query(
        "SELECT id, surname, firstname, email, tel \
         FROM students \
         ORDER BY surname, firstname",
    )
    .fetch_all(pool)
    .await?;

    // 3. Student-to-group mapping: (group_list_id, student_id) -> group_number
    let student_groups_rows = sqlx::query(
        "SELECT group_list_id, student_id, group_number \
         FROM colloscope_group_list_students",
    )
    .fetch_all(pool)
    .await?;

    let mut student_groups: HashMap<(i64, i64), i64> = HashMap::new();
    for row in student_groups_rows {
        let gl_id: i64 = row.get(0);
        let student_id: i64 = row.get(1);
        let group_num: i64 = row.get(2);
        student_groups.insert((gl_id, student_id), group_num);
    }

    // 4. Group names: group_list_id -> Vec<String>
    let group_name_rows = sqlx::query(
        "SELECT group_list_id, group_index, name \
         FROM group_list_group_names \
         ORDER BY group_list_id, group_index",
    )
    .fetch_all(pool)
    .await?;

    let mut group_names_map: HashMap<i64, Vec<String>> = HashMap::new();
    for row in group_name_rows {
        let gl_id: i64 = row.get(0);
        let group_index: i64 = row.get(1);
        let name: String = row.get(2);
        let names = group_names_map.entry(gl_id).or_default();
        let idx = group_index as usize;
        if names.len() <= idx {
            names.resize(idx + 1, String::new());
        }
        names[idx] = name;
    }

    let student_count = students.len();
    for (row_idx, student_row) in students.iter().enumerate() {
        let student_id: i64 = student_row.get(0);
        let surname: String = student_row.get(1);
        let firstname: String = student_row.get(2);
        let email: String = student_row.get(3);
        let tel: String = student_row.get(4);

        let row = (row_idx + 1) as u32;
        let (top_b, bot_b) = vertical_borders(row_idx, student_count);

        let data_fmt = formats::data_cell(top_b, bot_b, 2, 2);
        worksheet.write_with_format(row, 0, &surname, &data_fmt)?;
        worksheet.write_with_format(row, 1, &firstname, &data_fmt)?;
        worksheet.write_with_format(row, 2, &email, &data_fmt)?;
        worksheet.write_with_format(row, 3, &tel, &data_fmt)?;

        for (i, gl_id) in gl_ids.iter().enumerate() {
            let col = 4 + i as u16;
            let (left_b, right_b) = gl_border(i, gl_count);
            let cell_fmt = formats::data_cell(top_b, bot_b, left_b, right_b);

            let cell_text = student_groups
                .get(&(*gl_id, student_id))
                .map(|&group_num| {
                    group_names_map
                        .get(gl_id)
                        .map(|names| get_group_name(names, group_num))
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
