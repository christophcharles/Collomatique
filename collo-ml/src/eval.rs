//! Evaluation module for ColloML.
//!
//! This module provides the runtime evaluation system for ColloML programs.
//! It is organized into the following submodules:
//!
//! - `variables`: Variable types (ScriptVar, ExternVar, IlpVar, Origin)
//! - `values`: Expression values (ExprValue, CustomValue, NoObject)
//! - `checked_ast`: Type-checked AST and error types
//! - `local_env`: Local environment for expression evaluation
//! - `history`: Evaluation history tracking

mod checked_ast;
mod history;
mod local_env;
mod values;
mod variables;

#[cfg(test)]
mod tests;

// Re-export public types
pub use checked_ast::{CheckedAST, CompileError, EnvError, EvalError};
pub use history::{EvalHistory, VariableDefinitions};
pub use values::{CustomValue, ExprValue, NoObject, NoObjectEnv};
pub use variables::{strip_origins, ConstraintWithOrigin, ExternVar, IlpVar, Origin, ScriptVar};

// Re-export traits and types used by eval (for backwards compatibility)
pub use crate::traits::EvalObject;

// Re-exports for test compatibility (tests use `super::*`)
#[cfg(test)]
pub(crate) use crate::semantics::{ExprType, SimpleType};
#[cfg(test)]
pub(crate) use crate::traits::FieldConversionError;
#[cfg(test)]
pub(crate) use collomatique_ilp::LinExpr;
#[cfg(test)]
pub(crate) use std::collections::{BTreeSet, HashMap};
