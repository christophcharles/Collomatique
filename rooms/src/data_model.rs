use std::num::NonZeroU32;
use std::path::Path;

use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::types::Hour;

const ROOMS_COLUMNS: &[&str] = &[
    "Salle",
    "Étage",
    "X",
    "Y",
    "Tableaux",
    "Capacité",
    "Fenêtre",
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

const ALLOWED_SUBJECTS: &[&str] = &[
    "mathématiques",
    "physique",
    "chimie",
    "physique-chimie",
    "si",
    "svt",
    "informatique",
    "français",
    "lettres",
    "philosophie",
    "lettres-philosophie",
    "histoire",
    "géographie",
    "hgg",
    "esh",
    "anglais",
    "espagnol",
    "allemand",
    "italien",
    "latin",
    "grec",
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
}

/// A room available for scheduling.
#[derive(Debug, Clone)]
pub struct Room {
    /// Name of the room (e.g. "A101").
    pub name: NonEmptyString,
    /// Floor number.
    pub floor: u32,
    /// X coordinate on the floor plan.
    pub x: f32,
    /// Y coordinate on the floor plan.
    pub y: f32,
    /// Number of blackboards in the room.
    pub blackboards: u32,
    /// Maximum number of students the room can accommodate.
    pub capacity: NonZeroU32,
    /// Whether the room has a window.
    pub window: bool,
}

/// A scheduling request for a room.
#[derive(Debug, Clone)]
pub struct Request {
    /// Whether the interrogation is needed in period 1.
    pub p1: bool,
    /// Whether the interrogation is needed in period 2.
    pub p2: bool,
    /// Whether the interrogation is needed in period 3.
    pub p3: bool,
    /// Day of the week.
    pub day: Weekday,
    /// Hour of the interrogation.
    pub hour: Hour,
    /// Subject name (normalized: NFC, trimmed, lowercased).
    pub subject: NonEmptyString,
    /// Classes that can attend this interrogation slot (e.g. "MP", "PC").
    pub classes: Vec<NonEmptyString>,
    /// Name of the person requesting the room.
    pub requester: NonEmptyString,
    /// Name of the teacher that will use the room.
    pub teacher: NonEmptyString,
    /// Minimum number of blackboards needed.
    pub blackboards: u32,
    /// Whether a window is required.
    pub window: bool,
    /// Number of students to seat in the room.
    pub students: NonZeroU32,
    /// Number of students to seat in the prep room.
    pub prep_students: u32,
    /// Suggested room name. If absent, no preference. If the name is not in
    /// the rooms CSV, the teacher vouches for the (unmanaged) room being
    /// available.
    pub room_suggestion: Option<NonEmptyString>,
    /// Suggested prep room name. Same semantics as `room_suggestion`.
    pub prep_suggestion: Option<NonEmptyString>,
}

/// Parsed schedule data: rooms and requests.
#[derive(Debug, Clone)]
pub struct ScheduleData {
    /// Available rooms.
    pub rooms: Vec<Room>,
    /// Scheduling requests.
    pub requests: Vec<Request>,
}

/// Parse a rooms CSV and a requests CSV into a [`ScheduleData`].
pub fn parse_schedule(
    rooms_path: &Path,
    requests_path: &Path,
) -> Result<ScheduleData, ScheduleError> {
    let rooms = parse_rooms(rooms_path)?;
    let requests = parse_requests(requests_path)?;
    Ok(ScheduleData { rooms, requests })
}

/// Parse a rooms CSV file.
///
/// Expected columns: Salle, Étage, X, Y, Tableaux, Capacité, Fenêtre.
pub fn parse_rooms(path: &Path) -> Result<Vec<Room>, ScheduleError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    validate_headers(&headers, ROOMS_COLUMNS, "rooms")?;

    let mut rooms = Vec::new();

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
        let floor = parse_field::<u32>(&record, 1, row, "rooms", "Étage")?;
        let x = parse_field::<f32>(&record, 2, row, "rooms", "X")?;
        let y = parse_field::<f32>(&record, 3, row, "rooms", "Y")?;
        let blackboards = parse_field::<u32>(&record, 4, row, "rooms", "Tableaux")?;
        let capacity = parse_field::<NonZeroU32>(&record, 5, row, "rooms", "Capacité")?;
        let window = parse_bool_field(&record, 6, row, "rooms", "Fenêtre")?;

        rooms.push(Room {
            name,
            floor,
            x,
            y,
            blackboards,
            capacity,
            window,
        });
    }

    Ok(rooms)
}

/// Parse a requests CSV file.
///
/// Expected columns: P1, P2, P3, Jour, Heure, Discipline, Classes,
/// Responsable, Colleur, Tableaux, Fenêtre, Nb élèves, Nb prep, Salle, Prep.
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
        let subject_normalized: String = subject_raw.nfc().collect::<String>().to_lowercase();
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
        let room_suggestion = parse_optional_non_empty(&record, 13);
        let prep_suggestion = parse_optional_non_empty(&record, 14);

        requests.push(Request {
            p1,
            p2,
            p3,
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
            room_suggestion,
            prep_suggestion,
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
        if file == "rooms" {
            ScheduleError::RoomsRowError {
                row,
                message: format!("cannot parse \"{value}\" in column \"{column}\""),
            }
        } else {
            ScheduleError::RequestsRowError {
                row,
                message: format!("cannot parse \"{value}\" in column \"{column}\""),
            }
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
        _ => Err(if file == "rooms" {
            ScheduleError::RoomsRowError {
                row,
                message: format!("column \"{column}\": expected 0 or 1, got \"{value}\""),
            }
        } else {
            ScheduleError::RequestsRowError {
                row,
                message: format!("column \"{column}\": expected 0 or 1, got \"{value}\""),
            }
        }),
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
        if file == "rooms" {
            ScheduleError::RoomsRowError {
                row,
                message: format!("column \"{column}\": value must not be empty"),
            }
        } else {
            ScheduleError::RequestsRowError {
                row,
                message: format!("column \"{column}\": value must not be empty"),
            }
        }
    })
}

fn parse_optional_non_empty(record: &csv::StringRecord, index: usize) -> Option<NonEmptyString> {
    let value = record.get(index).unwrap_or("").trim();
    NonEmptyString::try_from(value).ok()
}
