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

pub async fn write_xlsx(pool: &SqlitePool, path: &Path) -> Result<(), Error> {
    let mut workbook = Workbook::new();

    let colloscope_ws = workbook.add_worksheet();
    colloscope_ws.set_name("Colloscope")?;
    colloscope_sheet::build(colloscope_ws, pool).await?;

    let groups_ws = workbook.add_worksheet();
    groups_ws.set_name("Groupes")?;
    groups_sheet::build(groups_ws, pool).await?;

    workbook.save(path)?;
    Ok(())
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
