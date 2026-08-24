use collomatique_ilp::int_linexpr::IntLinExpr;

use super::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for request in Var::compute_all_request_range(env) {
        let rooms = Var::compute_interrogation_room_range(env, &request);
        let sum: IntLinExpr<V> = rooms
            .into_iter()
            .map(|room| IntLinExpr::var(base_var(Var::RoomForInterrogation { request, room })))
            .sum();
        bundle = bundle.with_constraint(
            sum.eq(&IntLinExpr::constant(1)),
            ConstraintDesc::OneRoomPerRequest { request },
        );
    }

    for request in Var::compute_prep_request_range(env) {
        let rooms = Var::compute_prep_room_range(env, &request);
        let sum: IntLinExpr<V> = rooms
            .into_iter()
            .map(|room| IntLinExpr::var(base_var(Var::RoomForPrep { request, room })))
            .sum();
        bundle = bundle.with_constraint(
            sum.eq(&IntLinExpr::constant(1)),
            ConstraintDesc::OnePrepRoomPerRequest { request },
        );
    }

    bundle
}
