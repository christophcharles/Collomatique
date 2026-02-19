use std::collections::HashMap;

use rust_xlsxwriter::{Url, Worksheet};
use sqlx::{Row, SqlitePool};

use crate::Error;
use crate::formats;
use crate::get_group_name;

pub async fn build(
    worksheet: &mut Worksheet,
    pool: &SqlitePool,
    global: &crate::GlobalConfig,
    show_emails: bool,
    show_tel: bool,
) -> Result<usize, Error> {
    let bg = global.background_color.to_xlsx();
    let stripe = global
        .stripes_color
        .as_ref()
        .map(|c| c.to_xlsx())
        .unwrap_or(bg);

    // 1. Group lists that have prefilled student assignments
    let group_lists = sqlx::query(
        "SELECT DISTINCT pgs.group_list_id, gl.name \
         FROM prefilled_group_students pgs \
         JOIN group_lists gl ON gl.id = pgs.group_list_id \
         ORDER BY gl.name",
    )
    .fetch_all(pool)
    .await?;

    let gl_count = group_lists.len();

    // -- Row 0: Headers --
    let header_fmt = formats::header(bg);
    let mut fixed_headers: Vec<&str> = vec!["Nom", "Prénom"];
    if show_emails {
        fixed_headers.push("Courriel");
    }
    if show_tel {
        fixed_headers.push("Téléphone");
    }
    let gl_start_col = fixed_headers.len() as u16;
    for (col, name) in fixed_headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *name, &header_fmt)?;
    }

    let gl_ids: Vec<i64> = group_lists.iter().map(|r| r.get(0)).collect();

    for (i, gl_row) in group_lists.iter().enumerate() {
        let gl_name: String = gl_row.get(1);
        worksheet.write_with_format(0, gl_start_col + i as u16, &gl_name, &header_fmt)?;
    }

    // 2. Students sorted by name
    let students = sqlx::query(
        "SELECT id, surname, firstname, email, tel \
         FROM students \
         ORDER BY surname, firstname",
    )
    .fetch_all(pool)
    .await?;

    // 3. Student-to-group mapping: (group_list_id, student_id) -> group_index
    let student_groups_rows = sqlx::query(
        "SELECT group_list_id, student_id, group_index \
         FROM prefilled_group_students",
    )
    .fetch_all(pool)
    .await?;

    let mut student_groups: HashMap<(i64, i64), i64> = HashMap::new();
    for row in student_groups_rows {
        let gl_id: i64 = row.get(0);
        let student_id: i64 = row.get(1);
        let group_index: i64 = row.get(2);
        student_groups.insert((gl_id, student_id), group_index);
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
        let row_bg = if row_idx % 2 == 0 { stripe } else { bg };

        let data_fmt = formats::data_cell(top_b, bot_b, 2, 2, row_bg);
        worksheet.write_with_format(row, 0, &surname, &data_fmt)?;
        worksheet.write_with_format(row, 1, &firstname, &data_fmt)?;
        let mut c = 2u16;
        if show_emails {
            if email.is_empty() {
                worksheet.write_with_format(row, c, "", &data_fmt)?;
            } else {
                let url = Url::new(format!("mailto:{email}")).set_text(&email);
                worksheet.write_url_with_format(row, c, url, &data_fmt)?;
            }
            c += 1;
        }
        if show_tel {
            worksheet.write_with_format(row, c, &tel, &data_fmt)?;
            c += 1;
        }
        let _ = c;

        for (i, gl_id) in gl_ids.iter().enumerate() {
            let col = gl_start_col + i as u16;
            let (left_b, right_b) = gl_border(i, gl_count);
            let cell_fmt = formats::data_cell(top_b, bot_b, left_b, right_b, row_bg);

            let cell_text = student_groups
                .get(&(*gl_id, student_id))
                .map(|&group_index| {
                    group_names_map
                        .get(gl_id)
                        .map(|names| get_group_name(names, group_index))
                        .unwrap_or_else(|| (group_index + 1).to_string())
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
