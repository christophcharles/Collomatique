mod colloscope_sheet;
mod formats;
mod per_group_list_sheet;
mod per_student_groups_sheet;

use std::collections::BTreeMap;
use std::path::Path;

use rust_xlsxwriter::{Workbook, XlsxError};
use sqlx::{Row, SqlitePool};

/// rust_xlsxwriter paper-size index for A4 (210 × 297 mm)
const PAPER_SIZE_A4: u8 = 9;

/// Group sheets with at least this many group lists switch to landscape
const AUTO_LANDSCAPE_GROUP_LIST_THRESHOLD: usize = 4;

#[derive(Debug)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Debug)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub(crate) fn to_xlsx(&self) -> rust_xlsxwriter::Color {
        rust_xlsxwriter::Color::RGB(
            ((self.red as u32) << 16) | ((self.green as u32) << 8) | (self.blue as u32),
        )
    }
}

#[derive(Debug)]
pub enum Error {
    Xlsx(XlsxError),
    Sql(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Xlsx(e) => write!(f, "XLSX error: {e}"),
            Error::Sql(e) => write!(f, "SQL error: {e}"),
        }
    }
}

impl From<XlsxError> for Error {
    fn from(e: XlsxError) -> Self {
        Error::Xlsx(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::Sql(e)
    }
}

#[derive(Debug)]
pub struct GlobalConfig {
    pub background_color: Color,
    pub stripes_color: Option<Color>,
}

#[derive(Debug)]
pub struct ColloscopeConfig {
    pub sheet_name: String,
    pub extra_info_column_name: String,
    pub teacher_email: Option<String>,
    pub teacher_tel: Option<String>,
    pub orientation: PageOrientation,
    pub display_week_dates: bool,
    pub display_annotations: bool,
    pub no_interrogation_color: Color,
    pub annotation_color: Option<Color>,
    pub extra_colors: BTreeMap<String, Color>,
}

#[derive(Debug)]
pub struct PerStudentGroupsConfig {
    pub sheet_name: String,
    pub orientation: Option<PageOrientation>,
    pub show_emails: bool,
    pub show_tel: bool,
}

#[derive(Debug)]
pub struct PerGroupListConfig {
    pub orientation: PageOrientation,
    pub show_emails: bool,
    pub show_tel: bool,
    pub center_vertically: bool,
}

#[derive(Debug)]
pub struct Config {
    pub global: GlobalConfig,
    pub colloscope: Option<ColloscopeConfig>,
    pub all_groups: Option<PerStudentGroupsConfig>,
    pub automatic_groups: Option<PerStudentGroupsConfig>,
    pub prefilled_groups: Option<PerStudentGroupsConfig>,
    pub per_group_list: Option<PerGroupListConfig>,
}

pub(crate) fn sanitize_sheet_name(name: &str) -> Option<String> {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '[' | ']' | ':' | '*' | '?' | '/' | '\\' => '-',
            _ => c,
        })
        .collect();
    let trimmed = sanitized.trim().trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(31).collect())
}

fn apply_orientation(ws: &mut rust_xlsxwriter::Worksheet, orientation: &PageOrientation) {
    match orientation {
        PageOrientation::Portrait => ws.set_portrait(),
        PageOrientation::Landscape => ws.set_landscape(),
    };
}

