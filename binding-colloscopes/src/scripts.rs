use super::{
    vars::Var,
    views::{Env, ObjectId},
};
use collo_ml::problem::{Problem, ProblemBuilder, Script};
use collomatique_ilp::ObjectiveSense;

// mod constraints;
// mod reifications;

pub const SINGLE_MODULE: &'static str = include_str!("scripts/single-module.collo-ml");

#[cfg(test)]
mod tests;

pub fn build_default_problem(env: &Env) -> Result<Problem<ObjectId, Var>, String> {
    let mut builder = ProblemBuilder::<ObjectId, Var>::new(env)
        .expect("ObjectId, Var and Data should be compatible");

    let stored_script = builder
        .compile_script(Script {
            name: "default".to_string(),
            content: SINGLE_MODULE.to_string(),
        })
        .expect("Should compile single module");

    // Reifications are handled automatically via `reify` statements in the module
    // Just add constraints and objectives
    builder
        .add_constraints_and_objectives_with_compiled_script(
            stored_script,
            vec![("constraint".to_string(), vec![])],
            vec![(
                "objective".to_string(),
                vec![],
                1.0,
                ObjectiveSense::Minimize,
            )],
        )
        .map_err(|e| format!("{}", e))?;

    Ok(builder.build())
}
