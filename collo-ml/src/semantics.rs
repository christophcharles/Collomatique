use crate::ast::{Expr, Param, Span, Spanned};
use std::collections::HashMap;

mod string_case;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprType {
    Int,
    Bool,
    LinExpr,
    Constraint,
    List(Box<ExprType>),
    EmptyList,
    Object(String),
}

impl ExprType {
    pub fn is_primitive_type(&self) -> bool {
        match self {
            ExprType::Int | ExprType::Bool | ExprType::LinExpr | ExprType::Constraint => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            ExprType::List(_) => true,
            ExprType::EmptyList => true,
            _ => false,
        }
    }

    /// Checks if type is valid for arithmetic operations
    pub fn is_arithmetic(&self) -> bool {
        matches!(self, ExprType::Int | ExprType::LinExpr)
    }

    /// Checks if self can be coerced to target type.
    /// This is DIRECTIONAL: Int can coerce to LinExpr, but not vice versa.
    pub fn can_coerce_to(&self, target: &ExprType) -> bool {
        match (self, target) {
            // Exact match always works
            (a, b) if a == b => true,

            // Int → LinExpr (but NOT LinExpr → Int)
            (ExprType::Int, ExprType::LinExpr) => true,

            // EmptyList → [T] for any T (but NOT [T] → EmptyList)
            (ExprType::EmptyList, ExprType::List(_)) => true,

            // Recursive: [A] → [B] if A can coerce to B
            (ExprType::List(a), ExprType::List(b)) => a.can_coerce_to(b),

            // Everything else: no coercion
            _ => false,
        }
    }

    /// Finds a common type that both left and right can coerce to.
    /// This is BIDIRECTIONAL and SYMMETRIC.
    pub fn unify(left: &ExprType, right: &ExprType) -> Option<ExprType> {
        match (left, right) {
            // Exact match (including both EmptyList)
            (a, b) if a == b => Some(a.clone()),

            // EmptyList unifies with any List (bidirectional)
            (ExprType::EmptyList, ExprType::List(t)) | (ExprType::List(t), ExprType::EmptyList) => {
                Some(ExprType::List(t.clone()))
            }

            // Int/LinExpr unify to LinExpr (bidirectional)
            (ExprType::Int, ExprType::LinExpr) | (ExprType::LinExpr, ExprType::Int) => {
                Some(ExprType::LinExpr)
            }

            // Lists: unify element types recursively
            (ExprType::List(l), ExprType::List(r)) => {
                Self::unify(l, r).map(|unified| ExprType::List(Box::new(unified)))
            }

            // No unification possible
            _ => None,
        }
    }
}

impl From<crate::ast::TypeName> for ExprType {
    fn from(value: crate::ast::TypeName) -> Self {
        use crate::ast::TypeName;
        match value {
            TypeName::Bool => ExprType::Bool,
            TypeName::Int => ExprType::Int,
            TypeName::LinExpr => ExprType::LinExpr,
            TypeName::Constraint => ExprType::Constraint,
            TypeName::Object(name) => ExprType::Object(name),
            TypeName::List(sub_typ) => ExprType::List(Box::new((*sub_typ).into())),
        }
    }
}

impl std::fmt::Display for ExprType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprType::Bool => write!(f, "Bool"),
            ExprType::Int => write!(f, "Int"),
            ExprType::LinExpr => write!(f, "LinExpr"),
            ExprType::Constraint => write!(f, "Constraint"),
            ExprType::List(sub_type) => write!(f, "[{}]", sub_type.as_ref()),
            ExprType::EmptyList => write!(f, "[<unknown>]"),
            ExprType::Object(typ) => write!(f, "{}", typ),
        }
    }
}

/// Represents a type but an optional forcing
/// A type is *forced* if it cannot be coerced into any other types.
///
/// This is useful for 'as' expressions. Just after the 'as', coercion is
/// prohibited. Coercion is possible after the next operation but forced type can be reintroduced
/// with a new 'as'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeForced {
    Forced(ExprType),
    Regular(ExprType),
}

impl MaybeForced {
    pub fn force(typ: ExprType) -> MaybeForced {
        MaybeForced::Forced(typ)
    }

    pub fn loose(typ: ExprType) -> MaybeForced {
        MaybeForced::Regular(typ)
    }

    pub fn is_forced(&self) -> bool {
        match self {
            MaybeForced::Forced(_) => true,
            MaybeForced::Regular(_) => false,
        }
    }

    pub fn inner(&self) -> &ExprType {
        match self {
            MaybeForced::Forced(typ) => &typ,
            MaybeForced::Regular(typ) => &typ,
        }
    }

    pub fn into_inner(self) -> ExprType {
        match self {
            MaybeForced::Forced(typ) => typ,
            MaybeForced::Regular(typ) => typ,
        }
    }

    pub fn can_coerce_to(&self, target: &ExprType) -> bool {
        match self {
            MaybeForced::Forced(typ) => typ == target,
            MaybeForced::Regular(typ) => typ.can_coerce_to(target),
        }
    }

    pub fn unify(left: &MaybeForced, right: &MaybeForced) -> Option<MaybeForced> {
        match (left, right) {
            (MaybeForced::Forced(a), MaybeForced::Forced(b)) => {
                if a == b {
                    Some(MaybeForced::Forced(a.clone()))
                } else {
                    None
                }
            }
            (MaybeForced::Forced(a), MaybeForced::Regular(b)) => {
                if b.can_coerce_to(a) {
                    Some(MaybeForced::Forced(a.clone()))
                } else {
                    None
                }
            }
            (MaybeForced::Regular(a), MaybeForced::Forced(b)) => {
                if a.can_coerce_to(b) {
                    Some(MaybeForced::Forced(b.clone()))
                } else {
                    None
                }
            }
            (MaybeForced::Regular(a), MaybeForced::Regular(b)) => {
                ExprType::unify(a, b).map(MaybeForced::Forced)
            }
        }
    }

    pub fn loosen(&self) -> MaybeForced {
        MaybeForced::Regular(self.inner().clone())
    }
}

impl From<ExprType> for MaybeForced {
    fn from(value: ExprType) -> Self {
        MaybeForced::Regular(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    public: bool,
    args: ArgsType,
    output: ExprType,
}

impl std::fmt::Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args_types: Vec<_> = self.args.iter().map(|x| x.to_string()).collect();
        write!(f, "({}) -> {}", args_types.join(", "), self.output)
    }
}

pub type ArgsType = Vec<ExprType>;