pub async fn write_xlsx(pool: &SqlitePool, path: &Path, config: &Config) -> Result<(), Error> {
    let mut workbook = Workbook::new();

    if let Some(colloscope_config) = &config.colloscope {
        let colloscope_ws = workbook.add_worksheet();
        if let Some(safe_name) = sanitize_sheet_name(&colloscope_config.sheet_name) {
            colloscope_ws.set_name(&safe_name)?;
        }
        colloscope_sheet::build(colloscope_ws, pool, &config.global, colloscope_config).await?;
        colloscope_ws.set_paper_size(PAPER_SIZE_A4);
        colloscope_ws.set_print_center_horizontally(true);
        colloscope_ws.set_print_center_vertically(true);
        colloscope_ws.set_print_fit_to_pages(1, 1);
        apply_orientation(colloscope_ws, &colloscope_config.orientation);
    }

    if let Some(all_groups_config) = &config.all_groups {
        let all_groups_ws = workbook.add_worksheet();
        if let Some(safe_name) = sanitize_sheet_name(&all_groups_config.sheet_name) {
            all_groups_ws.set_name(&safe_name)?;
        }
        let gl_count = per_student_groups_sheet::build_all(
            all_groups_ws,
            pool,
            &config.global,
            all_groups_config.show_emails,
            all_groups_config.show_tel,
        )
        .await?;
        all_groups_ws.set_paper_size(PAPER_SIZE_A4);
        all_groups_ws.set_print_center_horizontally(true);
        all_groups_ws.set_print_center_vertically(true);
        all_groups_ws.set_print_fit_to_pages(1, 1);
        let orientation = all_groups_config.orientation.as_ref().unwrap_or(
            if gl_count >= AUTO_LANDSCAPE_GROUP_LIST_THRESHOLD {
                &PageOrientation::Landscape
            } else {
                &PageOrientation::Portrait
            },
        );
        apply_orientation(all_groups_ws, orientation);
    }

    if let Some(automatic_groups_config) = &config.automatic_groups {
        let has_automatic_groups: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM group_lists WHERE filling_type = 'automatic'",
        )
        .fetch_one(pool)
        .await?
            > 0;

        if has_automatic_groups {
            let groups_ws = workbook.add_worksheet();
            if let Some(safe_name) = sanitize_sheet_name(&automatic_groups_config.sheet_name) {
                groups_ws.set_name(&safe_name)?;
            }
            let gl_count = per_student_groups_sheet::build_automatic(
                groups_ws,
                pool,
                &config.global,
                automatic_groups_config.show_emails,
                automatic_groups_config.show_tel,
            )
            .await?;
            groups_ws.set_paper_size(PAPER_SIZE_A4);
            groups_ws.set_print_center_horizontally(true);
            groups_ws.set_print_center_vertically(true);
            groups_ws.set_print_fit_to_pages(1, 1);
            let orientation = automatic_groups_config.orientation.as_ref().unwrap_or(
                if gl_count >= AUTO_LANDSCAPE_GROUP_LIST_THRESHOLD {
                    &PageOrientation::Landscape
                } else {
                    &PageOrientation::Portrait
                },
            );
            apply_orientation(groups_ws, orientation);
        }
    }

    if let Some(prefilled_groups_config) = &config.prefilled_groups {
        let has_prefilled_groups: bool = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM group_lists WHERE filling_type = 'prefilled'",
        )
        .fetch_one(pool)
        .await?
            > 0;

        if has_prefilled_groups {
            let prefilled_ws = workbook.add_worksheet();
            if let Some(safe_name) = sanitize_sheet_name(&prefilled_groups_config.sheet_name) {
                prefilled_ws.set_name(&safe_name)?;
            }
            let gl_count = per_student_groups_sheet::build_prefilled(
                prefilled_ws,
                pool,
                &config.global,
                prefilled_groups_config.show_emails,
                prefilled_groups_config.show_tel,
            )
            .await?;
            prefilled_ws.set_paper_size(PAPER_SIZE_A4);
            prefilled_ws.set_print_center_horizontally(true);
            prefilled_ws.set_print_center_vertically(true);
            prefilled_ws.set_print_fit_to_pages(1, 1);
            let orientation = prefilled_groups_config.orientation.as_ref().unwrap_or(
                if gl_count >= AUTO_LANDSCAPE_GROUP_LIST_THRESHOLD {
                    &PageOrientation::Landscape
                } else {
                    &PageOrientation::Portrait
                },
            );
            apply_orientation(prefilled_ws, orientation);
        }
    }

    if let Some(per_group_list_config) = &config.per_group_list {
        let group_lists = sqlx::query(
            "SELECT DISTINCT gl.id, gl.name \
             FROM group_lists gl \
             WHERE EXISTS (SELECT 1 FROM colloscope_group_list_students WHERE group_list_id = gl.id) \
                OR EXISTS (SELECT 1 FROM prefilled_group_students WHERE group_list_id = gl.id) \
             ORDER BY gl.name",
        )
        .fetch_all(pool)
        .await?;

        for gl_row in &group_lists {
            let gl_id: i64 = gl_row.get(0);
            let gl_name: String = gl_row.get(1);

            let ws = workbook.add_worksheet();
            if let Some(safe_name) = sanitize_sheet_name(&gl_name) {
                ws.set_name(&safe_name)?;
            }
            per_group_list_sheet::build(
                ws,
                pool,
                &config.global,
                gl_id,
                &gl_name,
                per_group_list_config.show_emails,
                per_group_list_config.show_tel,
            )
            .await?;
            ws.set_paper_size(PAPER_SIZE_A4);
            ws.set_print_center_horizontally(true);
            ws.set_print_center_vertically(per_group_list_config.center_vertically);
            ws.set_print_fit_to_pages(1, 1);
            apply_orientation(ws, &per_group_list_config.orientation);
        }
    }

    workbook.save(path)?;
    Ok(())
}

pub(crate) fn generate_week_dates_title(first_week_str: &str, week_num: usize) -> Option<String> {
    let global_monday = chrono::NaiveDate::parse_from_str(first_week_str, "%Y-%m-%d").ok()?;
    let start = global_monday.checked_add_days(chrono::Days::new(7 * week_num as u64))?;
    let end = start.checked_add_days(chrono::Days::new(6))?;
    Some(format!(
        "  Du {} au {}  ",
        start.format("%d/%m/%Y"),
        end.format("%d/%m/%Y"),
    ))
}

pub(crate) fn generate_period_title(
    first_week_str: &Option<String>,
    period_index: usize,
    first_week_num: usize,
    week_count: usize,
) -> String {
    if week_count == 0 {
        return format!("Période {} (vide)", period_index + 1);
    }

    match first_week_str {
        Some(date_str) => {
            let Ok(global_monday) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
                return format!("Période {}", period_index + 1);
            };
            let start = global_monday
                .checked_add_days(chrono::Days::new(7 * first_week_num as u64))
                .expect("Valid start date");
            let end = start
                .checked_add_days(chrono::Days::new(7 * week_count as u64 - 1))
                .expect("Valid end date");
            format!(
                "Période {} du {} au {}",
                period_index + 1,
                start.format("%d/%m/%Y"),
                end.format("%d/%m/%Y"),
            )
        }
        None => format!("Période {}", period_index + 1),
    }
}

pub(crate) fn get_group_name(group_names: &[String], group_num: i64) -> String {
    let idx = group_num as usize;
    if let Some(name) = group_names.get(idx) {
        if !name.is_empty() {
            return name.to_string();
        }
    }
    (group_num + 1).to_string()
}

pub(crate) fn format_slot_time(day: i64, start_time_minutes: i64) -> String {
    let day_name = match day {
        0 => "Lundi",
        1 => "Mardi",
        2 => "Mercredi",
        3 => "Jeudi",
        4 => "Vendredi",
        5 => "Samedi",
        6 => "Dimanche",
        _ => "?",
    };
    let hours = start_time_minutes / 60;
    let minutes = start_time_minutes % 60;
    format!("{day_name} {hours:02}h{minutes:02}")
}
