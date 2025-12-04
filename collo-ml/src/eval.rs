use crate::ast::{Spanned, TypeName};
use crate::parser::Rule;
use crate::semantics::*;
use crate::traits::{EvalObject, FieldConversionError};
use collomatique_ilp::{Constraint, LinExpr};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ScriptVar<T: EvalObject> {
    pub name: String,
    pub from_list: Option<usize>,
    pub params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ExternVar<T: EvalObject> {
    pub name: String,
    pub params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum IlpVar<T: EvalObject> {
    Base(ExternVar<T>),
    Script(ScriptVar<T>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Origin<T: EvalObject> {
    pub fn_name: Spanned<String>,
    pub args: Vec<ExprValue<T>>,
    pub pretty_docstring: Vec<String>,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExprValue<T: EvalObject> {
    Int(i32),
    Bool(bool),
    LinExpr(LinExpr<IlpVar<T>>),
    Constraint(Vec<ConstraintWithOrigin<T>>),
    Object(T),
    List(ExprType, Vec<ExprValue<T>>),
}

impl<T: EvalObject> From<i32> for ExprValue<T> {
    fn from(value: i32) -> Self {
        ExprValue::Int(value)
    }
}

impl<T: EvalObject> From<bool> for ExprValue<T> {
    fn from(value: bool) -> Self {
        ExprValue::Bool(value)
    }
}

impl<T: EvalObject> From<LinExpr<IlpVar<T>>> for ExprValue<T> {
    fn from(value: LinExpr<IlpVar<T>>) -> Self {
        ExprValue::LinExpr(value)
    }
}

impl<T: EvalObject> From<Constraint<IlpVar<T>>> for ExprValue<T> {
    fn from(value: Constraint<IlpVar<T>>) -> Self {
        ExprValue::Constraint(Vec::from([ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }]))
    }
}

impl<T: EvalObject> From<ConstraintWithOrigin<T>> for ExprValue<T> {
    fn from(value: ConstraintWithOrigin<T>) -> Self {
        ExprValue::Constraint(Vec::from([value]))
    }
}

impl<T: EvalObject> ExprValue<T> {
    pub fn from_obj(obj: T) -> Self {
        ExprValue::Object(obj)
    }

    pub fn get_type(&self, env: &T::Env) -> ExprType {
        match self {
            ExprValue::Int(_) => ExprType::Int,
            ExprValue::Bool(_) => ExprType::Bool,
            ExprValue::LinExpr(_) => ExprType::LinExpr,
            ExprValue::Constraint(_) => ExprType::Constraint,
            ExprValue::Object(obj) => ExprType::Object(obj.typ_name(env)),
            ExprValue::List(typ, _list) => ExprType::List(Box::new(typ.clone())),
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

    pub fn is_primitive_type(&self, env: &T::Env) -> bool {
        self.get_type(env).is_primitive_type()
    }

    pub fn is_list(&self, env: &T::Env) -> bool {
        self.get_type(env).is_list()
    }

    pub fn is_arithmetic(&self, env: &T::Env) -> bool {
        self.get_type(env).is_arithmetic()
    }

    pub fn can_coerce_to(&self, env: &T::Env, target: &ExprType) -> bool {
        self.get_type(env).can_coerce_to(target)
    }

    pub fn coerce_to(&self, env: &T::Env, target: &ExprType) -> Option<ExprValue<T>> {
        if !self.can_coerce_to(env, target) {
            return None;
        }

        Some(match (self, target) {
            // Exact match always works
            (a, b) if a.get_type(env) == *b => a.clone(),

            // Int → LinExpr (but NOT LinExpr → Int)
            (ExprValue::Int(v), ExprType::LinExpr) => {
                ExprValue::LinExpr(LinExpr::constant((*v).into()))
            }

            // Recursive: [A] → [B] if A can coerce to B
            (ExprValue::List(a, list), ExprType::List(b)) if a.can_coerce_to(b) => ExprValue::List(
                *b.clone(),
                list.iter()
                    .map(|x| {
                        x.coerce_to(env, b)
                            .expect("Coercion should be valid for all list elements")
                    })
                    .collect(),
            ),

            _ => panic!("Inconsistency between can_coerce_to and coerce_to"),
        })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AnnotatedValue<T: EvalObject> {
    Forced(ExprValue<T>),
    Regular(ExprValue<T>),
    UntypedList,
}

impl<T: EvalObject> From<ExprValue<T>> for AnnotatedValue<T> {
    fn from(value: ExprValue<T>) -> Self {
        AnnotatedValue::Regular(value)
    }
}

impl<T: EvalObject> AnnotatedValue<T> {
    pub fn get_type(&self, env: &T::Env) -> AnnotatedType {
        match self {
            AnnotatedValue::Forced(value) => AnnotatedType::Forced(value.get_type(env)),
            AnnotatedValue::Regular(value) => AnnotatedType::Regular(value.get_type(env)),
            AnnotatedValue::UntypedList => AnnotatedType::UntypedList,
        }
    }

    pub fn is_primitive_type(&self, env: &T::Env) -> bool {
        self.get_type(env).is_primitive_type()
    }

    pub fn is_list(&self, env: &T::Env) -> bool {
        self.get_type(env).is_list()
    }

    pub fn is_arithmetic(&self, env: &T::Env) -> bool {
        self.get_type(env).is_arithmetic()
    }

    pub fn is_forced(&self, env: &T::Env) -> bool {
        self.get_type(env).is_forced()
    }

    pub fn matches(&self, env: &T::Env, target: &ExprType) -> bool {
        self.get_type(env).matches(target)
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

    pub fn can_coerce_to(&self, env: &T::Env, target: &ExprType) -> bool {
        self.get_type(env).can_coerce_to(target)
    }

    pub fn coerce_to(&self, env: &T::Env, target: &ExprType) -> Option<ExprValue<T>> {
        if !self.can_coerce_to(env, target) {
            return None;
        }

        match self {
            AnnotatedValue::Forced(val) | AnnotatedValue::Regular(val)
                if val.can_coerce_to(env, target) =>
            {
                Some(
                    val.coerce_to(env, target)
                        .expect("Coercion should be valid"),
                )
            }
            AnnotatedValue::UntypedList if target.is_list() => match target {
                ExprType::List(inner_typ) => Some(ExprValue::List(*inner_typ.clone(), Vec::new())),
                _ => panic!("Expected list!"),
            },
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoObject {}

#[derive(Debug, Clone)]
pub struct NoObjectEnv {}

impl EvalObject for NoObject {
    type Env = NoObjectEnv;
    type Cache = ();

    fn objects_with_typ(_env: &Self::Env, _name: &str) -> BTreeSet<Self> {
        BTreeSet::new()
    }

    fn typ_name(&self, _env: &Self::Env) -> String {
        panic!("No object is defined for NoObject")
    }

    fn type_id_to_name(type_id: std::any::TypeId) -> Result<String, FieldConversionError> {
        Err(FieldConversionError::UnknownTypeId(type_id))
    }

    fn field_access(
        &self,
        _env: &Self::Env,
        _cache: &mut Self::Cache,
        _field: &str,
    ) -> Option<ExprValue<Self>> {
        None
    }

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        HashMap::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedAST<T: EvalObject = NoObject> {
    global_env: GlobalEnv,
    type_info: TypeInfo,
    expr_types: HashMap<crate::ast::Span, AnnotatedType>,
    warnings: Vec<SemWarning>,
    _phantom: std::marker::PhantomData<T>,
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
pub enum EnvError<T: EvalObject> {
    #[error("Typename {typ_name} used for object {obj:?} has bad format")]
    BadTypeName { typ_name: String, obj: T },
}

#[derive(Clone, Debug, Error)]
pub enum EvalError {
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
    #[error("Param {param} is an inconsistent ExprValue")]
    InvalidExprValue { param: usize },
}

impl CheckedAST<NoObject> {
    pub fn quick_eval_fn(
        &self,
        fn_name: &str,
        args: Vec<ExprValue<NoObject>>,
    ) -> Result<ExprValue<NoObject>, EvalError> {
        let env = NoObjectEnv {};
        self.eval_fn(&env, fn_name, args)
    }
}

impl<T: EvalObject> CheckedAST<T> {
    pub fn new(
        input: &str,
        vars: HashMap<String, ArgsType>,
    ) -> Result<CheckedAST<T>, CompileError> {
        use crate::parser::ColloMLParser;
        use pest::Parser;

        let pairs = ColloMLParser::parse(Rule::file, input)?;
        let first_pair_opt = pairs.into_iter().next();
        let file = match first_pair_opt {
            Some(first_pair) => crate::ast::File::from_pest(first_pair)?,
            None => crate::ast::File::new(),
        };

        let (global_env, type_info, expr_types, errors, warnings) =
            GlobalEnv::new(T::type_schemas(), vars, &file)?;

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

    fn check_env(&self, env: &T::Env) -> Result<(), EvalError> {
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

    pub fn start_eval_history<'a>(
        &'a self,
        env: &'a T::Env,
    ) -> Result<EvalHistory<'a, T>, EvalError> {
        let cache = T::Cache::default();
        EvalHistory::new(self, env, cache)
    }

    pub fn eval_fn(
        &self,
        env: &T::Env,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<ExprValue<T>, EvalError> {
        let mut eval_history = self.start_eval_history(env)?;
        Ok(eval_history.eval_fn(fn_name, args)?.0)
    }

    pub fn eval_fn_with_variables(
        &self,
        env: &T::Env,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<(ExprValue<T>, VariableDefinitions<T>), EvalError> {
        let mut eval_history = self.start_eval_history(env)?;
        let (r, _o) = eval_history.eval_fn(fn_name, args)?;
        Ok((r, eval_history.into_var_def()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalEnv<T: EvalObject> {
    scopes: Vec<HashMap<String, ExprValue<T>>>,
    pending_scope: HashMap<String, ExprValue<T>>,
}

impl<T: EvalObject> Default for LocalEnv<T> {
    fn default() -> Self {
        LocalEnv {
            scopes: vec![],
            pending_scope: HashMap::new(),
        }
    }
}

impl<T: EvalObject> LocalEnv<T> {
    fn new() -> Self {
        LocalEnv::default()
    }

    fn lookup_ident(&self, ident: &str) -> Option<ExprValue<T>> {
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

    fn eval_expr(
        &mut self,
        eval_history: &mut EvalHistory<'_, T>,
        expr: &Spanned<crate::ast::Expr>,
    ) -> Result<AnnotatedValue<T>, EvalError> {
        use crate::ast::Expr;
        Ok(match &expr.node {
            Expr::Boolean(val) => ExprValue::Bool(*val).into(),
            Expr::Number(val) => ExprValue::Int(*val).into(),
            Expr::Ident(ident) => self
                .lookup_ident(&ident.node)
                .expect("Identifiers should be defined in a checked AST")
                .into(),
            Expr::Path { object, segments } => {
                assert!(!segments.is_empty());

                let initial_object = self.eval_expr(eval_history, &object)?;
                let mut current_value = initial_object.into_inner().expect("Object expected");

                for field in segments {
                    let obj = match current_value {
                        ExprValue::Object(obj) => obj,
                        _ => panic!("Object expected"),
                    };
                    current_value = obj
                        .field_access(&eval_history.env, &mut eval_history.cache, &field.node)
                        .ok_or(EvalError::MissingObjectField {
                            object: format!("{:?}", obj),
                            typ: obj.typ_name(&eval_history.env),
                            field: field.node.clone(),
                        })?;
                }

                current_value.into()
            }
            Expr::Cardinality(list_expr) => {
                let list_value = self.eval_expr(eval_history, &list_expr)?;
                let count = match list_value {
                    AnnotatedValue::Forced(ExprValue::List(_typ, list))
                    | AnnotatedValue::Regular(ExprValue::List(_typ, list)) => list.len(),
                    AnnotatedValue::UntypedList => 0usize,
                    _ => panic!("Unexpected type for list expression"),
                };
                ExprValue::Int(
                    i32::try_from(count).expect("List length should not exceed i32 capacity"),
                )
                .into()
            }
            Expr::ExplicitType { expr, typ } => {
                let value = self.eval_expr(eval_history, &expr)?;
                let target_type = ExprType::from(typ.node.clone());

                // A forced value can be cast anyway
                let loose_value = value.loosen();

                let coerced_value = loose_value
                    .coerce_to(eval_history.env, &target_type)
                    .expect("Resulting expression should be coercible to target type");

                coerced_value.into()
            }
            Expr::ListLiteral { elements } => {
                if elements.is_empty() {
                    return Ok(AnnotatedValue::UntypedList);
                }

                let element_values: Vec<_> = elements
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;

                let mut unified_type = element_values[0].get_type(eval_history.env);
                for item in &element_values[1..] {
                    let item_type = item.get_type(eval_history.env);
                    unified_type = AnnotatedType::unify(&unified_type, &item_type)
                        .expect("Types should be unifiable");
                }
                let target_type = unified_type
                    .into_inner()
                    .expect("Type should be determined");

                let coerced_elements: Vec<_> = element_values
                    .iter()
                    .map(|x| {
                        x.coerce_to(eval_history.env, &target_type)
                            .expect("Coercion to unified type should be possible")
                    })
                    .collect();

                ExprValue::List(target_type, coerced_elements).into()
            }
            Expr::ListRange { start, end } => {
                let start_value = self.eval_expr(eval_history, &start)?;
                let end_value = self.eval_expr(eval_history, &end)?;

                let coerced_start = start_value
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Int expected");
                let coerced_end = end_value
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Int expected");

                let start_num = match coerced_start {
                    ExprValue::Int(v) => v,
                    _ => panic!("Int expected"),
                };
                let end_num = match coerced_end {
                    ExprValue::Int(v) => v,
                    _ => panic!("Int expected"),
                };

                ExprValue::List(
                    ExprType::Int,
                    (start_num..end_num)
                        .into_iter()
                        .map(ExprValue::Int)
                        .collect(),
                )
                .into()
            }
            Expr::GlobalList(typ_name) => {
                let typ_as_str = match &typ_name.node {
                    TypeName::Object(obj) => obj.clone(),
                    _ => panic!("Object expected"),
                };
                let objects = T::objects_with_typ(&eval_history.env, &typ_as_str);

                ExprValue::List(
                    ExprType::from(typ_name.node.clone()),
                    objects
                        .iter()
                        .map(|x| ExprValue::Object(x.clone()))
                        .collect(),
                )
                .into()
            }
            Expr::FnCall { name, args } => {
                let args = args
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;
                eval_history
                    .add_fn_to_call_history(&name.node, args, true)?
                    .0
                    .into()
            }
            Expr::VarCall { name, args } => {
                let args: Vec<_> = args
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;
                if let Some(args_typ) = eval_history
                    .ast
                    .global_env
                    .get_predefined_vars()
                    .get(&name.node)
                {
                    let params = args
                        .into_iter()
                        .zip(args_typ.iter())
                        .map(|(arg, arg_typ)| {
                            arg.coerce_to(eval_history.env, arg_typ)
                                .expect("Coercion should work")
                        })
                        .collect();
                    ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar {
                        name: name.node.clone(),
                        params,
                    })))
                    .into()
                } else if let Some(var_desc) =
                    eval_history.ast.global_env.get_vars().get(&name.node)
                {
                    let params: Vec<_> = args
                        .iter()
                        .zip(var_desc.args.iter())
                        .map(|(arg, arg_typ)| {
                            arg.coerce_to(eval_history.env, arg_typ)
                                .expect("Coercion should work")
                        })
                        .collect();
                    eval_history.vars.insert(
                        (name.node.clone(), params.clone()),
                        var_desc.referenced_fn.clone(),
                    );
                    eval_history.add_fn_to_call_history(
                        &var_desc.referenced_fn,
                        params.iter().map(|x| x.clone().into()).collect(),
                        true,
                    )?;
                    ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar {
                        name: name.node.clone(),
                        from_list: None,
                        params,
                    })))
                    .into()
                } else {
                    panic!("Valid var expected")
                }
            }
            Expr::In { item, collection } => {
                let collection_value = self.eval_expr(eval_history, &*collection)?;
                let (elem_t, list) = match collection_value {
                    AnnotatedValue::Forced(ExprValue::List(elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(elem_t, list)) => (elem_t, list),
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        return Ok(ExprValue::Bool(false).into());
                    }
                    _ => panic!("List expected"),
                };

                let item_value = self.eval_expr(eval_history, &*item)?;
                let coerced_value = item_value
                    .coerce_to(eval_history.env, &elem_t)
                    .expect("Coercion should work");

                for elt in list {
                    if coerced_value == elt {
                        return Ok(ExprValue::Bool(true).into());
                    }
                }
                ExprValue::Bool(false).into()
            }
            Expr::And(expr1, expr2) => {
                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                match target {
                    AnnotatedType::Regular(ExprType::Bool) => {
                        let value1 = self.eval_expr(eval_history, &*expr1)?;
                        let boolean_value1 = value1
                            .coerce_to(eval_history.env, &ExprType::Bool)
                            .expect("Coercion should be valid");

                        let value2 = self.eval_expr(eval_history, &*expr2)?;
                        let boolean_value2 = value2
                            .coerce_to(eval_history.env, &ExprType::Bool)
                            .expect("Coercion should be valid");

                        let bool1 = match boolean_value1 {
                            ExprValue::Bool(val) => val,
                            _ => panic!("Expected boolean"),
                        };
                        let bool2 = match boolean_value2 {
                            ExprValue::Bool(val) => val,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::Bool(bool1 && bool2).into()
                    }
                    AnnotatedType::Regular(ExprType::Constraint) => {
                        let value1 = self.eval_expr(eval_history, &*expr1)?;
                        let constraint_value1 = value1
                            .coerce_to(eval_history.env, &ExprType::Constraint)
                            .expect("Coercion should be valid");

                        let value2 = self.eval_expr(eval_history, &*expr2)?;
                        let constraint_value2 = value2
                            .coerce_to(eval_history.env, &ExprType::Constraint)
                            .expect("Coercion should be valid");

                        let constraint1 = match constraint_value1 {
                            ExprValue::Constraint(constraints) => constraints,
                            _ => panic!("Expected boolean"),
                        };
                        let constraint2 = match constraint_value2 {
                            ExprValue::Constraint(constraints) => constraints,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::Constraint(
                            constraint1
                                .into_iter()
                                .chain(constraint2.into_iter())
                                .collect(),
                        )
                        .into()
                    }
                    _ => panic!("Expected Bool or Constraint"),
                }
            }
            Expr::Or(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let boolean_value1 = value1
                    .coerce_to(eval_history.env, &ExprType::Bool)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let boolean_value2 = value2
                    .coerce_to(eval_history.env, &ExprType::Bool)
                    .expect("Coercion should be valid");

                let bool1 = match boolean_value1 {
                    ExprValue::Bool(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let bool2 = match boolean_value2 {
                    ExprValue::Bool(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Bool(bool1 || bool2).into()
            }
            Expr::Not(not_expr) => {
                let value = self.eval_expr(eval_history, &*not_expr)?;
                let boolean_value = value
                    .coerce_to(eval_history.env, &ExprType::Bool)
                    .expect("Coercion should be valid");

                match boolean_value {
                    ExprValue::Bool(val) => ExprValue::Bool(!val).into(),
                    _ => panic!("Expected boolean"),
                }
            }
            Expr::ConstraintEq(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let lin_expr1_value = value1
                    .coerce_to(eval_history.env, &ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let lin_expr2_value = value2
                    .coerce_to(eval_history.env, &ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let lin_expr1 = match lin_expr1_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let lin_expr2 = match lin_expr2_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Constraint(Vec::from([lin_expr1.eq(&lin_expr2).into()])).into()
            }
            Expr::ConstraintLe(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let lin_expr1_value = value1
                    .coerce_to(eval_history.env, &ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let lin_expr2_value = value2
                    .coerce_to(eval_history.env, &ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let lin_expr1 = match lin_expr1_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let lin_expr2 = match lin_expr2_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Constraint(Vec::from([lin_expr1.leq(&lin_expr2).into()])).into()
            }
            Expr::ConstraintGe(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let lin_expr1_value = value1
                    .coerce_to(eval_history.env, &ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let lin_expr2_value = value2
                    .coerce_to(eval_history.env, &ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let lin_expr1 = match lin_expr1_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let lin_expr2 = match lin_expr2_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Constraint(Vec::from([lin_expr1.geq(&lin_expr2).into()])).into()
            }
            Expr::Eq(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let typ1 = value1.get_type(eval_history.env);

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let typ2 = value2.get_type(eval_history.env);

                let target =
                    AnnotatedType::unify(&typ1, &typ2).expect("It should be possible to unify");
                let target_typ = match target {
                    AnnotatedType::Forced(typ) | AnnotatedType::Regular(typ) => typ,
                    AnnotatedType::UntypedList => {
                        // We have two empty lists, they are equal
                        return Ok(ExprValue::Bool(true).into());
                    }
                };

                let coerced_value1 = value1
                    .coerce_to(eval_history.env, &target_typ)
                    .expect("Coercion should be valid");
                let coerced_value2 = value2
                    .coerce_to(eval_history.env, &target_typ)
                    .expect("Coercion should be valid");
                ExprValue::Bool(coerced_value1 == coerced_value2).into()
            }
            Expr::Ne(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let typ1 = value1.get_type(eval_history.env);

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let typ2 = value2.get_type(eval_history.env);

                let target =
                    AnnotatedType::unify(&typ1, &typ2).expect("It should be possible to unify");
                let target_typ = match target {
                    AnnotatedType::Forced(typ) | AnnotatedType::Regular(typ) => typ,
                    AnnotatedType::UntypedList => {
                        // We have two empty lists, they are equal
                        return Ok(ExprValue::Bool(false).into());
                    }
                };

                let coerced_value1 = value1
                    .coerce_to(eval_history.env, &target_typ)
                    .expect("Coercion should be valid");
                let coerced_value2 = value2
                    .coerce_to(eval_history.env, &target_typ)
                    .expect("Coercion should be valid");
                ExprValue::Bool(coerced_value1 != coerced_value2).into()
            }
            Expr::Lt(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let number1_value = value1
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let number2_value = value2
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let num1 = match number1_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let num2 = match number2_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Bool(num1 < num2).into()
            }
            Expr::Le(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let number1_value = value1
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let number2_value = value2
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let num1 = match number1_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let num2 = match number2_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Bool(num1 <= num2).into()
            }
            Expr::Gt(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let number1_value = value1
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let number2_value = value2
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let num1 = match number1_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let num2 = match number2_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Bool(num1 > num2).into()
            }
            Expr::Ge(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let number1_value = value1
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*expr2)?;
                let number2_value = value2
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let num1 = match number1_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let num2 = match number2_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Bool(num1 >= num2).into()
            }
            Expr::Add(left, right) => {
                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match target {
                    AnnotatedType::Regular(ExprType::Int) => {
                        let number_value1 = value1
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");
                        let number_value2 = value2
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");

                        let num1 = match number_value1 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected Int"),
                        };
                        let num2 = match number_value2 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected Int"),
                        };

                        ExprValue::Int(num1 + num2).into()
                    }
                    AnnotatedType::Regular(ExprType::LinExpr) => {
                        let linexpr1_value = value1
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");
                        let linexpr2_value = value2
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");

                        let linexpr1 = match linexpr1_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected LinExpr"),
                        };
                        let linexpr2 = match linexpr2_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected LinExpr"),
                        };

                        ExprValue::LinExpr(linexpr1 + linexpr2).into()
                    }
                    AnnotatedType::Regular(ExprType::List(elem_t)) => {
                        let list1 = match value1 {
                            AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                            | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                            AnnotatedValue::UntypedList => Vec::new(),
                            _ => panic!("List expected"),
                        };
                        let list2 = match value2 {
                            AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                            | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                            AnnotatedValue::UntypedList => Vec::new(),
                            _ => panic!("List expected"),
                        };
                        let list = list1
                            .into_iter()
                            .chain(list2.into_iter())
                            .map(|e| {
                                e.coerce_to(eval_history.env, &elem_t)
                                    .expect("Coercion should be valid")
                            })
                            .collect();
                        AnnotatedValue::Regular(ExprValue::List(*elem_t.clone(), list))
                    }
                    AnnotatedType::UntypedList => AnnotatedValue::UntypedList,
                    _ => panic!(
                        "Expected Int or LinExpr or List: {:?} for node {:?}",
                        target, expr
                    ),
                }
            }
            Expr::Sub(left, right) => {
                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match target {
                    AnnotatedType::Regular(ExprType::Int) => {
                        let number_value1 = value1
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");
                        let number_value2 = value2
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");

                        let num1 = match number_value1 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected boolean"),
                        };
                        let num2 = match number_value2 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::Int(num1 - num2).into()
                    }
                    AnnotatedType::Regular(ExprType::LinExpr) => {
                        let linexpr1_value = value1
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");
                        let linexpr2_value = value2
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");

                        let linexpr1 = match linexpr1_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected boolean"),
                        };
                        let linexpr2 = match linexpr2_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::LinExpr(linexpr1 - linexpr2).into()
                    }
                    AnnotatedType::UntypedList => AnnotatedValue::UntypedList,
                    AnnotatedType::Regular(ExprType::List(elem_t)) => {
                        let list1 = match value1 {
                            AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                            | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                            AnnotatedValue::UntypedList => Vec::new(),
                            _ => panic!("List expected"),
                        };
                        let list2 = match value2 {
                            AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                            | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                            AnnotatedValue::UntypedList => Vec::new(),
                            _ => panic!("List expected"),
                        };

                        let coerced_list2: BTreeSet<_> = list2
                            .into_iter()
                            .map(|e| {
                                e.coerce_to(eval_history.env, &elem_t)
                                    .expect("Coercion should be valid")
                            })
                            .collect();
                        let collection: Vec<_> = list1
                            .into_iter()
                            .filter_map(|e| {
                                let coerced_elem = e
                                    .coerce_to(eval_history.env, &elem_t)
                                    .expect("Coercion should be valid");

                                if coerced_list2.contains(&&coerced_elem) {
                                    return None;
                                }
                                Some(coerced_elem)
                            })
                            .collect();
                        AnnotatedValue::Regular(ExprValue::List(*elem_t.clone(), collection))
                    }
                    _ => panic!("Expected Int or LinExpr or List"),
                }
            }
            Expr::Neg(term) => {
                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let value = self.eval_expr(eval_history, &*term)?;

                match target {
                    AnnotatedType::Regular(ExprType::Int) => {
                        let number_value = value
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");

                        let num = match number_value {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected Int"),
                        };

                        ExprValue::Int(-num).into()
                    }
                    AnnotatedType::Regular(ExprType::LinExpr) => {
                        let linexpr_value = value
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");

                        let linexpr = match linexpr_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected LinExpr"),
                        };

                        ExprValue::LinExpr(-linexpr).into()
                    }
                    _ => panic!("Expected Int or LinExpr: {:?} for node {:?}", target, expr),
                }
            }
            Expr::Mul(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                let typ1 = value1
                    .get_type(eval_history.env)
                    .into_inner()
                    .expect("We should have an inner type for multiplication");
                let typ2 = value2
                    .get_type(eval_history.env)
                    .into_inner()
                    .expect("We should have an inner type for multiplication");

                match (&typ1, &typ2) {
                    (ExprType::Int, ExprType::Int) => {
                        let number_value1 = value1
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");
                        let number_value2 = value2
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");

                        let num1 = match number_value1 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected boolean"),
                        };
                        let num2 = match number_value2 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::Int(num1 * num2).into()
                    }
                    (ExprType::LinExpr, ExprType::Int) => {
                        let linexpr1_value = value1
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");
                        let number_value2 = value2
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");

                        let linexpr1 = match linexpr1_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected boolean"),
                        };
                        let num2 = match number_value2 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::LinExpr(num2 * linexpr1).into()
                    }
                    (ExprType::Int, ExprType::LinExpr) => {
                        let number_value1 = value1
                            .coerce_to(eval_history.env, &ExprType::Int)
                            .expect("Coercion should be valid");
                        let linexpr2_value = value2
                            .coerce_to(eval_history.env, &ExprType::LinExpr)
                            .expect("Coercion should be valid");

                        let num1 = match number_value1 {
                            ExprValue::Int(val) => val,
                            _ => panic!("Expected boolean"),
                        };
                        let linexpr2 = match linexpr2_value {
                            ExprValue::LinExpr(val) => val,
                            _ => panic!("Expected boolean"),
                        };

                        ExprValue::LinExpr(num1 * linexpr2).into()
                    }
                    _ => panic!(
                        "Unexpected type combination for product: {}, {}",
                        typ1, typ2
                    ),
                }
            }
            Expr::Div(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let number1_value = value1
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*right)?;
                let number2_value = value2
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let num1 = match number1_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let num2 = match number2_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Int(num1 / num2).into()
            }
            Expr::Mod(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let number1_value = value1
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(eval_history, &*right)?;
                let number2_value = value2
                    .coerce_to(eval_history.env, &ExprType::Int)
                    .expect("Coercion should be valid");

                let num1 = match number1_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let num2 = match number2_value {
                    ExprValue::Int(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Int(num1 % num2).into()
            }
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let target_typ = match target {
                    AnnotatedType::Forced(typ) | AnnotatedType::Regular(typ) => typ,
                    AnnotatedType::UntypedList => {
                        // Empty list either way, let's return an empty list
                        return Ok(AnnotatedValue::UntypedList);
                    }
                };

                let cond_value = self.eval_expr(eval_history, &condition)?;
                let coerced_cond = cond_value
                    .coerce_to(eval_history.env, &ExprType::Bool)
                    .expect("Coercion should be valid");

                let ExprValue::Bool(cond) = coerced_cond else {
                    panic!("Expected Bool");
                };

                let value = if cond {
                    self.eval_expr(eval_history, &then_expr)?
                } else {
                    self.eval_expr(eval_history, &else_expr)?
                };

                value
                    .coerce_to(eval_history.env, target_typ)
                    .expect("Coercion should be valid")
                    .into()
            }
            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let is_lin_expr = match target {
                    AnnotatedType::Forced(ExprType::LinExpr)
                    | AnnotatedType::Regular(ExprType::LinExpr) => true,
                    AnnotatedType::Forced(ExprType::Int)
                    | AnnotatedType::Regular(ExprType::Int) => false,
                    _ => panic!("Expected Int or LinExpr"),
                };

                let (_typ, list) = match collection_value {
                    AnnotatedValue::Regular(ExprValue::List(typ, list))
                    | AnnotatedValue::Forced(ExprValue::List(typ, list)) => (typ, list),
                    _ => panic!("Expected list"),
                };

                if is_lin_expr {
                    let mut output = LinExpr::constant(0.);

                    for elem in list {
                        self.register_identifier(&var.node, elem);
                        self.push_scope();

                        let cond = match filter {
                            None => true,
                            Some(f) => {
                                let filter_value = self.eval_expr(eval_history, &f)?;
                                let coerced_filter = filter_value
                                    .coerce_to(eval_history.env, &ExprType::Bool)
                                    .expect("Coercion should be valid");
                                match coerced_filter {
                                    ExprValue::Bool(v) => v,
                                    _ => panic!("Expected bool"),
                                }
                            }
                        };

                        if cond {
                            let new_value = self.eval_expr(eval_history, &body)?;
                            let coerced_body = new_value
                                .coerce_to(eval_history.env, &ExprType::LinExpr)
                                .expect("Coercion should be valid");
                            let new_lin_expr = match coerced_body {
                                ExprValue::LinExpr(v) => v,
                                _ => panic!("Expected LinExpr"),
                            };
                            output = output + new_lin_expr;
                        }

                        self.pop_scope();
                    }

                    ExprValue::LinExpr(output).into()
                } else {
                    let mut output = 0;

                    for elem in list {
                        self.register_identifier(&var.node, elem);
                        self.push_scope();

                        let cond = match filter {
                            None => true,
                            Some(f) => {
                                let filter_value = self.eval_expr(eval_history, &f)?;
                                let coerced_filter = filter_value
                                    .coerce_to(eval_history.env, &ExprType::Bool)
                                    .expect("Coercion should be valid");
                                match coerced_filter {
                                    ExprValue::Bool(v) => v,
                                    _ => panic!("Expected bool"),
                                }
                            }
                        };

                        if cond {
                            let new_value = self.eval_expr(eval_history, &body)?;
                            let coerced_body = new_value
                                .coerce_to(eval_history.env, &ExprType::Int)
                                .expect("Coercion should be valid");
                            let new_int_expr = match coerced_body {
                                ExprValue::Int(v) => v,
                                _ => panic!("Expected LinExpr"),
                            };
                            output = output + new_int_expr;
                        }

                        self.pop_scope();
                    }

                    ExprValue::Int(output).into()
                }
            }
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let is_constraint = match target {
                    AnnotatedType::Forced(ExprType::Constraint)
                    | AnnotatedType::Regular(ExprType::Constraint) => true,
                    AnnotatedType::Forced(ExprType::Bool)
                    | AnnotatedType::Regular(ExprType::Bool) => false,
                    _ => panic!("Expected Constraint or Bool"),
                };

                let (_typ, list) = match collection_value {
                    AnnotatedValue::Regular(ExprValue::List(typ, list))
                    | AnnotatedValue::Forced(ExprValue::List(typ, list)) => (typ, list),
                    _ => panic!("Expected list"),
                };

                if is_constraint {
                    let mut output = Vec::new();

                    for elem in list {
                        self.register_identifier(&var.node, elem);
                        self.push_scope();

                        let cond = match filter {
                            None => true,
                            Some(f) => {
                                let filter_value = self.eval_expr(eval_history, &f)?;
                                let coerced_filter = filter_value
                                    .coerce_to(eval_history.env, &ExprType::Bool)
                                    .expect("Coercion should be valid");
                                match coerced_filter {
                                    ExprValue::Bool(v) => v,
                                    _ => panic!("Expected bool"),
                                }
                            }
                        };

                        if cond {
                            let new_value = self.eval_expr(eval_history, &body)?;
                            let coerced_body = new_value
                                .coerce_to(eval_history.env, &ExprType::Constraint)
                                .expect("Coercion should be valid");
                            let new_constraint = match coerced_body {
                                ExprValue::Constraint(v) => v,
                                _ => panic!("Expected LinExpr"),
                            };
                            output.extend(new_constraint);
                        }

                        self.pop_scope();
                    }

                    ExprValue::Constraint(output).into()
                } else {
                    let mut output = true;

                    for elem in list {
                        self.register_identifier(&var.node, elem);
                        self.push_scope();

                        let cond = match filter {
                            None => true,
                            Some(f) => {
                                let filter_value = self.eval_expr(eval_history, &f)?;
                                let coerced_filter = filter_value
                                    .coerce_to(eval_history.env, &ExprType::Bool)
                                    .expect("Coercion should be valid");
                                match coerced_filter {
                                    ExprValue::Bool(v) => v,
                                    _ => panic!("Expected bool"),
                                }
                            }
                        };

                        if cond {
                            let new_value = self.eval_expr(eval_history, &body)?;
                            let coerced_body = new_value
                                .coerce_to(eval_history.env, &ExprType::Bool)
                                .expect("Coercion should be valid");
                            let new_bool_expr = match coerced_body {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected LinExpr"),
                            };
                            output = output && new_bool_expr;
                        }

                        self.pop_scope();
                    }

                    ExprValue::Bool(output).into()
                }
            }
            Expr::VarListCall { name, args } => {
                let var_lists = eval_history.ast.get_var_lists();
                let var_list_fn = var_lists
                    .get(&name.node)
                    .expect("Var list should be declared");
                let evaluated_args: Vec<_> = args
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;
                let fn_desc = eval_history
                    .ast
                    .global_env
                    .get_functions()
                    .get(var_list_fn)
                    .expect("Function should be valid");
                let coerced_args: Vec<_> = evaluated_args
                    .into_iter()
                    .zip(fn_desc.typ.args.iter())
                    .map(|(arg, arg_typ)| {
                        arg.coerce_to(eval_history.env, arg_typ)
                            .expect("Coercion should be valid")
                    })
                    .collect();

                let (constraints, _origin) = eval_history.add_fn_to_call_history(
                    var_list_fn,
                    coerced_args.iter().map(|x| x.clone().into()).collect(),
                    true,
                )?;
                eval_history.var_lists.insert(
                    (name.node.clone(), coerced_args.clone()),
                    var_list_fn.clone(),
                );

                let constraint_count = match constraints {
                    ExprValue::List(ExprType::Constraint, list) => list.len(),
                    _ => panic!("Expected [Constraint]"),
                };

                ExprValue::List(
                    ExprType::LinExpr,
                    (0..constraint_count)
                        .into_iter()
                        .map(|i| {
                            ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar {
                                name: name.node.clone(),
                                from_list: Some(i),
                                params: coerced_args.clone(),
                            })))
                        })
                        .collect(),
                )
                .into()
            }
            Expr::ListComprehension {
                body,
                vars_and_collections,
                filter,
            } => {
                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let inner_typ = match target {
                    AnnotatedType::Regular(ExprType::List(typ)) => typ.as_ref().clone(),
                    _ => panic!("Expected typed list: {:?}", target),
                };

                let list = self.build_naked_list_for_list_comprehension(
                    eval_history,
                    &body,
                    &vars_and_collections[..],
                    filter.as_ref().map(|x| x.as_ref()),
                )?;

                ExprValue::List(inner_typ, list).into()
            }
            Expr::Let { var, value, body } => {
                let value_value = self.eval_expr(eval_history, &value)?;
                let inner_value = value_value.into_inner().expect("Should have a known type");

                self.register_identifier(&var.node, inner_value);
                self.push_scope();

                let body_value = self.eval_expr(eval_history, &body)?;

                self.pop_scope();

                body_value
            }
        })
    }

    fn build_naked_list_for_list_comprehension(
        &mut self,
        eval_history: &mut EvalHistory<'_, T>,
        body: &Spanned<crate::ast::Expr>,
        vars_and_collections: &[(Spanned<String>, Spanned<crate::ast::Expr>)],
        filter: Option<&Spanned<crate::ast::Expr>>,
    ) -> Result<Vec<ExprValue<T>>, EvalError> {
        if vars_and_collections.is_empty() {
            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = self.eval_expr(eval_history, &f)?;
                    let coerced_filter = filter_value
                        .coerce_to(eval_history.env, &ExprType::Bool)
                        .expect("Coercion should be valid");
                    match coerced_filter {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected bool"),
                    }
                }
            };

            return Ok(if cond {
                let new_value = self.eval_expr(eval_history, &body)?;
                let inner_value = new_value
                    .into_inner()
                    .expect("Element in list comprehensions should have definite types");
                Vec::from([inner_value])
            } else {
                Vec::new()
            });
        }

        let (var, collection) = &vars_and_collections[0];
        let remaining_v_and_c = &vars_and_collections[1..];

        let collection_value = self.eval_expr(eval_history, &collection)?;

        let (_typ, list) = match collection_value {
            AnnotatedValue::Regular(ExprValue::List(typ, list))
            | AnnotatedValue::Forced(ExprValue::List(typ, list)) => (typ, list),
            _ => panic!("Expected typed list"),
        };

        let mut output = Vec::new();

        for elem in list {
            self.register_identifier(&var.node, elem);
            self.push_scope();

            output.extend(self.build_naked_list_for_list_comprehension(
                eval_history,
                body,
                remaining_v_and_c,
                filter,
            )?);

            self.pop_scope();
        }

        Ok(output)
    }
}

#[derive(Debug)]
pub struct EvalHistory<'a, T: EvalObject> {
    ast: &'a CheckedAST<T>,
    env: &'a T::Env,
    cache: T::Cache,
    funcs: BTreeMap<(String, Vec<ExprValue<T>>), (ExprValue<T>, Origin<T>)>,
    vars: BTreeMap<(String, Vec<ExprValue<T>>), String>,
    var_lists: BTreeMap<(String, Vec<ExprValue<T>>), String>,
}

impl<'a, T: EvalObject> EvalHistory<'a, T> {
    fn new(ast: &'a CheckedAST<T>, env: &'a T::Env, cache: T::Cache) -> Result<Self, EvalError> {
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

    fn prettify_expr_value(&mut self, value: &ExprValue<T>) -> String {
        match value {
            ExprValue::Object(obj) => match obj.pretty_print(self.env, &mut self.cache) {
                Some(s) => s,
                None => format!("{:?}", obj),
            },
            ExprValue::List(_typ, list) => {
                let pretty_values: Vec<_> =
                    list.iter().map(|x| self.prettify_expr_value(x)).collect();
                format!("[{}]", pretty_values.join(","))
            }
            ExprValue::Bool(v) => format!("{}", v),
            ExprValue::Int(v) => format!("{}", v),
            _ => format!("{:?}", value),
        }
    }

    fn prettify_docstring(
        &mut self,
        fn_desc: &FunctionDesc,
        args: &Vec<ExprValue<T>>,
    ) -> Vec<String> {
        let mut substitution_values = HashMap::new();
        for (arg_name, arg_value) in fn_desc.arg_names.iter().zip(args.iter()) {
            let pretty_value = self.prettify_expr_value(arg_value);
            substitution_values.insert(arg_name.clone(), pretty_value);
        }

        let re =
            regex::Regex::new(r"@\{([a-zA-Z_][a-zA-Z0-9_]*)\}").expect("Should be a valid regex");

        fn_desc
            .docstring
            .iter()
            .map(|d| {
                re.replace_all(d.trim_start(), |caps: &regex::Captures| {
                    let name = &caps[1];
                    substitution_values
                        .get(name)
                        .cloned()
                        .unwrap_or(caps[0].to_string())
                })
                .to_string()
            })
            .collect()
    }

    fn add_fn_to_call_history(
        &mut self,
        fn_name: &str,
        args: Vec<AnnotatedValue<T>>,
        allow_private: bool,
    ) -> Result<(ExprValue<T>, Origin<T>), EvalError> {
        let fn_desc = self
            .ast
            .global_env
            .get_functions()
            .get(fn_name)
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

        let mut coerced_args = vec![];
        let mut local_env = LocalEnv::new();
        for (param, ((arg, arg_typ), arg_name)) in args
            .clone()
            .into_iter()
            .zip(fn_desc.typ.args.iter())
            .zip(fn_desc.arg_names.iter())
            .enumerate()
        {
            let coerced_arg = arg
                .coerce_to(self.env, arg_typ)
                .ok_or(EvalError::TypeMismatch {
                    param: param,
                    expected: arg_typ.clone(),
                    found: arg.get_type(self.env).into(),
                })?;
            coerced_args.push(coerced_arg.clone());
            local_env.register_identifier(arg_name, coerced_arg);
        }

        if let Some(r) = self.funcs.get(&(fn_name.to_string(), coerced_args.clone())) {
            return Ok(r.clone());
        }

        local_env.push_scope();
        let annotated_result = local_env.eval_expr(self, &fn_desc.body)?;
        local_env.pop_scope();

        let origin = Origin {
            fn_name: Spanned::new(fn_name.to_string(), fn_desc.body.span.clone()),
            args: coerced_args.clone(),
            pretty_docstring: self.prettify_docstring(fn_desc, &coerced_args),
        };

        let result = annotated_result
            .coerce_to(self.env, &fn_desc.typ.output)
            .expect(&format!(
                "Coercion to output type should always work in a checked AST: {:?} -> {:?}",
                annotated_result, fn_desc.typ.output
            ))
            .with_origin(&origin);
        self.funcs.insert(
            (fn_name.to_string(), coerced_args),
            (result.clone(), origin.clone()),
        );

        Ok((result, origin))
    }
}

impl<'a, T: EvalObject> EvalHistory<'a, T> {
    pub fn validate_value(&self, val: &ExprValue<T>) -> bool {
        match val {
            ExprValue::Int(_) => true,
            ExprValue::Bool(_) => true,
            ExprValue::LinExpr(_) => true,
            ExprValue::Constraint(_) => true,
            ExprValue::Object(_) => self.ast.global_env.validate_type(&val.get_type(self.env)),
            ExprValue::List(typ, list) => {
                for elem in list {
                    let elem_t = elem.get_type(self.env);
                    if elem_t != *typ {
                        return false;
                    }
                    if !self.validate_value(elem) {
                        return false;
                    }
                }
                true
            }
        }
    }

    pub fn eval_fn(
        &mut self,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<(ExprValue<T>, Origin<T>), EvalError> {
        let mut checked_args = vec![];
        for (param, arg) in args.into_iter().enumerate() {
            if !self.validate_value(&arg) {
                return Err(EvalError::InvalidExprValue { param });
            }
            checked_args.push(arg.into());
        }

        self.add_fn_to_call_history(fn_name, checked_args.clone(), false)
    }

    pub fn into_var_def(self) -> VariableDefinitions<T> {
        let mut var_def = VariableDefinitions {
            vars: BTreeMap::new(),
            var_lists: BTreeMap::new(),
        };

        for ((v_name, v_args), fn_name) in self.vars {
            let (fn_call_result, new_origin) = self
                .funcs
                .get(&(fn_name.clone(), v_args.clone()))
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
                .insert((v_name, v_args), (constraint, new_origin.clone()));
        }

        for ((vl_name, vl_args), fn_name) in self.var_lists {
            let (fn_call_result, new_origin) = self
                .funcs
                .get(&(fn_name.clone(), vl_args.clone()))
                .expect("Fn call should be valid");
            let constraints: Vec<_> = match fn_call_result {
                ExprValue::List(ExprType::Constraint, cs) => cs
                    .iter()
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
                    .collect(),
                _ => panic!(
                    "Fn call should have returned a constraint list: {:?}",
                    fn_call_result
                ),
            };
            var_def
                .var_lists
                .insert((vl_name, vl_args), (constraints, new_origin.clone()));
        }

        var_def
    }
}

#[derive(Clone, Debug)]
pub struct VariableDefinitions<T: EvalObject> {
    pub vars: BTreeMap<(String, Vec<ExprValue<T>>), (Vec<Constraint<IlpVar<T>>>, Origin<T>)>,
    pub var_lists:
        BTreeMap<(String, Vec<ExprValue<T>>), (Vec<Vec<Constraint<IlpVar<T>>>>, Origin<T>)>,
}
