//! Checked AST and error types.
//!
//! This module defines:
//! - `CheckedAST`: A type-checked AST ready for evaluation
//! - `CompileError`: Errors that can occur during compilation
//! - `EnvError`: Errors related to the evaluation environment
//! - `EvalError`: Errors that can occur during evaluation

use super::history::{EvalHistory, VariableDefinitions};
use super::values::{ExprValue, NoObject, NoObjectEnv};
use crate::parser::Rule;
use crate::semantics::{
    ArgsType, ExprType, GlobalEnv, GlobalEnvError, SemError, SemWarning, TypeInfo,
};
use crate::traits::EvalObject;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedAST<T: EvalObject = NoObject> {
    pub(crate) global_env: GlobalEnv,
    pub(crate) type_info: TypeInfo,
    pub(crate) expr_types: HashMap<crate::ast::Span, ExprType>,
    pub(crate) warnings: Vec<SemWarning>,
    pub(crate) _phantom: std::marker::PhantomData<T>,
}

#[derive(Clone, Debug, Error)]
pub enum CompileError {
    #[error(transparent)]
    ParsingError(#[from] pest::error::Error<Rule>),
    #[error(transparent)]
    AstError(#[from] crate::ast::AstError),
    #[error(transparent)]
    InconsistentGlobalEnv(#[from] GlobalEnvError),
    #[error("Semantics error")]
    SemanticsError {
        errors: Vec<SemError>,
        warnings: Vec<SemWarning>,
    },
}

#[derive(Clone, Debug, Error)]
pub enum EnvError<T: EvalObject> {
    #[error("Typename {typ_name} used for object {obj:?} has bad format")]
    BadTypeName { typ_name: String, obj: T },
}

#[derive(Clone, Debug, Error)]
pub enum EvalError<T: EvalObject> {
    #[error("Object of type {0} returns its type as being {1}")]
    ObjectWithBadTypeName(String, String),
    #[error("Object {object} of type {typ} does not have field {field}")]
    MissingObjectField {
        object: String,
        typ: String,
        field: String,
    },
    #[error("Unknown function \"{0}\"")]
    UnknownFunction(String),
    #[error("Type mismatch for parameter {param}: expected {expected} but found {found:?}")]
    TypeMismatch {
        param: usize,
        expected: ExprType,
        found: ExprValue<T>,
    },
    #[error("Argument count mismatch for \"{identifier}\": expected {expected} arguments but found {found}")]
    ArgumentCountMismatch {
        identifier: String,
        expected: usize,
        found: usize,
    },
    #[error("Param {param} is an inconsistent ExprValue")]
    InvalidExprValue { param: usize },
    #[error("Panic: {0}")]
    Panic(Box<ExprValue<T>>),
}

impl CheckedAST<NoObject> {
    pub fn quick_eval_fn(
        &self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<NoObject>>,
    ) -> Result<ExprValue<NoObject>, EvalError<NoObject>> {
        let env = NoObjectEnv {};
        self.eval_fn(&env, module, fn_name, args)
    }
}

impl<T: EvalObject> CheckedAST<T> {
    /// Create a CheckedAST from source modules
    pub fn new(
        inputs: &BTreeMap<&str, &str>,
        vars: HashMap<String, ArgsType>,
    ) -> Result<CheckedAST<T>, CompileError> {
        use crate::parser::ColloMLParser;
        use pest::Parser;

        // Parse all modules
        let mut modules: BTreeMap<&str, crate::ast::File> = BTreeMap::new();
        for (name, src) in inputs {
            let pairs = ColloMLParser::parse(Rule::file, src)?;
            let first_pair_opt = pairs.into_iter().next();
            let file = match first_pair_opt {
                Some(first_pair) => crate::ast::File::from_pest(first_pair)?,
                None => crate::ast::File::new(),
            };
            modules.insert(*name, file);
        }

        let (global_env, type_info, expr_types, errors, warnings) =
            GlobalEnv::new(T::type_schemas(), vars, &modules)?;

        if !errors.is_empty() {
            return Err(CompileError::SemanticsError { errors, warnings });
        }

        Ok(CheckedAST {
            global_env,
            type_info,
            expr_types,
            warnings,
            _phantom: std::marker::PhantomData,
        })
    }

    pub(crate) fn check_env(&self, env: &T::Env) -> Result<(), EvalError<T>> {
        for (typ, _fields) in self.global_env.get_types() {
            let objects = T::objects_with_typ(env, typ.as_str());

            for object in &objects {
                let returned_typ = object.typ_name(&env);
                if returned_typ != *typ {
                    return Err(EvalError::ObjectWithBadTypeName(typ.clone(), returned_typ));
                }
            }
        }
        Ok(())
    }

    pub fn get_type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    pub fn get_warnings(&self) -> &Vec<SemWarning> {
        &self.warnings
    }

    /// Resolve a type name to ExprType using the symbol table
    pub fn resolve_type(
        &self,
        module: &str,
        typ: &crate::ast::Spanned<crate::ast::TypeName>,
    ) -> Result<ExprType, SemError> {
        self.global_env.resolve_type(typ, module)
    }

    pub fn get_functions(&self) -> HashMap<(String, String), (ArgsType, ExprType)> {
        self.global_env
            .get_functions()
            .iter()
            .filter_map(|((module, fn_name), fn_desc)| {
                if !fn_desc.public {
                    return None;
                }
                Some((
                    (module.clone(), fn_name.clone()),
                    (fn_desc.typ.args.clone(), fn_desc.typ.output.clone()),
                ))
            })
            .collect()
    }

    pub fn get_vars(&self) -> HashMap<(String, String), (String, String)> {
        self.global_env
            .get_vars()
            .iter()
            .map(|((module, var_name), var_desc)| {
                (
                    (module.clone(), var_name.clone()),
                    var_desc.referenced_fn.clone(),
                )
            })
            .collect()
    }

    pub fn get_var_lists(&self) -> HashMap<(String, String), (String, String)> {
        self.global_env
            .get_var_lists()
            .iter()
            .map(|((module, var_name), var_desc)| {
                (
                    (module.clone(), var_name.clone()),
                    var_desc.referenced_fn.clone(),
                )
            })
            .collect()
    }

    pub fn start_eval_history<'a>(
        &'a self,
        env: &'a T::Env,
    ) -> Result<EvalHistory<'a, T>, EvalError<T>> {
        let cache = T::Cache::default();
        EvalHistory::new(self, env, cache)
    }

    pub fn start_eval_history_with_cache<'a>(
        &'a self,
        env: &'a T::Env,
        cache: T::Cache,
    ) -> Result<EvalHistory<'a, T>, EvalError<T>> {
        EvalHistory::new(self, env, cache)
    }

    pub fn eval_fn(
        &self,
        env: &T::Env,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<ExprValue<T>, EvalError<T>> {
        let mut eval_history = self.start_eval_history(env)?;
        Ok(eval_history.eval_fn(module, fn_name, args)?.0)
    }

    pub fn eval_fn_with_variables(
        &self,
        env: &T::Env,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<(ExprValue<T>, VariableDefinitions<T>), EvalError<T>> {
        let mut eval_history = self.start_eval_history(env)?;
        let (r, _o) = eval_history.eval_fn(module, fn_name, args)?;
        Ok((r, eval_history.into_var_def()))
    }
}
