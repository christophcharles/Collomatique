use std::collections::{BTreeMap, HashMap, HashSet};

use rust_xlsxwriter::{Url, Worksheet};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::ids::{GroupListId, PeriodId};

use crate::Error;
use crate::format_slot_time;
use crate::formats;
use crate::get_group_name;

struct FixedColumns {
    subject_col: u16,
    teacher_col: u16,
    email_col: Option<u16>,
    tel_col: Option<u16>,
    slot_col: u16,
    extra_info_col: Option<u16>,
    count: u16,
}

impl FixedColumns {
    fn from_config(colloscope: &crate::ColloscopeConfig) -> Self {
        let subject_col = 0;
        let teacher_col = 1;
        let mut next = 2u16;

        let email_col = if colloscope.teacher_email.is_some() {
            let col = next;
            next += 1;
            Some(col)
        } else {
            None
        };

        let tel_col = if colloscope.teacher_tel.is_some() {
            let col = next;
            next += 1;
            Some(col)
        } else {
            None
        };

        let slot_col = next;
        next += 1;

        let extra_info_col = if colloscope.extra_info_column_name.is_some() {
            let col = next;
            next += 1;
            Some(col)
        } else {
            None
        };

        FixedColumns {
            subject_col,
            teacher_col,
            email_col,
            tel_col,
            slot_col,
            extra_info_col,
            count: next,
        }
    }
}

struct PeriodLayout {
    period_id: PeriodId,
    col_start: u16,
    num_weeks: usize,
    period_index: usize,
    first_week_num: usize,
}

