use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::num::NonZeroU32;

use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardRequirement {
    Zero,
    One,
    Two,
    Three { hard: bool },
}

impl BoardRequirement {
    pub fn from_u32(value: u32) -> Option<BoardRequirement> {
        match value {
            0 => Some(BoardRequirement::Zero),
            1 => Some(BoardRequirement::One),
            2 => Some(BoardRequirement::Two),
            3 => Some(BoardRequirement::Three { hard: false }),
            _ => None,
        }
    }

    pub fn from_str_column(value: &str) -> Option<BoardRequirement> {
        match value {
            "0" => Some(BoardRequirement::Zero),
            "1" => Some(BoardRequirement::One),
            "2" => Some(BoardRequirement::Two),
            "3" => Some(BoardRequirement::Three { hard: false }),
            "!3" => Some(BoardRequirement::Three { hard: true }),
            _ => None,
        }
    }

    pub fn hard_target(&self) -> u32 {
        match self {
            BoardRequirement::Zero => 0,
            BoardRequirement::One => 1,
            BoardRequirement::Two => 2,
            BoardRequirement::Three { hard } => {
                if *hard {
                    3
                } else {
                    2
                }
            }
        }
    }

    pub fn target(&self) -> u32 {
        match self {
            BoardRequirement::Zero => 0,
            BoardRequirement::One => 1,
            BoardRequirement::Two => 2,
            BoardRequirement::Three { .. } => 3,
        }
    }

