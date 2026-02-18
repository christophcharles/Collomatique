use std::collections::HashMap;

use rust_xlsxwriter::Worksheet;
use sqlx::{Row, SqlitePool};

use crate::Error;
use crate::format_slot_time;
use crate::formats;
use crate::get_group_name;

struct PeriodLayout {
    period_id: i64,
    col_start: u16,
    num_weeks: usize,
}

pub async fn build(worksheet: &mut Worksheet, pool: &SqlitePool) -> Result<(), Error> {
    worksheet.set_landscape();

    // 1. Period layout — periods ordered by position, with week count
    let period_rows = sqlx::query(
        "SELECT p.id, p.position, COUNT(pw.week_index) as num_weeks \
         FROM periods p \
         JOIN period_weeks pw ON pw.period_id = p.id \
         GROUP BY p.id \
         ORDER BY p.position",
    )
    .fetch_all(pool)
    .await?;

    let mut period_layout = Vec::new();
    let mut col_offset: u16 = 5; // first 5 columns: Matière, Colleur, Contact, Créneau, Salle
    for row in &period_rows {
        let period_id: i64 = row.get(0);
        let num_weeks: i64 = row.get(2);
        let nw = num_weeks as usize;
        period_layout.push(PeriodLayout {
            period_id,
            col_start: col_offset,
            num_weeks: nw,
        });
        col_offset += nw as u16;
    }

    let total_week_cols = col_offset - 5;
    if total_week_cols == 0 {
        return Ok(());
    }

    // 2. Period annotations — first non-empty annotation per period
    let annotation_rows = sqlx::query(
        "SELECT period_id, annotation \
         FROM period_weeks \
         WHERE annotation != '' \
         ORDER BY period_id, week_index",
    )
    .fetch_all(pool)
    .await?;

    let mut period_annotations: HashMap<i64, String> = HashMap::new();
    for row in annotation_rows {
        let period_id: i64 = row.get(0);
        let annotation: String = row.get(1);
        period_annotations.entry(period_id).or_insert(annotation);
    }

    // -- Row 0: Period labels --
    for pl in &period_layout {
        let label = period_annotations
            .get(&pl.period_id)
            .map(|a| a.as_str())
            .unwrap_or("Période");

        let fmt = formats::period_header();
        if pl.num_weeks == 1 {
            worksheet.write_with_format(0, pl.col_start, label, &fmt)?;
        } else {
            worksheet.merge_range(
                0,
                pl.col_start,
                0,
                pl.col_start + pl.num_weeks as u16 - 1,
                label,
                &fmt,
            )?;
        }
    }

    // -- Row 1: Fixed headers + week numbers --
    let fixed_headers = ["Matière", "Colleur", "Contact", "Créneau", "Salle"];
    let header_fmt = formats::header();
    for (col, name) in fixed_headers.iter().enumerate() {
        worksheet.write_with_format(1, col as u16, *name, &header_fmt)?;
    }

    let mut week_counter: u32 = 1;
    for pl in &period_layout {
        for w in 0..pl.num_weeks {
            let col = pl.col_start + w as u16;
            let (left, right) = period_border(w, pl.num_weeks);
            let fmt = formats::week_header(left, right);
            worksheet.write_with_format(1, col, format!("S{week_counter}"), &fmt)?;
            week_counter += 1;
        }
    }

    // 3. Load group list associations: (period_id, subject_id) -> group_list_id
    let assoc_rows = sqlx::query(
        "SELECT period_id, subject_id, group_list_id \
         FROM group_list_subject_associations",
    )
    .fetch_all(pool)
    .await?;

    let mut group_list_assocs: HashMap<(i64, i64), i64> = HashMap::new();
    for row in assoc_rows {
        let period_id: i64 = row.get(0);
        let subject_id: i64 = row.get(1);
        let group_list_id: i64 = row.get(2);
        group_list_assocs.insert((period_id, subject_id), group_list_id);
    }

    // 4. Load group names: group_list_id -> Vec<String>
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

    // 5. Load interrogation groups: (period_id, slot_id, week_index) -> Vec<group_number>
    let interrog_rows = sqlx::query(
        "SELECT period_id, slot_id, week_index, group_number \
         FROM colloscope_interrogation_groups \
         ORDER BY period_id, slot_id, week_index, group_number",
    )
    .fetch_all(pool)
    .await?;

    let mut interrog_map: HashMap<(i64, i64, i64), Vec<i64>> = HashMap::new();
    for row in interrog_rows {
        let period_id: i64 = row.get(0);
        let slot_id: i64 = row.get(1);
        let week_index: i64 = row.get(2);
        let group_number: i64 = row.get(3);
        interrog_map
            .entry((period_id, slot_id, week_index))
            .or_default()
            .push(group_number);
    }

    // 6. Subjects that have slots, ordered by position
    let subjects = sqlx::query(
        "SELECT sub.id, sub.name \
         FROM subjects sub \
         WHERE EXISTS (SELECT 1 FROM slots sl WHERE sl.subject_id = sub.id) \
         ORDER BY sub.position",
    )
    .fetch_all(pool)
    .await?;

    // -- Data rows --
    let mut row: u32 = 2;
    let mut first_subject = true;

    for subject_row in &subjects {
        let subject_id: i64 = subject_row.get(0);
        let subject_name: String = subject_row.get(1);

        // 7. Slots for this subject with teacher info
        let slots = sqlx::query(
            "SELECT sl.id, \
                    COALESCE(t.surname, '') as surname, \
                    COALESCE(NULLIF(t.email, ''), NULLIF(t.tel, ''), '') as contact, \
                    sl.day, sl.start_time, sl.extra_info \
             FROM slots sl \
             LEFT JOIN teachers t ON t.id = sl.teacher_id \
             WHERE sl.subject_id = ? \
             ORDER BY sl.position",
        )
        .bind(subject_id)
        .fetch_all(pool)
        .await?;

        if slots.is_empty() {
            continue;
        }

        // Separator row between subjects
        if !first_subject {
            for c in 0..5u16 {
                worksheet.write_with_format(row, c, "", &formats::empty_row(2, 2))?;
            }
            for pl in &period_layout {
                for w in 0..pl.num_weeks {
                    let (left, right) = period_border(w, pl.num_weeks);
                    let fmt = formats::empty_row(left, right);
                    worksheet.write_with_format(row, pl.col_start + w as u16, "", &fmt)?;
                }
            }
            row += 1;
        }
        first_subject = false;

        let subject_start_row = row;
        let slot_count = slots.len();

        for (slot_idx, slot_row) in slots.iter().enumerate() {
            let slot_id: i64 = slot_row.get(0);
            let surname: String = slot_row.get(1);
            let contact: String = slot_row.get(2);
            let day: i64 = slot_row.get(3);
            let start_time: i64 = slot_row.get(4);
            let extra_info: String = slot_row.get(5);

            let (top_b, bot_b) = vertical_borders(slot_idx, slot_count);

            let slot_time = format_slot_time(day, start_time);

            let data_fmt = formats::data_cell(top_b, bot_b, 2, 2);
            worksheet.write_with_format(row, 1, &surname, &data_fmt)?;
            worksheet.write_with_format(row, 2, &contact, &data_fmt)?;
            worksheet.write_with_format(row, 3, &slot_time, &data_fmt)?;
            worksheet.write_with_format(row, 4, &extra_info, &data_fmt)?;

            // Week columns
            for pl in &period_layout {
                let group_names = group_list_assocs
                    .get(&(pl.period_id, subject_id))
                    .and_then(|gl_id| group_names_map.get(gl_id));

                for w in 0..pl.num_weeks {
                    let col = pl.col_start + w as u16;
                    let (left_b, right_b) = period_border(w, pl.num_weeks);
                    let fmt = formats::week_cell(top_b, bot_b, left_b, right_b);

                    let cell_text = interrog_map
                        .get(&(pl.period_id, slot_id, w as i64))
                        .map(|groups| {
                            groups
                                .iter()
                                .map(|&g| {
                                    if let Some(names) = group_names {
                                        get_group_name(names, g)
                                    } else {
                                        (g + 1).to_string()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();

                    worksheet.write_with_format(row, col, &cell_text, &fmt)?;
                }
            }

            row += 1;
        }

        // Merge subject name vertically
        let subject_end_row = row - 1;
        let subject_fmt = formats::subject_cell(2, 2);
        if subject_start_row == subject_end_row {
            worksheet.write_with_format(subject_start_row, 0, &subject_name, &subject_fmt)?;
        } else {
            worksheet.merge_range(
                subject_start_row,
                0,
                subject_end_row,
                0,
                &subject_name,
                &subject_fmt,
            )?;
        }
    }

    // Column widths
    worksheet.set_column_width(0, 14)?;
    worksheet.set_column_width(1, 14)?;
    worksheet.set_column_width(2, 22)?;
    worksheet.set_column_width(3, 14)?;
    worksheet.set_column_width(4, 10)?;
    if total_week_cols > 0 {
        worksheet.set_column_range_width(5, 5 + total_week_cols - 1, 5)?;
    }

    Ok(())
}

/// Returns (top_border, bottom_border) levels for a slot row within a subject block
fn vertical_borders(slot_idx: usize, slot_count: usize) -> (u8, u8) {
    let is_first = slot_idx == 0;
    let is_last = slot_idx == slot_count - 1;
    match (is_first, is_last) {
        (true, true) => (2, 2),
        (true, false) => (2, 1),
        (false, true) => (1, 2),
        (false, false) => (1, 1),
    }
}

/// Returns (left_border, right_border) levels for a week column within a period
fn period_border(week_in_period: usize, num_weeks: usize) -> (u8, u8) {
    if num_weeks == 1 {
        (2, 2)
    } else if week_in_period == 0 {
        (2, 1)
    } else if week_in_period == num_weeks - 1 {
        (1, 2)
    } else {
        (1, 1)
    }
}
