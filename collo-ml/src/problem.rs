//! Problem construction module for ColloML.
//!
//! This module provides the problem building system for creating ILP problems
//! from ColloML programs. It is organized into the following submodules:
//!
//! - `types`: Type definitions (ReifiedVar, ProblemVar, ConstraintDesc, ProblemError)
//! - `builder`: Problem builder and evaluation data
//! - `solution`: Problem, Solution, and FeasableSolution types

mod builder;
mod solution;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use builder::ProblemBuilder;
pub use solution::{FeasableSolution, Problem, Solution};
pub use types::{ConstraintDesc, ExtraDesc, ProblemError, ProblemVar, ReifiedVar};

// Re-export CompileError from eval for convenience
pub use crate::eval::CompileError;

// Re-exports for test compatibility (tests use `super::*`)
#[cfg(test)]
pub(crate) use crate::eval::ExternVar;
#[cfg(test)]
pub(crate) use crate::semantics::{ExprType, SimpleType};
#[cfg(test)]
pub(crate) use crate::traits::{EvalObject, VarConversionError};
#[cfg(test)]
pub(crate) use crate::EvalVar;
#[cfg(test)]
pub(crate) use std::collections::{BTreeMap, HashMap};
