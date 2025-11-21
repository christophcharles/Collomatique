use crate::ast::Spanned;
use crate::parser::Rule;
use crate::semantics::*;
use collomatique_ilp::{Constraint, LinExpr, UsableData};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ScriptVar<T: Object> {
    pub name: String,
    pub from_list: Option<usize>,
    pub params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ExternVar<T: Object> {
    pub name: String,
    pub params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum IlpVar<T: Object> {
    Base(ExternVar<T>),
    Script(ScriptVar<T>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Origin<T: Object> {
    fn_name: Spanned<String>,
    args: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ConstraintWithOrigin<T: Object> {
    constraint: Constraint<IlpVar<T>>,
    origin: Option<Origin<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExprValue<T: Object> {
    Int(i32),
    Bool(bool),
    LinExpr(LinExpr<IlpVar<T>>),
    Constraint(Vec<ConstraintWithOrigin<T>>),
    Object(T),
    List(ExprType, Vec<ExprValue<T>>),
}

impl<T: Object> From<i32> for ExprValue<T> {
    fn from(value: i32) -> Self {
        ExprValue::Int(value)
    }
}

impl<T: Object> From<bool> for ExprValue<T> {
    fn from(value: bool) -> Self {
        ExprValue::Bool(value)
    }
}

impl<T: Object> From<LinExpr<IlpVar<T>>> for ExprValue<T> {
    fn from(value: LinExpr<IlpVar<T>>) -> Self {
        ExprValue::LinExpr(value)
    }
}

impl<T: Object> From<Constraint<IlpVar<T>>> for ExprValue<T> {
    fn from(value: Constraint<IlpVar<T>>) -> Self {
        ExprValue::Constraint(vec![ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }])
    }
}

impl<T: Object> From<ConstraintWithOrigin<T>> for ExprValue<T> {
    fn from(value: ConstraintWithOrigin<T>) -> Self {
        ExprValue::Constraint(vec![value])
    }
}

impl<T: Object> ExprValue<T> {
    pub fn from_obj(obj: T) -> Self {
        ExprValue::Object(obj)
    }

    pub fn get_type(&self) -> ExprType {
        match self {
            ExprValue::Int(_) => ExprType::Int,
            ExprValue::Bool(_) => ExprType::Bool,
            ExprValue::LinExpr(_) => ExprType::LinExpr,
            ExprValue::Constraint(_) => ExprType::Constraint,
            ExprValue::Object(obj) => ExprType::Object(obj.typ_name()),
            ExprValue::List(typ, _list) => typ.clone(),
        }
    }

    pub fn with_origin(&self, origin: &Origin<T>) -> ExprValue<T> {
        match self {
            ExprValue::Constraint(constraints) => ExprValue::Constraint(
                constraints
                    .iter()
                    .map(|c| ConstraintWithOrigin {
                        constraint: c.constraint.clone(),
                        origin: Some(match &c.origin {
                            Some(o) => o.clone(),
                            None => origin.clone(),
                        }),
                    })
                    .collect(),
            ),
            ExprValue::List(typ, list) => ExprValue::List(
                typ.clone(),
                list.iter().map(|x| x.with_origin(origin)).collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn is_primitive_type(&self) -> bool {
        self.get_type().is_primitive_type()
    }

    pub fn is_list(&self) -> bool {
        self.get_type().is_list()
    }

    pub fn is_arithmetic(&self) -> bool {
        self.get_type().is_arithmetic()
    }

    pub fn can_coerce_to(&self, target: &ExprType) -> bool {
        self.get_type().can_coerce_to(target)
    }

    pub fn coerce_to(&self, target: &ExprType) -> Option<ExprValue<T>> {
        if !self.can_coerce_to(target) {
            return None;
        }

        Some(match (self, target) {
            // Exact match always works
            (a, b) if a.get_type() == *b => a.clone(),

            // Int → LinExpr (but NOT LinExpr → Int)
            (ExprValue::Int(v), ExprType::LinExpr) => {
                ExprValue::LinExpr(LinExpr::constant((*v).into()))
            }

            // Recursive: [A] → [B] if A can coerce to B
            (ExprValue::List(a, list), ExprType::List(b)) if a.can_coerce_to(b) => ExprValue::List(
                *b.clone(),
                list.iter()
                    .map(|x| {
                        x.coerce_to(b)
                            .expect("Coercion should be valid for all list elements")
                    })
                    .collect(),
            ),

            _ => panic!("Inconsistency between can_coerce_to and coerce_to"),
        })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AnnotatedValue<T: Object> {
    Forced(ExprValue<T>),
    Regular(ExprValue<T>),
    UntypedList,
}

impl<T: Object> From<ExprValue<T>> for AnnotatedValue<T> {
    fn from(value: ExprValue<T>) -> Self {
        AnnotatedValue::Regular(value)
    }
}

impl<T: Object> AnnotatedValue<T> {
    pub fn get_type(&self) -> AnnotatedType {
        match self {
            AnnotatedValue::Forced(value) => AnnotatedType::Forced(value.get_type()),
            AnnotatedValue::Regular(value) => AnnotatedType::Regular(value.get_type()),
            AnnotatedValue::UntypedList => AnnotatedType::UntypedList,
        }
    }

    pub fn is_primitive_type(&self) -> bool {
        self.get_type().is_primitive_type()
    }

    pub fn is_list(&self) -> bool {
        self.get_type().is_list()
    }

    pub fn is_arithmetic(&self) -> bool {
        self.get_type().is_arithmetic()
    }

    pub fn is_forced(&self) -> bool {
        self.get_type().is_forced()
    }

    pub fn matches(&self, target: &ExprType) -> bool {
        self.get_type().matches(target)
    }

    pub fn inner(&self) -> Option<&ExprValue<T>> {
        match self {
            AnnotatedValue::Forced(val) => Some(val),
            AnnotatedValue::Regular(val) => Some(val),
            AnnotatedValue::UntypedList => None,
        }
    }

    pub fn into_inner(self) -> Option<ExprValue<T>> {
        match self {
            AnnotatedValue::Forced(val) => Some(val),
            AnnotatedValue::Regular(val) => Some(val),
            AnnotatedValue::UntypedList => None,
        }
    }

    pub fn can_coerce_to(&self, target: &ExprType) -> bool {
        self.get_type().can_coerce_to(target)
    }

    pub fn coerce_to(&self, target: &ExprType) -> Option<ExprValue<T>> {
        if !self.can_coerce_to(target) {
            return None;
        }

        match self {
            AnnotatedValue::Forced(val) | AnnotatedValue::Regular(val)
                if val.can_coerce_to(target) =>
            {
                Some(val.coerce_to(target).expect("Coercion should be valid"))
            }
            AnnotatedValue::UntypedList if target.is_list() => {
                Some(ExprValue::List(target.clone(), vec![]))
            }
            _ => panic!("Inconsistency between can_coerce_to and coerce_to"),
        }
    }

    pub fn loosen(&self) -> AnnotatedValue<T> {
        match self {
            AnnotatedValue::Forced(a) => AnnotatedValue::Regular(a.clone()),
            AnnotatedValue::Regular(a) => AnnotatedValue::Regular(a.clone()),
            AnnotatedValue::UntypedList => AnnotatedValue::UntypedList,
        }
    }

    pub fn enforce(&self) -> AnnotatedValue<T> {
        match self {
            AnnotatedValue::Forced(a) => AnnotatedValue::Forced(a.clone()),
            AnnotatedValue::Regular(a) => AnnotatedValue::Forced(a.clone()),
            AnnotatedValue::UntypedList => AnnotatedValue::UntypedList,
        }
    }
}

pub trait Object: UsableData {
    fn typ_name(&self) -> String;
    fn field_access(&self, field: &str) -> ExprValue<Self>;
}

#[derive(Clone, Debug)]
pub struct CheckedAST {
    global_env: GlobalEnv,
    type_info: TypeInfo,
    warnings: Vec<SemWarning>,
}

use thiserror::Error;
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
pub enum EnvError<T: Object> {
    #[error("Typename {typ_name} used for object {obj:?} has bad format")]
    BadTypeName { typ_name: String, obj: T },
}

#[derive(Debug, Clone)]
pub struct EvalEnv<T: Object> {
    typ_map: HashMap<String, Vec<T>>,
}

#[derive(Clone, Debug, Error)]
pub enum EvalError {
    #[error("Unknown function \"{0}\"")]
    UnknownFunction(String),
    #[error("Type mismatch for parameter {param}: expected {expected} but found {found}")]
    TypeMismatch {
        param: usize,
        expected: ExprType,
        found: AnnotatedType,
    },
    #[error("Argument count mismatch for \"{identifier}\": expected {expected} arguments but found {found}")]
    ArgumentCountMismatch {
        identifier: String,
        expected: usize,
        found: usize,
    },
}

impl<T: Object> EvalEnv<T> {
    pub fn new<I: IntoIterator<Item = T>>(objects: I) -> Result<Self, EnvError<T>> {
        let mut typ_map = HashMap::new();
        for obj in objects {
            let typ_name = obj.typ_name();
            if !is_typ_name_valid(&typ_name) {
                return Err(EnvError::BadTypeName { typ_name, obj });
            }

            match typ_map.get_mut(&typ_name) {
                None => {
                    typ_map.insert(typ_name, vec![obj]);
                }
                Some(list) => {
                    list.push(obj);
                }
            }
        }
        Ok(EvalEnv { typ_map })
    }
}

/// Help function
///
/// Checks if a name is valid (alphanumeric characters + underscore )
fn is_typ_name_valid(typ_name: &str) -> bool {
    typ_name.chars().all(|x| x.is_alphanumeric() || x == '_')
        && typ_name
            .chars()
            .next()
            .map(|x| !x.is_numeric())
            .unwrap_or(true)
}

#[cfg(test)]
mod is_typ_name_valid_tests {
    use super::is_typ_name_valid;

    #[test]
    fn is_typ_name_valid_tests() {
        assert!(is_typ_name_valid("Test"));
        assert!(is_typ_name_valid("TeSt"));
        assert!(is_typ_name_valid("__test__"));
        assert!(is_typ_name_valid("test123"));
        assert!(is_typ_name_valid("te2yts"));
        assert!(!is_typ_name_valid("32test"));
        assert!(!is_typ_name_valid("test with space"));
    }
}

impl CheckedAST {
    pub fn new(
        input: &str,
        types: HashMap<String, HashMap<String, ExprType>>,
        vars: HashMap<String, ArgsType>,
    ) -> Result<CheckedAST, CompileError> {
        use crate::parser::ColloMLParser;
        use pest::Parser;

        let pairs = ColloMLParser::parse(Rule::file, input)?;
        let first_pair_opt = pairs.into_iter().next();
        let file = match first_pair_opt {
            Some(first_pair) => crate::ast::File::from_pest(first_pair)?,
            None => crate::ast::File::new(),
        };

        let (global_env, type_info, errors, warnings) = GlobalEnv::new(types, vars, &file)?;

        if !errors.is_empty() {
            return Err(CompileError::SemanticsError { errors, warnings });
        }

        Ok(CheckedAST {
            global_env,
            type_info,
            warnings,
        })
    }

    pub fn check_env<T: Object>(&self, env: &EvalEnv<T>) -> bool {
        for (typ_name, _objs) in &env.typ_map {
            if !self
                .global_env
                .validate_type(&ExprType::Object(typ_name.clone()))
            {
                return false;
            }
        }
        true
    }

    pub fn get_type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    pub fn get_warnings(&self) -> &Vec<SemWarning> {
        &self.warnings
    }

    pub fn get_functions(&self) -> HashMap<String, (ArgsType, ExprType)> {
        self.global_env
            .get_functions()
            .iter()
            .filter_map(|(fn_name, fn_desc)| {
                if !fn_desc.public {
                    return None;
                }
                Some((
                    fn_name.clone(),
                    (fn_desc.typ.args.clone(), fn_desc.typ.output.clone()),
                ))
            })
            .collect()
    }

    pub fn get_vars(&self) -> HashMap<String, String> {
        self.global_env
            .get_vars()
            .iter()
            .map(|(var_name, var_desc)| (var_name.clone(), var_desc.referenced_fn.clone()))
            .collect()
    }

    pub fn get_var_lists(&self) -> HashMap<String, String> {
        self.global_env
            .get_var_lists()
            .iter()
            .map(|(var_name, var_desc)| (var_name.clone(), var_desc.referenced_fn.clone()))
            .collect()
    }

    pub fn eval_fn<T: Object>(
        &self,
        env: &EvalEnv<T>,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<ExprValue<T>, EvalError> {
        let fn_desc = self
            .global_env
            .get_functions()
            .get(fn_name)
            .ok_or(EvalError::UnknownFunction(fn_name.to_string()))?;

        if fn_desc.typ.args.len() != args.len() {
            return Err(EvalError::ArgumentCountMismatch {
                identifier: fn_name.to_string(),
                expected: fn_desc.typ.args.len(),
                found: args.len(),
            });
        }

        let mut coerced_args = vec![];
        let mut local_env = LocalEnv::new();
        for (param, ((arg, arg_typ), arg_name)) in args
            .into_iter()
            .zip(fn_desc.typ.args.iter())
            .zip(fn_desc.arg_names.iter())
            .enumerate()
        {
            let coerced_arg = arg.coerce_to(arg_typ).ok_or(EvalError::TypeMismatch {
                param: param,
                expected: arg_typ.clone(),
                found: arg.get_type().into(),
            })?;
            coerced_args.push(coerced_arg.clone());
            local_env.register_identifier(arg_name, coerced_arg);
        }

        local_env.push_scope();
        let annotated_result = local_env.eval_node(self, env, &fn_desc.body);
        local_env.pop_scope();

        let origin = Origin {
            fn_name: Spanned::new(fn_name.to_string(), fn_desc.span.clone()),
            args: coerced_args,
        };

        Ok(annotated_result
            .coerce_to(&fn_desc.typ.output)
            .expect("Coercion to output type should always work in a checked AST")
            .with_origin(&origin))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalEnv<T: Object> {
    scopes: Vec<HashMap<String, ExprValue<T>>>,
    pending_scope: HashMap<String, ExprValue<T>>,
}

impl<T: Object> Default for LocalEnv<T> {
    fn default() -> Self {
        LocalEnv {
            scopes: vec![],
            pending_scope: HashMap::new(),
        }
    }
}

impl<T: Object> LocalEnv<T> {
    fn new() -> Self {
        LocalEnv::default()
    }

    fn _lookup_ident(&self, ident: &str) -> Option<ExprValue<T>> {
        // We don't look in pending scope as these variables are not yet accessible
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(ident) {
                return Some(value.clone());
            };
        }
        None
    }

    fn push_scope(&mut self) {
        let mut old_scope = HashMap::new();
        std::mem::swap(&mut old_scope, &mut self.pending_scope);

        self.scopes.push(old_scope);
    }

    fn pop_scope(&mut self) {
        assert!(!self.scopes.is_empty());

        self.pending_scope = self.scopes.pop().unwrap();
        self.pending_scope.clear();
    }

    fn register_identifier(&mut self, ident: &str, value: ExprValue<T>) {
        assert!(!self.pending_scope.contains_key(ident));

        self.pending_scope.insert(ident.to_string(), value);
    }

    fn eval_node(
        &mut self,
        _ast: &CheckedAST,
        _env: &EvalEnv<T>,
        _node: &crate::ast::Expr,
    ) -> AnnotatedValue<T> {
        todo!()
    }
}
