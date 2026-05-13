use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::path::Path;

use collomatique_rooms_model::{
    Config, Hour, Incompat, InterrogationRoomPreference, Periods, PrepRoomPreference, Request,
    Room, ScheduleData, TeacherConflict, Window,
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

pub(crate) const REQUESTS_COLUMNS: &[&str] = &[
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
    "Isolé",
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
    #[error("{}", format_unregistered_suggested(.0))]
    UnregisteredSuggestedRooms(Vec<String>),
    #[error("{}", format_conflicting_preferences(.0))]
    ConflictingRoomPreferences(Vec<String>),
    #[error("{}", format_teacher_conflicts(.0))]
    TeacherConflicts(Vec<TeacherConflict>),
    #[error("Check mode: {0}")]
    CheckUnknownRoom(#[from] collomatique_constraints_rooms::CheckError),
    #[error("Check mode: failed to reconstruct extra variables: {0}")]
    CheckReconstructionFailed(String),
}

fn format_unregistered_suggested(rooms: &[String]) -> String {
    format!(
        "Suggested room(s) not found in rooms file: {}. \
         Use '!' prefix to demand a room, or register it in the rooms file.",
        rooms.join(", ")
    )
}

fn format_conflicting_preferences(rooms: &[String]) -> String {
    format!(
        "Room(s) with conflicting positive and negative preferences: {}",
        rooms.join(", ")
    )
}

fn format_teacher_conflicts(conflicts: &[TeacherConflict]) -> String {
    let items: Vec<String> = conflicts
        .iter()
        .map(|c| {
            format!(
                "teacher \"{}\" on {} at {} (requests: {:?})",
                c.teacher.as_ref() as &str,
                c.day,
                c.hour,
                c.requests
            )
        })
        .collect();
    format!(
        "Teacher continuity conflict(s): same teacher has multiple non-isolated requests \
         at the same time with overlapping periods: {}",
        items.join("; ")
    )
}

/// Parse a rooms CSV and a requests CSV into a [`ScheduleData`].
pub fn parse_schedule(
    rooms_path: &Path,
    requests_path: &Path,
    incompats_path: Option<&Path>,
    config: Config,
) -> Result<(ScheduleData, Vec<RoomPreferenceWarning>), ScheduleError> {
    let rooms = parse_rooms(rooms_path)?;
    let (requests, raw_request_rows, solution_columns, warnings) = parse_requests(requests_path)?;
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

    Ok((
        ScheduleData {
            rooms,
            requests,
            raw_request_rows,
            solution_columns,
            incompats,
            config,
        },
        warnings,
    ))
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
pub fn parse_requests(
    path: &Path,
) -> Result<
    (
        Vec<Request>,
        Vec<Vec<String>>,
        Vec<(Option<NonEmptyString>, Option<NonEmptyString>)>,
        Vec<RoomPreferenceWarning>,
    ),
    ScheduleError,
> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    let expected_columns = validate_requests_headers(&headers)?;

    let mut requests = Vec::new();
    let mut raw_rows = Vec::new();
    let mut solutions = Vec::new();
    let mut all_warnings = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result?;
        let row = idx + 1;

        if record.len() != expected_columns {
            return Err(ScheduleError::RequestsRowError {
                row,
                message: format!(
                    "expected {} columns, got {}",
                    expected_columns,
                    record.len()
                ),
            });
        }

        let raw_row: Vec<String> = (0..REQUESTS_COLUMNS.len())
            .map(|i| record.get(i).unwrap_or("").to_string())
            .collect();
        raw_rows.push(raw_row);

        let sol_salle = if expected_columns > REQUESTS_COLUMNS.len() {
            NonEmptyString::try_from(record.get(REQUESTS_COLUMNS.len()).unwrap_or("").trim()).ok()
        } else {
            None
        };
        let sol_prep = if expected_columns > REQUESTS_COLUMNS.len() + 1 {
            NonEmptyString::try_from(record.get(REQUESTS_COLUMNS.len() + 1).unwrap_or("").trim())
                .ok()
        } else {
            None
        };
        solutions.push((sol_salle, sol_prep));

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

        let subjects_raw = record.get(5).unwrap();
        let subjects: Vec<NonEmptyString> = subjects_raw
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                let normalized: String = s.nfc().collect();
                if !ALLOWED_SUBJECTS.contains(&normalized.as_str()) {
                    Err(ScheduleError::RequestsRowError {
                        row,
                        message: format!("unknown subject \"{s}\" in column \"Discipline\""),
                    })
                } else {
                    Ok(NonEmptyString::try_from(normalized.as_str()).unwrap())
                }
            })
            .collect::<Result<_, _>>()?;
        if subjects.is_empty() {
            return Err(ScheduleError::RequestsRowError {
                row,
                message: "column \"Discipline\": must have at least one subject".to_string(),
            });
        }

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

        let (room_preference, floor_suggestions, room_warnings) =
            parse_interrogation_room_preferences(&record, 13, row)?;
        let (prep_preference, prep_warnings) = parse_prep_room_preferences(&record, 14, row);
        let isolated = parse_bool_field(&record, 15, row, "requests", "Isolé")?;
        all_warnings.extend(room_warnings);
        all_warnings.extend(prep_warnings);

        for prep in &prep_preference {
            let prep_name = prep.room_name();
            let interro = room_preference.iter().find(|p| {
                matches!(
                    p,
                    InterrogationRoomPreference::Suggestion { .. }
                        | InterrogationRoomPreference::Demand { .. }
                ) && p.room_name() == prep_name
            });
            if let Some(interro) = interro {
                if !interro.can_share_with_prep() {
                    all_warnings.push(RoomPreferenceWarning::InterrogationAndPrepWithoutSharing {
                        row,
                        room: (prep_name.as_ref() as &str).to_string(),
                    });
                }
            }
        }

        requests.push(Request {
            periods: Periods { p1, p2, p3 },
            day,
            hour,
            subjects,
            classes,
            requester,
            teacher,
            blackboards,
            window,
            students,
            prep_students,
            room_preference,
            floor_suggestions,
            prep_preference,
            skip_room_continuity: isolated,
        });
    }

    Ok((requests, raw_rows, solutions, all_warnings))
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

