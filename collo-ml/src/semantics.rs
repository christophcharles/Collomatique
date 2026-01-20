//! Semantic analysis for the ColloML DSL.
//!
//! This module provides type checking, symbol resolution, and semantic validation
//! for ColloML programs. It is organized into the following submodules:
//!
//! - `types`: Core type system (ExprType, SimpleType, ConcreteType)
//! - `errors`: Semantic errors and warnings
//! - `global_env`: Global environment for modules (types, functions, variables)
//! - `local_env`: Local scope management during type checking
//! - `path_resolution`: Symbol path resolution
//! - `expr_checking`: Expression type checking
//! - `module_processing`: Multi-pass module compilation
//! - `string_case`: Naming convention utilities

mod errors;
mod expr_checking;
mod global_env;
mod local_env;
mod module_processing;
mod path_resolution;
pub mod string_case;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use errors::{ArgsType, GlobalEnvError, SemError, SemWarning};
pub use global_env::{FunctionDesc, GlobalEnv, TypeInfo};
pub use local_env::LocalEnvCheck;
pub use path_resolution::{resolve_path, ResolvedPathKind};
pub use types::{ConcreteType, ExprType, SimpleType};