pub type ObjectFields = HashMap<String, ExprType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEnv {
    defined_types: HashMap<String, ObjectFields>,
    functions: HashMap<String, (FunctionType, Span, bool)>,
    variables: HashMap<String, (ArgsType, Option<(Span, bool)>)>,
    variable_lists: HashMap<String, (ArgsType, Span, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeInfo {
    types: HashMap<crate::ast::Span, GenericType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericType {
    Function(FunctionType),
    Variable(ArgsType),
    Expr(ExprType),
}

impl From<FunctionType> for GenericType {
    fn from(value: FunctionType) -> Self {
        GenericType::Function(value)
    }
}

impl From<ExprType> for GenericType {
    fn from(value: ExprType) -> Self {
        GenericType::Expr(value)
    }
}

impl From<ArgsType> for GenericType {
    fn from(value: ArgsType) -> Self {
        GenericType::Variable(value)
    }
}

impl std::fmt::Display for GenericType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericType::Function(func) => write!(f, "{}", func),
            GenericType::Expr(typ) => write!(f, "{}", typ),
            GenericType::Variable(var_args) => {
                let args_types: Vec<_> = var_args.iter().map(|x| x.to_string()).collect();
                write!(f, "$({})", args_types.join(", "))
            }
        }
    }
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GlobalEnvError {
    #[error("Field {field} of object type {object_type} has unknown type {unknown_type}")]
    UnknownTypeInField {
        object_type: String,
        field: String,
        unknown_type: String,
    },
    #[error("Parameter number {param} for ILP variable {var} has unknown type {unknown_type}")]
    UnknownTypeForVariableArg {
        var: String,
        param: usize,
        unknown_type: String,
    },
}

impl GlobalEnv {
    fn validate_type(&self, typ: &ExprType) -> bool {
        match typ {
            ExprType::Bool => true,
            ExprType::Int => true,
            ExprType::LinExpr => true,
            ExprType::Constraint => true,
            ExprType::EmptyList => false, // Emptylist is used internally but is not a valid type
            ExprType::List(sub_typ) => self.validate_type(sub_typ.as_ref()),
            ExprType::Object(typ_name) => self.defined_types.contains_key(typ_name),
        }
    }

    fn lookup_fn(&mut self, name: &str) -> Option<(FunctionType, Span)> {
        let (fn_typ, span, used) = self.functions.get_mut(name)?;
        *used = true;
        Some((fn_typ.clone(), span.clone()))
    }

    fn register_fn(
        &mut self,
        name: &str,
        fn_typ: FunctionType,
        span: Span,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.functions.contains_key(name));

        self.functions.insert(
            name.to_string(),
            (
                fn_typ.clone(),
                span.clone(),
                should_be_used_by_default(name),
            ),
        );

        type_info.types.insert(span, fn_typ.into());
    }

    fn lookup_var(&mut self, name: &str) -> Option<(ArgsType, Option<Span>)> {
        let (args_typ, span_and_used_opt) = self.variables.get_mut(name)?;

        if let Some((_span, used)) = span_and_used_opt {
            *used = true;
        }

        Some((
            args_typ.clone(),
            span_and_used_opt.as_ref().map(|x| x.0.clone()),
        ))
    }

    fn register_var(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.variables.contains_key(name));

        self.variables.insert(
            name.to_string(),
            (
                args_typ.clone(),
                Some((span.clone(), should_be_used_by_default(name))),
            ),
        );

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_var_list(&mut self, name: &str) -> Option<(ArgsType, Span)> {
        let (args_typ, span, used) = self.variable_lists.get_mut(name)?;

        *used = true;

        Some((args_typ.clone(), span.clone()))
    }

    fn register_var_list(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.variable_lists.contains_key(name));

        self.variable_lists.insert(
            name.to_string(),
            (
                args_typ.clone(),
                span.clone(),
                should_be_used_by_default(name),
            ),
        );

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_field(&self, obj_type: &str, field: &str) -> Option<ExprType> {
        self.defined_types.get(obj_type)?.get(field).cloned()
    }
}

#[derive(Debug, Error)]
pub enum SemError {
    #[error("Unknown identifier \"{identifier}\" at {span:?}")]
    UnknownIdentifer { identifier: String, span: Span },
    #[error("Unknown variable \"{var}\" at {span:?}")]
    UnknownVariable { var: String, span: Span },
    #[error("Function type mismatch: \"{identifier}\" at {span:?} has type {found} but type {expected} expected.")]
    FunctionTypeMismatch {
        identifier: String,
        span: Span,
        expected: FunctionType,
        found: FunctionType,
    },
    #[error("Variable \"{identifier}\" at {span:?} is already defined ({here:?})")]
    VariableAlreadyDefined {
        identifier: String,
        span: Span,
        here: Option<Span>,
    },
    #[error("Function \"{identifier}\" at {span:?} is already defined ({here:?})")]
    FunctionAlreadyDefined {
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Type {typ} at {span:?} is unknown")]
    UnknownType { typ: String, span: Span },
    #[error("Type {typ} at {span:?} is a list and is disallowed in global collections")]
    ListNotAllowedInGlobalCollections { typ: String, span: Span },
    #[error("Type {typ} at {span:?} is a primitive type and is disallowed in global collections")]
    PrimitiveTypeNotAllowedInGlobalCollections { typ: String, span: Span },
    #[error("Parameter \"{identifier}\" is already defined ({here:?}).")]
    ParameterAlreadyDefined {
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Body type mismatch: body for function {func} at {span:?} has type {found} but type {expected} expected.")]
    BodyTypeMismatch {
        func: String,
        span: Span,
        expected: ExprType,
        found: ExprType,
    },
    #[error("Type mismatch at {span:?}: expected {expected} but found {found} ({context})")]
    TypeMismatch {
        span: Span,
        expected: ExprType,
        found: ExprType,
        context: String,
    },
    #[error("Argument count mismatch for \"{identifier}\" at {span:?}: expected {expected} arguments but found {found}")]
    ArgumentCountMismatch {
        identifier: String,
        span: Span,
        expected: usize,
        found: usize,
    },
    #[error("Unknown field \"{field}\" on type {object_type} at {span:?}")]
    UnknownField {
        object_type: String,
        field: String,
        span: Span,
    },
    #[error("Cannot access field \"{field}\" on non-object type {typ} at {span:?}")]
    FieldAccessOnNonObject {
        typ: ExprType,
        field: String,
        span: Span,
    },
}

#[derive(Debug, Error)]
pub enum SemWarning {
    #[error("Identifier \"{identifier}\" at {span:?} shadows previous definition at {previous:?}")]
    IdentifierShadowed {
        identifier: String,
        span: Span,
        previous: Span,
    },

    #[error(
        "Function \"{identifier}\" at {span:?} should use snake_case (consider \"{suggestion}\")"
    )]
    FunctionNamingConvention {
        identifier: String,
        span: Span,
        suggestion: String,
    },

