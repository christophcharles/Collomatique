mod automatic_groups_sheet;
mod colloscope_sheet;
mod formats;
mod prefilled_groups_sheet;

use std::path::Path;

use rust_xlsxwriter::{Workbook, XlsxError};
use sqlx::SqlitePool;

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

pub struct GlobalConfig {
    pub background_color: Color,
    pub stripes_color: Option<Color>,
}

pub struct ColloscopeConfig {
    pub sheet_name: String,
    pub extra_info_column_name: String,
    pub teacher_email: Option<String>,
    pub teacher_tel: Option<String>,
}

pub struct AutomaticGroupsConfig {
    pub sheet_name: String,
}

pub struct PrefilledGroupsConfig {
    pub sheet_name: String,
}

pub struct Config {
    pub global: GlobalConfig,
    pub colloscope: Option<ColloscopeConfig>,
    pub automatic_groups: Option<AutomaticGroupsConfig>,
    pub prefilled_groups: Option<PrefilledGroupsConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            global: GlobalConfig {
                background_color: Color::new(255, 255, 255),
                stripes_color: Some(Color::new(220, 220, 230)),
            },
            colloscope: Some(ColloscopeConfig {
                sheet_name: "Colloscope".into(),
                extra_info_column_name: "Info".into(),
                teacher_email: Some("Contact".into()),
                teacher_tel: None,
            }),
            automatic_groups: Some(AutomaticGroupsConfig {
                sheet_name: "Groupes automatiques".into(),
            }),
            prefilled_groups: Some(PrefilledGroupsConfig {
                sheet_name: "Groupes préremplis".into(),
            }),
        }
    }
}

pub async fn write_xlsx(pool: &SqlitePool, path: &Path, config: &Config) -> Result<(), Error> {
    let mut workbook = Workbook::new();

    if let Some(colloscope_config) = &config.colloscope {
        let colloscope_ws = workbook.add_worksheet();
        colloscope_ws.set_name(&colloscope_config.sheet_name)?;
        colloscope_sheet::build(colloscope_ws, pool, &config.global, colloscope_config).await?;
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
            groups_ws.set_name(&automatic_groups_config.sheet_name)?;
            automatic_groups_sheet::build(groups_ws, pool, &config.global).await?;
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
            prefilled_ws.set_name(&prefilled_groups_config.sheet_name)?;
            prefilled_groups_sheet::build(prefilled_ws, pool, &config.global).await?;
        }
    }

    workbook.save(path)?;
    Ok(())
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
