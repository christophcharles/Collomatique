//! Type definitions for the problem module.
//!
//! This module defines:
//! - `ReifiedVar`: A reified (script-defined) variable
//! - `ProblemError`: Errors that can occur during problem construction

use crate::ExprType;
use crate::database::DatabaseConnection;
use crate::eval::ExprValue;
use derivative::Derivative;
use std::sync::Arc;
use thiserror::Error;

use super::CompileError;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Clone(bound = "")
)]
pub struct ReifiedVar<D: DatabaseConnection> {
    pub(crate) module: String,
    pub(crate) name: String,
    pub(crate) params: Vec<Arc<ExprValue<D>>>,
}

#[derive(Derivative, Error)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub enum ProblemError<D: DatabaseConnection> {
    #[error("Variable {0} has non-integer type")]
    NonIntegerVariable(String),
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