    #[error(
        "Variable \"{identifier}\" at {span:?} should use PascalCase (consider \"{suggestion}\")"
    )]
    VariableNamingConvention {
        identifier: String,
        span: Span,
        suggestion: String,
    },

    #[error(
        "Parameter \"{identifier}\" at {span:?} should use snake_case (consider \"{suggestion}\")"
    )]
    ParameterNamingConvention {
        identifier: String,
        span: Span,
        suggestion: String,
    },
    #[error("Unused identifier \"{identifier}\" at {span:?}")]
    UnusedIdentifier { identifier: String, span: Span },
    #[error("Unused function \"{identifier}\" at {span:?}")]
    UnusedFunction { identifier: String, span: Span },
    #[error("Unused variable \"{identifier}\" at {span:?}")]
    UnusedVariable { identifier: String, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct LocalEnv {
    scopes: Vec<HashMap<String, (ExprType, Span, bool)>>,
    pending_scope: HashMap<String, (ExprType, Span, bool)>,
}

fn should_be_used_by_default(ident: &str) -> bool {
    assert!(ident.len() > 0);

    ident.chars().next().unwrap() == '_'
}

impl LocalEnv {
    fn new() -> Self {
        LocalEnv::default()
    }

    fn lookup_in_pending_scope(&self, ident: &str) -> Option<(ExprType, Span)> {
        self.pending_scope
            .get(ident)
            .map(|(typ, span, _used)| (typ.clone(), span.clone()))
    }

    fn lookup_ident(&mut self, ident: &str) -> Option<(ExprType, Span)> {
        // We don't look in pending scope as these variables are not yet accessible
        for scope in self.scopes.iter_mut().rev() {
            let Some((typ, span, used)) = scope.get_mut(ident) else {
                continue;
            };
            *used = true;
            return Some((typ.clone(), span.clone()));
        }
        None
    }

    fn push_scope(&mut self) {
        let mut old_scope = HashMap::new();
        std::mem::swap(&mut old_scope, &mut self.pending_scope);

        self.scopes.push(old_scope);
    }

    fn pop_scope(&mut self, warnings: &mut Vec<SemWarning>) {
        assert!(!self.scopes.is_empty());

        self.pending_scope = self.scopes.pop().unwrap();

        for (name, (_typ, span, used)) in &self.pending_scope {
            if !*used {
                warnings.push(SemWarning::UnusedIdentifier {
                    identifier: name.clone(),
                    span: span.clone(),
                });
            }
        }

        self.pending_scope.clear();
    }

    fn register_identifier(
        &mut self,
        ident: &str,
        span: Span,
        typ: ExprType,
        type_info: &mut TypeInfo,
        warnings: &mut Vec<SemWarning>,
    ) {
        assert!(!self.pending_scope.contains_key(ident));

        if let Some((_typ, old_span)) = self.lookup_ident(ident) {
            warnings.push(SemWarning::IdentifierShadowed {
                identifier: ident.to_string(),
                span: span.clone(),
                previous: old_span,
            });
        }

        self.pending_scope.insert(
            ident.to_string(),
            (typ.clone(), span.clone(), should_be_used_by_default(ident)),
        );
        type_info.types.insert(span, typ.into());
    }

    fn check_expr(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &crate::ast::Expr,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<MaybeForced> {
        use crate::ast::Expr;

        match expr {
            // ========== Literals ==========
            Expr::Number(_) => Some(ExprType::Int.into()),
            Expr::Boolean(_) => Some(ExprType::Bool.into()),

            Expr::Ident(ident) => self
                .check_ident(
                    global_env,
                    &ident.node,
                    &ident.span,
                    type_info,
                    errors,
                    warnings,
                )
                .map(|x| x.into()),
            Expr::Path { object, segments } => self
                .check_path(global_env, &object, segments, type_info, errors, warnings)
                .map(|x| x.into()),

            // ========== As construct ==========
            Expr::ExplicitType { expr, typ } => {
                // Check the inner expression
                let expr_type =
                    self.check_expr(global_env, &expr.node, type_info, errors, warnings);
                // 'as' is forcing explicitly a coercion. So we don't care if we are
                // getting a forced type.
                //
                // This means that chained 'as' are valid (even if a bit odd):
                // "5 as Int as LinExpr"
                let loose_type = expr_type.map(|x| x.into_inner());

                // Convert the declared type
                let target_type = ExprType::from(typ.node.clone());

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return loose_type.map(MaybeForced::Forced); // Fallback to inferred type
                }

                match loose_type {
                    Some(inferred) => {
                        // Check if the inferred type can coerce to the target type
                        if inferred.can_coerce_to(&target_type) {
                            // Success: use the target type
                            Some(MaybeForced::force(target_type))
                        } else {
                            // Error: can't coerce
                            errors.push(SemError::TypeMismatch {
                                span: expr.span.clone(),
                                expected: target_type.clone(),
                                found: inferred,
                                context: "explicit type annotation".to_string(),
                            });
                            // Return target type anyway (user's intent is clear)
                            Some(MaybeForced::force(target_type))
                        }
                    }
                    None => {
                        // Expression failed to type-check, but we have explicit type
                        // Use the explicit type as a hint for recovery
                        Some(MaybeForced::force(target_type))
                    }
                }
            }

            // ========== Arithmetic Operations ==========
            // Int + Int -> Int
            // LinExpr + Int -> LinExpr (coerce Int to LinExpr)
            // Int + LinExpr -> LinExpr (coerce Int to LinExpr)
            // LinExpr + LinExpr -> LinExpr
            Expr::Add(left, right) | Expr::Sub(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type.clone(), right_type) {
                    (Some(l), Some(r)) => match MaybeForced::unify(&l, &r) {
                        Some(unified) if unified.inner().is_arithmetic() => Some(unified.loosen()),
                        _ => {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: l.inner().clone(),
                                found: r.inner().clone(),
                                context: format!(
                                    "addition/subtraction requires Int or LinExpr, got {} and {}",
                                    l.inner(),
                                    r.inner()
                                ),
                            });
                            None
                        }
                    },
                    (Some(t), None) | (None, Some(t)) if t.inner().is_arithmetic() => Some(t),
                    (Some(t), None) | (None, Some(t)) => {
                        let span = if left_type.is_some() {
                            left.span.clone()
                        } else {
                            right.span.clone()
                        };
                        errors.push(SemError::TypeMismatch {
                            span,
                            expected: ExprType::Int,
                            found: t.inner().clone(),
                            context: "addition/subtraction requires Int or LinExpr".to_string(),
                        });
                        None
                    }
                    (None, None) => None,
                }
            }
            // Multiplication: Int * Int -> Int, Int * LinExpr -> LinExpr, LinExpr * Int -> LinExpr
            // But NOT LinExpr * LinExpr (non-linear!)
            Expr::Mul(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type.clone(), right_type) {
                    (Some(l), Some(r)) => {
                        // Special case: LinExpr * LinExpr is non-linear
                        if *l.inner() == ExprType::LinExpr && *r.inner() == ExprType::LinExpr {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::Int,
                                found: ExprType::LinExpr,
                                context: "cannot multiply two linear expressions (non-linear)"
                                    .to_string(),
                            });
                            return Some(ExprType::LinExpr.into()); // Fallback
                        }

                        // Try to unify (handles Int * Int, Int * LinExpr, LinExpr * Int)
                        match MaybeForced::unify(&l, &r) {
                            Some(unified) if unified.inner().is_arithmetic() => {
                                Some(unified.loosen())
                            }
                            _ => {
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.inner().clone(),
                                    found: r.inner().clone(),
                                    context: format!(
                                        "multiplication requires Int or LinExpr, got {} and {}",
                                        l.inner(),
                                        r.inner()
                                    ),
                                });
                                if l.inner().is_arithmetic() {
                                    Some(l)
                                } else {
                                    Some(ExprType::Int.into())
                                }
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) if t.inner().is_arithmetic() => Some(t),
                    (Some(t), None) | (None, Some(t)) => {
                        let span = if left_type.is_some() {
                            left.span.clone()
                        } else {
                            right.span.clone()
                        };
                        errors.push(SemError::TypeMismatch {
                            span,
                            expected: ExprType::Int,
                            found: t.inner().clone(),
                            context: "multiplication requires Int or LinExpr".to_string(),
                        });
                        Some(t) // Fallback
                    }
                    (None, None) => None,
                }
            }
            // Division and modulo: Int // Int -> Int, Int % Int -> Int
            // These are NOT allowed on LinExpr
            Expr::Div(left, right) | Expr::Mod(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both can coerce to Int
                        let l_ok = l.can_coerce_to(&ExprType::Int);
                        let r_ok = r.can_coerce_to(&ExprType::Int);

                        if !l_ok {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::Int,
                                found: l.inner().clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                        }
                        if !r_ok {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::Int,
                                found: r.inner().clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                        }

                        if l_ok || r_ok {
                            Some(ExprType::Int.into())
                        } else {
                            None
                        }
                    }
                    (Some(t), None) => {
                        if !t.can_coerce_to(&ExprType::Int) {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::Int,
                                found: t.inner().clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                            None
                        } else {
                            Some(ExprType::Int.into())
                        }
                    }
                    (None, Some(t)) => {
                        if !t.can_coerce_to(&ExprType::Int) {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::Int,
                                found: t.inner().clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                            None
                        } else {
                            Some(ExprType::Int.into())
                        }
                    }
                    (None, None) => None,
                }
            }

            // ========== Constraints operators ==========
            Expr::ConstraintEq(left, right)
            | Expr::ConstraintLe(left, right)
            | Expr::ConstraintGe(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both can coerce to LinExpr
                        let l_ok = l.can_coerce_to(&ExprType::LinExpr);
                        let r_ok = r.can_coerce_to(&ExprType::LinExpr);

                        if !l_ok {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::LinExpr,
                                found: l.into_inner(),
                                context: "constraint operator requires LinExpr or Int operands"
                                    .to_string(),
                            });
                        }
                        if !r_ok {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::LinExpr,
                                found: r.into_inner(),
                                context: "constraint operator requires LinExpr or Int operands"
                                    .to_string(),
                            });
                        }

                        // Always return Constraint (even on error, intent is clear)
                        Some(ExprType::Constraint.into())
                    }
                    _ => {
                        // Something failed, but we know user wanted a constraint
                        Some(ExprType::Constraint.into())
                    }
                }
            }

            // ========== Comparison Operations ==========
            // Int == Int -> Bool
            // LinExpr == LinExpr -> Constraint
            // LinExpr == Int -> Constraint (coerce Int to LinExpr)
            Expr::Eq(left, right) | Expr::Ne(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Try to unify the types
                        match MaybeForced::unify(&l, &r) {
                            Some(_unified) => Some(ExprType::Bool.into()),
                            None => {
                                // Types don't unify - incompatible
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.inner().clone(),
                                    found: r.into_inner(),
                                    context: "equality comparison requires compatible types"
                                        .to_string(),
                                });
                                Some(ExprType::Bool.into()) // Fallback
                            }
                        }
                    }
                    (Some(_), None) | (None, Some(_)) => Some(ExprType::Bool.into()), // One side failed
                    (None, None) => None,
                }
            }

            // Relational: Int < Int -> Bool
            Expr::Le(left, right)
            | Expr::Ge(left, right)
            | Expr::Lt(left, right)
            | Expr::Gt(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both can coerce to Int
                        let l_ok = l.can_coerce_to(&ExprType::Int);
                        let r_ok = r.can_coerce_to(&ExprType::Int);

                        if !l_ok {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::Int,
                                found: l.into_inner(),
                                context: "relational comparison requires Int operands".to_string(),
                            });
                        }
                        if !r_ok {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::Int,
                                found: r.into_inner(),
                                context: "relational comparison requires Int operands".to_string(),
                            });
                        }

                        // Always return Bool (even on error, intent is clear)
                        Some(ExprType::Bool.into())
                    }
                    (Some(_), None) | (None, Some(_)) => Some(ExprType::Bool.into()),
                    (None, None) => None,
                }
            }

            // ========== Boolean Operations ==========
            // Bool and Bool -> Bool, Constraint and Constraint -> Constraint
            Expr::And(left, right) | Expr::Or(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Try to unify the types
                        match MaybeForced::unify(&l, &r) {
                            Some(t) if *t.inner() == ExprType::Bool => Some(ExprType::Bool.into()),
                            Some(t) if *t.inner() == ExprType::Constraint => {
                                Some(ExprType::Constraint.into())
                            }
                            Some(unified) => {
                                // Unified to something else - not valid for and/or
                                errors.push(SemError::TypeMismatch {
                                    span: left.span.clone(),
                                    expected: ExprType::Bool,
                                    found: unified.into_inner(),
                                    context: "and/or requires Bool or Constraint operands"
                                        .to_string(),
                                });
                                None
                            }
                            None => {
                                // Can't unify - incompatible types
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.inner().clone(),
                                    found: r.into_inner(),
                                    context: "and/or requires both operands to have the same type (both Bool or both Constraint)".to_string(),
                                });
                                // Return whatever the left side was if valid
                                if *l.inner() == ExprType::Bool
                                    || *l.inner() == ExprType::Constraint
                                {
                                    Some(l)
                                } else {
                                    None
                                }
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) if *t.inner() == ExprType::Bool => {
                        Some(ExprType::Bool.into())
                    }
                    (Some(t), None) | (None, Some(t)) if *t.inner() == ExprType::Constraint => {
                        Some(ExprType::Constraint.into())
                    }
                    (Some(t), None) | (None, Some(t)) => {
                        // One side is not Bool/Constraint
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Bool,
                            found: t.inner().clone(),
                            context: "and/or requires Bool or Constraint operands".to_string(),
                        });
                        None
                    }
                    (None, None) => None,
                }
            }

            Expr::Not(expr) => {
                let expr_type =
                    self.check_expr(global_env, &expr.node, type_info, errors, warnings);

                match expr_type {
                    Some(typ) if typ.can_coerce_to(&ExprType::Bool) => Some(ExprType::Bool.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: ExprType::Bool,
                            found: typ.inner().clone(),
                            context: "not requires Bool operand".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            // ========== Membership Test ==========
            // x in collection -> Bool
            Expr::In { item, collection } => {
                let item_type =
                    self.check_expr(global_env, &item.node, type_info, errors, warnings);
                let coll_type =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);
                // We don't coerce elements from the list, so we can drop the MaybeForced
                let inner_coll_type = coll_type.map(|x| x.into_inner());

                match inner_coll_type {
                    Some(ExprType::List(elem_t)) => {
                        if let Some(item_t) = item_type {
                            // Check if item can coerce to the element type
                            if !item_t.can_coerce_to(&elem_t) {
                                errors.push(SemError::TypeMismatch {
                                    span: item.span.clone(),
                                    expected: *elem_t,
                                    found: item_t.into_inner(),
                                    context: "item type must match collection element type"
                                        .to_string(),
                                });
                            }
                        }
                    }
                    Some(ExprType::EmptyList) => {
                        // Can't check anything - we don't know the element type
                        // But it does not matter, an element is never in an empty collection
                        // So this will be false and the types don't matter.
                    }
                    Some(t) => {
                        // Not a list at all
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                            found: t,
                            context: "membership test requires a list".to_string(),
                        });
                    }
                    None => {
                        // Collection failed to type-check
                    }
                }

                // Always returns Bool
                Some(ExprType::Bool.into())
            }

            // ========== Quantifiers ==========
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let coll_type =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);
                // We don't coerce elements from the list, so we can drop the MaybeForced
                let inner_coll_type = coll_type.map(|x| x.into_inner());

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        identifier: var.node.clone(),
                        span: var.span.clone(),
                        suggestion,
                    });
                }

                // Extract element type from collection
                match inner_coll_type {
                    Some(ExprType::List(elem_t)) => {
                        // Register the loop variable with the element type
                        self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            *elem_t,
                            type_info,
                            warnings,
                        );
                    }
                    Some(ExprType::EmptyList) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                            found: ExprType::EmptyList,
                            context: "forall collection must have a known type (use 'as' for explicit typing)".to_string(),
                        });
                        return Some(ExprType::Constraint.into()); // Return early
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(t.clone())), // placeholder
                            found: t,
                            context: "forall collection must be a list".to_string(),
                        });
                        return Some(ExprType::Constraint.into()); // Return early
                    }
                    None => return None,
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type =
                        self.check_expr(global_env, &filter_expr.node, type_info, errors, warnings);

                    if let Some(typ) = filter_type {
                        if !typ.can_coerce_to(&ExprType::Bool) {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: ExprType::Bool,
                                found: typ.into_inner(),
                                context: "forall filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be Constraint or Bool)
                let body_type =
                    self.check_expr(global_env, &body.node, type_info, errors, warnings);

                self.pop_scope(warnings);

                match body_type {
                    Some(typ) if typ.can_coerce_to(&ExprType::Constraint) => {
                        Some(ExprType::Constraint.into())
                    }
                    Some(typ) if typ.can_coerce_to(&ExprType::Bool) => Some(ExprType::Bool.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: ExprType::Constraint,
                            found: typ.into_inner(),
                            context: "forall body must be Constraint or Bool".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                let coll_type =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);
                // We don't coerce elements from the list, so we can drop the MaybeForced
                let inner_coll_type = coll_type.map(|x| x.into_inner());

                // Check naming convention
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &var.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        identifier: var.node.clone(),
                        span: var.span.clone(),
                        suggestion,
                    });
                }

                // Extract element type from collection
                match inner_coll_type {
                    Some(ExprType::List(elem_t)) => {
                        // Register the loop variable with the element type
                        self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            *elem_t,
                            type_info,
                            warnings,
                        );
                    }
                    Some(ExprType::EmptyList) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                            found: ExprType::EmptyList,
                            context: "sum collection must have a known type (use 'as' for explicit typing)".to_string(),
                        });
                        return None; // Return early
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(t.clone())), // placeholder
                            found: t,
                            context: "sum collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None,
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type =
                        self.check_expr(global_env, &filter_expr.node, type_info, errors, warnings);

                    if let Some(typ) = filter_type {
                        if !typ.can_coerce_to(&ExprType::Bool) {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: ExprType::Bool,
                                found: typ.into_inner(),
                                context: "sum filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be arithmetic: Int or LinExpr)
                let body_type =
                    self.check_expr(global_env, &body.node, type_info, errors, warnings);

                self.pop_scope(warnings);

                match body_type {
                    Some(typ) if typ.inner().is_arithmetic() => Some(typ), // Return Int or LinExpr
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: ExprType::Int,
                            found: typ.into_inner(),
                            context: "sum body must be Int or LinExpr".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            // ========== If Expression ==========
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_type =
                    self.check_expr(global_env, &condition.node, type_info, errors, warnings);

                if let Some(typ) = cond_type {
                    if !typ.can_coerce_to(&ExprType::Bool) {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: ExprType::Bool,
                            found: typ.into_inner(),
                            context: "if condition must be Bool".to_string(),
                        });
                    }
                }

                let then_type =
                    self.check_expr(global_env, &then_expr.node, type_info, errors, warnings);
                let else_type =
                    self.check_expr(global_env, &else_expr.node, type_info, errors, warnings);

                match (then_type, else_type) {
                    (Some(t), Some(e)) => {
                        match MaybeForced::unify(&t, &e) {
                            Some(unified) => Some(unified.loosen()),
                            None => {
                                errors.push(SemError::TypeMismatch {
                                    span: else_expr.span.clone(),
                                    expected: t.inner().clone(),
                                    found: e.into_inner(),
                                    context: "if branches must have compatible types".to_string(),
                                });
                                Some(t) // Fallback to then type
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
            }

            // ========== ILP Variables ==========
            Expr::VarCall { name, args } => {
                match global_env.lookup_var(&name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(ExprType::LinExpr.into()) // Syntax indicates LinExpr intent
                    }
                    Some((var_args, _)) => {
                        if args.len() != var_args.len() {
                            errors.push(SemError::ArgumentCountMismatch {
                                identifier: name.node.clone(),
                                span: args
                                    .last()
                                    .map(|a| a.span.clone())
                                    .unwrap_or_else(|| name.span.clone()),
                                expected: var_args.len(),
                                found: args.len(),
                            });
                        }

                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type =
                                self.check_expr(global_env, &arg.node, type_info, errors, warnings);

                            if let Some(found_type) = arg_type {
                                if !found_type.can_coerce_to(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone(),
                                        found: found_type.into_inner(),
                                        context: format!(
                                            "argument {} to variable ${}",
                                            i + 1,
                                            name.node
                                        ),
                                    });
                                }
                            }
                        }

                        Some(ExprType::LinExpr.into())
                    }
                }
            }

            Expr::VarListCall { name, args } => {
                match global_env.lookup_var_list(&name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(ExprType::List(Box::new(ExprType::LinExpr)).into())
                        // Syntax indicates [LinExpr] intent
                    }
                    Some((var_args, _)) => {
                        if args.len() != var_args.len() {
                            errors.push(SemError::ArgumentCountMismatch {
                                identifier: name.node.clone(),
                                span: args
                                    .last()
                                    .map(|a| a.span.clone())
                                    .unwrap_or_else(|| name.span.clone()),
                                expected: var_args.len(),
                                found: args.len(),
                            });
                        }

                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type =
                                self.check_expr(global_env, &arg.node, type_info, errors, warnings);

                            if let Some(found_type) = arg_type {
                                if !found_type.can_coerce_to(expected_type) {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone(),
                                        found: found_type.into_inner(),
                                        context: format!(
                                            "argument {} to variable ${}",
                                            i + 1,
                                            name.node
                                        ),
                                    });
                                }
                            }
                        }

                        Some(ExprType::List(Box::new(ExprType::LinExpr)).into())
                    }
                }
            }

            // ========== Function Calls ==========
            Expr::FnCall { name, args } => match global_env.lookup_fn(&name.node) {
                None => {
                    errors.push(SemError::UnknownIdentifer {
                        identifier: name.node.clone(),
                        span: name.span.clone(),
                    });
                    None
                }
                Some((fn_type, _)) => {
                    if args.len() != fn_type.args.len() {
                        errors.push(SemError::ArgumentCountMismatch {
                            identifier: name.node.clone(),
                            span: args
                                .last()
                                .map(|a| a.span.clone())
                                .unwrap_or_else(|| name.span.clone()),
                            expected: fn_type.args.len(),
                            found: args.len(),
                        });
                    }

                    for (i, (arg, expected_type)) in
                        args.iter().zip(fn_type.args.iter()).enumerate()
                    {
                        let arg_type =
                            self.check_expr(global_env, &arg.node, type_info, errors, warnings);

                        if let Some(found_type) = arg_type {
                            if !found_type.can_coerce_to(expected_type) {
                                errors.push(SemError::TypeMismatch {
                                    span: arg.span.clone(),
                                    expected: expected_type.clone(),
                                    found: found_type.into_inner(),
                                    context: format!(
                                        "argument {} to function {}",
                                        i + 1,
                                        name.node
                                    ),
                                });
                            }
                        }
                    }

                    Some(fn_type.output.into())
                }
            },

            // ========== Collections ==========
            Expr::GlobalList(type_name) => {
                let typ = ExprType::from(type_name.node.clone());
                if !global_env.validate_type(&typ) {
                    errors.push(SemError::UnknownType {
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else if typ.is_primitive_type() {
                    errors.push(SemError::PrimitiveTypeNotAllowedInGlobalCollections {
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else if typ.is_list() {
                    errors.push(SemError::ListNotAllowedInGlobalCollections {
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else {
                    Some(ExprType::List(Box::new(typ)).into())
                }
            }

            Expr::Union(left, right) | Expr::Inter(left, right) | Expr::Diff(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Try to unify - handles List, EmptyList, and coercion automatically
                        match MaybeForced::unify(&l, &r) {
                            Some(unified) if unified.inner().is_list() => Some(unified.loosen()),
                            Some(non_list) => {
                                // Unified but not to a list type
                                errors.push(SemError::TypeMismatch {
                                    span: left.span.clone(),
                                    expected: ExprType::List(Box::new(non_list.into_inner())), // placeholder
                                    found: l.into_inner(),
                                    context: "collection operation requires List types".to_string(),
                                });
                                None
                            }
                            None => {
                                // Can't unify - incompatible types
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: l.inner().clone(),
                                    found: r.into_inner(),
                                    context:
                                        "collection operation requires compatible element types"
                                            .to_string(),
                                });
                                // Return left type as fallback if it's a list
                                if l.inner().is_list() {
                                    Some(l)
                                } else {
                                    None
                                }
                            }
                        }
                    }
                    (Some(t), None) | (None, Some(t)) if t.inner().is_list() => Some(t),
                    (Some(t), None) | (None, Some(t)) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::List(Box::new(t.inner().clone())), // placeholder
                            found: t.into_inner(),
                            context: "collection operation requires List types".to_string(),
                        });
                        None
                    }
                    (None, None) => None,
                }
            }

            Expr::ListLiteral { elements } => {
                if elements.is_empty() {
                    return Some(ExprType::EmptyList.into());
                }

                // Check all elements and unify their types
                let mut unified_type =
                    self.check_expr(global_env, &elements[0].node, type_info, errors, warnings);

                for item in &elements[1..] {
                    let item_type =
                        self.check_expr(global_env, &item.node, type_info, errors, warnings);

                    match (unified_type.clone(), item_type) {
                        (Some(u), Some(i)) => {
                            match MaybeForced::unify(&u, &i) {
                                Some(new_unified) => unified_type = Some(new_unified),
                                None => {
                                    errors.push(SemError::TypeMismatch {
                                        span: item.span.clone(),
                                        expected: u.inner().clone(),
                                        found: i.into_inner(),
                                        context: "all list elements must have compatible types"
                                            .to_string(),
                                    });
                                    // Keep the current unified type for further checking
                                }
                            }
                        }
                        (Some(u), None) => {
                            // Item failed to type-check, keep unified type
                            unified_type = Some(u);
                        }
                        (None, Some(i)) => {
                            // First element failed, use this item's type
                            unified_type = Some(i);
                        }
                        (None, None) => {
                            // Both failed
                            unified_type = None;
                        }
                    }
                }

                unified_type.map(|t| ExprType::List(Box::new(t.into_inner())).into())
            }

            Expr::ListRange { start, end } => {
                let start_type =
                    self.check_expr(global_env, &start.node, type_info, errors, warnings);
                let end_type = self.check_expr(global_env, &end.node, type_info, errors, warnings);

                match (start_type, end_type) {
                    (Some(s), Some(e)) => {
                        // Check if both can coerce to Int
                        let s_ok = s.can_coerce_to(&ExprType::Int);
                        let e_ok = e.can_coerce_to(&ExprType::Int);
                        println!("{:?}", e);

                        if !s_ok {
                            errors.push(SemError::TypeMismatch {
                                span: start.span.clone(),
                                expected: ExprType::Int,
                                found: s.into_inner(),
                                context: "list range requires Int limits".to_string(),
                            });
                        }
                        if !e_ok {
                            errors.push(SemError::TypeMismatch {
                                span: end.span.clone(),
                                expected: ExprType::Int,
                                found: e.into_inner(),
                                context: "list range requires Int limits".to_string(),
                            });
                        }

                        // Always return [Int] (even on error, intent is clear)
                        Some(ExprType::List(Box::new(ExprType::Int)).into())
                    }
                    (Some(_), None) | (None, Some(_)) => {
                        Some(ExprType::List(Box::new(ExprType::Int)).into())
                    }
                    (None, None) => None,
                }
            }

            Expr::ListComprehension {
                expr,
                vars_and_collections,
                filter,
            } => {
                let mut typ_error = false;
                for (var, collection) in vars_and_collections {
                    let coll_type =
                        self.check_expr(global_env, &collection.node, type_info, errors, warnings);
                    // We don't coerce elements from the list, so we can drop the MaybeForced
                    let inner_coll_type = coll_type.map(|x| x.into_inner());

                    // Check naming convention
                    if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                        &var.node,
                        string_case::NamingConvention::SnakeCase,
                    ) {
                        warnings.push(SemWarning::ParameterNamingConvention {
                            identifier: var.node.clone(),
                            span: var.span.clone(),
                            suggestion,
                        });
                    }

                    // Extract element type from collection
                    match inner_coll_type {
                        Some(ExprType::List(elem_t)) => {
                            // Register the loop variable with the element type
                            self.register_identifier(
                                &var.node,
                                var.span.clone(),
                                *elem_t,
                                type_info,
                                warnings,
                            );
                        }
                        Some(ExprType::EmptyList) => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                                found: ExprType::EmptyList,
                                context: "list comprehension collection must have a known type (use 'as' for explicit typing)".to_string(),
                            });
                            typ_error = true; // Can't infer result type
                        }
                        Some(t) => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: ExprType::List(Box::new(t.clone())), // placeholder
                                found: t,
                                context: "list comprehension collection must be a list".to_string(),
                            });
                            typ_error = true; // Can't infer result type
                        }
                        None => typ_error = true, // Can't infer result type
                    }

                    self.push_scope();
                }

                let elem_type = if !typ_error {
                    // Check filter (must be Bool)
                    if let Some(filter_expr) = filter {
                        let filter_type = self.check_expr(
                            global_env,
                            &filter_expr.node,
                            type_info,
                            errors,
                            warnings,
                        );

                        if let Some(typ) = filter_type {
                            if !typ.can_coerce_to(&ExprType::Bool) {
                                errors.push(SemError::TypeMismatch {
                                    span: filter_expr.span.clone(),
                                    expected: ExprType::Bool,
                                    found: typ.into_inner(),
                                    context: "list comprehension filter must be Bool".to_string(),
                                });
                            }
                        }
                    }

                    // Check the output expression - this determines the result element type
                    self.check_expr(global_env, &expr.node, type_info, errors, warnings)
                } else {
                    None
                };

                for (_var, _collection) in vars_and_collections {
                    self.pop_scope(warnings);
                }

                elem_type.map(|t| ExprType::List(Box::new(t.into_inner())).into())
            }

            // ========== Cardinality ==========
            Expr::Cardinality(collection) => {
                let elem_t =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);
                match elem_t {
                    Some(t) if t.inner().is_list() => (),
                    None => (),
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(t.inner().clone())),
                            found: t.into_inner(),
                            context: "cardinality is always computed on a collection".to_string(),
                        });
                    }
                }
                Some(ExprType::Int.into()) // Cardinality always gives an Int
            }
        }
    }

    fn check_ident(
        &mut self,
        _global_env: &GlobalEnv,
        ident: &String,
        span: &Span,
        _type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        _warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        let typ = match self.lookup_ident(&ident) {
            Some((typ, _)) => typ,
            None => {
                errors.push(SemError::UnknownIdentifer {
                    identifier: ident.clone(),
                    span: span.clone(),
                });
                return None;
            }
        };

        Some(typ)
    }

    fn check_path(
        &mut self,
        global_env: &mut GlobalEnv,
        object: &Spanned<Expr>,
        segments: &Vec<Spanned<String>>,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        assert!(!segments.is_empty(), "Path must have at least one segment");

        // First segment can be an expression
        let mut current_type = self
            .check_expr(global_env, &object.node, type_info, errors, warnings)?
            .into_inner();

        // Follow the path through fields
        for segment in segments {
            match &current_type {
                ExprType::Object(type_name) => {
                    // Look up the field in this object type
                    match global_env.lookup_field(type_name, &segment.node) {
                        Some(field_type) => {
                            current_type = field_type;
                        }
                        None => {
                            errors.push(SemError::UnknownField {
                                object_type: type_name.clone(),
                                field: segment.node.clone(),
                                span: segment.span.clone(),
                            });
                            return None;
                        }
                    }
                }
                _ => {
                    // Can't access fields on non-object types
                    errors.push(SemError::FieldAccessOnNonObject {
                        typ: current_type.clone(),
                        field: segment.node.clone(),
                        span: segment.span.clone(),
                    });
                    return None;
                }
            }
        }

        Some(current_type)
    }
}

