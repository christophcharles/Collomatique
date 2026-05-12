use std::collections::{BTreeMap, HashSet};
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
}

/// Whether a room preference is a suggestion or a demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomPreference {
    Suggestion(NonEmptyString),
    Demand(NonEmptyString),
}

impl RoomPreference {
    pub fn room_name(&self) -> &NonEmptyString {
        match self {
            RoomPreference::Suggestion(name) | RoomPreference::Demand(name) => name,
        }
    }
}

/// A room incompatibility: the room is unavailable at this day/hour for the given periods.
#[derive(Debug, Clone)]
pub struct Incompat {
    pub room: NonEmptyString,
    pub p1: bool,
    pub p2: bool,
    pub p3: bool,
    pub day: Weekday,
    pub hour: Hour,
}

/// A scheduling request for a room.
#[derive(Debug, Clone)]
pub struct Request {
    pub p1: bool,
    pub p2: bool,
    pub p3: bool,
    pub day: Weekday,
    pub hour: Hour,
    pub subject: NonEmptyString,
    pub classes: Vec<NonEmptyString>,
    pub requester: NonEmptyString,
    pub teacher: NonEmptyString,
    pub blackboards: u32,
    pub window: bool,
    pub students: NonZeroU32,
    pub prep_students: u32,
    pub room_preference: Option<RoomPreference>,
    pub prep_preference: Option<RoomPreference>,
}

/// Parsed schedule data: rooms and requests.
#[derive(Debug, Clone)]
pub struct ScheduleData {
    pub rooms: BTreeMap<NonEmptyString, Room>,
    pub requests: Vec<Request>,
    pub incompats: Vec<Incompat>,
}

impl ScheduleData {
    pub fn unregistered_rooms(&self) -> Vec<&str> {
        let mut unregistered: Vec<&str> = self
            .requests
            .iter()
            .flat_map(|req| {
                [req.room_preference.as_ref(), req.prep_preference.as_ref()]
                    .into_iter()
                    .flatten()
            })
            .map(|pref| AsRef::<str>::as_ref(pref.room_name()))
            .filter(|name| !self.rooms.contains_key(*name))
            .collect::<HashSet<&str>>()
            .into_iter()
            .collect();
        unregistered.sort();
        unregistered
    }
}