fn validate_requests_headers(headers: &csv::StringRecord) -> Result<usize, ScheduleError> {
    for (i, &col) in REQUESTS_COLUMNS.iter().enumerate() {
        match headers.get(i) {
            Some(h) if h.trim() == col => {}
            _ => return Err(ScheduleError::RequestsMissingColumn(col.to_string())),
        }
    }
    let extra = headers.len() - REQUESTS_COLUMNS.len();
    match extra {
        0 => Ok(REQUESTS_COLUMNS.len()),
        1 | 2 => {
            let col16 = headers.get(REQUESTS_COLUMNS.len()).unwrap_or("").trim();
            if col16 != "SolSalle" {
                return Err(ScheduleError::RequestsUnknownColumn(col16.to_string()));
            }
            if extra == 2 {
                let col17 = headers.get(REQUESTS_COLUMNS.len() + 1).unwrap_or("").trim();
                if col17 != "SolPrep" {
                    return Err(ScheduleError::RequestsUnknownColumn(col17.to_string()));
                }
            }
            Ok(REQUESTS_COLUMNS.len() + extra)
        }
        _ => {
            let first_bad = REQUESTS_COLUMNS.len() + 2;
            let name = headers.get(first_bad).unwrap_or("").trim().to_string();
            Err(ScheduleError::RequestsUnknownColumn(name))
        }
    }
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

fn parse_single_interrogation_room_preference(entry: &str) -> Option<InterrogationRoomPreference> {
    let value = entry.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(rest) = value.strip_prefix('~') {
        let room = NonEmptyString::try_from(rest.trim()).ok()?;
        return Some(InterrogationRoomPreference::Exclusion { room });
    }
    if let Some(rest) = value.strip_prefix('-') {
        let room = NonEmptyString::try_from(rest.trim()).ok()?;
        return Some(InterrogationRoomPreference::Avoidance { room });
    }
    let (is_demand, rest) = match value.strip_prefix('!') {
        Some(s) => (true, s.trim()),
        None => (false, value),
    };
    let (can_share_with_prep, name) = match rest.strip_suffix('+') {
        Some(s) => (true, s.trim()),
        None => (false, rest),
    };
    let room = NonEmptyString::try_from(name).ok()?;
    Some(if is_demand {
        InterrogationRoomPreference::Demand {
            room,
            can_share_with_prep,
        }
    } else {
        InterrogationRoomPreference::Suggestion {
            room,
            can_share_with_prep,
        }
    })
}

fn parse_single_prep_room_preference(entry: &str) -> Option<PrepRoomPreference> {
    let value = entry.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(name) = value.strip_prefix('!') {
        let name = name.trim();
        NonEmptyString::try_from(name)
            .ok()
            .map(PrepRoomPreference::Demand)
    } else {
        NonEmptyString::try_from(value)
            .ok()
            .map(PrepRoomPreference::Suggestion)
    }
}

fn format_interrogation_pref(pref: &InterrogationRoomPreference) -> String {
    let (prefix, suffix) = match pref {
        InterrogationRoomPreference::Demand {
            can_share_with_prep: true,
            ..
        } => ("!", "+"),
        InterrogationRoomPreference::Demand { .. } => ("!", ""),
        InterrogationRoomPreference::Suggestion {
            can_share_with_prep: true,
            ..
        } => ("", "+"),
        InterrogationRoomPreference::Suggestion { .. } => ("", ""),
        InterrogationRoomPreference::Avoidance { .. } => ("-", ""),
        InterrogationRoomPreference::Exclusion { .. } => ("~", ""),
    };
    format!("{prefix}{}{suffix}", pref.room_name().as_ref() as &str)
}

fn format_prep_pref(pref: &PrepRoomPreference) -> String {
    match pref {
        PrepRoomPreference::Demand(name) => format!("!{}", name.as_ref() as &str),
        PrepRoomPreference::Suggestion(name) => (name.as_ref() as &str).to_string(),
    }
}

#[derive(Debug, Clone)]
pub enum RoomPreferenceWarning {
    Redundancy {
        row: usize,
        column: &'static str,
        room: String,
        original_entries: Vec<String>,
        merged_result: String,
    },
    InterrogationAndPrepWithoutSharing {
        row: usize,
        room: String,
    },
    ConflictingPreferences {
        row: usize,
        room: String,
        positive_entries: Vec<String>,
        negative_entries: Vec<String>,
    },
}

fn parse_interrogation_room_preferences(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
) -> Result<
    (
        Vec<InterrogationRoomPreference>,
        Vec<u32>,
        Vec<RoomPreferenceWarning>,
    ),
    ScheduleError,
> {
    let value = record.get(index).unwrap_or("").trim();
    if value.is_empty() {
        return Ok((vec![], vec![], vec![]));
    }

    let mut parsed: Vec<InterrogationRoomPreference> = Vec::new();
    let mut floor_suggestions: Vec<u32> = Vec::new();

    for entry in value.split(';') {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(floor_str) = trimmed.strip_prefix('=') {
            let floor_str = floor_str.trim();
            let floor: u32 = floor_str
                .parse()
                .map_err(|_| ScheduleError::RequestsRowError {
                    row,
                    message: format!(
                        "column \"Salle\": invalid floor suggestion \"={floor_str}\", \
                     expected a non-negative integer after '='"
                    ),
                })?;
            if !floor_suggestions.contains(&floor) {
                floor_suggestions.push(floor);
            }
        } else if let Some(pref) = parse_single_interrogation_room_preference(trimmed) {
            parsed.push(pref);
        }
    }

    let mut by_room: BTreeMap<&str, Vec<&InterrogationRoomPreference>> = BTreeMap::new();
    for pref in &parsed {
        by_room
            .entry(pref.room_name().as_ref())
            .or_default()
            .push(pref);
    }

    let mut result = Vec::new();
    let mut warnings = Vec::new();

    for (room_name, entries) in &by_room {
        let positive: Vec<_> = entries
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    InterrogationRoomPreference::Suggestion { .. }
                        | InterrogationRoomPreference::Demand { .. }
                )
            })
            .collect();
        let negative: Vec<_> = entries
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    InterrogationRoomPreference::Avoidance { .. }
                        | InterrogationRoomPreference::Exclusion { .. }
                )
            })
            .collect();

        if !positive.is_empty() && !negative.is_empty() {
            warnings.push(RoomPreferenceWarning::ConflictingPreferences {
                row,
                room: room_name.to_string(),
                positive_entries: positive
                    .iter()
                    .map(|p| format_interrogation_pref(p))
                    .collect(),
                negative_entries: negative
                    .iter()
                    .map(|p| format_interrogation_pref(p))
                    .collect(),
            });
            continue;
        }

        let room = entries[0].room_name().clone();

        if !positive.is_empty() {
            let is_demand = positive
                .iter()
                .any(|p| matches!(p, InterrogationRoomPreference::Demand { .. }));
            let can_share = positive.iter().any(|p| p.can_share_with_prep());

            let merged = if is_demand {
                InterrogationRoomPreference::Demand {
                    room,
                    can_share_with_prep: can_share,
                }
            } else {
                InterrogationRoomPreference::Suggestion {
                    room,
                    can_share_with_prep: can_share,
                }
            };

            if entries.len() > 1 {
                warnings.push(RoomPreferenceWarning::Redundancy {
                    row,
                    column: "Salle",
                    room: room_name.to_string(),
                    original_entries: entries
                        .iter()
                        .map(|p| format_interrogation_pref(p))
                        .collect(),
                    merged_result: format_interrogation_pref(&merged),
                });
            }

            result.push(merged);
        } else {
            let is_exclusion = negative
                .iter()
                .any(|p| matches!(p, InterrogationRoomPreference::Exclusion { .. }));

            let merged = if is_exclusion {
                InterrogationRoomPreference::Exclusion { room }
            } else {
                InterrogationRoomPreference::Avoidance { room }
            };

            if entries.len() > 1 {
                warnings.push(RoomPreferenceWarning::Redundancy {
                    row,
                    column: "Salle",
                    room: room_name.to_string(),
                    original_entries: entries
                        .iter()
                        .map(|p| format_interrogation_pref(p))
                        .collect(),
                    merged_result: format_interrogation_pref(&merged),
                });
            }

            result.push(merged);
        }
    }

    Ok((result, floor_suggestions, warnings))
}

