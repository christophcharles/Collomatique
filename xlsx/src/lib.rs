mod colloscope_sheet;
mod formats;
mod groups_sheet;

use std::path::Path;

use rust_xlsxwriter::{Workbook, XlsxError};
use sqlx::SqlitePool;

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

pub struct Config {
    pub extra_info_column_name: String,
    pub teacher_email: Option<String>,
    pub teacher_tel: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            extra_info_column_name: "Info".into(),
            teacher_email: Some("Contact".into()),
            teacher_tel: None,
        }
    }
}

pub async fn write_xlsx(pool: &SqlitePool, path: &Path, config: &Config) -> Result<(), Error> {
    let mut workbook = Workbook::new();

    let colloscope_ws = workbook.add_worksheet();
    colloscope_ws.set_name("Colloscope")?;
    colloscope_sheet::build(colloscope_ws, pool, config).await?;

    let has_automatic_groups: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM group_lists WHERE filling_type = 'automatic'",
    )
    .fetch_one(pool)
    .await?
        > 0;

    if has_automatic_groups {
        let groups_ws = workbook.add_worksheet();
        groups_ws.set_name("Groupes automatiques")?;
        groups_sheet::build(groups_ws, pool).await?;
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
