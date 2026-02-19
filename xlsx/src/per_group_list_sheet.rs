use std::collections::BTreeMap;

use rust_xlsxwriter::{Url, Worksheet};
use sqlx::{Row, SqlitePool};

use crate::Error;
use crate::formats;
use crate::get_group_name;

pub async fn build(
    worksheet: &mut Worksheet,
    pool: &SqlitePool,
    global: &crate::GlobalConfig,
    gl_id: i64,
    gl_name: &str,
    show_emails: bool,
    show_tel: bool,
) -> Result<(), Error> {
    let bg = global.background_color.to_xlsx();
    let stripe = global
        .stripes_color
        .as_ref()
        .map(|c| c.to_xlsx())
        .unwrap_or(bg);

    // 1. Fetch group names for this group list
    let group_name_rows = sqlx::query(
        "SELECT group_index, name FROM group_list_group_names \
         WHERE group_list_id = ? ORDER BY group_index",
    )
    .bind(gl_id)
    .fetch_all(pool)
    .await?;

    let mut group_names: Vec<String> = Vec::new();
    for row in &group_name_rows {
        let group_index: i64 = row.get(0);
        let name: String = row.get(1);
        let idx = group_index as usize;
        if group_names.len() <= idx {
            group_names.resize(idx + 1, String::new());
        }
        group_names[idx] = name;
    }

    // 2. Fetch students per group (both automatic and prefilled sources)
    let student_rows = sqlx::query(
        "SELECT s.surname, s.firstname, s.email, s.tel, sg.group_idx \
         FROM students s \
         JOIN ( \
             SELECT student_id, group_number AS group_idx \
             FROM colloscope_group_list_students WHERE group_list_id = ? \
             UNION ALL \
             SELECT student_id, group_index AS group_idx \
             FROM prefilled_group_students WHERE group_list_id = ? \
         ) sg ON s.id = sg.student_id \
         ORDER BY sg.group_idx, s.surname, s.firstname",
    )
    .bind(gl_id)
    .bind(gl_id)
    .fetch_all(pool)
    .await?;

    // 3. Build data structure: group_index -> list of (surname, firstname, email, tel)
    let mut groups: BTreeMap<i64, Vec<(String, String, String, String)>> = BTreeMap::new();
    for row in &student_rows {
        let surname: String = row.get(0);
        let firstname: String = row.get(1);
        let email: String = row.get(2);
        let tel: String = row.get(3);
        let group_idx: i64 = row.get(4);
        groups
            .entry(group_idx)
            .or_default()
            .push((surname, firstname, email, tel));
    }

    // 4. Title row
    let header_fmt = formats::header(bg);
    let mut col_headers: Vec<&str> = vec!["Groupe", "Nom", "Prénom"];
    if show_emails {
        col_headers.push("Courriel");
    }
    if show_tel {
        col_headers.push("Téléphone");
    }
    let last_col = col_headers.len() as u16 - 1;
    worksheet.merge_range(0, 0, 0, last_col, gl_name, &header_fmt)?;

    // 5. Header row
    for (col, name) in col_headers.iter().enumerate() {
        worksheet.write_with_format(1, col as u16, *name, &header_fmt)?;
    }

    // 6. Write rows
    let mut current_row: u32 = 2;
    let group_count = groups.len();

    for (display_idx, (group_idx, students)) in groups.iter().enumerate() {
        let name = get_group_name(&group_names, *group_idx);
        let student_count = students.len();
        let row_bg = if display_idx % 2 == 0 { stripe } else { bg };

        let is_last_group = display_idx == group_count - 1;

        for (student_idx, (surname, firstname, email, tel)) in students.iter().enumerate() {
            let is_first_in_group = student_idx == 0;
            let is_last_in_group = student_idx == student_count - 1;

            // Horizontal borders: medium between groups, thin within
            let top_b: u8 = if is_first_in_group { 2 } else { 1 };
            let bottom_b: u8 = if is_last_in_group && is_last_group {
                2
            } else if is_last_in_group {
                // Will be overridden by next group's top, but set medium for last row of group
                2
            } else {
                1
            };

            // Col 0: group name (merged or single)
            if is_first_in_group {
                let group_fmt = formats::data_cell(top_b, bottom_b, 2, 2, row_bg);
                if student_count > 1 {
                    let merge_bottom_b: u8 = if is_last_group { 2 } else { 2 };
                    let merge_fmt = formats::data_cell(top_b, merge_bottom_b, 2, 2, row_bg);
                    worksheet.merge_range(
                        current_row,
                        0,
                        current_row + student_count as u32 - 1,
                        0,
                        &name,
                        &merge_fmt,
                    )?;
                } else {
                    worksheet.write_with_format(current_row, 0, &name, &group_fmt)?;
                }
            }

            // Cols 1+: surname, firstname, (email), (telephone)
            let student_fmt = formats::data_cell(top_b, bottom_b, 2, 2, row_bg);
            worksheet.write_with_format(current_row, 1, surname, &student_fmt)?;
            worksheet.write_with_format(current_row, 2, firstname, &student_fmt)?;
            let mut c = 3u16;
            if show_emails {
                if email.is_empty() {
                    worksheet.write_with_format(current_row, c, "", &student_fmt)?;
                } else {
                    let url = Url::new(format!("mailto:{email}")).set_text(email);
                    worksheet.write_url_with_format(current_row, c, url, &student_fmt)?;
                }
                c += 1;
            }
            if show_tel {
                worksheet.write_with_format(current_row, c, tel, &student_fmt)?;
                c += 1;
            }
            let _ = c;

            current_row += 1;
        }
    }

    // Column widths
    worksheet.set_column_width(0, 14)?;
    worksheet.set_column_width(1, 16)?;
    worksheet.set_column_width(2, 14)?;
    let mut c = 3u16;
    if show_emails {
        worksheet.set_column_width(c, 24)?;
        c += 1;
    }
    if show_tel {
        worksheet.set_column_width(c, 14)?;
        c += 1;
    }
    let _ = c;

    Ok(())
}
