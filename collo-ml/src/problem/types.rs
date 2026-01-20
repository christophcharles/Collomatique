//! Type definitions for the problem module.
//!
//! This module defines:
//! - `ReifiedVar`: A reified (script-defined) variable
//! - `ProblemVar`: Enum of all variable types (Base, Reified, Helper)
//! - `ConstraintDesc`: Description of constraint origin
//! - `ExtraDesc`: Extended description for reification problems
//! - `ProblemError`: Errors that can occur during problem construction

use crate::eval::{ExprValue, Origin};
use crate::traits::EvalObject;
use crate::{EvalVar, ExprType};
use thiserror::Error;

use super::CompileError;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ReifiedVar<T: EvalObject> {
    pub(crate) module: String,
    pub(crate) name: String,
    pub(crate) from_list: Option<usize>,
    pub(crate) params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ProblemVar<T: EvalObject, V: EvalVar<T>> {
    Base(V),
    Reified(ReifiedVar<T>),
    Helper(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintDesc<T: EvalObject> {
    Reified { var_name: String, origin: Origin<T> },
    InScript { origin: Origin<T> },
    Objectify { origin: Origin<T> },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtraDesc<T: EvalObject, V: EvalVar<T>> {
    Orig(ConstraintDesc<T>),
    InitCond(V),
}

#[derive(Clone, Debug, Error)]
pub enum ProblemError<T: EvalObject> {
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
    #[error("Panic: {0}")]
    Panic(Box<ExprValue<T>>),
}
