use collomatique_ilp::int_linexpr::IntLinExpr;

use super::{MyBundle, base_var, is_accepted_or_demanded};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for (room_name, room) in &env.data.rooms {
        for (req_idx, req) in env.data.requests.iter().enumerate() {
            let hard_target = req.boards.hard_target();
            if hard_target == 0 {
                continue;
            }

            if room.blackboards < hard_target as f32 && !is_accepted_or_demanded(req, room_name) {
                if env.has_interrogation_var(req_idx, room_name) {
                    bundle = bundle.with_constraint(
                        IntLinExpr::var(base_var(Var::RoomForInterrogation {
                            request: req_idx,
                            room: room_name.clone(),
                        }))
                        .leq(&IntLinExpr::constant(0)),
                        ConstraintDesc::BoardsInterrogation {
                            request: req_idx,
                            room: room_name.clone(),
                        },
                    );
                }
            }
        }
    }

    bundle
}
