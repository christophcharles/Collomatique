//! Evaluation history tracking.
//!
//! This module defines:
//! - `EvalHistory`: Tracks function calls and variable definitions during evaluation
//! - `VariableDefinitions`: The result of evaluation, containing variable constraints

use super::checked_ast::{CheckedAST, EvalError};
use super::local_env::LocalEvalEnv;
use super::values::ExprValue;
use super::variables::{IlpVar, Origin};
use crate::ast::Spanned;
use crate::semantics::FunctionDesc;
use crate::traits::EvalObject;
use collomatique_ilp::Constraint;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct EvalHistory<'a, T: EvalObject> {
    pub(crate) ast: &'a CheckedAST<T>,
    pub(crate) env: &'a T::Env,
    pub(crate) cache: T::Cache,
    pub(crate) funcs: BTreeMap<(String, String, Vec<ExprValue<T>>), (ExprValue<T>, Origin<T>)>,
    pub(crate) vars: BTreeMap<(String, String, Vec<ExprValue<T>>), (String, String)>,
    pub(crate) var_lists: BTreeMap<(String, String, Vec<ExprValue<T>>), (String, String)>,
}

impl<'a, T: EvalObject> EvalHistory<'a, T> {
    pub(crate) fn new(
        ast: &'a CheckedAST<T>,
        env: &'a T::Env,
        cache: T::Cache,
    ) -> Result<Self, EvalError<T>> {
        ast.check_env(env)?;

        Ok(EvalHistory {
            ast,
            env,
            cache,
            funcs: BTreeMap::new(),
            vars: BTreeMap::new(),
            var_lists: BTreeMap::new(),
        })
    }

    fn prettify_docstring(
        &mut self,
        fn_desc: &FunctionDesc,
        local_env: &mut LocalEvalEnv<T>,
    ) -> Result<Vec<String>, EvalError<T>> {
        fn_desc
            .docstring
            .iter()
            .map(|line| {
                let mut result = String::new();
                for part in line {
                    result.push_str(&part.prefix);
                    if let Some(expr) = &part.expr {
                        match local_env.eval_expr(self, expr)? {
                            ExprValue::String(s) => result.push_str(&s),
                            // Expression is wrapped in String(...) at parse time,
                            // so this should never happen - logic bug if it does
                            other => panic!(
                                "Docstring expression should evaluate to String, got {:?}",
                                other
                            ),
                        }
                    }
                }
                Ok(result.trim_start().to_string())
            })
            .collect()
    }

    pub(crate) fn add_fn_to_call_history(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
        allow_private: bool,
    ) -> Result<(ExprValue<T>, Origin<T>), EvalError<T>> {
        let fn_desc = self
            .ast
            .global_env
            .get_functions()
            .get(&(module.to_string(), fn_name.to_string()))
            .ok_or(EvalError::UnknownFunction(fn_name.to_string()))?;

        if !allow_private {
            if !fn_desc.public {
                return Err(EvalError::UnknownFunction(fn_name.to_string()));
            }
        }

        if fn_desc.typ.args.len() != args.len() {
            return Err(EvalError::ArgumentCountMismatch {
                identifier: fn_name.to_string(),
                expected: fn_desc.typ.args.len(),
                found: args.len(),
            });
        }

        let mut local_env = LocalEvalEnv::new(module);
        for (param, ((arg, arg_typ), arg_name)) in args
            .iter()
            .zip(fn_desc.typ.args.iter())
            .zip(fn_desc.arg_names.iter())
            .enumerate()
        {
            if !arg.fits_in_typ(&self.env, arg_typ) {
                return Err(EvalError::TypeMismatch {
                    param: param,
                    expected: arg_typ.clone(),
                    found: arg.clone(),
                });
            }
            local_env.register_identifier(arg_name, arg.clone());
        }

        if let Some(r) = self
            .funcs
            .get(&(module.to_string(), fn_name.to_string(), args.clone()))
        {
            return Ok(r.clone());
        }

        local_env.push_scope();
        let naked_result = local_env.eval_expr(self, &fn_desc.body);
        // Evaluate docstring expressions BEFORE popping scope (parameters still available)
        let pretty_docstring = self.prettify_docstring(fn_desc, &mut local_env)?;
        local_env.pop_scope();
        let naked_result = naked_result?;

        let origin = Origin {
            module: module.to_string(),
            fn_name: Spanned::new(fn_name.to_string(), fn_desc.body.span.clone()),
            args: args.clone(),
            pretty_docstring,
        };

        let result = naked_result.with_origin(&origin);
        self.funcs.insert(
            (module.to_string(), fn_name.to_string(), args),
            (result.clone(), origin.clone()),
        );

        Ok((result, origin))
    }
}

