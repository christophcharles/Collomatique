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
use crate::traits::EvalObject;
use collomatique_ilp::Constraint;
use derivative::Derivative;

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialOrd, Ord, PartialEq, Eq)]
pub struct ScriptVar<T: EvalObject> {
    pub module: String,
    pub name: String,
    pub from_list: Option<usize>,
    pub params: Vec<ExprValue<T>>,
    #[derivative(PartialOrd = "ignore", PartialEq = "ignore", Ord = "ignore")]
    params_str: String,
}

impl<T: EvalObject> ScriptVar<T> {
    pub fn new(
        env: &T::Env,
        cache: &mut T::Cache,
        module: String,
        name: String,
        from_list: Option<usize>,
        params: Vec<ExprValue<T>>,
    ) -> Self {
        let args: Vec<_> = params
            .iter()
            .map(|x| x.convert_to_string(env, cache))
            .collect();
        ScriptVar {
            module,
            name,
            from_list,
            params,
            params_str: args.join(", "),
        }
    }

    pub fn new_no_env(
        module: String,
        name: String,
        from_list: Option<usize>,
        params: Vec<ExprValue<T>>,
    ) -> Self {
        let args: Vec<_> = params.iter().map(|x| format!("{}", x)).collect();
        ScriptVar {
            module,
            name,
            from_list,
            params,
            params_str: args.join(", "),
        }
    }
}

impl<T: EvalObject> std::fmt::Display for ScriptVar<T> {
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

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialOrd, Ord, PartialEq, Eq)]
pub struct ExternVar<T: EvalObject> {
    pub name: String,
    pub params: Vec<ExprValue<T>>,
    #[derivative(PartialOrd = "ignore", PartialEq = "ignore", Ord = "ignore")]
    params_str: String,
}

impl<T: EvalObject> ExternVar<T> {
    pub fn new(
        env: &T::Env,
        cache: &mut T::Cache,
        name: String,
        params: Vec<ExprValue<T>>,
    ) -> Self {
        let args: Vec<_> = params
            .iter()
            .map(|x| x.convert_to_string(env, cache))
            .collect();
        ExternVar {
            name,
            params,
            params_str: args.join(", "),
        }
    }

    pub fn new_no_env(name: String, params: Vec<ExprValue<T>>) -> Self {
        let args: Vec<_> = params.iter().map(|x| format!("{}", x)).collect();
        ExternVar {
            name,
            params,
            params_str: args.join(", "),
        }
    }
}

impl<T: EvalObject> std::fmt::Display for ExternVar<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}({})", self.name, self.params_str)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum IlpVar<T: EvalObject> {
    Base(ExternVar<T>),
    Script(ScriptVar<T>),
}

impl<T: EvalObject> std::fmt::Display for IlpVar<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IlpVar::Base(b) => write!(f, "{}", b),
            IlpVar::Script(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Origin<T: EvalObject> {
    pub module: String,
    pub fn_name: Spanned<String>,
    pub args: Vec<ExprValue<T>>,
    pub pretty_docstring: Vec<String>,
}

impl<T: EvalObject> std::fmt::Display for Origin<T> {
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ConstraintWithOrigin<T: EvalObject> {
    pub constraint: Constraint<IlpVar<T>>,
    pub origin: Option<Origin<T>>,
}

impl<T: EvalObject> From<Constraint<IlpVar<T>>> for ConstraintWithOrigin<T> {
    fn from(value: Constraint<IlpVar<T>>) -> Self {
        ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }
    }
}

pub fn strip_origins<T: EvalObject>(
    set: &Vec<ConstraintWithOrigin<T>>,
) -> Vec<Constraint<IlpVar<T>>> {
    set.iter().map(|x| x.constraint.clone()).collect()
}
