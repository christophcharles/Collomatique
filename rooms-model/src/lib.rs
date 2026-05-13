use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::num::NonZeroU32;

use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

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
    pub blackboards: u32,
    pub whiteboards: u32,
    pub capacity: NonZeroU32,
    pub window: Window,
    pub priority: Option<u32>,
    pub reserved: bool,
}

/// Room preference for prep: suggestion or demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepRoomPreference {
    Suggestion(NonEmptyString),
    Demand(NonEmptyString),
}

impl PrepRoomPreference {
    pub fn room_name(&self) -> &NonEmptyString {
        match self {
            PrepRoomPreference::Suggestion(name) | PrepRoomPreference::Demand(name) => name,
        }
    }
}

/// Room preference for interrogation: suggestion or demand, with optional prep sharing.
/// Also supports negative preferences: avoidance (soft) and exclusion (hard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterrogationRoomPreference {
    Suggestion {
        room: NonEmptyString,
        can_share_with_prep: bool,
    },
    Demand {
        room: NonEmptyString,
        can_share_with_prep: bool,
    },
    Avoidance {
        room: NonEmptyString,
    },
    Exclusion {
        room: NonEmptyString,
    },
}

impl InterrogationRoomPreference {
    pub fn room_name(&self) -> &NonEmptyString {
        match self {
            InterrogationRoomPreference::Suggestion { room, .. }
            | InterrogationRoomPreference::Demand { room, .. }
            | InterrogationRoomPreference::Avoidance { room }
            | InterrogationRoomPreference::Exclusion { room } => room,
        }
    }

    pub fn can_share_with_prep(&self) -> bool {
        match self {
            InterrogationRoomPreference::Suggestion {
                can_share_with_prep,
                ..
            }
            | InterrogationRoomPreference::Demand {
                can_share_with_prep,
                ..
            } => *can_share_with_prep,
            InterrogationRoomPreference::Avoidance { .. }
            | InterrogationRoomPreference::Exclusion { .. } => false,
        }
    }
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
    pub blackboards: u32,
    pub window: bool,
    pub students: NonZeroU32,
    pub prep_students: u32,
    pub room_preference: Vec<InterrogationRoomPreference>,
    pub floor_suggestions: Vec<u32>,
    pub prep_preference: Vec<PrepRoomPreference>,
}

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

pub struct UnregisteredRooms<'a> {
    pub suggested: Vec<&'a str>,
    pub demanded: Vec<&'a str>,
}

impl ScheduleData {
    pub fn unregistered_rooms(&self) -> UnregisteredRooms<'_> {
        let mut suggested = HashSet::new();
        let mut demanded = HashSet::new();

        for req in &self.requests {
            for pref in &req.room_preference {
                let name = AsRef::<str>::as_ref(pref.room_name());
                if !self.rooms.contains_key(name) {
                    match pref {
                        InterrogationRoomPreference::Suggestion { .. } => {
                            suggested.insert(name);
                        }
                        InterrogationRoomPreference::Demand { .. } => {
                            demanded.insert(name);
                        }
                        InterrogationRoomPreference::Avoidance { .. }
                        | InterrogationRoomPreference::Exclusion { .. } => {}
                    }
                }
            }
            for pref in &req.prep_preference {
                let name = AsRef::<str>::as_ref(pref.room_name());
                if !self.rooms.contains_key(name) {
                    match pref {
                        PrepRoomPreference::Suggestion(_) => {
                            suggested.insert(name);
                        }
                        PrepRoomPreference::Demand(_) => {
                            demanded.insert(name);
                        }
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
            for pref in &req.room_preference {
                if let InterrogationRoomPreference::Demand { room, .. } = pref {
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
            for pref in &req.prep_preference {
                if let PrepRoomPreference::Demand(room) = pref {
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
                            .room_preference
                            .iter()
                            .any(|p| p.room_name() == room && p.can_share_with_prep());
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
}