pub fn build(
    worksheet: &mut Worksheet,
    data: &InnerData,
    global: &crate::GlobalConfig,
    colloscope: &crate::ColloscopeConfig,
) -> Result<(), Error> {
    let params = &data.params;
    let cols = FixedColumns::from_config(colloscope);

    let bg = global.background_color.to_xlsx();
    let stripe = global
        .stripes_color
        .as_ref()
        .map(|c| c.to_xlsx())
        .unwrap_or(bg);

    // 1. Period layout — periods in order, with week count (periods without weeks are skipped)
    let mut period_layout = Vec::new();
    let mut col_offset: u16 = cols.count;
    let mut accumulated_weeks: usize = 0;
    for (period_index, (period_id, weeks)) in params
        .periods
        .ordered_period_list
        .entries()
        .filter(|(_period_id, weeks)| !weeks.is_empty())
        .enumerate()
    {
        let nw = weeks.len();
        period_layout.push(PeriodLayout {
            period_id,
            col_start: col_offset,
            num_weeks: nw,
            period_index,
            first_week_num: accumulated_weeks,
        });
        col_offset += nw as u16;
        accumulated_weeks += nw;
    }

    let total_week_cols = col_offset - cols.count;

    // 2. First monday of the colloscope for period title date ranges
    let first_week: Option<chrono::NaiveDate> =
        params.periods.first_week.as_ref().map(|w| *w.monday());

    let show_week_dates = colloscope.display_week_dates && first_week.is_some();
    let header_row_offset: u32 = if show_week_dates { 1 } else { 0 };

    // Weeks with no interrogations — for distinct background color
    let mut no_interrog_weeks: HashSet<(PeriodId, usize)> = HashSet::new();
    // Annotations — collected early so we can use them for week background colors
    let mut annotations: HashMap<(PeriodId, usize), String> = HashMap::new();
    for (period_id, weeks) in params.periods.ordered_period_list.entries() {
        for (week_index, week) in weeks.iter().enumerate() {
            if !week.interrogations {
                no_interrog_weeks.insert((period_id, week_index));
            }
            if let Some(annotation) = &week.annotation {
                annotations.insert((period_id, week_index), annotation.to_string());
            }
        }
    }

    let no_interrog_bg = colloscope.no_interrogation_color.to_xlsx();
    let annotation_bg = colloscope.annotation_color.as_ref().map(|c| c.to_xlsx());

    let extra_colors_xlsx: BTreeMap<&str, rust_xlsxwriter::Color> = colloscope
        .extra_colors
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_xlsx()))
        .collect();

    let week_bg = |period_id: PeriodId,
                   week_index: usize,
                   default_bg: rust_xlsxwriter::Color|
     -> rust_xlsxwriter::Color {
        // 1. extra_colors match has highest priority
        if let Some(annotation_text) = annotations.get(&(period_id, week_index)) {
            if let Some(&color) = extra_colors_xlsx.get(annotation_text.as_str()) {
                return color;
            }
        }
        // 2. no-interrogation week
        if no_interrog_weeks.contains(&(period_id, week_index)) {
            return no_interrog_bg;
        }
        // 3. generic annotation color
        if let Some(abg) = annotation_bg {
            if annotations.contains_key(&(period_id, week_index)) {
                return abg;
            }
        }
        // 4. default
        default_bg
    };

    // -- Row 0: Period labels --
    for pl in &period_layout {
        let label = crate::generate_period_title(
            &first_week,
            pl.period_index,
            pl.first_week_num,
            pl.num_weeks,
        );

        let fmt = formats::period_header(bg);
        if pl.num_weeks == 1 {
            worksheet.write_with_format(0, pl.col_start, &label, &fmt)?;
        } else {
            worksheet.merge_range(
                0,
                pl.col_start,
                0,
                pl.col_start + pl.num_weeks as u16 - 1,
                &label,
                &fmt,
            )?;
        }
    }

    // -- Row 1 (optional): Week date ranges --
    if let Some(first_week) = &first_week {
        if show_week_dates {
            for pl in &period_layout {
                for w in 0..pl.num_weeks {
                    let col = pl.col_start + w as u16;
                    let (left, right) = period_border(w, pl.num_weeks);
                    let fmt = formats::week_dates(left, right, week_bg(pl.period_id, w, bg));
                    let label = crate::generate_week_dates_title(first_week, pl.first_week_num + w)
                        .unwrap_or_default();
                    worksheet.write_with_format(1, col, &label, &fmt)?;
                }
            }
        }
    }

    // -- Fixed headers + week numbers --
    let header_row = 1 + header_row_offset;
    let header_fmt = formats::header(bg);
    worksheet.write_with_format(header_row, cols.subject_col, "Matière", &header_fmt)?;
    worksheet.write_with_format(header_row, cols.teacher_col, "Colleur", &header_fmt)?;
    if let Some(email_col) = cols.email_col {
        let name = colloscope.teacher_email.as_deref().unwrap_or("Email");
        worksheet.write_with_format(header_row, email_col, name, &header_fmt)?;
    }
    if let Some(tel_col) = cols.tel_col {
        let name = colloscope.teacher_tel.as_deref().unwrap_or("Tél");
        worksheet.write_with_format(header_row, tel_col, name, &header_fmt)?;
    }
    worksheet.write_with_format(header_row, cols.slot_col, "Créneau", &header_fmt)?;
    if let Some(extra_info_col) = cols.extra_info_col {
        let name = colloscope
            .extra_info_column_name
            .as_deref()
            .unwrap_or("Info");
        worksheet.write_with_format(header_row, extra_info_col, name, &header_fmt)?;
    }

    let mut week_counter: u32 = 1;
    for pl in &period_layout {
        for w in 0..pl.num_weeks {
            let col = pl.col_start + w as u16;
            let (left, right) = period_border(w, pl.num_weeks);
            let fmt = formats::week_header(left, right, week_bg(pl.period_id, w, bg));
            worksheet.write_with_format(header_row, col, format!("S{week_counter}"), &fmt)?;
            week_counter += 1;
        }
    }

    // 3. Group names: group_list_id -> Vec<String>
    let group_names_map: HashMap<GroupListId, Vec<String>> = params
        .group_lists
        .group_list_map
        .entries()
        .map(|(gl_id, gl)| (gl_id, crate::group_names_vec(gl)))
        .collect();

    // -- Data rows --
    let mut row: u32 = 2 + header_row_offset;
    let mut first_subject = true;
    let mut stripe_index: usize = 0;

    for (subject_id, subject) in params.subjects.ordered_subject_list.entries() {
        let subject_id = &subject_id;
        let subject_name = &subject.parameters.name;

        // Slots for this subject, in order
        let slots = match params.slots.subject_map.get(subject_id) {
            Some(subject_slots) => &subject_slots.ordered_slots,
            None => continue,
        };

        if slots.is_empty() {
            continue;
        }

        // Separator row between subjects
        if !first_subject {
            for c in 0..cols.count {
                worksheet.write_with_format(row, c, "", &formats::empty_row(2, 2, bg))?;
            }
            for pl in &period_layout {
                for w in 0..pl.num_weeks {
                    let (left, right) = period_border(w, pl.num_weeks);
                    let fmt = formats::empty_row(left, right, week_bg(pl.period_id, w, bg));
                    worksheet.write_with_format(row, pl.col_start + w as u16, "", &fmt)?;
                }
            }
            row += 1;
        }
        first_subject = false;

        let subject_start_row = row;
        let slot_count = slots.len();

        for (slot_idx, (slot_id, slot)) in slots.iter().enumerate() {
            let teacher = params.teachers.teacher_map.get(&slot.teacher_id);
            let surname = teacher.map(|t| t.desc.surname.clone()).unwrap_or_default();
            let email = teacher
                .and_then(|t| t.desc.email.as_ref())
                .map(|e| e.to_string())
                .unwrap_or_default();
            let tel = teacher
                .and_then(|t| t.desc.tel.as_ref())
                .map(|t| t.to_string())
                .unwrap_or_default();
            let day = slot.start_time.weekday.num_days_from_monday() as i64;
            let start_time = slot.start_time.start_time.minutes_from_midnight() as i64;
            let extra_info = &slot.extra_info;

            let (top_b, bot_b) = vertical_borders(slot_idx, slot_count);
            let row_bg = if stripe_index % 2 == 0 { stripe } else { bg };

            let slot_time = format_slot_time(day, start_time);

            let data_fmt = formats::data_cell(top_b, bot_b, 2, 2, row_bg);
            worksheet.write_with_format(row, cols.teacher_col, &surname, &data_fmt)?;
            if let Some(email_col) = cols.email_col {
                if email.is_empty() {
                    worksheet.write_with_format(row, email_col, "", &data_fmt)?;
                } else {
                    let url = Url::new(format!("mailto:{email}")).set_text(&email);
                    worksheet.write_url_with_format(row, email_col, url, &data_fmt)?;
                }
            }
            if let Some(tel_col) = cols.tel_col {
                worksheet.write_with_format(row, tel_col, &tel, &data_fmt)?;
            }
            worksheet.write_with_format(row, cols.slot_col, &slot_time, &data_fmt)?;
            if let Some(extra_info_col) = cols.extra_info_col {
                worksheet.write_with_format(row, extra_info_col, extra_info, &data_fmt)?;
            }

            // Week columns
            for pl in &period_layout {
                let group_names = params
                    .group_lists
                    .subjects_associations
                    .get(&pl.period_id)
                    .and_then(|subject_map| subject_map.get(subject_id))
                    .and_then(|gl_id| group_names_map.get(gl_id));

                let colloscope_slot = data
                    .colloscope
                    .period_map
                    .get(&pl.period_id)
                    .and_then(|period| period.slot_map.get(slot_id));

                for w in 0..pl.num_weeks {
                    let col = pl.col_start + w as u16;
                    let (left_b, right_b) = period_border(w, pl.num_weeks);
                    let fmt = formats::week_cell(
                        top_b,
                        bot_b,
                        left_b,
                        right_b,
                        week_bg(pl.period_id, w, row_bg),
                    );

                    let cell_text = colloscope_slot
                        .and_then(|slot| slot.interrogations.get(w))
                        .and_then(|interrogation| interrogation.as_ref())
                        .map(|interrogation| {
                            interrogation
                                .assigned_groups
                                .iter()
                                .map(|&g| {
                                    let g = g as i64;
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

            stripe_index += 1;
            row += 1;
        }

        // Merge subject name vertically
        let subject_end_row = row - 1;
        let subject_fmt = formats::subject_cell(2, 2, bg);
        if subject_start_row == subject_end_row {
            worksheet.write_with_format(
                subject_start_row,
                cols.subject_col,
                subject_name,
                &subject_fmt,
            )?;
        } else {
            worksheet.merge_range(
                subject_start_row,
                cols.subject_col,
                subject_end_row,
                cols.subject_col,
                subject_name,
                &subject_fmt,
            )?;
        }
    }

    // Annotation row
    if colloscope.display_annotations && !annotations.is_empty() {
        for pl in &period_layout {
            for w in 0..pl.num_weeks {
                if let Some(text) = annotations.get(&(pl.period_id, w)) {
                    let col = pl.col_start + w as u16;
                    let fmt = formats::annotation(bg);
                    worksheet.write_with_format(row, col, format!("{text} "), &fmt)?;
                }
            }
        }
    }

    // Column widths
    worksheet.set_column_width(cols.subject_col, 14)?;
    worksheet.set_column_width(cols.teacher_col, 14)?;
    if let Some(email_col) = cols.email_col {
        worksheet.set_column_width(email_col, 22)?;
    }
    if let Some(tel_col) = cols.tel_col {
        worksheet.set_column_width(tel_col, 14)?;
    }
    worksheet.set_column_width(cols.slot_col, 14)?;
    if let Some(extra_info_col) = cols.extra_info_col {
        worksheet.set_column_width(extra_info_col, 10)?;
    }
    if total_week_cols > 0 {
        worksheet.set_column_range_width(cols.count, cols.count + total_week_cols - 1, 5)?;
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
