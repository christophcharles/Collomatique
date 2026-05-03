pub use collo_ml::SqliteDatabaseDriver;
use collo_ml::{SemError, SemWarning};
use std::fmt;

#[derive(Debug, Clone)]
pub enum SimpleScriptError {
    UnexpectedError(String),
    ParsingError(pest::error::Error<collo_ml::parser::Rule>),
    SemanticErrors {
        errors: Vec<SemError>,
        warnings: Vec<SemWarning>,
    },
}

impl fmt::Display for SimpleScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimpleScriptError::UnexpectedError(msg) => write!(f, "{}", msg),
            SimpleScriptError::ParsingError(err) => write!(f, "{}", err),
            SimpleScriptError::SemanticErrors { errors, .. } => {
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
        "collomatique_incompats",
        include_str!("scripts/collomatique_incompats.collo-ml"),
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
