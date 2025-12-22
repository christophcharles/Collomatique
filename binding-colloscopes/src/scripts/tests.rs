use std::collections::BTreeSet;

use crate::views;

use super::*;

#[test]
fn all_scripts_should_compile() {
    let data = collomatique_state_colloscopes::Data::new();
    let env = views::Env {
        params: data.get_inner_data().params.clone(),
        ignore_prefill_for_group_lists: BTreeSet::new(),
    };
    let mut builder = ProblemBuilder::<ObjectId, Var>::new(&env)
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
        let _stored_script = builder
            .compile_script(Script {
                name: name.to_string(),
                content: script.to_string(),
            })
            .expect(&format!("Should compile: {}\n\n{}", name, script));
    }
}

#[test]
fn all_scripts_should_compile_without_warnings() {
    let data = collomatique_state_colloscopes::Data::new();
    let env = views::Env {
        params: data.get_inner_data().params.clone(),
        ignore_prefill_for_group_lists: BTreeSet::new(),
    };
    let mut builder = ProblemBuilder::<ObjectId, Var>::new(&env)
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
        if !warnings.is_empty() {
            let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
            panic!(
                "Script {} should compile without warnings!\nScript content:\n{}\nWarnings:\n{:?}",
                name,
                script,
                warnings_str.join("\n")
            );
        }
    }
    for (name, script) in constraints::DEFAULT_CONSTRAINT_LIST {
        let stored_script = builder
            .compile_script(Script {
                name: name.to_string(),
                content: script.to_string(),
            })
            .expect(&format!("Should compile: {}\n\n{}", name, script));
        let warnings = stored_script.get_ast().get_warnings();
        if !warnings.is_empty() {
            let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
            panic!(
                "Script {} should compile without warnings!\nScript content:\n{}\nWarnings:\n{}",
                name,
                script,
                warnings_str.join("\n")
            );
        }
    }
}