    pub fn to_csv_string(&self) -> &'static str {
        match self {
            BoardRequirement::Zero => "0",
            BoardRequirement::One => "1",
            BoardRequirement::Two => "2",
            BoardRequirement::Three { hard: false } => "3",
            BoardRequirement::Three { hard: true } => "!3",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProximityType {
    Floor(u32),
    Room(NonEmptyString),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProximityDetails {
    pub fuzzy: bool,
    pub level: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterrogationRoomStatus {
    Accepted { can_share_with_prep: bool },
    Demanded { can_share_with_prep: bool },
    Excluded,
}

impl InterrogationRoomStatus {
    pub fn can_share_with_prep(&self) -> bool {
        match self {
            InterrogationRoomStatus::Accepted {
                can_share_with_prep,
            }
            | InterrogationRoomStatus::Demanded {
                can_share_with_prep,
            } => *can_share_with_prep,
            InterrogationRoomStatus::Excluded => false,
        }
    }

    pub fn is_demanded(&self) -> bool {
        matches!(self, InterrogationRoomStatus::Demanded { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepRoomStatus {
    Accepted,
    Demanded,
    Excluded,
}

impl PrepRoomStatus {
    pub fn is_demanded(&self) -> bool {
        matches!(self, PrepRoomStatus::Demanded)
    }
}

/// Hour of an interrogation, guaranteed to be between 8 and 19 inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hour(u32);

impl Hour {
    pub fn new(value: u32) -> Option<Hour> {
        (8..=19).contains(&value).then_some(Hour(value))
    }
}

impl std::fmt::Display for Hour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}h", self.0)
    }
}

impl std::ops::Deref for Hour {
    type Target = u32;

    fn deref(&self) -> &u32 {
        &self.0
    }
}

/// Type of window in a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    None,
    Interior,
    Exterior,
}

/// A room available for scheduling.
#[derive(Debug, Clone)]
pub struct Room {
    pub floor: u32,
    pub x: f32,
    pub y: f32,
    pub blackboards: f32,
    pub whiteboards: f32,
    pub capacity: NonZeroU32,
    pub window: Window,
    pub priority: Option<u32>,
    pub reserved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periods {
    pub p1: bool,
    pub p2: bool,
    pub p3: bool,
}

impl Periods {
    pub fn overlaps_with(&self, other: &Self) -> bool {
        (self.p1 && other.p1) || (self.p2 && other.p2) || (self.p3 && other.p3)
    }
}

#[derive(Debug, Clone)]
pub struct TimeZones {
    cuts: BTreeSet<Hour>,
}

impl TimeZones {
    pub fn new(cuts: BTreeSet<Hour>) -> Option<TimeZones> {
        let invalid_cut = Hour::new(8).unwrap();
        (!cuts.contains(&invalid_cut)).then_some(TimeZones { cuts })
    }

    pub fn cuts(&self) -> &BTreeSet<Hour> {
        &self.cuts
    }

    pub fn zone_label(&self, hour: Hour) -> Hour {
        self.cuts
            .range(..=hour)
            .next_back()
            .copied()
            .unwrap_or(Hour::new(8).unwrap())
    }
}

impl Default for TimeZones {
    fn default() -> Self {
        TimeZones {
            cuts: BTreeSet::from([Hour::new(10).unwrap(), Hour::new(16).unwrap()]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub oral_exam_periods: Periods,
    pub enforce_period_exhaustions: Periods,
    pub time_zones: TimeZones,
    pub max_priority: Option<u32>,
    pub soft_boards_weight: f64,
    pub soft_windows_weight: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            oral_exam_periods: Periods {
                p1: false,
                p2: false,
                p3: true,
            },
            enforce_period_exhaustions: Periods {
                p1: false,
                p2: false,
                p3: false,
            },
            time_zones: TimeZones::default(),
            max_priority: None,
            soft_boards_weight: 1.0,
            soft_windows_weight: 1.0,
        }
    }
}

/// A room incompatibility: the room is unavailable at this day/hour for the given periods.
#[derive(Debug, Clone)]
pub struct Incompat {
    pub room: NonEmptyString,
    pub periods: Periods,
    pub day: Weekday,
    pub hour: Hour,
}

/// A scheduling request for a room.
#[derive(Debug, Clone)]
pub struct Request {
    pub periods: Periods,
    pub day: Weekday,
    pub hour: Hour,
    pub subjects: Vec<NonEmptyString>,
    pub classes: Vec<NonEmptyString>,
    pub requester: NonEmptyString,
    pub teacher: NonEmptyString,
    pub boards: BoardRequirement,
    pub window: bool,
    pub students: NonZeroU32,
    pub prep_students: u32,
    pub room_statuses: BTreeMap<NonEmptyString, InterrogationRoomStatus>,
    pub proximity: BTreeMap<ProximityType, ProximityDetails>,
    pub prep_statuses: BTreeMap<NonEmptyString, PrepRoomStatus>,
    pub prep_proximity: BTreeMap<ProximityType, ProximityDetails>,
    pub skip_room_continuity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSol {
    pub room: NonEmptyString,
    pub mark_fixed: bool,
}

pub type SolutionColumns = Vec<(Option<RoomSol>, Option<RoomSol>)>;

/// Parsed schedule data: rooms and requests.
#[derive(Debug, Clone)]
pub struct ScheduleData {
    pub rooms: BTreeMap<NonEmptyString, Room>,
    pub requests: Vec<Request>,
    pub incompats: Vec<Incompat>,
    pub config: Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemandKind {
    Interrogation,
    Prep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemandConflictKind {
    InterrogationInterrogation,
    InterrogationPrep {
        can_share_with_prep: bool,
    },
    PrepOverCapacity {
        total_students: u32,
        capacity: NonZeroU32,
    },
    PrepUnknownCapacity {
        total_students: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemandConflict {
    pub room: NonEmptyString,
    pub day: Weekday,
    pub hour: Hour,
    pub kind: DemandConflictKind,
    pub requests: Vec<(usize, DemandKind)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherConflict {
    pub teacher: NonEmptyString,
    pub day: Weekday,
    pub hour: Hour,
    pub requests: Vec<usize>,
}

pub struct UnregisteredRooms<'a> {
    pub suggested: Vec<&'a str>,
    pub demanded: Vec<&'a str>,
}

impl ScheduleData {
    pub fn unregistered_rooms(&self) -> UnregisteredRooms<'_> {
        let mut suggested = HashSet::new();
        let mut demanded = HashSet::new();

        for req in &self.requests {
            for (name, status) in &req.room_statuses {
                let name_str: &str = name.as_ref();
                if !self.rooms.contains_key(name_str) {
                    match status {
                        InterrogationRoomStatus::Accepted { .. } => {
                            suggested.insert(name_str);
                        }
                        InterrogationRoomStatus::Demanded { .. } => {
                            demanded.insert(name_str);
                        }
                        InterrogationRoomStatus::Excluded => {}
                    }
                }
            }
            for key in req.proximity.keys() {
                if let ProximityType::Room(name) = key {
                    let name_str: &str = name.as_ref();
                    if !self.rooms.contains_key(name_str) {
                        suggested.insert(name_str);
                    }
                }
            }
            for (name, status) in &req.prep_statuses {
                let name_str: &str = name.as_ref();
                if !self.rooms.contains_key(name_str) {
                    match status {
                        PrepRoomStatus::Accepted => {
                            suggested.insert(name_str);
                        }
                        PrepRoomStatus::Demanded => {
                            demanded.insert(name_str);
                        }
                        PrepRoomStatus::Excluded => {}
                    }
                }
            }
            for key in req.prep_proximity.keys() {
                if let ProximityType::Room(name) = key {
                    let name_str: &str = name.as_ref();
                    if !self.rooms.contains_key(name_str) {
                        suggested.insert(name_str);
                    }
                }
            }
        }

        let demanded = &demanded - &suggested;

        let mut suggested: Vec<&str> = suggested.into_iter().collect();
        let mut demanded: Vec<&str> = demanded.into_iter().collect();
        suggested.sort();
        demanded.sort();

        UnregisteredRooms {
            suggested,
            demanded,
        }
    }

    pub fn demand_conflicts(&self) -> Vec<DemandConflict> {
        struct Demand {
            request: usize,
            interrogation: bool,
            periods: Periods,
        }

        let mut groups: BTreeMap<(NonEmptyString, Weekday, Hour), Vec<Demand>> = BTreeMap::new();

        for (req_idx, req) in self.requests.iter().enumerate() {
            for (room, status) in &req.room_statuses {
                if status.is_demanded() {
                    groups
                        .entry((room.clone(), req.day, req.hour))
                        .or_default()
                        .push(Demand {
                            request: req_idx,
                            interrogation: true,
                            periods: req.periods,
                        });
                }
            }
            for (room, status) in &req.prep_statuses {
                if status.is_demanded() {
                    groups
                        .entry((room.clone(), req.day, req.hour))
                        .or_default()
                        .push(Demand {
                            request: req_idx,
                            interrogation: false,
                            periods: req.periods,
                        });
                }
            }
        }

        let mut conflicts = Vec::new();

        for ((room, day, hour), demands) in &groups {
            if demands.len() < 2 {
                continue;
            }

            let interro: Vec<_> = demands.iter().filter(|d| d.interrogation).collect();
            let prep: Vec<_> = demands.iter().filter(|d| !d.interrogation).collect();

            for i in 0..interro.len() {
                for j in (i + 1)..interro.len() {
                    if interro[i].periods.overlaps_with(&interro[j].periods) {
                        conflicts.push(DemandConflict {
                            room: room.clone(),
                            day: *day,
                            hour: *hour,
                            kind: DemandConflictKind::InterrogationInterrogation,
                            requests: vec![
                                (interro[i].request, DemandKind::Interrogation),
                                (interro[j].request, DemandKind::Interrogation),
                            ],
                        });
                    }
                }
            }

            for i_demand in &interro {
                for p_demand in &prep {
                    if i_demand.periods.overlaps_with(&p_demand.periods) {
                        let can_share = self.requests[i_demand.request]
                            .room_statuses
                            .get(room)
                            .map_or(false, |s| s.can_share_with_prep());
                        conflicts.push(DemandConflict {
                            room: room.clone(),
                            day: *day,
                            hour: *hour,
                            kind: DemandConflictKind::InterrogationPrep {
                                can_share_with_prep: can_share,
                            },
                            requests: vec![
                                (i_demand.request, DemandKind::Interrogation),
                                (p_demand.request, DemandKind::Prep),
                            ],
                        });
                    }
                }
            }

            if prep.len() >= 2 {
                for period_idx in 0..3u8 {
                    let active: Vec<_> = prep
                        .iter()
                        .filter(|d| match period_idx {
                            0 => d.periods.p1,
                            1 => d.periods.p2,
                            _ => d.periods.p3,
                        })
                        .collect();

                    if active.len() < 2 {
                        continue;
                    }

                    let total_students: u32 = active
                        .iter()
                        .map(|d| self.requests[d.request].prep_students)
                        .sum();

                    let capacity = self.rooms.get(room).map(|r| r.capacity);

                    let kind = match capacity {
                        Some(cap) if total_students > cap.get() => {
                            Some(DemandConflictKind::PrepOverCapacity {
                                total_students,
                                capacity: cap,
                            })
                        }
                        Some(_) => None,
                        None => Some(DemandConflictKind::PrepUnknownCapacity { total_students }),
                    };

                    if let Some(kind) = kind {
                        conflicts.push(DemandConflict {
                            room: room.clone(),
                            day: *day,
                            hour: *hour,
                            kind,
                            requests: active
                                .iter()
                                .map(|d| (d.request, DemandKind::Prep))
                                .collect(),
                        });
                    }
                }
            }
        }

        conflicts
    }

    pub fn teacher_continuity_conflicts(&self) -> Vec<TeacherConflict> {
        let mut groups: BTreeMap<(NonEmptyString, Weekday, Hour), Vec<usize>> = BTreeMap::new();

        for (req_idx, req) in self.requests.iter().enumerate() {
            if req.skip_room_continuity {
                continue;
            }
            groups
                .entry((req.teacher.clone(), req.day, req.hour))
                .or_default()
                .push(req_idx);
        }

        let mut conflicts = Vec::new();

        for ((teacher, day, hour), requests) in &groups {
            if requests.len() < 2 {
                continue;
            }

            let has_overlap = requests.iter().enumerate().any(|(i, &r_i)| {
                requests[i + 1..].iter().any(|&r_j| {
                    self.requests[r_i]
                        .periods
                        .overlaps_with(&self.requests[r_j].periods)
                })
            });

            if has_overlap {
                conflicts.push(TeacherConflict {
                    teacher: teacher.clone(),
                    day: *day,
                    hour: *hour,
                    requests: requests.clone(),
                });
            }
        }

        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_label_default() {
        let tz = TimeZones::default();
        assert_eq!(tz.zone_label(Hour::new(8).unwrap()), Hour::new(8).unwrap());
        assert_eq!(tz.zone_label(Hour::new(9).unwrap()), Hour::new(8).unwrap());
        assert_eq!(
            tz.zone_label(Hour::new(10).unwrap()),
            Hour::new(10).unwrap()
        );
        assert_eq!(
            tz.zone_label(Hour::new(15).unwrap()),
            Hour::new(10).unwrap()
        );
        assert_eq!(
            tz.zone_label(Hour::new(16).unwrap()),
            Hour::new(16).unwrap()
        );
        assert_eq!(
            tz.zone_label(Hour::new(19).unwrap()),
            Hour::new(16).unwrap()
        );
    }

    #[test]
    fn zone_label_empty_cuts() {
        let tz = TimeZones::new(BTreeSet::new()).unwrap();
        for h in 8..=19 {
            assert_eq!(tz.zone_label(Hour::new(h).unwrap()), Hour::new(8).unwrap());
        }
    }

    #[test]
    fn zone_label_single_cut() {
        let tz = TimeZones::new(BTreeSet::from([Hour::new(14).unwrap()])).unwrap();
        assert_eq!(tz.zone_label(Hour::new(8).unwrap()), Hour::new(8).unwrap());
        assert_eq!(tz.zone_label(Hour::new(13).unwrap()), Hour::new(8).unwrap());
        assert_eq!(
            tz.zone_label(Hour::new(14).unwrap()),
            Hour::new(14).unwrap()
        );
        assert_eq!(
            tz.zone_label(Hour::new(19).unwrap()),
            Hour::new(14).unwrap()
        );
    }
}
