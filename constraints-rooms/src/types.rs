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
    FixPenalty,
    ProximityDeltaXPos {
        request: usize,
    },
    ProximityDeltaXNeg {
        request: usize,
    },
    ProximityDeltaYPos {
        request: usize,
    },
    ProximityDeltaYNeg {
        request: usize,
    },
    ProximityDeltaFloorPos {
        request: usize,
    },
    ProximityDeltaFloorNeg {
        request: usize,
    },
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
    Priority {
        request: usize,
        room: NonEmptyString,
        period: usize,
        day: Weekday,
        hour: Hour,
        priority: u32,
    },
    GlobalPriority {
        request: usize,
        room: NonEmptyString,
        day: Weekday,
        hour: Hour,
        priority: u32,
    },
    MaxPriorityInterrogation {
        request: usize,
        room: NonEmptyString,
        priority: u32,
        max_priority: u32,
    },
    MaxPriorityPrep {
        request: usize,
        room: NonEmptyString,
        priority: u32,
        max_priority: u32,
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
    ExcludedPrep {
        request: usize,
        room: NonEmptyString,
    },
    BoardsInterrogation {
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
    PinnedInterrogation {
        request: usize,
        room: NonEmptyString,
    },
    PinnedPrep {
        request: usize,
        room: NonEmptyString,
    },
    ProximityDefinition {
        request: usize,
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
                    "Capacité de la salle \"{}\" dépassée le {} à {} (P{})",
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
                    "Capacité de la salle \"{}\" dépassée le {} à {} (P{}) \
                     (capacité inconnue - impossible d'avoir une gestion plus fine)",
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                    period + 1,
                )
            }
            ConstraintDesc::IncompatInterrogation { request, room } => {
                let req = &data.requests[*request];
                format!(
                    "{} : salle \"{}\" déjà utilisée le {} à {} (incompats)",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    req.day,
                    req.hour,
                )
            }
            ConstraintDesc::IncompatPrep { request, room } => {
                let req = &data.requests[*request];
                format!(
                    "{} : salle de préparation \"{}\" déjà utilisée le {} à {} (incompats)",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    req.day,
                    req.hour,
                )
            }
            ConstraintDesc::ReservedInterrogation { request, room } => {
                format!(
                    "{} : salle \"{}\" réservée pour les oraux blancs de fin d'année",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::ReservedPrep { request, room } => {
                format!(
                    "{} : salle de préparation \"{}\" réservée pour les oraux blancs de fin d'année",
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
            ConstraintDesc::Priority {
                request,
                room,
                period,
                day,
                hour,
                priority,
            } => {
                format!(
                    "{} : salle \"{}\" utilisée sans avoir épuisé les salles \
                     de priorité {} pour la période P{} le {} à {}",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    priority,
                    period + 1,
                    day,
                    hour,
                )
            }
            ConstraintDesc::GlobalPriority {
                request,
                room,
                day,
                hour,
                priority,
            } => {
                format!(
                    "{} : salle \"{}\" utilisée sans avoir épuisé les salles \
                     de priorité {} disponibles sur l'année le {} à {}",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    priority,
                    day,
                    hour,
                )
            }
            ConstraintDesc::MaxPriorityInterrogation {
                request,
                room,
                priority,
                max_priority,
            } => {
                format!(
                    "{} : la salle \"{}\" a une priorité trop élevée ({} > {})",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    priority,
                    max_priority,
                )
            }
            ConstraintDesc::MaxPriorityPrep {
                request,
                room,
                priority,
                max_priority,
            } => {
                format!(
                    "{} : la salle de préparation \"{}\" a une priorité trop élevée ({} > {})",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    priority,
                    max_priority,
                )
            }
            ConstraintDesc::OneInterrogationPerRoom {
                room,
                period,
                day,
                hour,
            } => {
                format!(
                    "La salle \"{}\" est utilisée simultanément par plusieurs colleurs \
                     le {} à {} (P{})",
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    day,
                    hour,
                    period + 1,
                )
            }
            ConstraintDesc::ExcludedInterrogation { request, room } => {
                let req = &data.requests[*request];
                format!(
                    "{} : l'enseignant {} a demandé à ne pas avoir la salle \"{}\" \
                     de manière catégorique",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(&req.teacher),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::ExcludedPrep { request, room } => {
                let req = &data.requests[*request];
                format!(
                    "{} : l'enseignant {} a demandé à ne pas avoir la salle de préparation \"{}\" \
                     de manière catégorique",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(&req.teacher),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::BoardsInterrogation { request, room } => {
                let req = &data.requests[*request];
                format!(
                    "{} : la salle \"{}\" n'a pas assez de tableaux noirs ({} requis)",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    req.boards.hard_target(),
                )
            }
            ConstraintDesc::RoomContinuityEqual {
                request_a,
                request_b,
                room,
            } => {
                format!(
                    "Continuité de l'utilisation de la salle : les requêtes {} et {} \
                     devraient utiliser la même salle. Une seule utilise la salle \"{}\"",
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
                    "Continuité de l'utilisation de la salle : {} ne peut utiliser \
                     la salle \"{}\" car elle est indisponible pour la requête {}",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                    neighbor_request,
                )
            }
            ConstraintDesc::PinnedInterrogation { request, room } => {
                format!(
                    "{} : salle d'interrogation \"{}\" fixée par l'utilisateur",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::PinnedPrep { request, room } => {
                format!(
                    "{} : salle de préparation \"{}\" fixée par l'utilisateur",
                    format_request(data, *request),
                    <NonEmptyString as AsRef<str>>::as_ref(room),
                )
            }
            ConstraintDesc::ProximityDefinition { request } => {
                format!(
                    "{} : définition de la distance interrogation-préparation",
                    format_request(data, *request),
                )
            }
        }
    }
}
