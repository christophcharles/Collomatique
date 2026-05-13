use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::path::Path;

use collomatique_rooms_model::{
    Config, Hour, Incompat, Periods, Request, Room, RoomPreference, ScheduleData, Window,
};
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

const ROOMS_COLUMNS: &[&str] = &[
    "Salle",
    "Étage",
    "X",
    "Y",
    "Tableaux noirs",
    "Tableaux blancs",
    "Capacité",
    "Fenêtre",
    "Priorité",
    "Réservée",
];

const REQUESTS_COLUMNS: &[&str] = &[
    "P1",
    "P2",
    "P3",
    "Jour",
    "Heure",
    "Discipline",
    "Classes",
    "Responsable",
    "Colleur",
    "Tableaux",
    "Fenêtre",
    "Nb élèves",
    "Nb prep",
    "Salle",
    "Prep",
];

const INCOMPATS_COLUMNS: &[&str] = &["Salle", "P1", "P2", "P3", "Jour", "Heure"];

const ALLOWED_SUBJECTS: &[&str] = &[
    "Mathématiques",
    "Physique",
    "Chimie",
    "Physique-Chimie",
    "Sciences de l'ingénieur",
    "Sciences de la Vie et de la Terre",
    "Informatique",
    "Français",
    "Lettres",
    "Philosophie",
    "Lettres-Philosophie",
    "Histoire",
    "Géographie",
    "Histoire-Géographie-Géopolitique",
    "Économie, Sociologie et Histoire du monde contemporain",
    "Anglais",
    "Espagnol",
    "Allemand",
    "Italien",
    "Latin",
    "Grec",
];

const ALLOWED_CLASSES: &[&str] = &[
    "MPSI", "MP2I", "MP", "MPI", "MP*", "MPI*", "PCSI 1", "PCSI 2", "PC", "PC*", "PCC", "BCPST 1",
    "BCPST 2", "ECG 1A", "ECG 1B", "ECG 2A", "ECG 2B", "LS 1", "LS 2",
];

/// Errors that can occur while parsing schedule CSV files.
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
    #[error("In rooms file: unexpected column \"{0}\"")]
    RoomsUnknownColumn(String),
    #[error("In requests file: unexpected column \"{0}\"")]
    RequestsUnknownColumn(String),
    #[error("In rooms file, row {row}: {message}")]
    RoomsRowError { row: usize, message: String },
    #[error("In requests file, row {row}: {message}")]
    RequestsRowError { row: usize, message: String },
    #[error("In requests file, row {row}: invalid day \"{value}\"")]
    InvalidDay { row: usize, value: String },
    #[error("In requests file, row {row}: hour must be between 8 and 19, got {value}")]
    InvalidHour { row: usize, value: u32 },
    #[error(
        "In rooms file: duplicate room name \"{name}\" (first defined at row {first_row}, duplicated at row {duplicate_row})"
    )]
    RoomsDuplicateName {
        name: String,
        first_row: usize,
        duplicate_row: usize,
    },
    #[error("In incompats file: missing expected column \"{0}\"")]
    IncompatsMissingColumn(String),
    #[error("In incompats file: unexpected column \"{0}\"")]
    IncompatsUnknownColumn(String),
    #[error("In incompats file, row {row}: {message}")]
    IncompatsRowError { row: usize, message: String },
    #[error("In incompats file, row {row}: room \"{room}\" is not declared in the rooms file")]
    IncompatsUndeclaredRoom { row: usize, room: String },
}

/// Parse a rooms CSV and a requests CSV into a [`ScheduleData`].
pub fn parse_schedule(
    rooms_path: &Path,
    requests_path: &Path,
    incompats_path: Option<&Path>,
    config: Config,
) -> Result<ScheduleData, ScheduleError> {
    let rooms = parse_rooms(rooms_path)?;
    let requests = parse_requests(requests_path)?;
    let incompats = match incompats_path {
        Some(path) => parse_incompats(path)?,
        None => Vec::new(),
    };

    for (idx, incompat) in incompats.iter().enumerate() {
        if !rooms.contains_key(&incompat.room) {
            return Err(ScheduleError::IncompatsUndeclaredRoom {
                row: idx + 1,
                room: incompat.room.to_string(),
            });
        }
    }

    Ok(ScheduleData {
        rooms,
        requests,
        incompats,
        config,
    })
}

