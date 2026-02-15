//! Checked AST and error types.
//!
//! This module defines:
//! - `CheckedAST`: A type-checked AST ready for evaluation
//! - `CompileError`: Errors that can occur during compilation
//! - `EnvError`: Errors related to the evaluation environment
//! - `EvalError`: Errors that can occur during evaluation

use derivative::Derivative;

use super::history::{EvalHistory, VariableDefinitions};
use super::values::ExprValue;
use crate::database::{DatabaseConnection, DatabaseDriver};
use crate::parser::Rule;
use crate::semantics::{
    ArgsType, ExprType, GlobalEnv, GlobalEnvError, SemError, SemWarning, TypeInfo,
};
use crate::traits::EvalObject;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

#[derive(Derivative)]
#[derivative(
    Clone(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct CheckedAST<T: EvalObject, D: DatabaseDriver> {
    pub(crate) global_env: GlobalEnv<D>,
    pub(crate) type_info: TypeInfo,
    pub(crate) expr_types: HashMap<crate::ast::Span, ExprType>,
    pub(crate) resolved_types: HashMap<crate::ast::Span, ExprType>,
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

#[derive(Derivative, Error)]
#[derivative(Clone(bound = "T: EvalObject"), Debug(bound = "T: EvalObject"))]
pub enum EvalError<T: EvalObject, D: DatabaseConnection> {
    #[error("Unknown function \"{0}\"")]
    UnknownFunction(String),
    #[error("Type mismatch for parameter {param}: expected {expected} but found {found:?}")]
    TypeMismatch {
        param: usize,
        expected: ExprType,
        found: ExprValue<T, D>,
    },
    #[error(
        "Argument count mismatch for \"{identifier}\": expected {expected} arguments but found {found}"
    )]
    ArgumentCountMismatch {
        identifier: String,
        expected: usize,
        found: usize,
    },
    #[error("Param {param} is an inconsistent ExprValue")]
    InvalidExprValue { param: usize },
    #[error("Panic: {0}")]
    Panic(Box<ExprValue<T, D>>),
}

impl<T: EvalObject, D: DatabaseDriver> CheckedAST<T, D> {
    /// Create a CheckedAST from source modules
    pub async fn new(
        inputs: &BTreeMap<&str, &str>,
        vars: HashMap<String, ArgsType>,
    ) -> Result<CheckedAST<T, D>, CompileError> {
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

        let (global_env, type_info, expr_types, resolved_types, errors, warnings) =
            GlobalEnv::new(vars, &modules).await?;

        if !errors.is_empty() {
            return Err(CompileError::SemanticsError { errors, warnings });
        }

        Ok(CheckedAST {
            global_env,
            type_info,
            expr_types,
            resolved_types,
            warnings,
            _phantom: std::marker::PhantomData,
        })
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

    /// Get a resolved type from the cache (populated during semantic analysis)
    pub fn get_resolved_type(&self, span: &crate::ast::Span) -> &ExprType {
        self.resolved_types
            .get(span)
            .expect("Type should have been resolved during semantic analysis")
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

    pub fn start_eval_history(&self) -> EvalHistory<'_, T, D> {
        EvalHistory {
            ast: self,
            funcs: BTreeMap::new(),
            vars: BTreeMap::new(),
            var_lists: BTreeMap::new(),
            queries: BTreeMap::new(),
        }
    }

    pub async fn eval_fn(
        &self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T, D::Connection>>,
    ) -> Result<ExprValue<T, D::Connection>, EvalError<T, D::Connection>> {
        let mut eval_history = self.start_eval_history();
        Ok(eval_history.eval_fn(module, fn_name, args).await?.0)
    }

    pub async fn eval_fn_with_variables(
        &self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T, D::Connection>>,
    ) -> Result<
        (
            ExprValue<T, D::Connection>,
            VariableDefinitions<T, D::Connection>,
        ),
        EvalError<T, D::Connection>,
    > {
        let mut eval_history = self.start_eval_history();
        let (r, _o) = eval_history.eval_fn(module, fn_name, args).await?;
        Ok((r, eval_history.into_var_def()))
    }
}