impl<'a, T: EvalObject> EvalHistory<'a, T> {
    pub fn validate_value(&self, val: &ExprValue<T>) -> bool {
        match val {
            ExprValue::None => true,
            ExprValue::Int(_) => true,
            ExprValue::Bool(_) => true,
            ExprValue::LinExpr(_) => true,
            ExprValue::Constraint(_) => true,
            ExprValue::String(_) => true,
            ExprValue::Object(obj) => self
                .ast
                .global_env
                .validate_object_type(&obj.typ_name(&self.env)),
            ExprValue::List(list) => {
                for elem in list {
                    if !self.validate_value(elem) {
                        return false;
                    }
                }
                true
            }
            ExprValue::Tuple(elements) => elements.iter().all(|e| self.validate_value(e)),
            ExprValue::Struct(fields) => fields.values().all(|v| self.validate_value(v)),
            ExprValue::Custom(custom) => {
                // Validate that the custom type exists and recursively validate content
                let key = match &custom.variant {
                    None => custom.type_name.clone(),
                    Some(v) => format!("{}::{}", custom.type_name, v),
                };
                self.ast
                    .global_env
                    .get_custom_types()
                    .contains_key(&(custom.module.clone(), key))
                    && self.validate_value(&custom.content)
            }
        }
    }

    pub fn eval_fn(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<(ExprValue<T>, Origin<T>), EvalError<T>> {
        let mut checked_args = vec![];
        for (param, arg) in args.into_iter().enumerate() {
            if !self.validate_value(&arg) {
                return Err(EvalError::InvalidExprValue { param });
            }
            checked_args.push(arg.into());
        }

        self.add_fn_to_call_history(module, fn_name, checked_args.clone(), false)
    }

    pub fn into_var_def_and_cache(self) -> (VariableDefinitions<T>, T::Cache) {
        let mut var_def = VariableDefinitions {
            vars: BTreeMap::new(),
            var_lists: BTreeMap::new(),
        };

        for ((v_module, v_name, v_args), (fn_module, fn_name)) in self.vars {
            let (fn_call_result, new_origin) = self
                .funcs
                .get(&(fn_module.clone(), fn_name.clone(), v_args.clone()))
                .expect("Fn call should be valid");
            let constraint = match fn_call_result {
                ExprValue::Constraint(c) => c
                    .iter()
                    .map(|c_with_o| c_with_o.constraint.clone())
                    .collect::<Vec<_>>(),
                _ => panic!(
                    "Fn call should have returned a constraint: {:?}",
                    fn_call_result
                ),
            };
            var_def
                .vars
                .insert((v_module, v_name, v_args), (constraint, new_origin.clone()));
        }

        for ((vl_module, vl_name, vl_args), (fn_module, fn_name)) in self.var_lists {
            let (fn_call_result, new_origin) = self
                .funcs
                .get(&(fn_module.clone(), fn_name.clone(), vl_args.clone()))
                .expect("Fn call should be valid");
            let constraints: Vec<_> = match fn_call_result {
                ExprValue::List(cs) if cs.iter().all(|x| matches!(x, ExprValue::Constraint(_))) => {
                    cs.iter()
                        .map(|c| match c {
                            ExprValue::Constraint(c) => c
                                .iter()
                                .map(|c_with_o| c_with_o.constraint.clone())
                                .collect::<Vec<_>>(),
                            _ => panic!(
                                "Elements of the returned list should be constraints: {:?}",
                                c
                            ),
                        })
                        .collect()
                }
                _ => panic!(
                    "Fn call should have returned a constraint list: {:?}",
                    fn_call_result
                ),
            };
            var_def.var_lists.insert(
                (vl_module, vl_name, vl_args),
                (constraints, new_origin.clone()),
            );
        }

        (var_def, self.cache)
    }

    pub fn into_var_def(self) -> VariableDefinitions<T> {
        self.into_var_def_and_cache().0
    }
}

#[derive(Clone, Debug)]
pub struct VariableDefinitions<T: EvalObject> {
    pub vars:
        BTreeMap<(String, String, Vec<ExprValue<T>>), (Vec<Constraint<IlpVar<T>>>, Origin<T>)>,
    pub var_lists:
        BTreeMap<(String, String, Vec<ExprValue<T>>), (Vec<Vec<Constraint<IlpVar<T>>>>, Origin<T>)>,
}