impl TypeInfo {
    pub fn new() -> Self {
        TypeInfo::default()
    }
}

impl GlobalEnv {
    pub fn new(
        defined_types: HashMap<String, ObjectFields>,
        variables: HashMap<String, ArgsType>,
        file: &crate::ast::File,
    ) -> Result<(Self, TypeInfo, Vec<SemError>, Vec<SemWarning>), GlobalEnvError> {
        let mut temp_env = GlobalEnv {
            defined_types,
            functions: HashMap::new(),
            variables: variables
                .into_iter()
                .map(|(var_name, args_type)| (var_name, (args_type, None)))
                .collect(),
            variable_lists: HashMap::new(),
        };

        for (object_type, field_desc) in &temp_env.defined_types {
            for (field, typ) in field_desc {
                if !temp_env.validate_type(typ) {
                    return Err(GlobalEnvError::UnknownTypeInField {
                        object_type: object_type.clone(),
                        field: field.clone(),
                        unknown_type: typ.to_string(),
                    });
                }
            }
        }

        for (var, args) in &temp_env.variables {
            for (param, typ) in args.0.iter().enumerate() {
                if !temp_env.validate_type(typ) {
                    return Err(GlobalEnvError::UnknownTypeForVariableArg {
                        var: var.clone(),
                        param,
                        unknown_type: typ.to_string(),
                    });
                }
            }
        }

        let mut type_info = TypeInfo::new();
        let mut errors = vec![];
        let mut warnings = vec![];

        for statement in &file.statements {
            temp_env.expand_with_statement(
                &statement.node,
                &mut type_info,
                &mut errors,
                &mut warnings,
            );
        }

        temp_env.check_unused_fn(&mut warnings);
        temp_env.check_unused_var(&mut warnings);

        Ok((temp_env, type_info, errors, warnings))
    }

