use crate::ast::{Spanned, TypeName};
use crate::parser::Rule;
use crate::semantics::*;
use collomatique_ilp::{Constraint, LinExpr, UsableData};
use std::collections::{BTreeSet, HashMap};

#[cfg(test)]
mod tests;

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

impl<T: Object> From<Constraint<IlpVar<T>>> for ConstraintWithOrigin<T> {
    fn from(value: Constraint<IlpVar<T>>) -> Self {
        ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExprValue<T: Object> {
    Int(i32),
    Bool(bool),
    LinExpr(LinExpr<IlpVar<T>>),
    Constraint(Vec<ConstraintWithOrigin<T>>),
    Object(T),
    List(ExprType, BTreeSet<ExprValue<T>>),
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
                Some(ExprValue::List(target.clone(), BTreeSet::new()))
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoObject {}

impl Object for NoObject {
    fn typ_name(&self) -> String {
        panic!("No object has no type defined")
    }

    fn field_access(&self, _field: &str) -> ExprValue<Self> {
        panic!("No object has no fields")
    }
}

#[derive(Clone, Debug)]
pub struct CheckedAST {
    global_env: GlobalEnv,
    type_info: TypeInfo,
    expr_types: HashMap<crate::ast::Span, AnnotatedType>,
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

impl<T: Object> Default for EvalEnv<T> {
    fn default() -> Self {
        EvalEnv {
            typ_map: HashMap::new(),
        }
    }
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
    #[error("Param {param} is an inconsistent ExprValue")]
    InvalidExprValue { param: usize },
}

impl<T: Object> EvalEnv<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_objects<I: IntoIterator<Item = T>>(objects: I) -> Result<Self, EnvError<T>> {
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

        let (global_env, type_info, expr_types, errors, warnings) =
            GlobalEnv::new(types, vars, &file)?;

        if !errors.is_empty() {
            return Err(CompileError::SemanticsError { errors, warnings });
        }

        Ok(CheckedAST {
            global_env,
            type_info,
            expr_types,
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

    pub fn validate_value<T: Object>(&self, val: &ExprValue<T>) -> bool {
        match val {
            ExprValue::Int(_) => true,
            ExprValue::Bool(_) => true,
            ExprValue::LinExpr(_) => true,
            ExprValue::Constraint(_) => true,
            ExprValue::Object(_) => self.global_env.validate_type(&val.get_type()),
            ExprValue::List(typ, list) => {
                for elem in list {
                    let elem_t = elem.get_type();
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

    pub fn quick_eval_fn(
        &self,
        fn_name: &str,
        args: Vec<ExprValue<NoObject>>,
    ) -> Result<ExprValue<NoObject>, EvalError> {
        let env = EvalEnv::<NoObject>::new();
        self.eval_fn(&env, fn_name, args)
    }

    pub fn eval_fn<T: Object>(
        &self,
        env: &EvalEnv<T>,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<ExprValue<T>, EvalError> {
        let mut checked_args = vec![];
        for (param, arg) in args.into_iter().enumerate() {
            if !self.validate_value(&arg) {
                return Err(EvalError::InvalidExprValue { param });
            }
            checked_args.push(arg.into());
        }
        self.eval_fn_internal(env, fn_name, checked_args, false)
    }

    fn eval_fn_internal<T: Object>(
        &self,
        env: &EvalEnv<T>,
        fn_name: &str,
        args: Vec<AnnotatedValue<T>>,
        allow_private: bool,
    ) -> Result<ExprValue<T>, EvalError> {
        let fn_desc = self
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
        let annotated_result = local_env.eval_expr(self, env, &fn_desc.body);
        local_env.pop_scope();

        let origin = Origin {
            fn_name: Spanned::new(fn_name.to_string(), fn_desc.body.span.clone()),
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
        ast: &CheckedAST,
        env: &EvalEnv<T>,
        expr: &Spanned<crate::ast::Expr>,
    ) -> AnnotatedValue<T> {
        use crate::ast::Expr;
        match &expr.node {
            Expr::Boolean(val) => ExprValue::Bool(*val).into(),
            Expr::Number(val) => ExprValue::Int(*val).into(),
            Expr::Ident(ident) => self
                .lookup_ident(&ident.node)
                .expect("Identifiers should be defined in a checked AST")
                .into(),
            Expr::Path { object, segments } => {
                assert!(!segments.is_empty());

                let initial_object = self.eval_expr(ast, env, &object);
                let mut current_value = initial_object.into_inner().expect("Object expected");

                for field in segments {
                    let obj = match current_value {
                        ExprValue::Object(obj) => obj,
                        _ => panic!("Object expected"),
                    };
                    current_value = obj.field_access(&field.node);
                }

                current_value.into()
            }
            Expr::Cardinality(list_expr) => {
                let list_value = self.eval_expr(ast, env, &list_expr);
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
                let value = self.eval_expr(ast, env, &expr);
                let target_type = ExprType::from(typ.node.clone());

                // A forced value can be cast anyway
                let loose_value = value.loosen();

                let coerced_value = loose_value
                    .coerce_to(&target_type)
                    .expect("Resulting expression should be coercible to target type");

                coerced_value.into()
            }
            Expr::ListLiteral { elements } => {
                if elements.is_empty() {
                    return AnnotatedValue::UntypedList;
                }

                let element_values: Vec<_> = elements
                    .iter()
                    .map(|x| self.eval_expr(ast, env, &x))
                    .collect();

                let mut unified_type = element_values[0].get_type();
                for item in &element_values[1..] {
                    let item_type = item.get_type();
                    unified_type = AnnotatedType::unify(&unified_type, &item_type)
                        .expect("Types should be unifiable");
                }
                let target_type = unified_type
                    .into_inner()
                    .expect("Type should be determined");

                let coerced_elements: BTreeSet<_> = element_values
                    .iter()
                    .map(|x| {
                        x.coerce_to(&target_type)
                            .expect("Coercion to unified type should be possible")
                    })
                    .collect();

                ExprValue::List(target_type, coerced_elements).into()
            }
            Expr::ListRange { start, end } => {
                let start_value = self.eval_expr(ast, env, &start);
                let end_value = self.eval_expr(ast, env, &end);

                let coerced_start = start_value.coerce_to(&ExprType::Int).expect("Int expected");
                let coerced_end = end_value.coerce_to(&ExprType::Int).expect("Int expected");

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
                let objects = match env.typ_map.get(&typ_as_str) {
                    Some(list) => list,
                    None => {
                        return ExprValue::List(
                            ExprType::from(typ_name.node.clone()),
                            BTreeSet::new(),
                        )
                        .into()
                    }
                };

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
                let args = args.iter().map(|x| self.eval_expr(ast, env, &x)).collect();
                let result = ast
                    .eval_fn_internal(env, &name.node, args, true)
                    .expect("Function call should evaluate");

                result.into()
            }
            Expr::VarCall { name, args } => {
                let args: Vec<_> = args.iter().map(|x| self.eval_expr(ast, env, &x)).collect();
                if let Some(args_typ) = ast.global_env.get_predefined_vars().get(&name.node) {
                    let params = args
                        .into_iter()
                        .zip(args_typ.iter())
                        .map(|(arg, arg_typ)| arg.coerce_to(arg_typ).expect("Coercion should work"))
                        .collect();
                    ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar {
                        name: name.node.clone(),
                        params,
                    })))
                    .into()
                } else if let Some(var_desc) = ast.global_env.get_vars().get(&name.node) {
                    let params = args
                        .into_iter()
                        .zip(var_desc.args.iter())
                        .map(|(arg, arg_typ)| arg.coerce_to(arg_typ).expect("Coercion should work"))
                        .collect();
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
                let collection_value = self.eval_expr(ast, env, &*collection);
                let (elem_t, list) = match collection_value {
                    AnnotatedValue::Forced(ExprValue::List(elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(elem_t, list)) => (elem_t, list),
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        return ExprValue::Bool(false).into();
                    }
                    _ => panic!("List expected"),
                };

                let item_value = self.eval_expr(ast, env, &*item);
                let coerced_value = item_value.coerce_to(&elem_t).expect("Coercion should work");

                for elt in list {
                    if coerced_value == elt {
                        return ExprValue::Bool(true).into();
                    }
                }
                ExprValue::Bool(false).into()
            }
            Expr::Union(collection1, collection2) => {
                let collection1_value = self.eval_expr(ast, env, &*collection1);
                let list1 = match collection1_value {
                    AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        BTreeSet::new()
                    }
                    _ => panic!("List expected"),
                };

                let collection2_value = self.eval_expr(ast, env, &*collection2);
                let list2 = match collection2_value {
                    AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        BTreeSet::new()
                    }
                    _ => panic!("List expected"),
                };

                let target = ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                match target {
                    AnnotatedType::UntypedList => AnnotatedValue::UntypedList,
                    AnnotatedType::Regular(ExprType::List(elem_t)) => {
                        let list = list1
                            .into_iter()
                            .chain(list2.into_iter())
                            .map(|e| e.coerce_to(&elem_t).expect("Coercion should be valid"))
                            .collect();
                        AnnotatedValue::Regular(ExprValue::List(*elem_t.clone(), list))
                    }
                    _ => panic!("Expected list as target type: {}", target),
                }
            }
            Expr::Inter(collection1, collection2) => {
                let collection1_value = self.eval_expr(ast, env, &*collection1);
                let list1 = match collection1_value {
                    AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        BTreeSet::new()
                    }
                    _ => panic!("List expected"),
                };

                let collection2_value = self.eval_expr(ast, env, &*collection2);
                let list2 = match collection2_value {
                    AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        BTreeSet::new()
                    }
                    _ => panic!("List expected"),
                };

                let target = ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                match target {
                    AnnotatedType::UntypedList => AnnotatedValue::UntypedList,
                    AnnotatedType::Regular(ExprType::List(elem_t)) => {
                        let coerced_list2: BTreeSet<_> = list2
                            .into_iter()
                            .map(|e| e.coerce_to(&elem_t).expect("Coercion should be valid"))
                            .collect();
                        let coerced_list1: BTreeSet<_> = list1
                            .into_iter()
                            .map(|e| e.coerce_to(&elem_t).expect("Coercion should be valid"))
                            .collect();
                        let collection = coerced_list1
                            .intersection(&coerced_list2)
                            .cloned()
                            .collect();
                        AnnotatedValue::Regular(ExprValue::List(*elem_t.clone(), collection))
                    }
                    _ => panic!("Expected list as target type: {}", target),
                }
            }
            Expr::Diff(collection1, collection2) => {
                let collection1_value = self.eval_expr(ast, env, &*collection1);
                let list1 = match collection1_value {
                    AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        BTreeSet::new()
                    }
                    _ => panic!("List expected"),
                };

                let collection2_value = self.eval_expr(ast, env, &*collection2);
                let list2 = match collection2_value {
                    AnnotatedValue::Forced(ExprValue::List(_elem_t, list))
                    | AnnotatedValue::Regular(ExprValue::List(_elem_t, list)) => list,
                    AnnotatedValue::UntypedList => {
                        // The list is empty - so "in" must return false
                        BTreeSet::new()
                    }
                    _ => panic!("List expected"),
                };

                let target = ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                match target {
                    AnnotatedType::UntypedList => AnnotatedValue::UntypedList,
                    AnnotatedType::Regular(ExprType::List(elem_t)) => {
                        let coerced_list2: BTreeSet<_> = list2
                            .into_iter()
                            .map(|e| e.coerce_to(&elem_t).expect("Coercion should be valid"))
                            .collect();
                        let coerced_list1: BTreeSet<_> = list1
                            .into_iter()
                            .map(|e| e.coerce_to(&elem_t).expect("Coercion should be valid"))
                            .collect();
                        let collection =
                            coerced_list1.difference(&coerced_list2).cloned().collect();
                        AnnotatedValue::Regular(ExprValue::List(*elem_t.clone(), collection))
                    }
                    _ => panic!("Expected list as target type: {}", target),
                }
            }
            Expr::And(expr1, expr2) => {
                let target = ast
                    .expr_types
                    .get(&expr.span)
                    .expect("Semantic analysis should have given a target type");

                match target {
                    AnnotatedType::Regular(ExprType::Bool) => {
                        let value1 = self.eval_expr(ast, env, &*expr1);
                        let boolean_value1 = value1
                            .coerce_to(&ExprType::Bool)
                            .expect("Coercion should be valid");

                        let value2 = self.eval_expr(ast, env, &*expr2);
                        let boolean_value2 = value2
                            .coerce_to(&ExprType::Bool)
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
                        let value1 = self.eval_expr(ast, env, &*expr1);
                        let constraint_value1 = value1
                            .coerce_to(&ExprType::Constraint)
                            .expect("Coercion should be valid");

                        let value2 = self.eval_expr(ast, env, &*expr2);
                        let constraint_value2 = value2
                            .coerce_to(&ExprType::Constraint)
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
                let value1 = self.eval_expr(ast, env, &*expr1);
                let boolean_value1 = value1
                    .coerce_to(&ExprType::Bool)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let boolean_value2 = value2
                    .coerce_to(&ExprType::Bool)
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
                let value = self.eval_expr(ast, env, &*not_expr);
                let boolean_value = value
                    .coerce_to(&ExprType::Bool)
                    .expect("Coercion should be valid");

                match boolean_value {
                    ExprValue::Bool(val) => ExprValue::Bool(!val).into(),
                    _ => panic!("Expected boolean"),
                }
            }
            Expr::ConstraintEq(expr1, expr2) => {
                let value1 = self.eval_expr(ast, env, &*expr1);
                let lin_expr1_value = value1
                    .coerce_to(&ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let lin_expr2_value = value2
                    .coerce_to(&ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let lin_expr1 = match lin_expr1_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let lin_expr2 = match lin_expr2_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Constraint(vec![lin_expr1.eq(&lin_expr2).into()]).into()
            }
            Expr::ConstraintLe(expr1, expr2) => {
                let value1 = self.eval_expr(ast, env, &*expr1);
                let lin_expr1_value = value1
                    .coerce_to(&ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let lin_expr2_value = value2
                    .coerce_to(&ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let lin_expr1 = match lin_expr1_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let lin_expr2 = match lin_expr2_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Constraint(vec![lin_expr1.leq(&lin_expr2).into()]).into()
            }
            Expr::ConstraintGe(expr1, expr2) => {
                let value1 = self.eval_expr(ast, env, &*expr1);
                let lin_expr1_value = value1
                    .coerce_to(&ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let lin_expr2_value = value2
                    .coerce_to(&ExprType::LinExpr)
                    .expect("Coercion should be valid");

                let lin_expr1 = match lin_expr1_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };
                let lin_expr2 = match lin_expr2_value {
                    ExprValue::LinExpr(val) => val,
                    _ => panic!("Expected boolean"),
                };

                ExprValue::Constraint(vec![lin_expr1.geq(&lin_expr2).into()]).into()
            }
            Expr::Eq(expr1, expr2) => {
                let value1 = self.eval_expr(ast, env, &*expr1);
                let typ1 = value1.get_type();

                let value2 = self.eval_expr(ast, env, &*expr2);
                let typ2 = value2.get_type();

                let target =
                    AnnotatedType::unify(&typ1, &typ2).expect("It should be possible to unify");
                let target_typ = match target {
                    AnnotatedType::Forced(typ) | AnnotatedType::Regular(typ) => typ,
                    AnnotatedType::UntypedList => {
                        // We have two empty lists, they are equal
                        return ExprValue::Bool(true).into();
                    }
                };

                let coerced_value1 = value1
                    .coerce_to(&target_typ)
                    .expect("Coercion should be valid");
                let coerced_value2 = value2
                    .coerce_to(&target_typ)
                    .expect("Coercion should be valid");
                ExprValue::Bool(coerced_value1 == coerced_value2).into()
            }
            Expr::Ne(expr1, expr2) => {
                let value1 = self.eval_expr(ast, env, &*expr1);
                let typ1 = value1.get_type();

                let value2 = self.eval_expr(ast, env, &*expr2);
                let typ2 = value2.get_type();

                let target =
                    AnnotatedType::unify(&typ1, &typ2).expect("It should be possible to unify");
                let target_typ = match target {
                    AnnotatedType::Forced(typ) | AnnotatedType::Regular(typ) => typ,
                    AnnotatedType::UntypedList => {
                        // We have two empty lists, they are equal
                        return ExprValue::Bool(false).into();
                    }
                };

                let coerced_value1 = value1
                    .coerce_to(&target_typ)
                    .expect("Coercion should be valid");
                let coerced_value2 = value2
                    .coerce_to(&target_typ)
                    .expect("Coercion should be valid");
                ExprValue::Bool(coerced_value1 != coerced_value2).into()
            }
            Expr::Lt(expr1, expr2) => {
                let value1 = self.eval_expr(ast, env, &*expr1);
                let number1_value = value1
                    .coerce_to(&ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let number2_value = value2
                    .coerce_to(&ExprType::Int)
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
                let value1 = self.eval_expr(ast, env, &*expr1);
                let number1_value = value1
                    .coerce_to(&ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let number2_value = value2
                    .coerce_to(&ExprType::Int)
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
                let value1 = self.eval_expr(ast, env, &*expr1);
                let number1_value = value1
                    .coerce_to(&ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let number2_value = value2
                    .coerce_to(&ExprType::Int)
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
                let value1 = self.eval_expr(ast, env, &*expr1);
                let number1_value = value1
                    .coerce_to(&ExprType::Int)
                    .expect("Coercion should be valid");

                let value2 = self.eval_expr(ast, env, &*expr2);
                let number2_value = value2
                    .coerce_to(&ExprType::Int)
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
            _ => todo!("Node not implemented: {:?}", expr),
        }
    }
}
