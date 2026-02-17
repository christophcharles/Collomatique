//! Traits for the ColloML evaluation system.
//!
//! This module provides the `EvalVar` trait for defining ILP variables.

mod errors;
mod eval_var;

pub use errors::{TypeConversionError, VarConversionError};
pub use eval_var::EvalVar;
