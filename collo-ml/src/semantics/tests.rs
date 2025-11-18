use super::*;
use crate::parser::{ColloMLParser, Rule};
use pest::Parser;
use std::collections::HashMap;

// Test modules organized by functionality
mod basic_functions;
mod coercion;
mod collections;
mod control_flow;
mod operators;
mod scoping;
mod statements;
mod type_system;
mod warnings;

/// Helper function to analyze a CoLLo-ML program and return type information, errors, and warnings
pub(crate) fn analyze(
    input: &str,
    types: HashMap<String, ObjectFields>,
    vars: HashMap<String, ArgsType>,
) -> (TypeInfo, Vec<SemError>, Vec<SemWarning>) {
    let pairs = ColloMLParser::parse(Rule::file, input).expect("Parse failed");
    let file = crate::ast::File::from_pest(pairs.into_iter().next().unwrap())
        .expect("AST conversion failed");

    let (_global_env, type_info, errors, warnings) =
        GlobalEnv::new(types, vars, &file).expect("GlobalEnv creation failed");

    (type_info, errors, warnings)
}

/// Helper to create a simple object type with no fields
pub(crate) fn simple_object(name: &str) -> HashMap<String, ObjectFields> {
    let mut types = HashMap::new();
    types.insert(name.to_string(), HashMap::new());
    types
}

/// Helper to create an object type with fields
pub(crate) fn object_with_fields(
    name: &str,
    fields: Vec<(&str, ExprType)>,
) -> HashMap<String, ObjectFields> {
    let mut types = HashMap::new();
    let mut field_map = HashMap::new();
    for (field_name, field_type) in fields {
        field_map.insert(field_name.to_string(), field_type);
    }
    types.insert(name.to_string(), field_map);
    types
}

/// Helper to create a variable with specific argument types
pub(crate) fn var_with_args(name: &str, args: Vec<ExprType>) -> HashMap<String, ArgsType> {
    let mut vars = HashMap::new();
    vars.insert(name.to_string(), args);
    vars
}
