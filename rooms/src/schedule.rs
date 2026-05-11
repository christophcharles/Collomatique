use std::path::Path;

use collomatique_time::Weekday;
use thiserror::Error;

const ROOMS_FIXED_COLUMNS: &[&str] = &["Salle", "Étage", "X", "Y"];

const REQUESTS_FIXED_COLUMNS: &[&str] = &[
    "Période",
    "Jour",
    "Heure",
    "Matière",
    "Responsable",
    "Colleur",
    "Étage",
    "X",
    "Y",
    "Prep",
];

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("Error reading CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("In rooms file: missing expected column \"{0}\"")]
    RoomsMissingColumn(String),
    #[error("In requests file: missing expected column \"{0}\"")]
    RequestsMissingColumn(String),
    #[error("In rooms file: unknown column \"{0}\"")]
    RoomsUnknownColumn(String),
    #[error("In requests file: unknown column \"{0}\"")]
    RequestsUnknownColumn(String),
    #[error(
        "Characteristic columns mismatch: rooms file has {expected:?} but requests file has {actual:?}"
    )]
    CharacteristicsMismatch {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    #[error("In rooms file, row {row}: {message}")]
    RoomsRowError { row: usize, message: String },
    #[error("In requests file, row {row}: {message}")]
    RequestsRowError { row: usize, message: String },
    #[error("In requests file, row {row}, characteristic \"{name}\": min ({min}) > max ({max})")]
    MinGreaterThanMax {
        row: usize,
        name: String,
        min: i32,
        max: i32,
    },
    #[error("In requests file, row {row}: invalid day \"{value}\"")]
    InvalidDay { row: usize, value: String },
}

#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub floor: i32,
    pub x: f64,
    pub y: f64,
    pub characteristic_values: Vec<i32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CharacteristicConstraint {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub period: i32,
    pub day: Weekday,
    pub hour: i32,
    pub subject: String,
    pub responsible: String,
    pub colleur: String,
    pub floor: i32,
    pub x: f64,
    pub y: f64,
    pub prep: bool,
    pub constraints: Vec<CharacteristicConstraint>,
}

#[derive(Debug, Clone)]
pub struct RoomScheduleData {
    pub characteristics: Vec<String>,
    pub rooms: Vec<Room>,
    pub requests: Vec<Request>,
}

pub fn parse_schedule(
    rooms_path: &Path,
    requests_path: &Path,
) -> Result<RoomScheduleData, ScheduleError> {
    let (characteristics, rooms) = parse_rooms(rooms_path)?;
    let requests = parse_requests(requests_path, &characteristics)?;
    Ok(RoomScheduleData {
        characteristics,
        rooms,
        requests,
    })
}

fn parse_rooms(path: &Path) -> Result<(Vec<String>, Vec<Room>), ScheduleError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();

    for (i, expected) in ROOMS_FIXED_COLUMNS.iter().enumerate() {
        match headers.get(i) {
            Some(h) if h.trim() == *expected => {}
            _ => return Err(ScheduleError::RoomsMissingColumn(expected.to_string())),
        }
    }

    let mut characteristics = Vec::new();
    for i in ROOMS_FIXED_COLUMNS.len()..headers.len() {
        let name = headers.get(i).unwrap_or("").trim();
        if name.is_empty() {
            return Err(ScheduleError::RoomsUnknownColumn(String::new()));
        }
        characteristics.push(name.to_string());
    }

    let expected_len = ROOMS_FIXED_COLUMNS.len() + characteristics.len();
    let mut rooms = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result?;
        let row = idx + 1;

        if record.len() != expected_len {
            return Err(ScheduleError::RoomsRowError {
                row,
                message: format!("expected {} columns, got {}", expected_len, record.len()),
            });
        }

        let name = record.get(0).unwrap().trim().to_string();
        let floor = parse_field::<i32>(&record, 1, row, "rooms", "Étage")?;
        let x = parse_field::<f64>(&record, 2, row, "rooms", "X")?;
        let y = parse_field::<f64>(&record, 3, row, "rooms", "Y")?;

        let mut characteristic_values = Vec::with_capacity(characteristics.len());
        for (i, char_name) in characteristics.iter().enumerate() {
            characteristic_values.push(parse_field::<i32>(
                &record,
                ROOMS_FIXED_COLUMNS.len() + i,
                row,
                "rooms",
                char_name,
            )?);
        }

        rooms.push(Room {
            name,
            floor,
            x,
            y,
            characteristic_values,
        });
    }

    Ok((characteristics, rooms))
}

