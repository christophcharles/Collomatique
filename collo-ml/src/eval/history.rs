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
use crate::database::{DatabaseConnection, DatabaseDriver, SqlQueryError};
use crate::semantics::FunctionDesc;
use crate::traits::EvalObject;
use collomatique_ilp::Constraint;
use derivative::Derivative;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct EvalHistory<'a, T: EvalObject, D: DatabaseDriver> {
    pub(crate) ast: &'a CheckedAST<T, D>,
    pub(crate) funcs: BTreeMap<
        (String, String, Vec<Arc<ExprValue<T, D::Connection>>>),
        (Arc<ExprValue<T, D::Connection>>, Origin<T, D::Connection>),
    >,
    pub(crate) vars:
        BTreeMap<(String, String, Vec<Arc<ExprValue<T, D::Connection>>>), (String, String)>,
    pub(crate) var_lists:
        BTreeMap<(String, String, Vec<Arc<ExprValue<T, D::Connection>>>), (String, String)>,
    pub(crate) queries: BTreeMap<
        (String, String, Vec<Arc<ExprValue<T, D::Connection>>>),
        Arc<ExprValue<T, D::Connection>>,
    >,
}

impl<'a, T: EvalObject, D: DatabaseDriver> EvalHistory<'a, T, D> {
    pub(crate) fn new(ast: &'a CheckedAST<T, D>) -> Self {
        EvalHistory {
            ast,
            funcs: BTreeMap::new(),
            vars: BTreeMap::new(),
            var_lists: BTreeMap::new(),
            queries: BTreeMap::new(),
        }
    }

    async fn prettify_docstring(
        &mut self,
        fn_desc: &FunctionDesc,
        local_env: &Arc<LocalEvalEnv<T, D>>,
    ) -> Result<Vec<String>, EvalError<T, D::Connection>> {
        let mut lines = Vec::new();
        for line in &fn_desc.docstring {
            let mut result = String::new();
            for part in line {
                result.push_str(&part.prefix);
                if let Some(expr) = &part.expr {
                    let eval_result =
                        Box::pin(Arc::clone(local_env).eval_expr(self, Arc::clone(expr))).await?;
                    match &*eval_result {
                        ExprValue::String(s) => result.push_str(s),
                        // Expression is wrapped in String(...) at parse time,
                        // so this should never happen - logic bug if it does
                        other => panic!(
                            "Docstring expression should evaluate to String, got {:?}",
                            other
                        ),
                    }
                }
            }
            lines.push(result.trim_start().to_string());
        }
        Ok(lines)
    }

    pub(crate) async fn add_fn_to_call_history(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<Arc<ExprValue<T, D::Connection>>>,
        allow_private: bool,
    ) -> Result<
        (Arc<ExprValue<T, D::Connection>>, Origin<T, D::Connection>),
        EvalError<T, D::Connection>,
    > {
        let fn_desc = self
            .ast
            .global_env
            .get_functions()
            .get(&(module.to_string(), fn_name.to_string()))
            .ok_or(EvalError::UnknownFunction(fn_name.to_string()))?;

        if !allow_private && !fn_desc.public {
            return Err(EvalError::UnknownFunction(fn_name.to_string()));
        }

        if fn_desc.typ.args.len() != args.len() {
            return Err(EvalError::ArgumentCountMismatch {
                identifier: fn_name.to_string(),
                expected: fn_desc.typ.args.len(),
                found: args.len(),
            });
        }

        let root_env = LocalEvalEnv::new(module);
        let mut builder = LocalEvalEnv::start_subscope(root_env);
        for (param, ((arg, arg_typ), arg_name)) in args
            .iter()
            .zip(fn_desc.typ.args.iter())
            .zip(fn_desc.arg_names.iter())
            .enumerate()
        {
            if !arg.fits_in_typ(arg_typ) {
                return Err(EvalError::TypeMismatch {
                    param,
                    expected: arg_typ.clone(),
                    found: ExprValue::clone(arg),
                });
            }
            builder.register_identifier(arg_name, Arc::clone(arg));
        }

        if let Some(r) = self
            .funcs
            .get(&(module.to_string(), fn_name.to_string(), args.clone()))
        {
            return Ok((Arc::clone(&r.0), r.1.clone()));
        }

        let local_env = builder.build_subscope();
        let naked_result =
            Box::pin(Arc::clone(&local_env).eval_expr(self, Arc::clone(&fn_desc.body))).await;
        let pretty_docstring = Box::pin(self.prettify_docstring(fn_desc, &local_env)).await?;
        let naked_result = naked_result?;

        let origin = Origin {
            module: module.to_string(),
            fn_name: Spanned::new(fn_name.to_string(), fn_desc.body.span.clone()),
            args: args.clone(),
            pretty_docstring,
        };

        let result = Arc::new(naked_result.with_origin(&origin));
        self.funcs.insert(
            (module.to_string(), fn_name.to_string(), args),
            (Arc::clone(&result), origin.clone()),
        );

        Ok((result, origin))
    }

