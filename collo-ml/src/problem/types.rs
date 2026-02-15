//! Type definitions for the problem module.
//!
//! This module defines:
//! - `ReifiedVar`: A reified (script-defined) variable
//! - `ProblemVar`: Enum of all variable types (Base, Reified, Helper)
//! - `ConstraintDesc`: Description of constraint origin
//! - `ExtraDesc`: Extended description for reification problems
//! - `ProblemError`: Errors that can occur during problem construction

use crate::database::DatabaseConnection;
use crate::eval::{ExprValue, Origin};
use crate::{EvalVar, ExprType};
use derivative::Derivative;
use std::sync::Arc;
use thiserror::Error;

use super::CompileError;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = ""),
    Ord(bound = ""),
    Clone(bound = "")
)]
pub struct ReifiedVar<D: DatabaseConnection> {
    pub(crate) module: String,
    pub(crate) name: String,
    pub(crate) from_list: Option<usize>,
    pub(crate) params: Vec<Arc<ExprValue<D>>>,
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = "", feature_allow_slow_enum = "true"),
    Ord(bound = "", feature_allow_slow_enum = "true"),
    Clone(bound = "")
)]
pub enum ProblemVar<D: DatabaseConnection, V: EvalVar> {
    Base(V),
    Reified(ReifiedVar<D>),
    Helper(u64),
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = "", feature_allow_slow_enum = "true"),
    Ord(bound = "", feature_allow_slow_enum = "true")
)]
pub enum ConstraintDesc<D: DatabaseConnection> {
    Reified { var_name: String, origin: Origin<D> },
    InScript { origin: Origin<D> },
    Objectify { origin: Origin<D> },
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = "", feature_allow_slow_enum = "true"),
    Ord(bound = "", feature_allow_slow_enum = "true")
)]
pub enum ExtraDesc<D: DatabaseConnection, V: EvalVar> {
    Orig(ConstraintDesc<D>),
    InitCond(V),
}

#[derive(Derivative, Error)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub enum ProblemError<D: DatabaseConnection> {
    #[error("Variable {0} has non-integer type")]
    NonIntegerVariable(String),
    #[error("TypeId {0:?} from EvalVar cannot be represented with EvalObject")]
    EvalVarIncompatibleWithEvalObject(std::any::TypeId),
    #[error("Function \"{0}\" was not found in script (maybe it is not public?)")]
    UnknownFunction(String),
    #[error("Function \"{func}\" expects {expected} arguments but got {found}")]
    ArgumentCountMismatch {
        func: String,
        expected: usize,
        found: usize,
    },
    #[error(transparent)]
    CompileError(#[from] CompileError),
    #[error("Function {func} returns {returned} instead of {expected}")]
    WrongReturnType {
        func: String,
        returned: ExprType,
        expected: ExprType,
    },
    #[error("Function \"{0}\" expects a database connection but none was provided")]
    MissingDatabaseConnection(String),
    #[error("Panic: {0}")]
    Panic(Box<ExprValue<D>>),
}
