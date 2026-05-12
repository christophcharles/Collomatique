use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{IntConstraintBundle, Modeler, Var as ModelerVar};

use crate::RoomModel;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{RoomScheduleInput, Var, VarEnv};

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

pub fn build_model(input: RoomScheduleInput) -> RoomModel {
    let env = VarEnv::new(input);
    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);
    modeler
        .apply_bundle(crate::constraints::build(&env).into_general())
        .expect("no duplicate extras");
    modeler
        .build(&env)
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e))
}
