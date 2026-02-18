use collomatique_state_colloscopes::PeriodId;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;
use rust_xlsxwriter::{Worksheet, XlsxError};

use crate::formats;
use crate::get_group_name;

struct PeriodLayout {
    period_id: PeriodId,
    col_start: u16,
    num_weeks: usize,
}

pub fn build(
    worksheet: &mut Worksheet,
    params: &Parameters,
    colloscope: &Colloscope,
) -> Result<(), XlsxError> {
    worksheet.set_landscape();

    // Compute period layout
    let mut period_layout = Vec::new();
    let mut col_offset: u16 = 5; // first 5 columns: Matière, Colleur, Contact, Créneau, Salle
    for (period_id, weeks) in &params.periods.ordered_period_list {
        let num_weeks = weeks.len();
        period_layout.push(PeriodLayout {
            period_id: *period_id,
            col_start: col_offset,
            num_weeks,
        });
        col_offset += num_weeks as u16;
    }

    let total_week_cols = col_offset - 5;
    if total_week_cols == 0 {
        return Ok(());
    }

    // -- Row 0: Period labels --
    for pl in &period_layout {
        let period_weeks = params
            .periods
            .find_period(pl.period_id)
            .expect("Period ID should be valid");

        let label = period_weeks
            .iter()
            .find_map(|w| w.annotation.as_ref())
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

    // -- Data rows --
    let mut row: u32 = 2;
    let mut first_subject = true;

    for (subject_id, subject) in &params.subjects.ordered_subject_list {
        let subject_name = &subject.parameters.name;

        // Get slots for this subject
        let Some(subject_slots) = params.slots.subject_map.get(subject_id) else {
            continue;
        };
        if subject_slots.ordered_slots.is_empty() {
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
        let slot_count = subject_slots.ordered_slots.len();

        for (slot_idx, (slot_id, slot)) in subject_slots.ordered_slots.iter().enumerate() {
            let (top_b, bot_b) = vertical_borders(slot_idx, slot_count);

            // Teacher info
            let (surname, contact) =
                if let Some(teacher) = params.teachers.teacher_map.get(&slot.teacher_id) {
                    let contact = teacher
                        .desc
                        .email
                        .as_ref()
                        .map(|e| e.as_str())
                        .or(teacher.desc.tel.as_ref().map(|t| t.as_str()))
                        .unwrap_or("");
                    (teacher.desc.surname.as_str(), contact)
                } else {
                    ("", "")
                };

            let slot_time = slot.start_time.capitalize();
            let room = &slot.extra_info;

            let data_fmt = formats::data_cell(top_b, bot_b, 2, 2);
            worksheet.write_with_format(row, 1, surname, &data_fmt)?;
            worksheet.write_with_format(row, 2, contact, &data_fmt)?;
            worksheet.write_with_format(row, 3, &slot_time, &data_fmt)?;
            worksheet.write_with_format(row, 4, room.as_str(), &data_fmt)?;

            // Week columns
            for pl in &period_layout {
                // Find group_list_id for this subject in this period
                let group_list_params = params
                    .group_lists
                    .subjects_associations
                    .get(&pl.period_id)
                    .and_then(|assocs| assocs.get(subject_id))
                    .and_then(|gl_id| params.group_lists.group_list_map.get(gl_id))
                    .map(|gl| &gl.params);

                // Find colloscope slot data
                let colloscope_slot = colloscope
                    .period_map
                    .get(&pl.period_id)
                    .and_then(|period| period.slot_map.get(slot_id));

                for w in 0..pl.num_weeks {
                    let col = pl.col_start + w as u16;
                    let (left_b, right_b) = period_border(w, pl.num_weeks);
                    let fmt = formats::week_cell(top_b, bot_b, left_b, right_b);

                    let cell_text =
                        cell_text_for_interrogation(colloscope_slot, w, group_list_params);

                    worksheet.write_with_format(row, col, &cell_text, &fmt)?;
                }
            }

            row += 1;
        }

        // Merge subject name vertically
        let subject_end_row = row - 1;
        let subject_fmt = formats::subject_cell(2, 2);
        if subject_start_row == subject_end_row {
            worksheet.write_with_format(
                subject_start_row,
                0,
                subject_name.as_str(),
                &subject_fmt,
            )?;
        } else {
            worksheet.merge_range(
                subject_start_row,
                0,
                subject_end_row,
                0,
                subject_name.as_str(),
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

fn cell_text_for_interrogation(
    colloscope_slot: Option<&collomatique_state_colloscopes::colloscopes::ColloscopeSlot>,
    week_index: usize,
    group_list_params: Option<&collomatique_state_colloscopes::group_lists::GroupListParameters>,
) -> String {
    let Some(cs) = colloscope_slot else {
        return String::new();
    };
    let Some(Some(interrog)) = cs.interrogations.get(week_index) else {
        return String::new();
    };

    let group_names: Vec<String> = interrog
        .assigned_groups
        .iter()
        .map(|&g| {
            if let Some(glp) = group_list_params {
                get_group_name(glp, g)
            } else {
                (g + 1).to_string()
            }
        })
        .collect();

    group_names.join(", ")
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
