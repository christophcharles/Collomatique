use collomatique_ilp::Variable;
use collomatique_ilp::linexpr::LinExpr;
use collomatique_ilp_modeler::Var as ModelerVar;
use collomatique_ilp_modeler::bundle::{ConstraintBundle, ExtraEntry};

use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};

type V = ModelerVar<Var, ExtraVarName>;
type MyBundle = ConstraintBundle<
    'static,
    Var,
    ExtraVarName,
    ConstraintDesc,
    VarEnv,
    collomatique_ilp_modeler::bundle::ReifyError<Var, ExtraVarName>,
>;

fn extra_var(e: ExtraVarName) -> V {
    ModelerVar::Extra(e)
}

fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

fn build_delta_pair(
    bundle: MyBundle,
    env: &VarEnv,
    req_idx: usize,
    pos_name: ExtraVarName,
    neg_name: ExtraVarName,
    coord_fn: fn(&collomatique_rooms_model::Room) -> f64,
    weight: f64,
) -> MyBundle {
    let empty_define =
        ExtraEntry::new(Variable::non_negative(), |_helpers, _ctx, _name| Ok(vec![]));
    let empty_define2 =
        ExtraEntry::new(Variable::non_negative(), |_helpers, _ctx, _name| Ok(vec![]));

    let interr_sum: LinExpr<V> = Var::compute_interrogation_room_range(env, &req_idx)
        .into_iter()
        .filter_map(|room_name| {
            let coord = coord_fn(env.data.rooms.get(&room_name)?);
            Some(
                coord
                    * LinExpr::var(base_var(Var::RoomForInterrogation {
                        request: req_idx,
                        room: room_name,
                    })),
            )
        })
        .sum();

    let prep_sum: LinExpr<V> = Var::compute_prep_room_range(env, &req_idx)
        .into_iter()
        .filter_map(|room_name| {
            let coord = coord_fn(env.data.rooms.get(&room_name)?);
            Some(
                coord
                    * LinExpr::var(base_var(Var::RoomForPrep {
                        request: req_idx,
                        room: room_name,
                    })),
            )
        })
        .sum();

    let delta: LinExpr<V> = &interr_sum - &prep_sum;
    let pos_var: LinExpr<V> = LinExpr::var(extra_var(pos_name.clone()));
    let neg_var: LinExpr<V> = LinExpr::var(extra_var(neg_name.clone()));
    let rhs: LinExpr<V> = &pos_var - &neg_var;

    let bundle = bundle
        .with_extra(pos_name.clone(), empty_define)
        .expect("no duplicate extras")
        .with_extra(neg_name.clone(), empty_define2)
        .expect("no duplicate extras")
        .with_constraint(
            delta.eq(&rhs),
            ConstraintDesc::ProximityDefinition { request: req_idx },
        );

    let obj_expr: LinExpr<V> = pos_var + neg_var;
    bundle.with_minimize(weight, obj_expr)
}

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    let xy_weight = env.data.config.proximity_weight;
    let floor_weight = env.data.config.proximity_floor_weight;

    for (req_idx, req) in env.data.requests.iter().enumerate() {
        if req.prep_students < 1 {
            continue;
        }

        bundle = build_delta_pair(
            bundle,
            env,
            req_idx,
            ExtraVarName::ProximityDeltaXPos { request: req_idx },
            ExtraVarName::ProximityDeltaXNeg { request: req_idx },
            |room| room.x as f64,
            xy_weight,
        );

        bundle = build_delta_pair(
            bundle,
            env,
            req_idx,
            ExtraVarName::ProximityDeltaYPos { request: req_idx },
            ExtraVarName::ProximityDeltaYNeg { request: req_idx },
            |room| room.y as f64,
            xy_weight,
        );

        bundle = build_delta_pair(
            bundle,
            env,
            req_idx,
            ExtraVarName::ProximityDeltaFloorPos { request: req_idx },
            ExtraVarName::ProximityDeltaFloorNeg { request: req_idx },
            |room| room.floor as f64,
            floor_weight,
        );
    }

    bundle
}
