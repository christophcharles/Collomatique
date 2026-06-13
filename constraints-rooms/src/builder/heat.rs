use collomatique_ilp::LinExpr;

use super::{MyBundle, base_var};
use crate::heat_map::compute_heat_map;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    let weight = env.data.config.heat_weight;
    let prep_weight = env.data.config.prep_heat_weight;

    for (req_idx, req) in env.data.requests.iter().enumerate() {
        let heat = compute_heat_map(req, &env.data.rooms, false);
        for (room_name, &value) in &heat {
            if !env.has_interrogation_var(req_idx, room_name) {
                continue;
            }
            bundle = bundle.with_maximize(
                weight * value,
                LinExpr::var(base_var(Var::RoomForInterrogation {
                    request: req_idx,
                    room: room_name.clone(),
                })),
            );
        }

        let prep_heat = compute_heat_map(req, &env.data.rooms, true);
        for (room_name, &value) in &prep_heat {
            if !env.has_prep_var(req_idx, room_name) {
                continue;
            }
            bundle = bundle.with_maximize(
                prep_weight * value,
                LinExpr::var(base_var(Var::RoomForPrep {
                    request: req_idx,
                    room: room_name.clone(),
                })),
            );
        }
    }

    bundle
}
