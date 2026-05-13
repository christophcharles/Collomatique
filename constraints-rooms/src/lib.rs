mod builder;
mod types;
pub mod vars;

pub use builder::build_model;
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::Var;

use std::collections::HashMap;

use collomatique_ilp::ConfigData;
use collomatique_rooms_model::ScheduleData;
use non_empty_string::NonEmptyString;

pub type RoomModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;

pub struct Assignment {
    pub request: usize,
    pub room: NonEmptyString,
    pub prep_room: Option<NonEmptyString>,
}

#[derive(Debug, Clone)]
pub enum CheckError {
    UnknownInterrogationRoom { request: usize, room: String },
    UnknownPrepRoom { request: usize, room: String },
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::UnknownInterrogationRoom { request, room } => {
                write!(
                    f,
                    "request {request}: SolSalle room \"{room}\" is not a valid \
                     interrogation room for this request"
                )
            }
            CheckError::UnknownPrepRoom { request, room } => {
                write!(
                    f,
                    "request {request}: SolPrep room \"{room}\" is not a valid \
                     prep room for this request"
                )
            }
        }
    }
}

impl std::error::Error for CheckError {}

pub fn build_config_from_solution(
    data: &ScheduleData,
    solutions: &[(Option<NonEmptyString>, Option<NonEmptyString>)],
) -> Result<ConfigData<Var>, CheckError> {
    let env = vars::VarEnv::new(data);
    let mut values: HashMap<Var, f64> = HashMap::new();

    for request in Var::compute_all_request_range(&env) {
        let rooms = Var::compute_interrogation_room_range(&env, &request);
        let sol_room = &solutions[request].0;

        if let Some(room_name) = sol_room {
            if !rooms.contains(room_name) {
                return Err(CheckError::UnknownInterrogationRoom {
                    request,
                    room: <NonEmptyString as AsRef<str>>::as_ref(room_name).to_string(),
                });
            }
            for room in &rooms {
                let val = if room == room_name { 1.0 } else { 0.0 };
                values.insert(
                    Var::RoomForInterrogation {
                        request,
                        room: room.clone(),
                    },
                    val,
                );
            }
        } else {
            for room in &rooms {
                values.insert(
                    Var::RoomForInterrogation {
                        request,
                        room: room.clone(),
                    },
                    0.0,
                );
            }
        }
    }

    for request in Var::compute_prep_request_range(&env) {
        let rooms = Var::compute_prep_room_range(&env, &request);
        let sol_room = &solutions[request].1;

        if let Some(room_name) = sol_room {
            if !rooms.contains(room_name) {
                return Err(CheckError::UnknownPrepRoom {
                    request,
                    room: <NonEmptyString as AsRef<str>>::as_ref(room_name).to_string(),
                });
            }
            for room in &rooms {
                let val = if room == room_name { 1.0 } else { 0.0 };
                values.insert(
                    Var::RoomForPrep {
                        request,
                        room: room.clone(),
                    },
                    val,
                );
            }
        } else {
            for room in &rooms {
                values.insert(
                    Var::RoomForPrep {
                        request,
                        room: room.clone(),
                    },
                    0.0,
                );
            }
        }
    }

    Ok(ConfigData::from(values))
}

pub fn extract_assignments(data: &ScheduleData, config: &ConfigData<Var>) -> Vec<Assignment> {
    let env = vars::VarEnv::new(data);
    let mut assignments = Vec::new();

    for request in Var::compute_all_request_range(&env) {
        let rooms = Var::compute_interrogation_room_range(&env, &request);
        let room = rooms
            .into_iter()
            .find(|room| {
                config
                    .get(Var::RoomForInterrogation {
                        request,
                        room: room.clone(),
                    })
                    .is_some_and(|v| v > 0.5)
            })
            .expect("each request must have exactly one assigned room");

        let prep_room = if Var::compute_prep_request_range(&env).contains(&request) {
            let prep_rooms = Var::compute_prep_room_range(&env, &request);
            Some(
                prep_rooms
                    .into_iter()
                    .find(|room| {
                        config
                            .get(Var::RoomForPrep {
                                request,
                                room: room.clone(),
                            })
                            .is_some_and(|v| v > 0.5)
                    })
                    .expect("each prep request must have exactly one assigned prep room"),
            )
        } else {
            None
        };

        assignments.push(Assignment {
            request,
            room,
            prep_room,
        });
    }

    assignments
}
