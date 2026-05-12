use collomatique_rooms_model::Hour;
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {
    PriorityExhausted {
        period: usize,
        day: Weekday,
        hour: Hour,
        priority: u32,
    },
    PriorityPenalty,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {
    OneRoomPerRequest {
        request: usize,
    },
    OnePrepRoomPerRequest {
        request: usize,
    },
    RoomNotOverused {
        room: NonEmptyString,
        period: usize,
        day: Weekday,
        hour: Hour,
    },
    UndeclaredRoomExclusive {
        room: NonEmptyString,
        period: usize,
        day: Weekday,
        hour: Hour,
    },
    IncompatInterrogation {
        request: usize,
        room: NonEmptyString,
    },
    IncompatPrep {
        request: usize,
        room: NonEmptyString,
    },
    ReservedInterrogation {
        request: usize,
        room: NonEmptyString,
    },
    ReservedPrep {
        request: usize,
        room: NonEmptyString,
    },
    WindowInterrogation {
        request: usize,
        room: NonEmptyString,
    },
    WindowPrep {
        request: usize,
        room: NonEmptyString,
    },
    Priority {
        request: usize,
        room: NonEmptyString,
        period: usize,
        day: Weekday,
        hour: Hour,
    },
    MaxPriorityInterrogation {
        request: usize,
        room: NonEmptyString,
    },
    MaxPriorityPrep {
        request: usize,
        room: NonEmptyString,
    },
}
