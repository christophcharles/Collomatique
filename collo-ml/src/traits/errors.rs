use crate::ExprType;
use thiserror::Error;

/// Error used in TryFrom auto-impl for converting between types
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TypeConversionError {
    #[error("Cannot convert value: type not compatible with eval object")]
    BadType,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum VarConversionError {
    #[error("Cannot convert variable: unknown name \"{0}\"")]
    Unknown(String),
    #[error(
        "Cannot convert variable: wrong parameter count for {name}. Expected {expected} got {found}"
    )]
    WrongParameterCount {
        name: String,
        expected: usize,
        found: usize,
    },
    #[error(
        "Cannot convert variable {name}: parameter {param} has wrong type. Expected {expected}"
    )]
    WrongParameterType {
        name: String,
        param: usize,
        expected: ExprType,
    },
}
