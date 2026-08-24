use collomatique_ilp::int_linexpr::IntLinExpr;

use super::{MyBundle, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for (room_name, room) in &env.data.rooms {
        if !room.reserved {
            continue;
        }

        for (req_idx, req) in env.data.requests.iter().enumerate() {
            if !req
                .periods
                .overlaps_with(&env.data.config.oral_exam_periods)
            {
                continue;
            }

            if env.has_interrogation_var(req_idx, room_name) {
                bundle = bundle.with_constraint(
                    IntLinExpr::var(base_var(Var::RoomForInterrogation {
                        request: req_idx,
                        room: room_name.clone(),
                    }))
                    .leq(&IntLinExpr::constant(0)),
                    ConstraintDesc::ReservedInterrogation {
                        request: req_idx,
                        room: room_name.clone(),
                    },
                );
            }

            if env.has_prep_var(req_idx, room_name) {
                bundle = bundle.with_constraint(
                    IntLinExpr::var(base_var(Var::RoomForPrep {
                        request: req_idx,
                        room: room_name.clone(),
                    }))
                    .leq(&IntLinExpr::constant(0)),
                    ConstraintDesc::ReservedPrep {
                        request: req_idx,
                        room: room_name.clone(),
                    },
                );
            }
        }
    }

    bundle
}
