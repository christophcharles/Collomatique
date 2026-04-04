use super::vars::Var;
pub use collo_ml::SqliteDatabaseDriver;
use collo_ml::eval::CompileError;
use collo_ml::problem::{ProblemBuilder, ProblemError};
use collo_ml::{DatabaseDriver, SemError, SemWarning};
use collomatique_ilp::ObjectiveSense;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum SimpleProblemError {
    UnexpectedError(String),
    ParsingError(pest::error::Error<collo_ml::parser::Rule>),
    SemanticErrors {
        errors: Vec<SemError>,
        warnings: Vec<SemWarning>,
    },
}

impl fmt::Display for SimpleProblemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimpleProblemError::UnexpectedError(msg) => write!(f, "{}", msg),
            SimpleProblemError::ParsingError(err) => write!(f, "{}", err),
            SimpleProblemError::SemanticErrors { errors, .. } => {
                for err in errors {
                    write!(f, "{}", err)?;
                }
                Ok(())
            }
        }
    }
}

const DB_MODULE_SOURCE: &str = const_format::concatcp!(
    "pub type Db = #{~\"",
    collomatique_sqlite_state::SCHEMA_SQL,
    "\"~};"
);

pub const MODULES: &[(&str, &str)] = &[
    (
        "collomatique",
        include_str!("scripts/collomatique.collo-ml"),
    ),
    ("collomatique_db", DB_MODULE_SOURCE),
    (
        "collomatique_types",
        include_str!("scripts/collomatique_types.collo-ml"),
    ),
    (
        "collomatique_queries",
        include_str!("scripts/collomatique_queries.collo-ml"),
    ),
    (
        "collomatique_vars",
        include_str!("scripts/collomatique_vars.collo-ml"),
    ),
    (
        "collomatique_forbidden_groups",
        include_str!("scripts/collomatique_forbidden_groups.collo-ml"),
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
        "collomatique_periodicity_amount_for_every_arbitrary_block",
        include_str!("scripts/collomatique_periodicity_amount_for_every_arbitrary_block.collo-ml"),
    ),
    (
        "collomatique_periodicity_amount_in_year",
        include_str!("scripts/collomatique_periodicity_amount_in_year.collo-ml"),
    ),
    (
        "collomatique_periodicity_exactly_periodic",
        include_str!("scripts/collomatique_periodicity_exactly_periodic.collo-ml"),
    ),
    (
        "collomatique_periodicity_helpers",
        include_str!("scripts/collomatique_periodicity_helpers.collo-ml"),
    ),
    (
        "collomatique_periodicity_once_for_every_block_of_weeks",
        include_str!("scripts/collomatique_periodicity_once_for_every_block_of_weeks.collo-ml"),
    ),
    (
        "collomatique_slot_pairings",
        include_str!("scripts/collomatique_slot_pairings.collo-ml"),
    ),
    (
        "collomatique_balancing_helpers",
        include_str!("scripts/collomatique_balancing_helpers.collo-ml"),
    ),
    (
        "collomatique_balancing_rotation",
        include_str!("scripts/collomatique_balancing_rotation.collo-ml"),
    ),
    (
        "collomatique_balancing_slot_rotation",
        include_str!("scripts/collomatique_balancing_slot_rotation.collo-ml"),
    ),
    (
        "collomatique_balancing_avoid_twice_in_a_row",
        include_str!("scripts/collomatique_balancing_avoid_twice_in_a_row.collo-ml"),
    ),
    (
        "collomatique_balancing_year_rotation",
        include_str!("scripts/collomatique_balancing_year_rotation.collo-ml"),
    ),
    (
        "collomatique_balancing_period_rotation",
        include_str!("scripts/collomatique_balancing_period_rotation.collo-ml"),
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
    (
        "collomatique_pairings",
        include_str!("scripts/collomatique_pairings.collo-ml"),
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

pub async fn default_problem_builder<T: DatabaseDriver>(
    main_module: &str,
) -> Result<ProblemBuilder<T, Var>, SimpleProblemError> {
    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", main_module);

    let mut builder = ProblemBuilder::<T, Var>::new(&modules).await.map_err(|e| {
        // Filter ProblemError into SimpleProblemError
        match e {
            ProblemError::CompileError(compile_error) => match compile_error {
                CompileError::ParsingError(parse_err) => {
                    SimpleProblemError::ParsingError(parse_err)
                }
                CompileError::SemanticsError { errors, warnings } => {
                    SimpleProblemError::SemanticErrors { errors, warnings }
                }
                other => SimpleProblemError::UnexpectedError(format!("{}", other)),
            },
            other => SimpleProblemError::UnexpectedError(format!("{}", other)),
        }
    })?;

    let functions = builder.get_fn_from_module("main");

    for (fn_name, _) in &functions {
        if fn_name == "constraint" || fn_name.starts_with("constraint_") {
            builder
                .add_constraint("main", fn_name, vec![])
                .map_err(|e| SimpleProblemError::UnexpectedError(format!("{}", e)))?;
        } else if fn_name == "objective" || fn_name.starts_with("objective_") {
            builder
                .add_objective("main", fn_name, vec![], 1.0, ObjectiveSense::Minimize)
                .map_err(|e| SimpleProblemError::UnexpectedError(format!("{}", e)))?;
        }
    }

    Ok(builder)
}
