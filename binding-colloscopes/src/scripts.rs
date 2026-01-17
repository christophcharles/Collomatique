use super::{vars::Var, views::ObjectId};
use collo_ml::problem::ProblemBuilder;
use collomatique_ilp::ObjectiveSense;
use std::collections::BTreeMap;

pub const MODULES: &[(&str, &str)] = &[
    (
        "collomatique",
        include_str!("scripts/collomatique.collo-ml"),
    ),
    (
        "collomatique_vars",
        include_str!("scripts/collomatique_vars.collo-ml"),
    ),
    (
        "collomatique_group_count_per_group_list",
        include_str!("scripts/collomatique_group_count_per_group_list.collo-ml"),
    ),
    (
        "collomatique_group_count_per_interrogation",
        include_str!("scripts/collomatique_group_count_per_interrogation.collo-ml"),
    ),
    (
        "collomatique_groups_filled_by_ascending_order",
        include_str!("scripts/collomatique_groups_filled_by_ascending_order.collo-ml"),
    ),
    (
        "collomatique_incompats",
        include_str!("scripts/collomatique_incompats.collo-ml"),
    ),
    (
        "collomatique_interrogation_cost",
        include_str!("scripts/collomatique_interrogation_cost.collo-ml"),
    ),
    (
        "collomatique_limits",
        include_str!("scripts/collomatique_limits.collo-ml"),
    ),
    (
        "collomatique_one_interrogation_at_once",
        include_str!("scripts/collomatique_one_interrogation_at_once.collo-ml"),
    ),
    (
        "collomatique_sealed_groups",
        include_str!("scripts/collomatique_sealed_groups.collo-ml"),
    ),
    (
        "collomatique_students_have_groups",
        include_str!("scripts/collomatique_students_have_groups.collo-ml"),
    ),
    (
        "collomatique_students_per_group",
        include_str!("scripts/collomatique_students_per_group.collo-ml"),
    ),
    (
        "collomatique_students_per_group_for_subject",
        include_str!("scripts/collomatique_students_per_group_for_subject.collo-ml"),
    ),
];

pub const MAIN_MODULE: &str = include_str!("scripts/main.collo-ml");

pub fn get_default_main_module() -> &'static str {
    MAIN_MODULE
}

pub fn get_modules() -> &'static [(&'static str, &'static str)] {
    MODULES
}

#[cfg(test)]
mod tests;

pub fn default_problem_builder(main_module: &str) -> Result<ProblemBuilder<ObjectId, Var>, String> {
    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", main_module);

    let mut builder =
        ProblemBuilder::<ObjectId, Var>::new(&modules).map_err(|e| format!("{}", e))?;

    builder
        .add_constraint("main", "constraint", vec![])
        .map_err(|e| format!("{}", e))?;

    builder
        .add_objective("main", "objective", vec![], 1.0, ObjectiveSense::Minimize)
        .map_err(|e| format!("{}", e))?;

    Ok(builder)
}
