use collomatique_rooms_model::{Hour, ScheduleData};
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
    GlobalPriorityExhausted {
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
    GlobalPriority {
        request: usize,
        room: NonEmptyString,
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
    OneInterrogationPerRoom {
        room: NonEmptyString,
        period: usize,
        day: Weekday,
        hour: Hour,
    },
    ExcludedInterrogation {
        request: usize,
        room: NonEmptyString,
    },
    RoomContinuityEqual {
        request_a: usize,
        request_b: usize,
        room: NonEmptyString,
    },
    RoomContinuityExcluded {
        request: usize,
        room: NonEmptyString,
        neighbor_request: usize,
    },
}

fn format_request(data: &ScheduleData, request: usize) -> String {
    let req = &data.requests[request];
    let subjects: Vec<&str> = req.subjects.iter().map(|s| s.as_ref()).collect();
    format!(
        "requête {} ({}, colleur : {}, responsable : {})",
        request,
        subjects.join(";"),
        <NonEmptyString as AsRef<str>>::as_ref(&req.teacher),
        <NonEmptyString as AsRef<str>>::as_ref(&req.requester),
    )
}

impl ConstraintDesc {
    pub fn user_readable(&self, data: &ScheduleData) -> String {
        match self {
            ConstraintDesc::OneRoomPerRequest { request } => {
                format!(
                    "{} : aucune salle d'interrogation attribuée",
                    format_request(data, *request),
                )
            }
            ConstraintDesc::OnePrepRoomPerRequest { request } => {
                format!(
                    "{} : aucune salle de préparation attribuée",
                    format_request(data, *request),
                )
            }
            ConstraintDesc::RoomNotOverused {
                room,
                period,
                day,
                hour,
            } => {
                format!(
                    "Salle \"{}\" sur-utilisée le {} à {} (P{})",
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                    period + 1,
                )
            }
            ConstraintDesc::UndeclaredRoomExclusive {
                room,
                period,
                day,
                hour,
            } => {
                format!(
                    "Salle non-déclarée \"{}\" utilisée par plusieurs requêtes le {} à {} (P{})",
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                    period + 1,
                )
            }
            ConstraintDesc::IncompatInterrogation { request, room } => {
                format!(
                    "{} : salle \"{}\" incompatible pour l'interrogation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::IncompatPrep { request, room } => {
                format!(
                    "{} : salle \"{}\" incompatible pour la préparation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::ReservedInterrogation { request, room } => {
                format!(
                    "{} : salle \"{}\" ne satisfait pas les contraintes de capacité/équipement \
                     pour l'interrogation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::ReservedPrep { request, room } => {
                format!(
                    "{} : salle \"{}\" ne satisfait pas les contraintes pour la préparation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::WindowInterrogation { request, room } => {
                format!(
                    "{} : salle \"{}\" n'a pas de fenêtre (fenêtre requise)",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::WindowPrep { request, room } => {
                format!(
                    "{} : salle \"{}\" n'a pas de fenêtre pour la préparation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::Priority {
                request,
                room,
                period,
                day,
                hour,
            } => {
                format!(
                    "{} : salle \"{}\" non prioritaire le {} à {} (P{})",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                    period + 1,
                )
            }
            ConstraintDesc::GlobalPriority {
                request,
                room,
                day,
                hour,
            } => {
                format!(
                    "{} : salle \"{}\" non prioritaire le {} à {}",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                )
            }
            ConstraintDesc::MaxPriorityInterrogation { request, room } => {
                format!(
                    "{} : salle \"{}\" dépasse la priorité max pour l'interrogation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::MaxPriorityPrep { request, room } => {
                format!(
                    "{} : salle \"{}\" dépasse la priorité max pour la préparation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::OneInterrogationPerRoom {
                room,
                period,
                day,
                hour,
            } => {
                format!(
                    "Salle \"{}\" : plusieurs interrogations le {} à {} (P{})",
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                    period + 1,
                )
            }
            ConstraintDesc::ExcludedInterrogation { request, room } => {
                format!(
                    "{} : salle \"{}\" est exclue pour l'interrogation",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::RoomContinuityEqual {
                request_a,
                request_b,
                room,
            } => {
                format!(
                    "Continuité de salle : les requêtes {} et {} devraient partager la salle \"{}\"",
                    request_a,
                    request_b,
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::RoomContinuityExcluded {
                request,
                room,
                neighbor_request,
            } => {
                format!(
                    "Continuité de salle : {} ne devrait pas utiliser \"{}\" \
                     (conflit avec requête {})",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    neighbor_request,
                )
            }
        }
    }
}
