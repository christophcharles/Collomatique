use collomatique_ilp::LinExpr;
use collomatique_ilp::int_linexpr::IntLinExpr;

use super::{MyBundle, base_var, is_accepted_or_demanded};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    let soft_weight = env.data.config.soft_boards_weight;

    for (room_name, room) in &env.data.rooms {
        for (req_idx, req) in env.data.requests.iter().enumerate() {
            let hard_target = req.boards.hard_target();
            let target = req.boards.target();

            if target == 0 {
                continue;
            }

            let accepted_or_demanded = is_accepted_or_demanded(req, room_name);

            if room.blackboards < hard_target as f32 && !accepted_or_demanded {
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
            } else if room.blackboards < target as f32 && accepted_or_demanded {
                if env.has_interrogation_var(req_idx, room_name) {
                    let shortfall = (target as f32 - room.blackboards) as f64;
                    bundle = bundle.with_minimize(
                        soft_weight * shortfall,
                        LinExpr::var(base_var(Var::RoomForInterrogation {
                            request: req_idx,
                            room: room_name.clone(),
                        })),
                    );
                }
            }
        }
    }

    bundle
}