fn parse_prep_room_preferences(
    record: &csv::StringRecord,
    index: usize,
    row: usize,
) -> (Vec<PrepRoomPreference>, Vec<RoomPreferenceWarning>) {
    let value = record.get(index).unwrap_or("").trim();
    if value.is_empty() {
        return (vec![], vec![]);
    }

    let parsed: Vec<PrepRoomPreference> = value
        .split(';')
        .filter_map(parse_single_prep_room_preference)
        .collect();

    let mut by_room: BTreeMap<&str, Vec<&PrepRoomPreference>> = BTreeMap::new();
    for pref in &parsed {
        by_room
            .entry(pref.room_name().as_ref())
            .or_default()
            .push(pref);
    }

    let mut result = Vec::new();
    let mut warnings = Vec::new();

    for (room_name, entries) in &by_room {
        let is_demand = entries
            .iter()
            .any(|p| matches!(p, PrepRoomPreference::Demand(_)));
        let room = entries[0].room_name().clone();

        let merged = if is_demand {
            PrepRoomPreference::Demand(room)
        } else {
            PrepRoomPreference::Suggestion(room)
        };

        if entries.len() > 1 {
            warnings.push(RoomPreferenceWarning::Redundancy {
                row,
                column: "Prep",
                room: room_name.to_string(),
                original_entries: entries.iter().map(|p| format_prep_pref(p)).collect(),
                merged_result: format_prep_pref(&merged),
            });
        }

        result.push(merged);
    }

    (result, warnings)
}
