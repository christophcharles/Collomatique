use crate::ast::Spanned;
use crate::parser::Rule;
use crate::semantics::*;
use crate::traits::{EvalObject, FieldConversionError};
use collomatique_ilp::{Constraint, LinExpr};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[cfg(test)]
mod tests;

use derivative::Derivative;

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialOrd, Ord, PartialEq, Eq)]
pub struct ScriptVar<T: EvalObject> {
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
        name: String,
        from_list: Option<usize>,
        params: Vec<ExprValue<T>>,
    ) -> Self {
        let args: Vec<_> = params
            .iter()
            .map(|x| x.convert_to_string(env, cache))
            .collect();
        ScriptVar {
            name,
            from_list,
            params,
            params_str: args.join(", "),
        }
    }

    pub fn new_no_env(name: String, from_list: Option<usize>, params: Vec<ExprValue<T>>) -> Self {
        let args: Vec<_> = params.iter().map(|x| format!("{}", x)).collect();
        ScriptVar {
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
    pub fn_name: Spanned<String>,
    pub args: Vec<ExprValue<T>>,
    pub pretty_docstring: Vec<String>,
}

impl<T: EvalObject> std::fmt::Display for Origin<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pretty_docstring.is_empty() {
            let args_str: Vec<_> = self.args.iter().map(|x| x.to_string()).collect();

            write!(f, "{}({})", self.fn_name.node, args_str.join(", "))
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExprValue<T: EvalObject> {
    None,
    Int(i32),
    Bool(bool),
    LinExpr(LinExpr<IlpVar<T>>),
    Constraint(Vec<ConstraintWithOrigin<T>>),
    String(String),
    Object(T),
    List(Vec<ExprValue<T>>),
}

impl<T: EvalObject> std::fmt::Display for ExprValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprValue::None => write!(f, "none"),
            ExprValue::Int(v) => write!(f, "{}", v),
            ExprValue::Bool(v) => write!(f, "{}", v),
            ExprValue::LinExpr(lin_expr) => write!(f, "{}", lin_expr),
            ExprValue::Constraint(c_with_o) => {
                let strs: Vec<_> = c_with_o.iter().map(|x| x.constraint.to_string()).collect();
                write!(f, "{}", strs.join(", "))
            }
            ExprValue::String(str_literal) => {
                let mut closing_delim = String::from("\"");
                while str_literal.contains(&closing_delim) {
                    closing_delim.push('~');
                }
                write!(
                    f,
                    "{}{}{}",
                    closing_delim.chars().rev().collect::<String>(),
                    str_literal,
                    closing_delim
                )
            }
            ExprValue::Object(obj) => write!(f, "{:?}", obj),
            ExprValue::List(list) => {
                let strs: Vec<_> = list.iter().map(|x| x.to_string()).collect();
                write!(f, "[{}]", strs.join(", "))
            }
        }
    }
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
            ExprValue::List(list) => {
                ExprValue::List(list.iter().map(|x| x.with_origin(origin)).collect())
            }
            _ => self.clone(),
        }
    }

    pub fn is_primitive_type(&self) -> bool {
        matches!(
            self,
            Self::Bool(_) | Self::Constraint(_) | Self::LinExpr(_) | Self::Int(_) | Self::None
        )
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    pub fn is_arithmetic(&self) -> bool {
        matches!(self, Self::Int(_) | Self::LinExpr(_))
    }

    pub fn fits_in_typ(&self, env: &T::Env, target: &ExprType) -> bool {
        match self {
            // for non-list, it is just of matter of checking that the typ is in the sum
            Self::None => target.get_variants().contains(&SimpleType::None),
            Self::Int(_) => target.get_variants().contains(&SimpleType::Int),
            Self::Bool(_) => target.get_variants().contains(&SimpleType::Bool),
            Self::LinExpr(_) => target.get_variants().contains(&SimpleType::LinExpr),
            Self::Constraint(_) => target.get_variants().contains(&SimpleType::Constraint),
            Self::String(_) => target.get_variants().contains(&SimpleType::String),
            Self::Object(obj) => target
                .get_variants()
                .contains(&SimpleType::Object(obj.typ_name(env))),
            // if we have an empty list, we just need to check that ExprType is a list
            Self::List(list) if list.is_empty() => target.has_list(),
            // if not empty, we have to check recursively for all list types in the sum
            Self::List(list) => {
                for variant in target.get_variants() {
                    let SimpleType::List(inner_typ) = variant else {
                        continue;
                    };

                    if list.iter().all(|x| x.fits_in_typ(env, &inner_typ)) {
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn can_convert_to(&self, env: &T::Env, target: &ConcreteType) -> bool {
        match (self, target.inner()) {
            // Can always convert to its own type
            (Self::None, SimpleType::None) => true,
            (Self::Int(_), SimpleType::Int) => true,
            (Self::Bool(_), SimpleType::Bool) => true,
            (Self::LinExpr(_), SimpleType::LinExpr) => true,
            (Self::Constraint(_), SimpleType::Constraint) => true,
            (Self::String(_), SimpleType::String) => true,
            (Self::Object(obj), SimpleType::Object(name)) if obj.typ_name(env) == *name => true,
            // For empty list, we can convert to any list type
            (Self::List(list), SimpleType::EmptyList) if list.is_empty() => true,
            (Self::List(list), SimpleType::List(_)) if list.is_empty() => true,
            // For lists, we can convert to another if all the elements are
            // convertible.
            (Self::List(list), SimpleType::List(inner_typ)) => {
                let inner_target = inner_typ.as_simple().expect("Type should be concrete");
                let concrete_inner = inner_target
                    .clone()
                    .into_concrete()
                    .expect("Type should be concrete");
                list.iter().all(|x| x.can_convert_to(env, &concrete_inner))
            }
            // Special cases: we can convert from Int to LinExpr
            (Self::Int(_), SimpleType::LinExpr) => true,
            // Anything converts to String
            (_, SimpleType::String) => true,
            // Everything else forbidden
            _ => false,
        }
    }

    pub fn convert_to(
        self,
        env: &T::Env,
        cache: &mut T::Cache,
        target: &ConcreteType,
    ) -> Option<ExprValue<T>> {
        if !self.can_convert_to(env, target) {
            return None;
        }

        Some(match (self, target.inner()) {
            // This should also work for empty lists as the iterator will be empty
            (Self::List(list), SimpleType::List(inner_typ)) => {
                let inner_target = inner_typ
                    .as_simple()
                    .expect("Inner list target type should have already been checked");
                let concrete_inner = inner_target
                    .clone()
                    .into_concrete()
                    .expect("Type should be concrete");
                Self::List(
                    list.into_iter()
                        .map(|x| x.convert_to(env, cache, &concrete_inner))
                        .collect::<Option<_>>()?,
                )
            }
            (Self::Int(val), SimpleType::LinExpr) => Self::LinExpr(LinExpr::constant(val as f64)),
            // Conversion to string
            (Self::String(v), SimpleType::String) => Self::String(v),
            (v, SimpleType::String) => Self::String(v.convert_to_string(env, cache)),
            // Assume can_convert_to is correct so we just have the default behavior: return the current value
            (orig, _) => orig,
        })
    }

    fn convert_to_string(&self, env: &T::Env, cache: &mut T::Cache) -> String {
        match self {
            Self::Object(obj) => match obj.pretty_print(env, cache) {
                Some(v) => v,
                None => format!("{:?}", obj),
            },
            Self::List(list) => {
                let inners: Vec<_> = list
                    .iter()
                    .map(|x| x.convert_to_string(env, cache))
                    .collect();
                format!("[{}]", inners.join(", "))
            }
            v => format!("{}", v),
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
    expr_types: HashMap<crate::ast::Span, ExprType>,
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
        fn_name: &str,
        args: Vec<ExprValue<NoObject>>,
    ) -> Result<ExprValue<NoObject>, EvalError<NoObject>> {
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

    fn check_env(&self, env: &T::Env) -> Result<(), EvalError<T>> {
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
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<ExprValue<T>, EvalError<T>> {
        let mut eval_history = self.start_eval_history(env)?;
        Ok(eval_history.eval_fn(fn_name, args)?.0)
    }

    pub fn eval_fn_with_variables(
        &self,
        env: &T::Env,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<(ExprValue<T>, VariableDefinitions<T>), EvalError<T>> {
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
    ) -> Result<ExprValue<T>, EvalError<T>> {
        use crate::ast::Expr;
        Ok(match &expr.node {
            Expr::None => ExprValue::None,
            Expr::Boolean(val) => ExprValue::Bool(*val),
            Expr::Number(val) => ExprValue::Int(*val),
            Expr::StringLiteral(val) => ExprValue::String(val.clone()),
            Expr::Ident(ident) => self
                .lookup_ident(&ident.node)
                .expect("Identifiers should be defined in a checked AST"),
            Expr::Path { object, segments } => {
                assert!(!segments.is_empty());

                let mut current_value = self.eval_expr(eval_history, &object)?;

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

                current_value
            }
            Expr::Cardinality(list_expr) => {
                let list_value = self.eval_expr(eval_history, &list_expr)?;
                let count = match list_value {
                    ExprValue::List(list) => list.len(),
                    _ => panic!("Unexpected type for list expression"),
                };
                ExprValue::Int(
                    i32::try_from(count).expect("List length should not exceed i32 capacity"),
                )
            }
            Expr::ExplicitType { expr, typ: _ } => {
                let value = self.eval_expr(eval_history, &expr)?;
                // we do nothing: the semantic analysis has already checked everything
                // and types are relevant only in the semantic phase
                value
            }
            Expr::TypeConversion { expr, typ } => {
                let value = self.eval_expr(eval_history, &expr)?;
                let target_type = ExprType::try_from(typ.clone())
                    .expect("At this point types should be valid")
                    .to_simple()
                    .expect("as should have a simple type as operand");
                let concrete_target = target_type
                    .into_concrete()
                    .expect("Should be concrete at this point");

                let converted_value = value
                    .convert_to(eval_history.env, &mut eval_history.cache, &concrete_target)
                    .expect("Resulting expression should be convertible to target type");

                converted_value
            }
            Expr::ListLiteral { elements } => {
                let element_values: Vec<_> = elements
                    .iter()
                    .map(|x| self.eval_expr(eval_history, &x))
                    .collect::<Result<_, _>>()?;

                ExprValue::List(element_values)
            }
            Expr::ListRange { start, end } => {
                let start_value = self.eval_expr(eval_history, &start)?;
                let end_value = self.eval_expr(eval_history, &end)?;

                let start_num = match start_value {
                    ExprValue::Int(v) => v,
                    _ => panic!("Int expected"),
                };
                let end_num = match end_value {
                    ExprValue::Int(v) => v,
                    _ => panic!("Int expected"),
                };

                ExprValue::List(
                    (start_num..end_num)
                        .into_iter()
                        .map(ExprValue::Int)
                        .collect(),
                )
            }
            Expr::GlobalList(typ_name) => {
                let expr_type = ExprType::try_from(typ_name.clone())
                    .expect("At this point, types should be valid");

                let mut collection = vec![];
                for variant in expr_type.get_variants() {
                    let typ_as_str = match &variant {
                        SimpleType::Object(obj) => obj.clone(),
                        _ => panic!("Object expected"),
                    };
                    let objects = T::objects_with_typ(&eval_history.env, &typ_as_str);
                    collection.extend(objects.into_iter().map(|x| ExprValue::Object(x)));
                }

                ExprValue::List(collection)
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
                if let Some(_args_typ) = eval_history
                    .ast
                    .global_env
                    .get_predefined_vars()
                    .get(&name.node)
                {
                    ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new(
                        eval_history.env,
                        &mut eval_history.cache,
                        name.node.clone(),
                        args,
                    ))))
                } else if let Some(var_desc) =
                    eval_history.ast.global_env.get_vars().get(&name.node)
                {
                    eval_history.vars.insert(
                        (name.node.clone(), args.clone()),
                        var_desc.referenced_fn.clone(),
                    );
                    eval_history.add_fn_to_call_history(
                        &var_desc.referenced_fn,
                        args.clone(),
                        true,
                    )?;
                    ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar::new(
                        eval_history.env,
                        &mut eval_history.cache,
                        name.node.clone(),
                        None,
                        args,
                    ))))
                    .into()
                } else {
                    panic!("Valid var expected")
                }
            }
            Expr::In { item, collection } => {
                let collection_value = self.eval_expr(eval_history, &*collection)?;
                let list = match collection_value {
                    ExprValue::List(list) => list,
                    _ => panic!("List expected"),
                };

                let item_value = self.eval_expr(eval_history, &*item)?;
                for elt in list {
                    if item_value == elt {
                        return Ok(ExprValue::Bool(true));
                    }
                }
                ExprValue::Bool(false)
            }
            Expr::And(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 && v2),
                    (ExprValue::Constraint(mut c1), ExprValue::Constraint(c2)) => {
                        c1.reserve(c2.len());
                        c1.extend(c2);
                        ExprValue::Constraint(c1)
                    }
                    (value1, value2) => panic!(
                        "Unexpected types for AND operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Or(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 || v2),
                    (value1, value2) => panic!(
                        "Unexpected types for OR operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Not(not_expr) => {
                let value = self.eval_expr(eval_history, &*not_expr)?;

                match value {
                    ExprValue::Bool(v) => ExprValue::Bool(!v),
                    value => panic!("Unexpected type for NOT operand: {:?}", value),
                }
            }
            Expr::ConstraintEq(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                let concrete_lin_expr = SimpleType::LinExpr.into_concrete().unwrap();
                if !value1.can_convert_to(&eval_history.env, &concrete_lin_expr) {
                    panic!("Operand for === does not convert to LinExpr: {:?}", value1);
                }
                if !value2.can_convert_to(&eval_history.env, &concrete_lin_expr) {
                    panic!("Operand for === does not convert to LinExpr: {:?}", value2);
                }

                let ExprValue::LinExpr(lin_expr1) = value1
                    .convert_to(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &concrete_lin_expr,
                    )
                    .unwrap()
                else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) = value2
                    .convert_to(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &concrete_lin_expr,
                    )
                    .unwrap()
                else {
                    panic!("Should be a LinExpr result")
                };

                ExprValue::Constraint(Vec::from([lin_expr1.eq(&lin_expr2).into()]))
            }
            Expr::ConstraintLe(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                let concrete_lin_expr = SimpleType::LinExpr.into_concrete().unwrap();
                if !value1.can_convert_to(&eval_history.env, &concrete_lin_expr) {
                    panic!("Operand for === does not convert to LinExpr: {:?}", value1);
                }
                if !value2.can_convert_to(&eval_history.env, &concrete_lin_expr) {
                    panic!("Operand for === does not convert to LinExpr: {:?}", value2);
                }

                let ExprValue::LinExpr(lin_expr1) = value1
                    .convert_to(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &concrete_lin_expr,
                    )
                    .unwrap()
                else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) = value2
                    .convert_to(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &concrete_lin_expr,
                    )
                    .unwrap()
                else {
                    panic!("Should be a LinExpr result")
                };

                ExprValue::Constraint(Vec::from([lin_expr1.leq(&lin_expr2).into()]))
            }
            Expr::ConstraintGe(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                let concrete_lin_expr = SimpleType::LinExpr.into_concrete().unwrap();
                if !value1.can_convert_to(&eval_history.env, &concrete_lin_expr) {
                    panic!("Operand for === does not convert to LinExpr: {:?}", value1);
                }
                if !value2.can_convert_to(&eval_history.env, &concrete_lin_expr) {
                    panic!("Operand for === does not convert to LinExpr: {:?}", value2);
                }

                let ExprValue::LinExpr(lin_expr1) = value1
                    .convert_to(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &concrete_lin_expr,
                    )
                    .unwrap()
                else {
                    panic!("Should be a LinExpr result")
                };
                let ExprValue::LinExpr(lin_expr2) = value2
                    .convert_to(
                        &eval_history.env,
                        &mut eval_history.cache,
                        &concrete_lin_expr,
                    )
                    .unwrap()
                else {
                    panic!("Should be a LinExpr result")
                };

                ExprValue::Constraint(Vec::from([lin_expr1.geq(&lin_expr2).into()]))
            }
            Expr::Eq(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;
                ExprValue::Bool(value1 == value2)
            }
            Expr::Ne(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;
                ExprValue::Bool(value1 != value2)
            }
            Expr::Lt(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 < v2),
                    (value1, value2) => {
                        panic!("Unexpected types for < operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Le(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 <= v2),
                    (value1, value2) => panic!(
                        "Unexpected types for <= operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Gt(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 > v2),
                    (value1, value2) => {
                        panic!("Unexpected types for > operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Ge(expr1, expr2) => {
                let value1 = self.eval_expr(eval_history, &*expr1)?;
                let value2 = self.eval_expr(eval_history, &*expr2)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Bool(v1 >= v2),
                    (value1, value2) => panic!(
                        "Unexpected types for >= operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Add(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                    | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                        let new_lin_expr = LinExpr::constant(int_value as f64);
                        ExprValue::LinExpr(lin_expr_value + new_lin_expr)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => ExprValue::LinExpr(v1 + v2),
                    (ExprValue::String(s1), ExprValue::String(s2)) => ExprValue::String(s1 + &s2),
                    (ExprValue::List(mut list1), ExprValue::List(list2)) => {
                        list1.reserve(list2.len());
                        list1.extend(list2);
                        ExprValue::List(list1)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for + operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Sub(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 - v2),
                    (ExprValue::Int(v1), ExprValue::LinExpr(v2)) => {
                        let new_lin_expr = LinExpr::constant(v1 as f64);
                        ExprValue::LinExpr(new_lin_expr - v2)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::Int(v2)) => {
                        let new_lin_expr = LinExpr::constant(v2 as f64);
                        ExprValue::LinExpr(v1 - new_lin_expr)
                    }
                    (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => ExprValue::LinExpr(v1 - v2),
                    (ExprValue::List(list1), ExprValue::List(list2)) => {
                        let list = list1.into_iter().filter(|x| !list2.contains(x)).collect();
                        ExprValue::List(list)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for - operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Neg(term) => {
                let value = self.eval_expr(eval_history, &*term)?;

                match value {
                    ExprValue::Int(v) => ExprValue::Int(-v),
                    ExprValue::LinExpr(v) => ExprValue::LinExpr(-v),
                    value => panic!("Unexpected type for (-) operand: {:?}", value),
                }
            }
            Expr::Panic(inner_expr) => {
                let value = self.eval_expr(eval_history, &*inner_expr)?;
                return Err(EvalError::Panic(Box::new(value)));
            }
            Expr::Mul(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 * v2),
                    (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                    | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                        ExprValue::LinExpr(int_value * lin_expr_value)
                    }
                    (value1, value2) => {
                        panic!("Unexpected types for * operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::Div(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 / v2),
                    (value1, value2) => panic!(
                        "Unexpected types for // operand: {:?}, {:?}",
                        value1, value2
                    ),
                }
            }
            Expr::Mod(left, right) => {
                let value1 = self.eval_expr(eval_history, &*left)?;
                let value2 = self.eval_expr(eval_history, &*right)?;

                match (value1, value2) {
                    (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 % v2),
                    (value1, value2) => {
                        panic!("Unexpected types for % operand: {:?}, {:?}", value1, value2)
                    }
                }
            }
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_value = self.eval_expr(eval_history, &condition)?;
                let ExprValue::Bool(cond) = cond_value else {
                    panic!("Expected Bool for if condition");
                };

                if cond {
                    self.eval_expr(eval_history, &then_expr)?
                } else {
                    self.eval_expr(eval_history, &else_expr)?
                }
            }
            Expr::Match {
                match_expr,
                branches,
            } => {
                let value = self.eval_expr(eval_history, match_expr)?;

                for branch in branches {
                    let does_typ_match = match &branch.as_typ {
                        Some(t) => {
                            let target_type = ExprType::try_from(t.clone())
                                .expect("At this point types should be valid");
                            value.fits_in_typ(&eval_history.env, &target_type)
                        }
                        None => true,
                    };

                    if !does_typ_match {
                        continue;
                    }

                    // At this point we have a valid type. We convert if needed
                    let converted_value = match &branch.into_typ {
                        Some(t) => {
                            let target_type = ExprType::try_from(t.clone())
                                .expect("At this point types should be valid")
                                .to_simple()
                                .expect("as should have a simple type as operand");
                            let concrete_target = target_type
                                .into_concrete()
                                .expect("Should be concrete at this point");

                            value
                                .clone()
                                .convert_to(
                                    eval_history.env,
                                    &mut eval_history.cache,
                                    &concrete_target,
                                )
                                .expect("Resulting expression should be convertible to target type")
                        }
                        None => value.clone(),
                    };

                    // Let's add the identifier to the scope
                    self.register_identifier(&branch.ident.node, converted_value);
                    self.push_scope();

                    // Now we check the where clause
                    let where_clause_passes = match &branch.filter {
                        None => true,
                        Some(filter_expr) => {
                            let cond_value = match self.eval_expr(eval_history, &filter_expr) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            let ExprValue::Bool(cond) = cond_value else {
                                panic!("Expected Bool for where clause");
                            };
                            cond
                        }
                    };

                    if !where_clause_passes {
                        // Where clause failed, we remove the scope and move to the next branch
                        self.pop_scope();
                        continue;
                    }

                    let output = self.eval_expr(eval_history, &branch.body);

                    self.pop_scope();
                    return output;
                }

                panic!("Match should be exhaustive during evaluation");
            }
            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;
                let ExprValue::List(list) = collection_value else {
                    panic!("Expected collection for sum. Got: {:?}", collection_value);
                };

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let mut output = match target {
                    a if a.is_lin_expr() => ExprValue::LinExpr(LinExpr::constant(0.)),
                    a if a.is_int() => ExprValue::Int(0),
                    a if a.is_list() => ExprValue::List(Vec::with_capacity(list.len())), // Heuristic for length
                    a if a.is_string() => ExprValue::String(String::new()),
                    _ => panic!("Expected Int, LinExpr, String or List output"),
                };

                for elem in list {
                    self.register_identifier(&var.node, elem);
                    self.push_scope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = match self.eval_expr(eval_history, &f) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            match filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        let new_value = match self.eval_expr(eval_history, &body) {
                            Ok(v) => v,
                            Err(e) => {
                                self.pop_scope();
                                return Err(e);
                            }
                        };
                        output = match (output, new_value) {
                            (ExprValue::Int(v1), ExprValue::Int(v2)) => ExprValue::Int(v1 + v2),
                            (ExprValue::Int(int_value), ExprValue::LinExpr(lin_expr_value))
                            | (ExprValue::LinExpr(lin_expr_value), ExprValue::Int(int_value)) => {
                                let new_lin_expr = LinExpr::constant(int_value as f64);
                                ExprValue::LinExpr(lin_expr_value + new_lin_expr)
                            }
                            (ExprValue::LinExpr(v1), ExprValue::LinExpr(v2)) => {
                                ExprValue::LinExpr(v1 + v2)
                            }
                            (ExprValue::String(s1), ExprValue::String(s2)) => {
                                ExprValue::String(s1 + &s2)
                            }
                            (ExprValue::List(mut list), ExprValue::List(new_list)) => {
                                list.reserve(new_list.len());
                                list.extend(new_list);
                                ExprValue::List(list)
                            }
                            (value1, value2) => panic!(
                                "Unexpected types for sum operand: {:?}, {:?}",
                                value1, value2
                            ),
                        };
                    }

                    self.pop_scope();
                }

                output
            }
            Expr::Fold {
                var,
                collection,
                accumulator,
                init_value,
                filter,
                body,
                reversed,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;
                let ExprValue::List(mut list) = collection_value else {
                    panic!("Expected collection for sum. Got: {:?}", collection_value);
                };
                if *reversed {
                    list.reverse();
                }

                let mut output = self.eval_expr(eval_history, &init_value)?;

                for elem in list {
                    self.register_identifier(&var.node, elem);
                    self.register_identifier(&accumulator.node, output.clone());
                    self.push_scope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = match self.eval_expr(eval_history, &f) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            match filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        output = match self.eval_expr(eval_history, &body) {
                            Ok(v) => v,
                            Err(e) => {
                                self.pop_scope();
                                return Err(e);
                            }
                        };
                    }

                    self.pop_scope();
                }

                output
            }
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let collection_value = self.eval_expr(eval_history, &collection)?;
                let ExprValue::List(list) = collection_value else {
                    panic!("Expected collection for sum. Got: {:?}", collection_value);
                };

                let target = eval_history
                    .ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                let mut output = match target {
                    a if a.is_bool() => ExprValue::Bool(true),
                    a if a.is_constraint() => ExprValue::Constraint(Vec::with_capacity(list.len())), // Heuristic for length
                    _ => panic!("Expected Bool or Constraint output"),
                };

                for elem in list {
                    self.register_identifier(&var.node, elem);
                    self.push_scope();

                    let cond = match filter {
                        None => true,
                        Some(f) => {
                            let filter_value = match self.eval_expr(eval_history, &f) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.pop_scope();
                                    return Err(e);
                                }
                            };
                            match filter_value {
                                ExprValue::Bool(v) => v,
                                _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                            }
                        }
                    };

                    if cond {
                        let new_value = match self.eval_expr(eval_history, &body) {
                            Ok(v) => v,
                            Err(e) => {
                                self.pop_scope();
                                return Err(e);
                            }
                        };
                        output = match (output, new_value) {
                            (ExprValue::Bool(v1), ExprValue::Bool(v2)) => ExprValue::Bool(v1 && v2),
                            (ExprValue::Constraint(mut c1), ExprValue::Constraint(c2)) => {
                                c1.reserve(c2.len());
                                c1.extend(c2);
                                ExprValue::Constraint(c1)
                            }
                            (value1, value2) => panic!(
                                "Unexpected types for forall operand: {:?}, {:?}",
                                value1, value2
                            ),
                        };
                    }

                    self.pop_scope();
                }

                output
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

                let (constraints, _origin) = eval_history.add_fn_to_call_history(
                    var_list_fn,
                    evaluated_args.clone(),
                    true,
                )?;
                eval_history.var_lists.insert(
                    (name.node.clone(), evaluated_args.clone()),
                    var_list_fn.clone(),
                );

                let constraint_count = match constraints {
                    ExprValue::List(list) => list.len(),
                    _ => panic!("Expected [Constraint]"),
                };

                ExprValue::List(
                    (0..constraint_count)
                        .into_iter()
                        .map(|i| {
                            ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar::new(
                                eval_history.env,
                                &mut eval_history.cache,
                                name.node.clone(),
                                Some(i),
                                evaluated_args.clone(),
                            ))))
                        })
                        .collect(),
                )
            }
            Expr::ListComprehension {
                body,
                vars_and_collections,
                filter,
            } => {
                let list = self.build_naked_list_for_list_comprehension(
                    eval_history,
                    &body,
                    &vars_and_collections[..],
                    filter.as_ref().map(|x| x.as_ref()),
                )?;

                ExprValue::List(list)
            }
            Expr::Let { var, value, body } => {
                let value_value = self.eval_expr(eval_history, &value)?;

                self.register_identifier(&var.node, value_value);
                self.push_scope();

                let body_value = self.eval_expr(eval_history, &body);

                self.pop_scope();

                body_value?
            }
        })
    }

    fn build_naked_list_for_list_comprehension(
        &mut self,
        eval_history: &mut EvalHistory<'_, T>,
        body: &Spanned<crate::ast::Expr>,
        vars_and_collections: &[(Spanned<String>, Spanned<crate::ast::Expr>)],
        filter: Option<&Spanned<crate::ast::Expr>>,
    ) -> Result<Vec<ExprValue<T>>, EvalError<T>> {
        if vars_and_collections.is_empty() {
            let cond = match filter {
                None => true,
                Some(f) => {
                    let filter_value = self.eval_expr(eval_history, &f)?;
                    match filter_value {
                        ExprValue::Bool(v) => v,
                        _ => panic!("Expected Bool for filter. Got: {:?}", filter_value),
                    }
                }
            };

            return Ok(if cond {
                Vec::from([self.eval_expr(eval_history, &body)?])
            } else {
                Vec::new()
            });
        }

        let (var, collection) = &vars_and_collections[0];
        let remaining_v_and_c = &vars_and_collections[1..];

        let collection_value = self.eval_expr(eval_history, &collection)?;
        let ExprValue::List(list) = collection_value else {
            panic!("Expected list. Got: {:?}", collection_value);
        };

        let mut output = Vec::new();

        for elem in list {
            self.register_identifier(&var.node, elem);
            self.push_scope();

            let extension = self.build_naked_list_for_list_comprehension(
                eval_history,
                body,
                remaining_v_and_c,
                filter,
            );

            self.pop_scope();

            output.extend(extension?);
        }

        Ok(output)
    }
}

use lazy_static::lazy_static;

lazy_static! {
    static ref RE: regex::Regex =
        regex::Regex::new(r"@\{([a-zA-Z_][a-zA-Z0-9_]*)\}").expect("Should be a valid regex");
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
    fn new(ast: &'a CheckedAST<T>, env: &'a T::Env, cache: T::Cache) -> Result<Self, EvalError<T>> {
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
            ExprValue::List(list) => {
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

        fn_desc
            .docstring
            .iter()
            .map(|d| {
                RE.replace_all(d.trim_start(), |caps: &regex::Captures| {
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
        args: Vec<ExprValue<T>>,
        allow_private: bool,
    ) -> Result<(ExprValue<T>, Origin<T>), EvalError<T>> {
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

        let mut local_env = LocalEnv::new();
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

        if let Some(r) = self.funcs.get(&(fn_name.to_string(), args.clone())) {
            return Ok(r.clone());
        }

        local_env.push_scope();
        let naked_result = local_env.eval_expr(self, &fn_desc.body);
        local_env.pop_scope();
        let naked_result = naked_result?;

        let origin = Origin {
            fn_name: Spanned::new(fn_name.to_string(), fn_desc.body.span.clone()),
            args: args.clone(),
            pretty_docstring: self.prettify_docstring(fn_desc, &args),
        };

        let result = naked_result.with_origin(&origin);
        self.funcs.insert(
            (fn_name.to_string(), args),
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
        }
    }

    pub fn eval_fn(
        &mut self,
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

        self.add_fn_to_call_history(fn_name, checked_args.clone(), false)
    }

    pub fn into_var_def_and_cache(self) -> (VariableDefinitions<T>, T::Cache) {
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
            var_def
                .var_lists
                .insert((vl_name, vl_args), (constraints, new_origin.clone()));
        }

        (var_def, self.cache)
    }

    pub fn into_var_def(self) -> VariableDefinitions<T> {
        self.into_var_def_and_cache().0
    }
}

#[derive(Clone, Debug)]
pub struct VariableDefinitions<T: EvalObject> {
    pub vars: BTreeMap<(String, Vec<ExprValue<T>>), (Vec<Constraint<IlpVar<T>>>, Origin<T>)>,
    pub var_lists:
        BTreeMap<(String, Vec<ExprValue<T>>), (Vec<Vec<Constraint<IlpVar<T>>>>, Origin<T>)>,
}
