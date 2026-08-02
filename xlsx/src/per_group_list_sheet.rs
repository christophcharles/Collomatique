use std::collections::BTreeMap;

use rust_xlsxwriter::{Url, Worksheet};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::group_lists::GroupListFilling;
use collomatique_state_colloscopes::ids::{GroupListId, StudentId};

use crate::Error;
use crate::formats;
use crate::get_group_name;

pub fn build(
    worksheet: &mut Worksheet,
    data: &InnerData,
    global: &crate::GlobalConfig,
    gl_id: GroupListId,
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

    let group_list = data.params.group_lists.group_list_map.get(&gl_id);

    // 1. Group names for this group list
    let group_names: Vec<String> = group_list.map(crate::group_names_vec).unwrap_or_default();

    // 2. Students per group (both automatic and prefilled sources)
    let mut members: Vec<(i64, StudentId)> = Vec::new();
    if let Some(placements) = data.colloscope.group_list(gl_id) {
        for (student_id, group_number) in placements {
            members.push((*group_number as i64, *student_id));
        }
    }
    if let Some(GroupListFilling::Prefilled { groups }) = group_list.map(|gl| gl.filling()) {
        for (group_index, group) in groups.iter().enumerate() {
            for student_id in &group.students {
                members.push((group_index as i64, *student_id));
            }
        }
    }

    // 3. Build data structure: group_index -> list of (surname, firstname, email, tel),
    //    sorted by name within each group
    let mut groups: BTreeMap<i64, Vec<(String, String, String, String)>> = BTreeMap::new();
    for (group_idx, student_id) in members {
        let Some(student) = data.params.students.student_map.get(&student_id) else {
            continue;
        };
        let desc = &student.desc;
        groups.entry(group_idx).or_default().push((
            desc.surname.clone(),
            desc.firstname.clone(),
            desc.email
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_default(),
            desc.tel.as_ref().map(|t| t.to_string()).unwrap_or_default(),
        ));
    }
    for students in groups.values_mut() {
        students.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
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
        let fmt = match col {
            1 => formats::header_cell(2, 1, bg),
            2 => formats::header_cell(1, 2, bg),
            _ => formats::header(bg),
        };
        worksheet.write_with_format(1, col as u16, *name, &fmt)?;
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

            // Col 0: group name (merged or single), with mailto link if emails exist
            if is_first_in_group {
                let emails: Vec<&str> = students
                    .iter()
                    .map(|(_, _, email, _)| email.as_str())
                    .filter(|e| !e.is_empty())
                    .collect();

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
                    if !emails.is_empty() && show_emails {
                        let mailto = format!("mailto:{}", emails.join(","));
                        let url = Url::new(mailto).set_text(&name);
                        worksheet.write_url_with_format(current_row, 0, url, &merge_fmt)?;
                    }
                } else if !emails.is_empty() && show_emails {
                    let mailto = format!("mailto:{}", emails.join(","));
                    let url = Url::new(mailto).set_text(&name);
                    worksheet.write_url_with_format(current_row, 0, url, &group_fmt)?;
                } else {
                    worksheet.write_with_format(current_row, 0, &name, &group_fmt)?;
                }
            }

            // Cols 1+: surname, firstname, (email), (telephone)
            let nom_fmt = formats::data_cell(top_b, bottom_b, 2, 1, row_bg);
            let prenom_fmt = formats::data_cell(top_b, bottom_b, 1, 2, row_bg);
            worksheet.write_with_format(current_row, 1, surname, &nom_fmt)?;
            worksheet.write_with_format(current_row, 2, firstname, &prenom_fmt)?;
            let mut c = 3u16;
            if show_emails {
                let data_fmt = formats::data_cell(top_b, bottom_b, 2, 2, row_bg);
                if email.is_empty() {
                    worksheet.write_with_format(current_row, c, "", &data_fmt)?;
                } else {
                    let url = Url::new(format!("mailto:{email}")).set_text(email);
                    worksheet.write_url_with_format(current_row, c, url, &data_fmt)?;
                }
                c += 1;
            }
            if show_tel {
                let data_fmt = formats::data_cell(top_b, bottom_b, 2, 2, row_bg);
                worksheet.write_with_format(current_row, c, tel, &data_fmt)?;
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