/// Parse a rooms CSV file.
pub fn parse_rooms(path: &Path) -> Result<BTreeMap<NonEmptyString, Room>, ScheduleError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    validate_headers(&headers, ROOMS_COLUMNS, "rooms")?;

    let mut rooms = BTreeMap::new();
    let mut seen: HashMap<NonEmptyString, usize> = HashMap::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result?;
        let row = idx + 1;

        if record.len() != ROOMS_COLUMNS.len() {
            return Err(ScheduleError::RoomsRowError {
                row,
                message: format!(
                    "expected {} columns, got {}",
                    ROOMS_COLUMNS.len(),
                    record.len()
                ),
            });
        }

        let name = parse_non_empty_field(&record, 0, row, "rooms", "Salle")?;

        if let Some(&first_row) = seen.get(&name) {
            return Err(ScheduleError::RoomsDuplicateName {
                name: name.to_string(),
                first_row,
                duplicate_row: row,
            });
        }
        seen.insert(name.clone(), row);

        let floor = parse_field::<u32>(&record, 1, row, "rooms", "Étage")?;
        let x = parse_field::<f32>(&record, 2, row, "rooms", "X")?;
        let y = parse_field::<f32>(&record, 3, row, "rooms", "Y")?;
        let blackboards = parse_field::<u32>(&record, 4, row, "rooms", "Tableaux noirs")?;
        let whiteboards = parse_field::<u32>(&record, 5, row, "rooms", "Tableaux blancs")?;
        let capacity = parse_field::<NonZeroU32>(&record, 6, row, "rooms", "Capacité")?;
        let window = parse_window_field(&record, 7, row)?;
        let priority = parse_priority_field(&record, 8, row)?;
        let reserved = parse_bool_field(&record, 9, row, "rooms", "Réservée")?;

        rooms.insert(
            name,
            Room {
                floor,
                x,
                y,
                blackboards,
                whiteboards,
                capacity,
                window,
                priority,
                reserved,
            },
        );
    }

    Ok(rooms)
}

/// Parse a requests CSV file.
pub fn parse_requests(path: &Path) -> Result<Vec<Request>, ScheduleError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    validate_headers(&headers, REQUESTS_COLUMNS, "requests")?;

    let mut requests = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result?;
        let row = idx + 1;

        if record.len() != REQUESTS_COLUMNS.len() {
            return Err(ScheduleError::RequestsRowError {
                row,
                message: format!(
                    "expected {} columns, got {}",
                    REQUESTS_COLUMNS.len(),
                    record.len()
                ),
            });
        }

        let p1 = parse_bool_field(&record, 0, row, "requests", "P1")?;
        let p2 = parse_bool_field(&record, 1, row, "requests", "P2")?;
        let p3 = parse_bool_field(&record, 2, row, "requests", "P3")?;

        let day_str = record.get(3).unwrap().trim();
        let day = Weekday::from_french(day_str).ok_or_else(|| ScheduleError::InvalidDay {
            row,
            value: day_str.to_string(),
        })?;

        let hour_val = parse_field::<u32>(&record, 4, row, "requests", "Heure")?;
        let hour = Hour::new(hour_val).ok_or(ScheduleError::InvalidHour {
            row,
            value: hour_val,
        })?;

        let subject_raw = record.get(5).unwrap().trim();
        let subject_normalized: String = subject_raw.nfc().collect();
        if !ALLOWED_SUBJECTS.contains(&subject_normalized.as_str()) {
            return Err(ScheduleError::RequestsRowError {
                row,
                message: format!("unknown subject \"{subject_raw}\""),
            });
        }
        let subject = NonEmptyString::try_from(subject_normalized.as_str()).unwrap();

        let classes_raw = record.get(6).unwrap();
        let classes: Vec<NonEmptyString> = classes_raw
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                if !ALLOWED_CLASSES.contains(&s) {
                    Err(ScheduleError::RequestsRowError {
                        row,
                        message: format!("unknown class \"{s}\" in column \"Classes\""),
                    })
                } else {
                    Ok(NonEmptyString::try_from(s).unwrap())
                }
            })
            .collect::<Result<_, _>>()?;
        if classes.is_empty() {
            return Err(ScheduleError::RequestsRowError {
                row,
                message: "column \"Classes\": must have at least one class".to_string(),
            });
        }

        let requester = parse_non_empty_field(&record, 7, row, "requests", "Responsable")?;
        let teacher = parse_non_empty_field(&record, 8, row, "requests", "Colleur")?;
        let blackboards = parse_field::<u32>(&record, 9, row, "requests", "Tableaux")?;
        let window = parse_bool_field(&record, 10, row, "requests", "Fenêtre")?;
        let students = parse_field::<NonZeroU32>(&record, 11, row, "requests", "Nb élèves")?;
        let prep_students = parse_field::<u32>(&record, 12, row, "requests", "Nb prep")?;
        let room_preference = parse_room_preference(&record, 13);
        let prep_preference = parse_room_preference(&record, 14);

        requests.push(Request {
            periods: Periods { p1, p2, p3 },
            day,
            hour,
            subject,
            classes,
            requester,
            teacher,
            blackboards,
            window,
            students,
            prep_students,
            room_preference,
            prep_preference,
        });
    }

    Ok(requests)
}

