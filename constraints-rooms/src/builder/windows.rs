use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_rooms_model::Window;

use super::{MyBundle, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for (room_name, room) in &env.data.rooms {
        if room.window != Window::None {
            continue;
        }

        for (req_idx, req) in env.data.requests.iter().enumerate() {
            if !req.window {
                continue;
            }

            if env.has_interrogation_var(req_idx, room_name) {
                bundle = bundle.with_constraint(
                    IntLinExpr::var(base_var(Var::RoomForInterrogation {
                        request: req_idx,
                        room: room_name.clone(),
                    }))
                    .leq(&IntLinExpr::constant(0)),
                    ConstraintDesc::WindowInterrogation {
                        request: req_idx,
                        room: room_name.clone(),
                    },
                );
            }
        }
    }

    bundle
}
