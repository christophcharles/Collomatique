use super::*;
use crate::database::SqliteDatabaseDriver;
use crate::parser::{ColloMLParser, Rule};
use crate::semantics::global_env::ObjectFields;
use pest::Parser;
use std::collections::{BTreeMap, HashMap};

// Test modules organized by functionality
mod basic_functions;
mod coercion;
mod collections;
mod control_flow;
mod custom_types;
mod database;
mod database_schema;
mod enums;
mod folds;
mod let_expr;
mod match_expr;
mod modules;
mod operators;
mod recursivity;
mod scoping;
mod statements;
mod structs;
mod sum_types;
mod tuples;
mod type_system;
mod warnings;

/// Helper function to analyze a CoLLo-ML program and return the GlobalEnv, errors, and warnings
pub(crate) async fn analyze_with_env(
    input: &str,
) -> (
    GlobalEnv<SqliteDatabaseDriver>,
    Vec<SemError>,
    Vec<SemWarning>,
) {
    let pairs = ColloMLParser::parse(Rule::file, input).expect("Parse failed");
    let file = crate::ast::File::from_pest(pairs.into_iter().next().unwrap())
        .expect("AST conversion failed");
    let modules = BTreeMap::from([("main", file)]);
    let (global_env, _type_info, _expr_types, _resolved_types, errors, warnings) =
        GlobalEnv::<SqliteDatabaseDriver>::new(HashMap::new(), HashMap::new(), &modules)
            .await
            .expect("GlobalEnv creation failed");
    (global_env, errors, warnings)
}

/// Helper function to analyze a CoLLo-ML program and return type information, errors, and warnings
pub(crate) async fn analyze(
    input: &str,
    types: HashMap<String, ObjectFields>,
    vars: HashMap<String, ArgsType>,
) -> (TypeInfo, Vec<SemError>, Vec<SemWarning>) {
    let pairs = ColloMLParser::parse(Rule::file, input).expect("Parse failed");
    let file = crate::ast::File::from_pest(pairs.into_iter().next().unwrap())
        .expect("AST conversion failed");

    let modules = BTreeMap::from([("main", file)]);
    let (_global_env, type_info, _expr_types, _resolved_types, errors, warnings) =
        GlobalEnv::<SqliteDatabaseDriver>::new(types, vars, &modules)
            .await
            .expect("GlobalEnv creation failed");

    (type_info, errors, warnings)
}

/// Helper to create a variable with specific argument types
pub(crate) fn var_with_args(name: &str, args: Vec<SimpleType>) -> HashMap<String, ArgsType> {
    let mut vars = HashMap::new();
    vars.insert(
        name.to_string(),
        args.into_iter()
            .map(|x| ExprType::simple(x))
            .collect::<Vec<_>>(),
    );
    vars
}

/// Helper function to analyze a multi-module CoLLo-ML program
pub(crate) async fn analyze_multi(
    module_sources: &[(&str, &str)], // (module_name, source_code)
    types: HashMap<String, ObjectFields>,
    vars: HashMap<String, ArgsType>,
) -> (TypeInfo, Vec<SemError>, Vec<SemWarning>) {
    let modules: BTreeMap<&str, crate::ast::File> = module_sources
        .iter()
        .map(|(name, source)| {
            let pairs = ColloMLParser::parse(Rule::file, source).expect("Parse failed");
            let file = crate::ast::File::from_pest(pairs.into_iter().next().unwrap())
                .expect("AST conversion failed");
            (*name, file)
        })
        .collect();

    let (_global_env, type_info, _expr_types, _resolved_types, errors, warnings) =
        GlobalEnv::<SqliteDatabaseDriver>::new(types, vars, &modules)
            .await
            .expect("GlobalEnv creation failed");

    (type_info, errors, warnings)
}