    pub(crate) async fn add_query_to_call_history(
        &mut self,
        module: &str,
        name: &str,
        args: Vec<Arc<ExprValue<T, D::Connection>>>,
    ) -> Result<Arc<ExprValue<T, D::Connection>>, EvalError<T, D::Connection>> {
        if let Some(cached) =
            self.queries
                .get(&(module.to_string(), name.to_string(), args.clone()))
        {
            return Ok(Arc::clone(cached));
        }

        let query_desc = self
            .ast
            .global_env
            .get_queries()
            .get(&(module.to_string(), name.to_string()))
            .expect("Semantic analysis should have validated this query exists");
        let sql = query_desc.query_string.node.clone();
        let out_type = query_desc.typ.output.clone();

        let db_handle = {
            let mut val: &ExprValue<T, D::Connection> = &args[0];
            loop {
                match val {
                    ExprValue::Database(h) => break h.clone(),
                    ExprValue::Custom(c) => val = &c.content,
                    _ => panic!(
                        "First query argument must be a Database (semantic phase should have caught this)"
                    ),
                }
            }
        };

        let params: Vec<ExprValue<T, D::Connection>> =
            args[1..].iter().map(|a| (**a).clone()).collect();
        let global_env = &self.ast.global_env;

        let result = db_handle.query(&sql, params, out_type, global_env).await;

        match result {
            Ok(value) => {
                let value = Arc::new(value);
                self.queries.insert(
                    (module.to_string(), name.to_string(), args),
                    Arc::clone(&value),
                );
                Ok(value)
            }
            Err(SqlQueryError::QueryFailed(msg)) => {
                Err(EvalError::Panic(Box::new(ExprValue::String(msg))))
            }
            Err(other) => panic!(
                "Unexpected query error (should have been caught in semantic phase): {other}"
            ),
        }
    }
}

impl<'a, T: EvalObject, D: DatabaseDriver> EvalHistory<'a, T, D> {
    pub fn validate_value(&self, val: &ExprValue<T, D::Connection>) -> bool {
        match val {
            ExprValue::None => true,
            ExprValue::Int(_) => true,
            ExprValue::Bool(_) => true,
            ExprValue::LinExpr(_) => true,
            ExprValue::Constraint(_) => true,
            ExprValue::String(_) => true,
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
            ExprValue::Database(_) => true,
            ExprValue::_Phantom(..) => unreachable!(),
        }
    }

    pub async fn eval_fn(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T, D::Connection>>,
    ) -> Result<(ExprValue<T, D::Connection>, Origin<T, D::Connection>), EvalError<T, D::Connection>>
    {
        let mut checked_args = vec![];
        for (param, arg) in args.into_iter().enumerate() {
            if !self.validate_value(&arg) {
                return Err(EvalError::InvalidExprValue { param });
            }
            checked_args.push(Arc::new(arg));
        }

        let (result, origin) =
            Box::pin(self.add_fn_to_call_history(module, fn_name, checked_args, false)).await?;
        Ok((Arc::unwrap_or_clone(result), origin))
    }

    pub fn into_var_def(self) -> VariableDefinitions<T, D::Connection> {
        let mut var_def = VariableDefinitions {
            vars: BTreeMap::new(),
            var_lists: BTreeMap::new(),
        };

        for ((v_module, v_name, v_args), (fn_module, fn_name)) in self.vars {
            let (fn_call_result, new_origin) = self
                .funcs
                .get(&(fn_module.clone(), fn_name.clone(), v_args.clone()))
                .expect("Fn call should be valid");
            let constraint = match &**fn_call_result {
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
            let constraints: Vec<_> = match &**fn_call_result {
                ExprValue::List(cs)
                    if cs.iter().all(|x| matches!(&**x, ExprValue::Constraint(_))) =>
                {
                    cs.iter()
                        .map(|c| match &**c {
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

        var_def
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "T: EvalObject"), Debug(bound = "T: EvalObject"))]
pub struct VariableDefinitions<T: EvalObject, D: DatabaseConnection> {
    pub vars: BTreeMap<
        (String, String, Vec<Arc<ExprValue<T, D>>>),
        (Vec<Constraint<IlpVar<T, D>>>, Origin<T, D>),
    >,
    pub var_lists: BTreeMap<
        (String, String, Vec<Arc<ExprValue<T, D>>>),
        (Vec<Vec<Constraint<IlpVar<T, D>>>>, Origin<T, D>),
    >,
}
