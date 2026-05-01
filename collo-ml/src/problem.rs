//! Problem construction module for ColloML.
//!
//! This module provides the problem building system for creating ILP problems
//! from ColloML programs. It is organized into the following submodules:
//!
//! - `types`: Type definitions (ReifiedVar, ScriptError)
//! - `builder`: Problem builder
//! - `solution`: Problem, Solution, and FeasableSolution types

mod builder;
mod script_feeder;
mod solution;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use builder::ProblemBuilder;
pub use script_feeder::ScriptFeeder;
pub use solution::{
    FeasableSolution, Problem, ProblemConstraintSource, ProblemInternalVar, Solution,
};
pub use types::{ReifiedVar, ScriptError};

// Re-export from ilp-modeler for convenience
pub use collomatique_ilp_modeler::{ConstraintSource, InternalVar};

// Re-export CompileError from eval for convenience
pub use crate::eval::CompileError;

// Re-exports for test compatibility (tests use `super::*`)
#[cfg(test)]
pub(crate) use crate::DescribeVar;
#[cfg(test)]
pub(crate) use crate::EvalVar;
#[cfg(test)]
pub(crate) use crate::database::DatabaseConnection;
#[cfg(test)]
pub(crate) use crate::eval::ExternVar;
#[cfg(test)]
pub(crate) use crate::eval::SqliteDatabaseDriver;
#[cfg(test)]
pub(crate) use crate::semantics::{ExprType, SimpleType};
#[cfg(test)]
pub(crate) use crate::traits::VarConversionError;
#[cfg(test)]
pub(crate) use std::collections::{BTreeMap, HashMap};
