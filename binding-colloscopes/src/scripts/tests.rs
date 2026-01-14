use std::collections::BTreeSet;

use crate::views;

use super::*;

#[test]
fn single_module_should_compile() {
    let data = collomatique_state_colloscopes::Data::new();
    let env = views::Env {
        params: data.get_inner_data().params.clone(),
        ignore_prefill_for_group_lists: BTreeSet::new(),
    };
    let builder = ProblemBuilder::<ObjectId, Var>::new(&env)
        .expect("ObjectId, Var and Data should be compatible");

    let _stored_script = builder
        .compile_script(Script {
            name: "default".to_string(),
            content: SINGLE_MODULE.to_string(),
        })
        .expect("Should compile single module");
}

#[test]
fn single_module_should_compile_without_warnings() {
    let data = collomatique_state_colloscopes::Data::new();
    let env = views::Env {
        params: data.get_inner_data().params.clone(),
        ignore_prefill_for_group_lists: BTreeSet::new(),
    };
    let builder = ProblemBuilder::<ObjectId, Var>::new(&env)
        .expect("ObjectId, Var and Data should be compatible");

    let stored_script = builder
        .compile_script(Script {
            name: "default".to_string(),
            content: SINGLE_MODULE.to_string(),
        })
        .expect("Should compile single module");

    let warnings = stored_script.get_ast().get_warnings();
    if !warnings.is_empty() {
        let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
        panic!(
            "Single module should compile without warnings!\nWarnings:\n{}",
            warnings_str.join("\n")
        );
    }
}
