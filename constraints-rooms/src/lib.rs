mod builder;
mod heat_map;
mod types;
pub mod vars;

pub use builder::{build_model, build_modeler};
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::Var;

use std::collections::HashMap;

use collomatique_ilp::ConfigData;
use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_rooms_model::{ScheduleData, SolutionColumns};
use non_empty_string::NonEmptyString;

pub type RoomModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;

pub type RoomModeler<'m> = collomatique_ilp_modeler::Modeler<
    'm,
    Var,
    ExtraVarName,
    ConstraintDesc,
    vars::VarEnv,
    ReifyError<Var, ExtraVarName>,
>;

pub type RoomConstraintBundle = collomatique_ilp_modeler::ConstraintBundle<
    'static,
    Var,
    ExtraVarName,
    ConstraintDesc,
    vars::VarEnv,
    ReifyError<Var, ExtraVarName>,
>;

pub struct Assignment {
    pub request: usize,
    pub room: NonEmptyString,
    pub prep_room: Option<NonEmptyString>,
}

#[derive(Debug, Clone)]
pub enum SolutionWarning {
    UnknownInterrogationRoom { request: usize, room: String },
    UnknownPrepRoom { request: usize, room: String },
}

impl std::fmt::Display for SolutionWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolutionWarning::UnknownInterrogationRoom { request, room } => {
                write!(
                    f,
                    "request {request}: SolSalle room \"{room}\" is not a valid \
                     interrogation room for this request"
                )
            }
            SolutionWarning::UnknownPrepRoom { request, room } => {
                write!(
                    f,
                    "request {request}: SolPrep room \"{room}\" is not a valid \
                     prep room for this request"
                )
            }
        }
    }
}

impl std::error::Error for SolutionWarning {}

pub struct SolutionReconstruction {
    pub marked_pins: ConfigData<Var>,
    pub unmarked_pins: ConfigData<Var>,
    pub full_config: ConfigData<Var>,
    pub warnings: Vec<SolutionWarning>,
}

pub fn reconstruct_solution(
    data: &ScheduleData,
    solutions: &SolutionColumns,
) -> SolutionReconstruction {
    let env = vars::VarEnv::new(data);
    let mut full_values: HashMap<Var, f64> = HashMap::new();
    let mut marked_pin_values: HashMap<Var, f64> = HashMap::new();
    let mut unmarked_pin_values: HashMap<Var, f64> = HashMap::new();
    let mut warnings = Vec::new();

    for request in Var::compute_all_request_range(&env) {
        let rooms = Var::compute_interrogation_room_range(&env, &request);
        let sol_entry = &solutions[request].0;

        if let Some(sol) = sol_entry {
            let room_name = &sol.room;
            if rooms.contains(room_name) {
                let pin_map = if sol.mark_fixed {
                    &mut marked_pin_values
                } else {
                    &mut unmarked_pin_values
                };
                pin_map.insert(
                    Var::RoomForInterrogation {
                        request,
                        room: room_name.clone(),
                    },
                    1.0,
                );
                for room in &rooms {
                    let val = if room == room_name { 1.0 } else { 0.0 };
                    full_values.insert(
                        Var::RoomForInterrogation {
                            request,
                            room: room.clone(),
                        },
                        val,
                    );
                }
            } else {
                warnings.push(SolutionWarning::UnknownInterrogationRoom {
                    request,
                    room: <NonEmptyString as AsRef<str>>::as_ref(room_name).to_string(),
                });
                for room in &rooms {
                    full_values.insert(
                        Var::RoomForInterrogation {
                            request,
                            room: room.clone(),
                        },
                        0.0,
                    );
                }
            }
        } else {
            for room in &rooms {
                full_values.insert(
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
        let sol_entry = &solutions[request].1;

        if let Some(sol) = sol_entry {
            let room_name = &sol.room;
            if rooms.contains(room_name) {
                let pin_map = if sol.mark_fixed {
                    &mut marked_pin_values
                } else {
                    &mut unmarked_pin_values
                };
                pin_map.insert(
                    Var::RoomForPrep {
                        request,
                        room: room_name.clone(),
                    },
                    1.0,
                );
                for room in &rooms {
                    let val = if room == room_name { 1.0 } else { 0.0 };
                    full_values.insert(
                        Var::RoomForPrep {
                            request,
                            room: room.clone(),
                        },
                        val,
                    );
                }
            } else {
                warnings.push(SolutionWarning::UnknownPrepRoom {
                    request,
                    room: <NonEmptyString as AsRef<str>>::as_ref(room_name).to_string(),
                });
                for room in &rooms {
                    full_values.insert(
                        Var::RoomForPrep {
                            request,
                            room: room.clone(),
                        },
                        0.0,
                    );
                }
            }
        } else {
            for room in &rooms {
                full_values.insert(
                    Var::RoomForPrep {
                        request,
                        room: room.clone(),
                    },
                    0.0,
                );
            }
        }
    }

    SolutionReconstruction {
        marked_pins: ConfigData::from(marked_pin_values),
        unmarked_pins: ConfigData::from(unmarked_pin_values),
        full_config: ConfigData::from(full_values),
        warnings,
    }
}

pub fn build_config_from_solution(
    data: &ScheduleData,
    solutions: &SolutionColumns,
) -> Result<ConfigData<Var>, SolutionWarning> {
    let recon = reconstruct_solution(data, solutions);
    if let Some(w) = recon.warnings.into_iter().next() {
        return Err(w);
    }
    Ok(recon.full_config)
}

pub struct PinningBundles {
    pub fixed: RoomConstraintBundle,
    pub unfixed: RoomConstraintBundle,
    pub warnings: Vec<SolutionWarning>,
}

pub fn build_pinning_bundles(data: &ScheduleData, solutions: &SolutionColumns) -> PinningBundles {
    let recon = reconstruct_solution(data, solutions);
    let desc_fn = |var: &Var, _value: f64| match var {
        Var::RoomForInterrogation { request, room } => ConstraintDesc::PinnedInterrogation {
            request: *request,
            room: room.clone(),
        },
        Var::RoomForPrep { request, room } => ConstraintDesc::PinnedPrep {
            request: *request,
            room: room.clone(),
        },
    };
    let fixed = RoomConstraintBundle::from_config_data(&recon.marked_pins, &desc_fn);
    let unfixed = RoomConstraintBundle::from_config_data(&recon.unmarked_pins, &desc_fn);
    PinningBundles {
        fixed,
        unfixed,
        warnings: recon.warnings,
    }
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
