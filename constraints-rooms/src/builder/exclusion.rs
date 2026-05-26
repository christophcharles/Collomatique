use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_rooms_model::InterrogationRoomStatus;

use super::{MyBundle, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for (req_idx, req) in env.data.requests.iter().enumerate() {
        for (room, status) in &req.room_statuses {
            if matches!(status, InterrogationRoomStatus::Excluded) {
                if env.has_interrogation_var(req_idx, room) {
                    bundle = bundle.with_constraint(
                        IntLinExpr::var(base_var(Var::RoomForInterrogation {
                            request: req_idx,
                            room: room.clone(),
                        }))
                        .leq(&IntLinExpr::constant(0)),
                        ConstraintDesc::ExcludedInterrogation {
                            request: req_idx,
                            room: room.clone(),
                        },
                    );
                }
            }
        }
    }

    bundle
}
