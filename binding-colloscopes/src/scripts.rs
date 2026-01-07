use super::{
    vars::Var,
    views::{Env, ObjectId},
};
use collo_ml::problem::{Problem, ProblemBuilder, ProblemError, Script};
use collomatique_ilp::ObjectiveSense;

mod constraints;
mod reifications;

#[cfg(test)]
mod tests;

pub fn build_default_problem(env: &Env) -> Result<Problem<ObjectId, Var>, String> {
    let mut builder = ProblemBuilder::<ObjectId, Var>::new(env)
        .expect("ObjectId, Var and Data should be compatible");
    for (name, script) in reifications::DEFAULT_REIFICATION_LIST {
        let stored_script = builder
            .compile_script(Script {
                name: name.to_string(),
                content: script.to_string(),
            })
            .expect(&format!("Should compile: {}\n\n{}", name, script));
        let funcs = stored_script.get_ast().get_functions();
        let to_reify = funcs
            .into_iter()
            .filter_map(|(name, (_args, output))| {
                if !output.is_constraint() {
                    return None;
                }
                let var_name = collo_ml::string_case::to_pascal_case(&name);
                Some((name, var_name))
            })
            .collect();
        builder
            .add_reified_variables_with_compiled_script(stored_script, to_reify)
            .map_err(|e| match e {
                ProblemError::Panic(value) => format!("{}", value),
                _ => panic!(
                    "Should be compatible with builder: {}\n\n{}\n\nError: {}",
                    name, script, e
                ),
            })?;
    }
    for (name, script) in constraints::DEFAULT_CONSTRAINT_LIST {
        let stored_script = builder
            .compile_script(Script {
                name: name.to_string(),
                content: script.to_string(),
            })
            .expect(&format!("Should compile: {}\n\n{}", name, script));
        let pub_funcs = stored_script.get_ast().get_functions();
        let funcs = if pub_funcs.contains_key("constraint") {
            vec![("constraint".to_string(), vec![])]
        } else {
            vec![]
        };
        let objectives = if pub_funcs.contains_key("objective") {
            vec![(
                "objective".to_string(),
                vec![],
                1.0,
                ObjectiveSense::Minimize,
            )]
        } else {
            vec![]
        };
        builder
            .add_constraints_and_objectives_with_compiled_script(
                stored_script.clone(),
                funcs,
                objectives,
            )
            .map_err(|e| match e {
                ProblemError::Panic(value) => format!("{}", value),
                _ => panic!("Should compile: {}\n\n{}\n\nError: {}", name, script, e),
            })?;
    }
    Ok(builder.build())
}
