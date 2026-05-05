use crate::native_extras::build_native_extras;
use crate::problem::Problem;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp::Variable;
use collomatique_ilp_modeler::Modeler;
use collomatique_ilp_modeler::bundle::ReifyError;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemBuilder;

pub fn default_problem_builder() -> ProblemBuilder {
    ProblemBuilder
}

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ExtraVarName, ConstraintDesc, VarEnv, ReifyError<Var, ExtraVarName>>;

pub async fn build_problem(db: &sqlx::SqlitePool) -> Problem {
    let env = VarEnv::load(db).await;

    let mut modeler: MyModeler<'_> = Modeler::from_described(&env);

    let original_var_list: HashMap<Var, Variable> = modeler
        .base_vars()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let native_bundle = build_native_extras(&env);
    modeler
        .apply_bundle(native_bundle.into_general())
        .expect("no duplicate extras from native");

    let groups_bundle = crate::groups::build(&env);
    modeler
        .apply_bundle(groups_bundle.into_general())
        .expect("no duplicate extras from groups");

    let schedule_structure_bundle = crate::schedule_structure::build(&env);
    modeler
        .apply_bundle(schedule_structure_bundle.into_general())
        .expect("no duplicate extras from schedule_structure");

    let pairings_bundle = crate::pairings::build(&env);
    modeler
        .apply_bundle(pairings_bundle.into_general())
        .expect("no duplicate extras from pairings");

    let misc_bundle = crate::misc::build(&env);
    modeler
        .apply_bundle(misc_bundle.into_general())
        .expect("no duplicate extras from misc");

    let periodicity_bundle = crate::periodicity::build(&env);
    modeler
        .apply_bundle(periodicity_bundle.into_general())
        .expect("no duplicate extras from periodicity");

    let balancing_bundle = crate::balancing::build(&env);
    modeler
        .apply_bundle(balancing_bundle.into_general())
        .expect("no duplicate extras from balancing");

    let model = modeler
        .build(&env)
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e));

    Problem::from_model(model, original_var_list)
}
