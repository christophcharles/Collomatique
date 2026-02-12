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
use crate::traits::EvalObject;
use crate::{EvalVar, ExprType};
use derivative::Derivative;
use thiserror::Error;

use super::CompileError;

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject"),
    Ord(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject")
)]
pub struct ReifiedVar<T: EvalObject, D: DatabaseConnection> {
    pub(crate) module: String,
    pub(crate) name: String,
    pub(crate) from_list: Option<usize>,
    pub(crate) params: Vec<ExprValue<T, D>>,
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject", feature_allow_slow_enum = "true"),
    Ord(bound = "T: EvalObject", feature_allow_slow_enum = "true"),
    Clone(bound = "T: EvalObject")
)]
pub enum ProblemVar<T: EvalObject, D: DatabaseConnection, V: EvalVar> {
    Base(V),
    Reified(ReifiedVar<T, D>),
    Helper(u64),
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = "T: EvalObject"),
    Debug(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject", feature_allow_slow_enum = "true"),
    Ord(bound = "T: EvalObject", feature_allow_slow_enum = "true")
)]
pub enum ConstraintDesc<T: EvalObject, D: DatabaseConnection> {
    Reified {
        var_name: String,
        origin: Origin<T, D>,
    },
    InScript {
        origin: Origin<T, D>,
    },
    Objectify {
        origin: Origin<T, D>,
    },
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject", feature_allow_slow_enum = "true"),
    Ord(bound = "T: EvalObject", feature_allow_slow_enum = "true")
)]
pub enum ExtraDesc<T: EvalObject, D: DatabaseConnection, V: EvalVar> {
    Orig(ConstraintDesc<T, D>),
    InitCond(V),
}

#[derive(Derivative, Error)]
#[derivative(Clone(bound = "T: EvalObject"), Debug(bound = "T: EvalObject"))]
pub enum ProblemError<T: EvalObject, D: DatabaseConnection> {
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
    Panic(Box<ExprValue<T, D>>),
}