fn validate_headers(
    headers: &csv::StringRecord,
    expected: &[&str],
    file: &str,
) -> Result<(), ScheduleError> {
    for (i, &col) in expected.iter().enumerate() {
        match headers.get(i) {
            Some(h) if h.trim() == col => {}
            _ => {
                return Err(if file == "rooms" {
                    ScheduleError::RoomsMissingColumn(col.to_string())
                } else if file == "incompats" {
                    ScheduleError::IncompatsMissingColumn(col.to_string())
                } else {
                    ScheduleError::RequestsMissingColumn(col.to_string())
                });
            }
        }
    }
    for i in expected.len()..headers.len() {
        let name = headers.get(i).unwrap_or("").trim().to_string();
        return Err(if file == "rooms" {
            ScheduleError::RoomsUnknownColumn(name)
        } else if file == "incompats" {
            ScheduleError::IncompatsUnknownColumn(name)
        } else {
            ScheduleError::RequestsUnknownColumn(name)
        });
    }
    Ok(())
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
        let message = format!("cannot parse \"{value}\" in column \"{column}\"");
        if file == "rooms" {
            ScheduleError::RoomsRowError { row, message }
        } else if file == "incompats" {
            ScheduleError::IncompatsRowError { row, message }
        } else {
            ScheduleError::RequestsRowError { row, message }
        }
    })
}

fn parse_bool_field(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
    file: &str,
    column: &str,
) -> Result<bool, ScheduleError> {
    let value = record.get(index).unwrap().trim();
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => {
            let message = format!("column \"{column}\": expected 0 or 1, got \"{value}\"");
            Err(if file == "rooms" {
                ScheduleError::RoomsRowError { row, message }
            } else if file == "incompats" {
                ScheduleError::IncompatsRowError { row, message }
            } else {
                ScheduleError::RequestsRowError { row, message }
            })
        }
    }
}

fn parse_non_empty_field(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
    file: &str,
    column: &str,
) -> Result<NonEmptyString, ScheduleError> {
    let value = record.get(index).unwrap().trim();
    NonEmptyString::try_from(value).map_err(|_| {
        let message = format!("column \"{column}\": value must not be empty");
        if file == "rooms" {
            ScheduleError::RoomsRowError { row, message }
        } else if file == "incompats" {
            ScheduleError::IncompatsRowError { row, message }
        } else {
            ScheduleError::RequestsRowError { row, message }
        }
    })
}

fn parse_window_field(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
) -> Result<Window, ScheduleError> {
    let value = record.get(index).unwrap().trim();
    match value {
        "Non" => Ok(Window::None),
        "Intérieur" => Ok(Window::Interior),
        "Extérieur" => Ok(Window::Exterior),
        _ => Err(ScheduleError::RoomsRowError {
            row,
            message: format!(
                "column \"Fenêtre\": expected Non, Intérieur or Extérieur, got \"{value}\""
            ),
        }),
    }
}

fn parse_priority_field(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
) -> Result<Option<u32>, ScheduleError> {
    let value = record.get(index).unwrap().trim();
    if value == "-1" {
        return Ok(None);
    }
    value
        .parse::<u32>()
        .map(Some)
        .map_err(|_| ScheduleError::RoomsRowError {
            row,
            message: format!("cannot parse \"{value}\" in column \"Priorité\""),
        })
}

/// Parse an incompats CSV file.
pub fn parse_incompats(path: &Path) -> Result<Vec<Incompat>, ScheduleError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    validate_headers(&headers, INCOMPATS_COLUMNS, "incompats")?;

    let mut incompats = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result?;
        let row = idx + 1;

        if record.len() != INCOMPATS_COLUMNS.len() {
            return Err(ScheduleError::IncompatsRowError {
                row,
                message: format!(
                    "expected {} columns, got {}",
                    INCOMPATS_COLUMNS.len(),
                    record.len()
                ),
            });
        }

        let room = parse_non_empty_field(&record, 0, row, "incompats", "Salle")?;
        let p1 = parse_bool_field(&record, 1, row, "incompats", "P1")?;
        let p2 = parse_bool_field(&record, 2, row, "incompats", "P2")?;
        let p3 = parse_bool_field(&record, 3, row, "incompats", "P3")?;

        let day_str = record.get(4).unwrap().trim();
        let day =
            Weekday::from_french(day_str).ok_or_else(|| ScheduleError::IncompatsRowError {
                row,
                message: format!("invalid day \"{day_str}\""),
            })?;

        let hour_val = parse_field::<u32>(&record, 5, row, "incompats", "Heure")?;
        let hour = Hour::new(hour_val).ok_or(ScheduleError::IncompatsRowError {
            row,
            message: format!("hour must be between 8 and 19, got {hour_val}"),
        })?;

        incompats.push(Incompat {
            room,
            periods: Periods { p1, p2, p3 },
            day,
            hour,
        });
    }

    Ok(incompats)
}

fn parse_room_preference(record: &csv::StringRecord, index: usize) -> Option<RoomPreference> {
    let value = record.get(index).unwrap_or("").trim();
    if value.is_empty() {
        return None;
    }
    if let Some(name) = value.strip_prefix('!') {
        let name = name.trim();
        NonEmptyString::try_from(name)
            .ok()
            .map(RoomPreference::Demand)
    } else {
        NonEmptyString::try_from(value)
            .ok()
            .map(RoomPreference::Suggestion)
    }
}
