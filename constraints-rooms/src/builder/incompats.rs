use std::collections::BTreeSet;

use collomatique_ilp::int_linexpr::IntLinExpr;
use non_empty_string::NonEmptyString;

use super::{MyBundle, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    let mut blocked_interrogation: BTreeSet<(usize, NonEmptyString)> = BTreeSet::new();
    let mut blocked_prep: BTreeSet<(usize, NonEmptyString)> = BTreeSet::new();

    for (req_idx, req) in env.data.requests.iter().enumerate() {
        for incompat in &env.data.incompats {
            if req.day != incompat.day || req.hour != incompat.hour {
                continue;
            }
            if !req.periods.overlaps_with(&incompat.periods) {
                continue;
            }

            let room = &incompat.room;

            if env.has_interrogation_var(req_idx, room) {
                blocked_interrogation.insert((req_idx, room.clone()));
            }
            if env.has_prep_var(req_idx, room) {
                blocked_prep.insert((req_idx, room.clone()));
            }
        }
    }

    for (request, room) in blocked_interrogation {
        bundle = bundle.with_constraint(
            IntLinExpr::var(base_var(Var::RoomForInterrogation {
                request,
                room: room.clone(),
            }))
            .leq(&IntLinExpr::constant(0)),
            ConstraintDesc::IncompatInterrogation { request, room },
        );
    }

    for (request, room) in blocked_prep {
        bundle = bundle.with_constraint(
            IntLinExpr::var(base_var(Var::RoomForPrep {
                request,
                room: room.clone(),
            }))
            .leq(&IntLinExpr::constant(0)),
            ConstraintDesc::IncompatPrep { request, room },
        );
    }

    bundle
}
