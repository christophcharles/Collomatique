use super::{vars::Var, views::ObjectId};
use collo_ml::{
    problem::{Problem, ProblemBuilder, Script},
    ExprType,
};
use collomatique_state_colloscopes::Data;

mod constraints;
mod reifications;

#[cfg(test)]
mod tests;

pub fn build_default_problem(data: &Data) -> Problem<ObjectId, Var> {
    let mut builder = ProblemBuilder::<ObjectId, Var>::new(data)
        .expect("ObjectId, Var and Data should be compatible");
    for (name, script) in reifications::DEFAULT_REIFICATION_LIST {
        let stored_script = builder
            .compile_script(Script {
                name: name.to_string(),
                content: script.to_string(),
            })
            .expect(&format!("Should compile: {}\n\n{}", name, script));
        let warnings = stored_script.get_ast().get_warnings().clone();
        let funcs = stored_script.get_ast().get_functions();
        let to_reify = funcs
            .into_iter()
            .filter_map(|(name, (_args, output))| {
                if output != ExprType::Constraint {
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

        if !warnings.is_empty() {
            let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
            eprintln!("Warnings: {}", warnings_str.join("\n"));
        }
    }
    for (name, script) in constraints::DEFAULT_CONSTRAINT_LIST {
        let warnings = builder
            .add_constraints(
                Script {
                    name: name.to_string(),
                    content: script.to_string(),
                },
                vec![("constraint".to_string(), vec![])],
            )
            .expect(&format!("Should compile: {}\n\n{}", name, script));

        if !warnings.is_empty() {
            let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
            eprintln!("Warnings: {}", warnings_str.join("\n"));
        }
    }
    builder.build()
}
