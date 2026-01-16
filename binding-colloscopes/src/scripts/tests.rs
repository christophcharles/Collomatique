use std::collections::BTreeMap;
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
    let modules = BTreeMap::from([("default", SINGLE_MODULE)]);
    let _builder =
        ProblemBuilder::<ObjectId, Var>::new(&env, &modules).expect("Should compile single module");
}

#[test]
fn single_module_should_compile_without_warnings() {
    let data = collomatique_state_colloscopes::Data::new();
    let env = views::Env {
        params: data.get_inner_data().params.clone(),
        ignore_prefill_for_group_lists: BTreeSet::new(),
    };
    let modules = BTreeMap::from([("default", SINGLE_MODULE)]);
    let builder =
        ProblemBuilder::<ObjectId, Var>::new(&env, &modules).expect("Should compile single module");

    let warnings = builder.get_warnings();
    if !warnings.is_empty() {
        let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
        panic!(
            "Single module should compile without warnings!\nWarnings:\n{}",
            warnings_str.join("\n")
        );
    }
}
