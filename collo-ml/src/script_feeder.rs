//! Script feeder module for ColloML.
//!
//! This module provides `ScriptFeeder`, which evaluates ColloML scripts and
//! produces `IntConstraintBundle`s. It is the generic, ColloML-specific part
//! of the constraint pipeline.

mod script_feeder;
mod types;

#[cfg(test)]
mod tests;

pub use script_feeder::ScriptFeeder;
pub use types::{ReifiedVar, ScriptError};

pub use crate::eval::CompileError;
pub use collomatique_ilp_modeler::{ConstraintSource, InternalVar};

#[cfg(test)]
pub(crate) use crate::DescribeVar;
#[cfg(test)]
pub(crate) use crate::EvalVar;
#[cfg(test)]
pub(crate) use crate::SqliteDatabaseConnection;
#[cfg(test)]
pub(crate) use crate::SqliteDatabaseDriver;
#[cfg(test)]
pub(crate) use crate::database::DatabaseConnection;
#[cfg(test)]
pub(crate) use crate::eval::{ExternVar, Origin};
#[cfg(test)]
pub(crate) use crate::semantics::{ExprType, SimpleType};
#[cfg(test)]
pub(crate) use crate::traits::VarConversionError;
#[cfg(test)]
pub(crate) use collomatique_ilp_modeler::bundle::ReifyError;
#[cfg(test)]
pub(crate) use collomatique_ilp_modeler::{Model, Modeler};
#[cfg(test)]
pub(crate) use std::collections::{BTreeMap, HashMap};
