//! Variable types for ILP evaluation.
//!
//! This module defines the variable types used in the evaluation system:
//! - `ScriptVar`: Variables defined in ColloML scripts
//! - `ExternVar`: External variables from the environment
//! - `IlpVar`: Enum combining both variable types
//! - `Origin`: Tracks where a constraint originated from
//! - `ConstraintWithOrigin`: A constraint paired with its origin

use super::values::ExprValue;
use crate::ast::Spanned;
use crate::database::DatabaseConnection;
use collomatique_ilp::Constraint;
use derivative::Derivative;
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialOrd(bound = ""),
    Ord(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct ScriptVar<D: DatabaseConnection> {
    pub module: String,
    pub name: String,
    pub from_list: Option<usize>,
    pub params: Vec<Arc<ExprValue<D>>>,
}

impl<D: DatabaseConnection> ScriptVar<D> {
    pub fn new(
        module: String,
        name: String,
        from_list: Option<usize>,
        params: Vec<Arc<ExprValue<D>>>,
    ) -> Self {
        ScriptVar {
            module,
            name,
            from_list,
            params,
        }
    }
}

impl<D: DatabaseConnection> std::fmt::Display for ScriptVar<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params_str: Vec<_> = self.params.iter().map(|x| x.convert_to_string()).collect();
        let params_str = params_str.join(", ");
        match self.from_list {
            Some(i) => {
                write!(f, "${}({})[{}]", self.name, params_str, i)
            }
            None => {
                write!(f, "${}({})", self.name, params_str)
            }
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialOrd(bound = ""),
    Ord(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct ExternVar<D: DatabaseConnection> {
    pub name: String,
    pub params: Vec<Arc<ExprValue<D>>>,
}

impl<D: DatabaseConnection> ExternVar<D> {
    pub fn new(name: String, params: Vec<Arc<ExprValue<D>>>) -> Self {
        ExternVar { name, params }
    }
}

impl<D: DatabaseConnection> std::fmt::Display for ExternVar<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params_str: Vec<_> = self.params.iter().map(|x| x.convert_to_string()).collect();
        write!(f, "${}({})", self.name, params_str.join(", "))
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = "", feature_allow_slow_enum = "true"),
    Ord(bound = "", feature_allow_slow_enum = "true")
)]
pub enum IlpVar<D: DatabaseConnection> {
    Base(ExternVar<D>),
    Script(ScriptVar<D>),
}

impl<D: DatabaseConnection> std::fmt::Display for IlpVar<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IlpVar::Base(b) => write!(f, "{}", b),
            IlpVar::Script(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = ""),
    Ord(bound = "")
)]
pub struct Origin<D: DatabaseConnection> {
    pub module: String,
    pub fn_name: Spanned<String>,
    pub args: Vec<Arc<ExprValue<D>>>,
    pub pretty_docstring: Vec<String>,
}

impl<D: DatabaseConnection> std::fmt::Display for Origin<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pretty_docstring.is_empty() {
            let args_str: Vec<_> = self.args.iter().map(|x| x.to_string()).collect();

            write!(
                f,
                "{}::{}({})",
                self.module,
                self.fn_name.node,
                args_str.join(", ")
            )
        } else {
            write!(f, "{}", self.pretty_docstring.join("\n"))
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    PartialOrd(bound = ""),
    Ord(bound = "")
)]
pub struct ConstraintWithOrigin<D: DatabaseConnection> {
    pub constraint: Constraint<IlpVar<D>>,
    pub origin: Option<Origin<D>>,
}

impl<D: DatabaseConnection> From<Constraint<IlpVar<D>>> for ConstraintWithOrigin<D> {
    fn from(value: Constraint<IlpVar<D>>) -> Self {
        ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }
    }
}

pub fn strip_origins<D: DatabaseConnection>(
    set: &Vec<ConstraintWithOrigin<D>>,
) -> Vec<Constraint<IlpVar<D>>> {
    set.iter().map(|x| x.constraint.clone()).collect()
}
