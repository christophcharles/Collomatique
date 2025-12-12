use super::{
    vars::Var,
    views::{Env, ObjectId},
};
use collo_ml::problem::{Problem, ProblemBuilder, Script};

mod constraints;
mod reifications;

#[cfg(test)]
mod tests;

pub fn build_default_problem(env: &Env) -> Problem<ObjectId, Var> {
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
            .expect(&format!(
                "Should be compatible with builder: {}\n\n{}",
                name, script
            ));
    }
    for (name, script) in constraints::DEFAULT_CONSTRAINT_LIST {
        let _warnings = builder
            .add_constraints(
                Script {
                    name: name.to_string(),
                    content: script.to_string(),
                },
                vec![("constraint".to_string(), vec![])],
            )
            .expect(&format!("Should compile: {}\n\n{}", name, script));
    }
    builder.build()
}