fn parse_requests(path: &Path, characteristics: &[String]) -> Result<Vec<Request>, ScheduleError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();

    for (i, expected) in REQUESTS_FIXED_COLUMNS.iter().enumerate() {
        match headers.get(i) {
            Some(h) if h.trim() == *expected => {}
            _ => return Err(ScheduleError::RequestsMissingColumn(expected.to_string())),
        }
    }

    let expected_extra: Vec<String> = characteristics
        .iter()
        .flat_map(|name| [format!("{name} Min"), format!("{name} Max")])
        .collect();

    let actual_extra: Vec<String> = (REQUESTS_FIXED_COLUMNS.len()..headers.len())
        .map(|i| headers.get(i).unwrap_or("").trim().to_string())
        .collect();

    if actual_extra != expected_extra {
        let actual_chars: Vec<String> = actual_extra
            .chunks(2)
            .filter_map(|pair| {
                pair.first()
                    .and_then(|s| s.strip_suffix(" Min"))
                    .map(|s| s.to_string())
            })
            .collect();

        for col in &actual_extra {
            if !expected_extra.contains(col) {
                return Err(ScheduleError::RequestsUnknownColumn(col.clone()));
            }
        }
        for col in &expected_extra {
            if !actual_extra.contains(col) {
                return Err(ScheduleError::RequestsMissingColumn(col.clone()));
            }
        }

        return Err(ScheduleError::CharacteristicsMismatch {
            expected: characteristics.to_vec(),
            actual: actual_chars,
        });
    }

    let expected_len = REQUESTS_FIXED_COLUMNS.len() + 2 * characteristics.len();
    let mut requests = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result?;
        let row = idx + 1;

        if record.len() != expected_len {
            return Err(ScheduleError::RequestsRowError {
                row,
                message: format!("expected {} columns, got {}", expected_len, record.len()),
            });
        }

        let period = parse_field::<i32>(&record, 0, row, "requests", "Période")?;

        let day_str = record.get(1).unwrap().trim();
        let day = Weekday::from_french(day_str).ok_or_else(|| ScheduleError::InvalidDay {
            row,
            value: day_str.to_string(),
        })?;

        let hour = parse_field::<i32>(&record, 2, row, "requests", "Heure")?;
        let subject = record.get(3).unwrap().trim().to_string();
        let responsible = record.get(4).unwrap().trim().to_string();
        let colleur = record.get(5).unwrap().trim().to_string();
        let floor = parse_field::<i32>(&record, 6, row, "requests", "Étage")?;
        let x = parse_field::<f64>(&record, 7, row, "requests", "X")?;
        let y = parse_field::<f64>(&record, 8, row, "requests", "Y")?;

        let prep_val = parse_field::<i32>(&record, 9, row, "requests", "Prep")?;
        let prep = match prep_val {
            0 => false,
            1 => true,
            other => {
                return Err(ScheduleError::RequestsRowError {
                    row,
                    message: format!("Prep must be 0 or 1, got {other}"),
                });
            }
        };

        let mut constraints = Vec::with_capacity(characteristics.len());
        for (i, char_name) in characteristics.iter().enumerate() {
            let min_idx = REQUESTS_FIXED_COLUMNS.len() + 2 * i;
            let max_idx = min_idx + 1;

            let min = parse_optional_field::<i32>(
                &record,
                min_idx,
                row,
                "requests",
                &format!("{char_name} Min"),
            )?;
            let max = parse_optional_field::<i32>(
                &record,
                max_idx,
                row,
                "requests",
                &format!("{char_name} Max"),
            )?;

            if let (Some(min_v), Some(max_v)) = (min, max) {
                if min_v > max_v {
                    return Err(ScheduleError::MinGreaterThanMax {
                        row,
                        name: char_name.clone(),
                        min: min_v,
                        max: max_v,
                    });
                }
            }

            constraints.push(CharacteristicConstraint { min, max });
        }

        requests.push(Request {
            period,
            day,
            hour,
            subject,
            responsible,
            colleur,
            floor,
            x,
            y,
            prep,
            constraints,
        });
    }

    Ok(requests)
}

fn parse_field<T: std::str::FromStr>(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
    file: &str,
    column: &str,
) -> Result<T, ScheduleError> {
    let value = record.get(index).unwrap().trim();
    value.parse::<T>().map_err(|_| {
        let err = match file {
            "rooms" => ScheduleError::RoomsRowError {
                row,
                message: format!("cannot parse \"{value}\" in column \"{column}\""),
            },
            _ => ScheduleError::RequestsRowError {
                row,
                message: format!("cannot parse \"{value}\" in column \"{column}\""),
            },
        };
        err
    })
}

fn parse_optional_field<T: std::str::FromStr>(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
    file: &str,
    column: &str,
) -> Result<Option<T>, ScheduleError> {
    let value = record.get(index).unwrap().trim();
    if value.is_empty() {
        return Ok(None);
    }
    parse_field::<T>(record, index, row, file, column).map(Some)
}