    fn check_unused_fn(&self, warnings: &mut Vec<SemWarning>) {
        for (name, (fn_typ, span, used)) in &self.functions {
            if !fn_typ.public && !used {
                warnings.push(SemWarning::UnusedFunction {
                    identifier: name.clone(),
                    span: span.clone(),
                });
            }
        }
    }

    fn check_unused_var(&self, warnings: &mut Vec<SemWarning>) {
        for (name, (_args_typ, span_and_used_opt)) in &self.variables {
            let Some(span_and_used) = span_and_used_opt else {
                continue;
            };
            if !span_and_used.1 {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: name.clone(),
                    span: span_and_used.0.clone(),
                });
            }
        }
    }

    fn expand_with_statement(
        &mut self,
        statement: &crate::ast::Statement,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match statement {
            crate::ast::Statement::Let {
                docstring: _,
                public,
                name,
                params,
                output_type,
                body,
            } => self.expand_with_let_statement(
                *public,
                name,
                params,
                output_type,
                body,
                type_info,
                errors,
                warnings,
            ),
            crate::ast::Statement::Reify {
                docstring: _,
                constraint_name,
                name,
                var_list,
            } => self.expand_with_reify_statement(
                constraint_name,
                name,
                *var_list,
                type_info,
                errors,
                warnings,
            ),
        }
    }

    fn expand_with_let_statement(
        &mut self,
        public: bool,
        name: &Spanned<String>,
        params: &Vec<Param>,
        output_type: &Spanned<crate::ast::TypeName>,
        body: &Spanned<Expr>,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match self.lookup_fn(&name.node) {
            Some((_fn_type, span)) => {
                errors.push(SemError::FunctionAlreadyDefined {
                    identifier: name.node.clone(),
                    span: name.span.clone(),
                    here: span.clone(),
                });
            }
            None => {
                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &name.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::FunctionNamingConvention {
                        identifier: name.node.clone(),
                        span: name.span.clone(),
                        suggestion,
                    });
                }

                let mut local_env = LocalEnv::new();
                let mut error_in_typs = false;
                for param in params {
                    let param_typ = param.typ.node.clone().into();
                    if !self.validate_type(&param_typ) {
                        errors.push(SemError::UnknownType {
                            typ: param_typ.to_string(),
                            span: param.typ.span.clone(),
                        });
                        error_in_typs = true;
                    } else if let Some((_typ, span)) =
                        local_env.lookup_in_pending_scope(&param.name.node)
                    {
                        errors.push(SemError::ParameterAlreadyDefined {
                            identifier: param.name.node.clone(),
                            span: param.name.span.clone(),
                            here: span,
                        });
                    } else {
                        if let Some(suggestion) =
                            string_case::generate_suggestion_for_naming_convention(
                                &param.name.node,
                                string_case::NamingConvention::SnakeCase,
                            )
                        {
                            warnings.push(SemWarning::ParameterNamingConvention {
                                identifier: param.name.node.clone(),
                                span: param.name.span.clone(),
                                suggestion,
                            });
                        }
                        local_env.register_identifier(
                            &param.name.node,
                            param.name.span.clone(),
                            param_typ,
                            type_info,
                            warnings,
                        );
                    }
                }

                local_env.push_scope();
                let body_type_opt =
                    local_env.check_expr(self, &body.node, type_info, errors, warnings);
                local_env.pop_scope(warnings);

                if let Some(body_type) = body_type_opt {
                    let out_typ = ExprType::from(output_type.node.clone());
                    if !self.validate_type(&out_typ) {
                        errors.push(SemError::UnknownType {
                            typ: out_typ.to_string(),
                            span: output_type.span.clone(),
                        });
                        error_in_typs = true;
                    } else {
                        // Allow coercion
                        let types_match = match (out_typ.clone(), body_type.clone()) {
                            (a, b) if b.can_coerce_to(&a) => true,
                            _ => false,
                        };

                        if !types_match {
                            errors.push(SemError::BodyTypeMismatch {
                                func: name.node.clone(),
                                span: body.span.clone(),
                                expected: out_typ,
                                found: body_type.into_inner(),
                            });
                        }
                    }
                }

                if !error_in_typs {
                    let args = params
                        .iter()
                        .map(|param| param.typ.node.clone().into())
                        .collect();
                    let fn_typ = FunctionType {
                        public,
                        args,
                        output: output_type.node.clone().into(),
                    };
                    self.register_fn(&name.node, fn_typ, name.span.clone(), type_info);
                }
            }
        }
    }

    fn expand_with_reify_statement(
        &mut self,
        constraint_name: &Spanned<String>,
        name: &Spanned<String>,
        var_list: bool,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match self.lookup_fn(&constraint_name.node) {
            None => errors.push(SemError::UnknownIdentifer {
                identifier: constraint_name.node.clone(),
                span: constraint_name.span.clone(),
            }),
            Some(fn_type) => {
                let needed_output_type = if var_list {
                    ExprType::List(Box::new(ExprType::Constraint))
                } else {
                    ExprType::Constraint
                };
                let correct_type = fn_type.0.output.can_coerce_to(&needed_output_type);
                if !correct_type {
                    let expected_type = FunctionType {
                        output: needed_output_type,
                        ..fn_type.0.clone()
                    };
                    errors.push(SemError::FunctionTypeMismatch {
                        identifier: constraint_name.node.clone(),
                        span: constraint_name.span.clone(),
                        expected: expected_type,
                        found: fn_type.0,
                    });
                    return;
                }

                if var_list {
                    match self.lookup_var_list(&name.node) {
                        Some((_args, span)) => errors.push(SemError::VariableAlreadyDefined {
                            identifier: name.node.clone(),
                            span: name.span.clone(),
                            here: Some(span),
                        }),
                        None => {
                            if let Some(suggestion) =
                                string_case::generate_suggestion_for_naming_convention(
                                    &name.node,
                                    string_case::NamingConvention::PascalCase,
                                )
                            {
                                warnings.push(SemWarning::VariableNamingConvention {
                                    identifier: name.node.clone(),
                                    span: name.span.clone(),
                                    suggestion,
                                });
                            }
                            self.register_var_list(
                                &name.node,
                                fn_type.0.args.clone(),
                                name.span.clone(),
                                type_info,
                            );
                        }
                    }
                } else {
                    match self.lookup_var(&name.node) {
                        Some((_args, span_opt)) => errors.push(SemError::VariableAlreadyDefined {
                            identifier: name.node.clone(),
                            span: name.span.clone(),
                            here: span_opt,
                        }),
                        None => {
                            if let Some(suggestion) =
                                string_case::generate_suggestion_for_naming_convention(
                                    &name.node,
                                    string_case::NamingConvention::PascalCase,
                                )
                            {
                                warnings.push(SemWarning::VariableNamingConvention {
                                    identifier: name.node.clone(),
                                    span: name.span.clone(),
                                    suggestion,
                                });
                            }
                            self.register_var(
                                &name.node,
                                fn_type.0.args.clone(),
                                name.span.clone(),
                                type_info,
                            );
                        }
                    }
                }
            }
        }
    }
}
