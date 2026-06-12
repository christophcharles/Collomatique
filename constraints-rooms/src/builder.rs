mod assignment;
mod boards;
mod capacity;
mod continuity;
mod exclusion;
mod exclusivity;
mod incompats;
mod priority;
mod reservation;
mod windows;

use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{IntConstraintBundle, Modeler, Var as ModelerVar};
use collomatique_rooms_model::ScheduleData;

use crate::RoomModel;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};

pub(crate) type V = ModelerVar<Var, ExtraVarName>;

pub(crate) type MyBundle = IntConstraintBundle<
    'static,
    Var,
    ExtraVarName,
    ConstraintDesc,
    VarEnv,
    ReifyError<Var, ExtraVarName>,
>;

type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

pub(crate) fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

pub(crate) fn extra_var(e: ExtraVarName) -> V {
    ModelerVar::Extra(e)
}

pub fn build_modeler(data: &ScheduleData) -> (MyModeler<'static>, VarEnv) {
    let env = VarEnv::new(data);
    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);

    modeler
        .apply_bundle(assignment::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(capacity::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(boards::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(exclusion::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(exclusivity::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(incompats::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(reservation::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(windows::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(priority::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .apply_bundle(continuity::build(&env).into_general())
        .expect("no duplicate extras");

    (modeler, env)
}

pub fn build_model(data: &ScheduleData) -> RoomModel {
    let (modeler, env) = build_modeler(data);
    modeler
        .build_with_log(&env, &mut |msg| eprintln!("{msg}"))
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e))
}
