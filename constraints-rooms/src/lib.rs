mod builder;
mod constraints;
mod types;
pub mod vars;

pub use builder::build_model;
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::{RequestInput, RoomScheduleInput, Var};

use collomatique_ilp::ConfigData;
use non_empty_string::NonEmptyString;

pub type RoomModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;

pub struct Assignment {
    pub request: usize,
    pub room: NonEmptyString,
    pub prep_room: Option<NonEmptyString>,
}

pub fn extract_assignments(input: &RoomScheduleInput, config: &ConfigData<Var>) -> Vec<Assignment> {
    let env = vars::VarEnv::new(input.clone());
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
