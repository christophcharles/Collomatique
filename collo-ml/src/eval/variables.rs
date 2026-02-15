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
use crate::traits::EvalObject;
use collomatique_ilp::Constraint;
use derivative::Derivative;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject"),
    Ord(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject")
)]
pub struct ScriptVar<T: EvalObject, D: DatabaseConnection> {
    pub module: String,
    pub name: String,
    pub from_list: Option<usize>,
    pub params: Vec<Arc<ExprValue<T, D>>>,
    #[derivative(PartialOrd = "ignore", PartialEq = "ignore", Ord = "ignore")]
    params_str: Arc<str>,
}

impl<T: EvalObject, D: DatabaseConnection> ScriptVar<T, D> {
    pub fn new(
        var_str_cache: &mut BTreeMap<Vec<Arc<ExprValue<T, D>>>, Arc<str>>,
        module: String,
        name: String,
        from_list: Option<usize>,
        params: Vec<Arc<ExprValue<T, D>>>,
    ) -> Self {
        let params_str = if let Some(cached) = var_str_cache.get(&params) {
            cached.clone()
        } else {
            let args: Vec<_> = params.iter().map(|x| x.convert_to_string()).collect();
            let s: Arc<str> = args.join(", ").into();
            var_str_cache.insert(params.clone(), s.clone());
            s
        };
        ScriptVar {
            module,
            name,
            from_list,
            params,
            params_str,
        }
    }
}

impl<T: EvalObject, D: DatabaseConnection> std::fmt::Display for ScriptVar<T, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.from_list {
            Some(i) => {
                write!(f, "${}({})[{}]", self.name, self.params_str, i)
            }
            None => {
                write!(f, "${}({})", self.name, self.params_str)
            }
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject"),
    Ord(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject")
)]
pub struct ExternVar<T: EvalObject, D: DatabaseConnection> {
    pub name: String,
    pub params: Vec<Arc<ExprValue<T, D>>>,
    #[derivative(PartialOrd = "ignore", PartialEq = "ignore", Ord = "ignore")]
    params_str: Arc<str>,
}

impl<T: EvalObject, D: DatabaseConnection> ExternVar<T, D> {
    pub fn new(
        var_str_cache: &mut BTreeMap<Vec<Arc<ExprValue<T, D>>>, Arc<str>>,
        name: String,
        params: Vec<Arc<ExprValue<T, D>>>,
    ) -> Self {
        let params_str = if let Some(cached) = var_str_cache.get(&params) {
            cached.clone()
        } else {
            let args: Vec<_> = params.iter().map(|x| x.convert_to_string()).collect();
            let s: Arc<str> = args.join(", ").into();
            var_str_cache.insert(params.clone(), s.clone());
            s
        };
        ExternVar {
            name,
            params,
            params_str,
        }
    }
}

impl<T: EvalObject, D: DatabaseConnection> std::fmt::Display for ExternVar<T, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}({})", self.name, self.params_str)
    }
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
pub enum IlpVar<T: EvalObject, D: DatabaseConnection> {
    Base(ExternVar<T, D>),
    Script(ScriptVar<T, D>),
}

impl<T: EvalObject, D: DatabaseConnection> std::fmt::Display for IlpVar<T, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IlpVar::Base(b) => write!(f, "{}", b),
            IlpVar::Script(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject"),
    Ord(bound = "T: EvalObject")
)]
pub struct Origin<T: EvalObject, D: DatabaseConnection> {
    pub module: String,
    pub fn_name: Spanned<String>,
    pub args: Vec<Arc<ExprValue<T, D>>>,
    pub pretty_docstring: Vec<String>,
}

impl<T: EvalObject, D: DatabaseConnection> std::fmt::Display for Origin<T, D> {
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
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject"),
    Ord(bound = "T: EvalObject")
)]
pub struct ConstraintWithOrigin<T: EvalObject, D: DatabaseConnection> {
    pub constraint: Constraint<IlpVar<T, D>>,
    pub origin: Option<Origin<T, D>>,
}

impl<T: EvalObject, D: DatabaseConnection> From<Constraint<IlpVar<T, D>>>
    for ConstraintWithOrigin<T, D>
{
    fn from(value: Constraint<IlpVar<T, D>>) -> Self {
        ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }
    }
}

pub fn strip_origins<T: EvalObject, D: DatabaseConnection>(
    set: &Vec<ConstraintWithOrigin<T, D>>,
) -> Vec<Constraint<IlpVar<T, D>>> {
    set.iter().map(|x| x.constraint.clone()).collect()
}
