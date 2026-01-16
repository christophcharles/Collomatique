use super::{
    vars::Var,
    views::{Env, ObjectId},
};
use collo_ml::problem::{Problem, ProblemBuilder};
use collomatique_ilp::ObjectiveSense;
use std::collections::BTreeMap;

// mod constraints;
// mod reifications;

pub const SINGLE_MODULE: &'static str = include_str!("scripts/single-module.collo-ml");

#[cfg(test)]
mod tests;

pub fn build_default_problem(env: &Env) -> Result<Problem<ObjectId, Var>, String> {
    let modules = BTreeMap::from([("default", SINGLE_MODULE)]);

    let mut builder =
        ProblemBuilder::<ObjectId, Var>::new(env, &modules).map_err(|e| format!("{}", e))?;

    // Reifications are handled automatically via `reify` statements in the module
    // Just add constraints and objectives
    builder
        .add_constraint("default", "constraint", vec![])
        .map_err(|e| format!("{}", e))?;

    builder
        .add_objective(
            "default",
            "objective",
            vec![],
            1.0,
            ObjectiveSense::Minimize,
        )
        .map_err(|e| format!("{}", e))?;

    Ok(builder.build())
}
