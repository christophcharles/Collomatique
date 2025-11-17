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
    pub fn is_list(&self) -> bool {
        match self {
            ExprType::List(_) => true,
            ExprType::EmptyList => true,
            _ => false,
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
            ExprType::EmptyList => false, // GenericList is used internally but is not a valid type
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
    ) -> Option<ExprType> {
        use crate::ast::Expr;

        match expr {
            // ========== Literals ==========
            Expr::Number(_) => Some(ExprType::Int),
            Expr::Boolean(_) => Some(ExprType::Bool),

            Expr::Path(path) => self.check_path(
                global_env, &path.node, &path.span, type_info, errors, warnings,
            ),

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

                match (left_type, right_type) {
                    (Some(ExprType::Int), Some(ExprType::Int)) => Some(ExprType::Int),
                    (Some(ExprType::LinExpr), Some(ExprType::Int))
                    | (Some(ExprType::Int), Some(ExprType::LinExpr))
                    | (Some(ExprType::LinExpr), Some(ExprType::LinExpr)) => Some(ExprType::LinExpr),
                    (Some(l), Some(r)) if l == ExprType::Int || l == ExprType::LinExpr => {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l.clone(),
                            found: r.clone(),
                            context: format!(
                                "addition/subtraction requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        Some(l) // Fallback
                    }
                    (Some(l), Some(r)) if r == ExprType::Int || r == ExprType::LinExpr => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: r.clone(),
                            found: l.clone(),
                            context: format!(
                                "addition/subtraction requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        Some(r) // Fallback
                    }
                    (Some(l), Some(r)) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: l.clone(),
                            context: format!(
                                "addition/subtraction requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Int,
                            found: r.clone(),
                            context: format!(
                                "addition/subtraction requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        Some(l) // Fallback
                    }
                    (Some(l), None) if l == ExprType::Int || l == ExprType::LinExpr => Some(l),
                    (Some(l), None) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: l.clone(),
                            context: format!(
                                "addition/subtraction requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        Some(l) // Fallback
                    }
                    (None, Some(r)) if r == ExprType::Int || r == ExprType::LinExpr => Some(r),
                    (None, Some(r)) => {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Int,
                            found: r.clone(),
                            context: format!(
                                "addition/subtraction requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        Some(r) // Fallback
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

                match (left_type, right_type) {
                    (Some(ExprType::Int), Some(ExprType::Int)) => Some(ExprType::Int),
                    (Some(ExprType::LinExpr), Some(ExprType::Int))
                    | (Some(ExprType::Int), Some(ExprType::LinExpr)) => Some(ExprType::LinExpr),
                    (Some(ExprType::LinExpr), Some(ExprType::LinExpr)) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: ExprType::LinExpr,
                            context: "cannot multiply two linear expressions (non-linear)"
                                .to_string(),
                        });
                        Some(ExprType::LinExpr) // Fallback
                    }
                    (Some(l), Some(r)) if l == ExprType::Int || l == ExprType::LinExpr => {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l.clone(),
                            found: r.clone(),
                            context: format!(
                                "multiplication requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        Some(l) // Fallback
                    }
                    (Some(l), Some(r)) if r == ExprType::Int || r == ExprType::LinExpr => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: r.clone(),
                            found: l.clone(),
                            context: format!(
                                "multiplication requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        Some(r) // Fallback
                    }
                    (Some(l), Some(r)) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: l.clone(),
                            context: format!(
                                "multiplication requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Int,
                            found: r.clone(),
                            context: format!(
                                "multiplication requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        Some(l) // Fallback
                    }
                    (Some(l), None) if l == ExprType::Int || l == ExprType::LinExpr => Some(l),
                    (Some(l), None) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: l.clone(),
                            context: format!(
                                "multiplication requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        Some(l) // Fallback
                    }
                    (None, Some(r)) if r == ExprType::Int || r == ExprType::LinExpr => Some(r),
                    (None, Some(r)) => {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Int,
                            found: r.clone(),
                            context: format!(
                                "multiplication requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        Some(r) // Fallback
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

                if let Some(typ) = &left_type {
                    if *typ != ExprType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: typ.clone(),
                            context: "division/modulo requires Int operands".to_string(),
                        });
                    }
                }

                if let Some(typ) = &right_type {
                    if *typ != ExprType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Int,
                            found: typ.clone(),
                            context: "division/modulo requires Int operands".to_string(),
                        });
                    }
                }

                match (&left_type, &right_type) {
                    (Some(l), _) => Some(l.clone()),
                    (_, Some(r)) => Some(r.clone()),
                    _ => None,
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
                    // Both Int -> Bool
                    (Some(ExprType::Int), Some(ExprType::Int)) => Some(ExprType::Bool),
                    // Both Bool -> Bool
                    (Some(ExprType::Bool), Some(ExprType::Bool)) => Some(ExprType::Bool),
                    // Any LinExpr -> Constraint
                    (Some(ExprType::LinExpr), Some(ExprType::Int))
                    | (Some(ExprType::Int), Some(ExprType::LinExpr))
                    | (Some(ExprType::LinExpr), Some(ExprType::LinExpr)) => {
                        Some(ExprType::Constraint)
                    }
                    // Same object types -> Bool
                    (Some(ExprType::Object(l)), Some(ExprType::Object(r))) if l == r => {
                        Some(ExprType::Bool)
                    }
                    (Some(ExprType::List(l)), Some(ExprType::List(r))) if l == r => {
                        Some(ExprType::Bool)
                    }
                    (Some(ExprType::List(_)), Some(ExprType::EmptyList)) => Some(ExprType::Bool),
                    (Some(ExprType::EmptyList), Some(ExprType::List(_))) => Some(ExprType::Bool),
                    (Some(ExprType::EmptyList), Some(ExprType::EmptyList)) => Some(ExprType::Bool),
                    (Some(l), Some(r)) => {
                        if l != r {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: l.clone(),
                                found: r,
                                context: "equality comparison requires matching types".to_string(),
                            });
                        }
                        Some(ExprType::Bool)
                    }
                    _ => None,
                }
            }

            // Relational: Int <= Int -> Bool, LinExpr <= LinExpr -> Constraint
            Expr::Le(left, right) | Expr::Ge(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    // Both Int -> Bool
                    (Some(ExprType::Int), Some(ExprType::Int)) => Some(ExprType::Bool),
                    // Any LinExpr -> Constraint
                    (Some(ExprType::LinExpr), Some(ExprType::Int))
                    | (Some(ExprType::Int), Some(ExprType::LinExpr))
                    | (Some(ExprType::LinExpr), Some(ExprType::LinExpr)) => {
                        Some(ExprType::Constraint)
                    }
                    (Some(l), Some(r)) if l == ExprType::Int || l == ExprType::LinExpr => {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l.clone(),
                            found: r.clone(),
                            context: format!(
                                "relational comparison requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        None
                    }
                    (Some(l), Some(r)) if r == ExprType::Int || r == ExprType::LinExpr => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: r.clone(),
                            found: l.clone(),
                            context: format!(
                                "relational comparison requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        None
                    }
                    (Some(l), Some(r)) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Int,
                            found: l.clone(),
                            context: format!(
                                "relational comparison requires Int or LinExpr operands, got {}",
                                l
                            ),
                        });
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Int,
                            found: r.clone(),
                            context: format!(
                                "relational comparison requires Int or LinExpr operands, got {}",
                                r
                            ),
                        });
                        None
                    }
                    _ => None,
                }
            }

            // Relational: Int < Int -> Bool
            Expr::Lt(left, right) | Expr::Gt(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    // Both Int -> Bool
                    (Some(ExprType::Int), Some(ExprType::Int)) => Some(ExprType::Bool),
                    (Some(l), Some(r)) => {
                        if l != ExprType::Int {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::Int,
                                found: l.clone(),
                                context: format!(
                                    "strict relational comparison requires Int, got {}",
                                    l
                                ),
                            });
                        }
                        if r != ExprType::Int {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::Int,
                                found: r.clone(),
                                context: format!(
                                    "strict relational comparison requires Int, got {}",
                                    r
                                ),
                            });
                        }
                        None
                    }
                    _ => None,
                }
            }

            // ========== Boolean Operations ==========
            // Bool and Bool -> Bool
            Expr::And(left, right) | Expr::Or(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(ExprType::Bool), Some(ExprType::Bool)) => Some(ExprType::Bool),
                    (Some(ExprType::Bool), None) | (None, Some(ExprType::Bool)) => {
                        Some(ExprType::Bool)
                    }
                    (Some(ExprType::Constraint), Some(ExprType::Constraint)) => {
                        Some(ExprType::Constraint)
                    }
                    (Some(ExprType::Constraint), Some(ExprType::Bool)) => {
                        Some(ExprType::Constraint)
                    }
                    (Some(ExprType::Bool), Some(ExprType::Constraint)) => {
                        Some(ExprType::Constraint)
                    }
                    (Some(ExprType::Constraint), None) | (None, Some(ExprType::Constraint)) => {
                        Some(ExprType::Constraint)
                    }
                    (Some(l), Some(r))
                        if (l == ExprType::Bool || l == ExprType::Constraint)
                            && (r == ExprType::Bool || r == ExprType::Constraint) =>
                    {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l.clone(),
                            found: r.clone(),
                            context: "and/or requires both Bool or both Constraint".to_string(),
                        });
                        None
                    }
                    (Some(l), Some(r)) => {
                        if l != ExprType::Bool && l != ExprType::Constraint {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::Bool,
                                found: l.clone(),
                                context: "and/or requires Bool or Constraint".to_string(),
                            });
                        }
                        if r != ExprType::Bool && r != ExprType::Constraint {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::Bool,
                                found: r.clone(),
                                context: "and/or requires Bool or Constraint".to_string(),
                            });
                        }
                        None
                    }
                    (Some(l), None) => {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: ExprType::Bool,
                            found: l.clone(),
                            context: "and/or requires Bool or Constraint".to_string(),
                        });
                        None
                    }
                    (None, Some(r)) => {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: ExprType::Bool,
                            found: r.clone(),
                            context: "and/or requires Bool or Constraint".to_string(),
                        });
                        None
                    }
                    _ => None,
                }
            }

            Expr::Not(expr) => {
                let expr_type =
                    self.check_expr(global_env, &expr.node, type_info, errors, warnings);

                match expr_type {
                    Some(ExprType::Bool) => Some(ExprType::Bool),
                    Some(ExprType::Constraint) => Some(ExprType::Constraint),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: ExprType::Bool,
                            found: typ.clone(),
                            context: "not requires Bool or Constraint operand".to_string(),
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

                match coll_type {
                    Some(ExprType::List(elem_t)) => {
                        if let Some(item_t) = item_type {
                            if item_t != *elem_t {
                                errors.push(SemError::TypeMismatch {
                                    span: item.span.clone(),
                                    expected: *elem_t,
                                    found: item_t,
                                    context: "item type must match collection element type"
                                        .to_string(),
                                });
                            }
                        }
                    }
                    Some(ExprType::EmptyList) => {
                        // OK
                    }
                    _ => {
                        if let Some(list_t) = coll_type {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: ExprType::List(Box::new(list_t.clone())),
                                found: list_t,
                                context: "collection must have a list type".to_string(),
                            });
                        }
                    }
                }

                Some(ExprType::Bool)
            }

            // ========== Quantifiers ==========
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let elem_type =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);

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

                // Extract element type from List
                if elem_type == Some(ExprType::EmptyList) {
                    errors.push(SemError::TypeMismatch {
                        span: collection.span.clone(),
                        expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                        found: ExprType::EmptyList,
                        context: "forall collection must have a known type (empty list are forbidden and useless)".to_string(),
                    });
                    return Some(ExprType::Constraint);
                }
                if let Some(ExprType::List(inner)) = elem_type {
                    self.register_identifier(
                        &var.node,
                        var.span.clone(),
                        *inner,
                        type_info,
                        warnings,
                    );
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type =
                        self.check_expr(global_env, &filter_expr.node, type_info, errors, warnings);

                    if let Some(typ) = filter_type {
                        if typ != ExprType::Bool {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: ExprType::Bool,
                                found: typ,
                                context: "forall filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be Constraint)
                let body_type =
                    self.check_expr(global_env, &body.node, type_info, errors, warnings);

                if let Some(typ) = body_type {
                    if typ != ExprType::Constraint && typ != ExprType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: ExprType::Constraint,
                            found: typ,
                            context: "forall body must be Constraint".to_string(),
                        });
                    }
                }

                self.pop_scope(warnings);

                Some(ExprType::Constraint)
            }

            Expr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                let elem_type =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);

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

                // Extract element type from List
                if elem_type == Some(ExprType::EmptyList) {
                    errors.push(SemError::TypeMismatch {
                        span: collection.span.clone(),
                        expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                        found: ExprType::EmptyList,
                        context: "sum collection must have a known type (empty list are forbidden and useless)".to_string(),
                    });
                    return None;
                }
                if let Some(ExprType::List(inner)) = elem_type {
                    self.register_identifier(
                        &var.node,
                        var.span.clone(),
                        *inner,
                        type_info,
                        warnings,
                    );
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type =
                        self.check_expr(global_env, &filter_expr.node, type_info, errors, warnings);

                    if let Some(typ) = filter_type {
                        if typ != ExprType::Bool {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: ExprType::Bool,
                                found: typ,
                                context: "sum filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be LinExpr or Int)
                let body_type =
                    self.check_expr(global_env, &body.node, type_info, errors, warnings);

                if let Some(typ) = &body_type {
                    if *typ != ExprType::LinExpr && *typ != ExprType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: ExprType::LinExpr,
                            found: typ.clone(),
                            context: "sum body must be LinExpr or Int".to_string(),
                        });

                        self.pop_scope(warnings);
                        None
                    } else {
                        self.pop_scope(warnings);
                        Some(typ.clone())
                    }
                } else {
                    self.pop_scope(warnings);
                    None
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
                    if typ != ExprType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: ExprType::Bool,
                            found: typ,
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
                        // Allow coercion: Int -> LinExpr, Bool -> Constraint
                        let coerced_type = match (&t, &e) {
                            // Borrow instead of clone
                            (a, b) if a == b => Some(t), // Already same, use t
                            (ExprType::LinExpr, ExprType::Int)
                            | (ExprType::Int, ExprType::LinExpr) => Some(ExprType::LinExpr),
                            (ExprType::Constraint, ExprType::Bool)
                            | (ExprType::Bool, ExprType::Constraint) => Some(ExprType::Constraint),
                            _ => {
                                errors.push(SemError::TypeMismatch {
                                    span: else_expr.span.clone(),
                                    expected: t.clone(),
                                    found: e.clone(),
                                    context: "if branches must have the same type".to_string(),
                                });
                                Some(t) // Fallback to then type
                            }
                        };

                        coerced_type
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
                        Some(ExprType::LinExpr) // Because of the syntax, we know the user wanted a LinExpr
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
                                if &found_type != expected_type {
                                    errors.push(SemError::TypeMismatch {
                                        span: arg.span.clone(),
                                        expected: expected_type.clone(),
                                        found: found_type,
                                        context: format!(
                                            "argument {} to variable ${}",
                                            i + 1,
                                            name.node
                                        ),
                                    });
                                }
                            }
                        }

                        Some(ExprType::LinExpr)
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
                            if &found_type != expected_type {
                                errors.push(SemError::TypeMismatch {
                                    span: arg.span.clone(),
                                    expected: expected_type.clone(),
                                    found: found_type,
                                    context: format!(
                                        "argument {} to function {}",
                                        i + 1,
                                        name.node
                                    ),
                                });
                            }
                        }
                    }

                    Some(fn_type.output)
                }
            },

            // ========== Collections ==========
            Expr::GlobalList(type_name) => {
                let obj_type = ExprType::Object(type_name.node.clone());
                if !global_env.validate_type(&obj_type) {
                    errors.push(SemError::UnknownType {
                        typ: type_name.node.clone(),
                        span: type_name.span.clone(),
                    });
                    None
                } else {
                    Some(ExprType::List(Box::new(obj_type)))
                }
            }

            Expr::Union(left, right) | Expr::Inter(left, right) | Expr::Diff(left, right) => {
                let left_type =
                    self.check_expr(global_env, &left.node, type_info, errors, warnings);
                let right_type =
                    self.check_expr(global_env, &right.node, type_info, errors, warnings);

                match (left_type, right_type) {
                    (Some(ExprType::List(l)), Some(ExprType::List(r))) => {
                        if l == r {
                            Some(ExprType::List(l))
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: *l.clone(),
                                found: *r,
                                context: "collection operation requires matching element types"
                                    .to_string(),
                            });
                            Some(ExprType::List(l))
                        }
                    }
                    (Some(ExprType::List(l)), Some(ExprType::EmptyList))
                    | (Some(ExprType::EmptyList), Some(ExprType::List(l))) => {
                        Some(ExprType::List(l))
                    }
                    (Some(ExprType::EmptyList), Some(ExprType::EmptyList)) => {
                        Some(ExprType::EmptyList)
                    }
                    (Some(l), Some(r)) => {
                        if !l.is_list() {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: ExprType::List(Box::new(l.clone())),
                                found: l,
                                context: "collection operation requires List types".to_string(),
                            });
                        }
                        if !r.is_list() {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: ExprType::List(Box::new(r.clone())),
                                found: r,
                                context: "collection operation requires List types".to_string(),
                            });
                        }
                        None
                    }
                    (Some(ExprType::List(l)), None) | (None, Some(ExprType::List(l))) => {
                        Some(ExprType::List(l))
                    }
                    (Some(ExprType::EmptyList), None) | (None, Some(ExprType::EmptyList)) => {
                        Some(ExprType::EmptyList)
                    }
                    _ => None,
                }
            }

            // ========== Lists ==========
            Expr::ListLiteral { elements } => {
                if elements.is_empty() {
                    // Empty list - we can't infer type
                    return Some(ExprType::EmptyList);
                }

                let first_type =
                    self.check_expr(global_env, &elements[0].node, type_info, errors, warnings);

                for item in &elements[1..] {
                    let item_type =
                        self.check_expr(global_env, &item.node, type_info, errors, warnings);

                    if let (Some(expected), Some(found)) = (&first_type, item_type) {
                        if expected != &found {
                            errors.push(SemError::TypeMismatch {
                                span: item.span.clone(),
                                expected: expected.clone(),
                                found,
                                context: "all list elements must have the same type".to_string(),
                            });
                        }
                    }
                }

                first_type.map(|t| ExprType::List(Box::new(t)))
            }

            Expr::ListComprehension {
                expr,
                var,
                collection,
                filter,
            } => {
                let coll_type =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);

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

                // Extract element type
                if coll_type == Some(ExprType::EmptyList) {
                    errors.push(SemError::TypeMismatch {
                        span: collection.span.clone(),
                        expected: ExprType::List(Box::new(ExprType::Int)), // placeholder
                        found: ExprType::EmptyList,
                        context: "list comprehension collection must have a known type (empty list are forbidden and useless)".to_string(),
                    });
                    return None;
                }
                if let Some(ExprType::List(inner)) = coll_type {
                    self.register_identifier(
                        &var.node,
                        var.span.clone(),
                        *inner,
                        type_info,
                        warnings,
                    );
                }

                self.push_scope();

                // Check filter
                if let Some(filter_expr) = filter {
                    let filter_type =
                        self.check_expr(global_env, &filter_expr.node, type_info, errors, warnings);

                    if let Some(typ) = filter_type {
                        if typ != ExprType::Bool {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: ExprType::Bool,
                                found: typ,
                                context: "list comprehension filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check the expression that produces elements
                let elem_type =
                    self.check_expr(global_env, &expr.node, type_info, errors, warnings);

                self.pop_scope(warnings);

                elem_type.map(|t| ExprType::List(Box::new(t)))
            }

            // ========== Cardinality ==========
            Expr::Cardinality(collection) => {
                let elem_t =
                    self.check_expr(global_env, &collection.node, type_info, errors, warnings);
                match elem_t {
                    Some(ExprType::List(_)) => (),
                    Some(ExprType::EmptyList) => (),
                    None => (),
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: ExprType::List(Box::new(t.clone())),
                            found: t,
                            context: "cardinality is always computed on a collection".to_string(),
                        });
                    }
                }
                Some(ExprType::Int)
            }
        }
    }

    fn check_path(
        &mut self,
        global_env: &GlobalEnv,
        path: &crate::ast::Path,
        _span: &Span,
        _type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        _warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        assert!(
            !path.segments.is_empty(),
            "Path must have at least one segment"
        );

        // First segment must be a local identifier
        let first_segment = &path.segments[0];
        let mut current_type = match self.lookup_ident(&first_segment.node) {
            Some((typ, _)) => typ,
            None => {
                errors.push(SemError::UnknownIdentifer {
                    identifier: first_segment.node.clone(),
                    span: first_segment.span.clone(),
                });
                return None;
            }
        };

        // Follow the path through fields
        for segment in &path.segments[1..] {
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
                &output_type.node,
                body,
                type_info,
                errors,
                warnings,
            ),
            crate::ast::Statement::Reify {
                docstring: _,
                constraint_name,
                var_name,
            } => self.expand_with_reify_statement(
                constraint_name,
                var_name,
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
        output_type: &crate::ast::TypeName,
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
                let mut error_in_param_typs = false;
                for param in params {
                    let param_typ = param.typ.node.clone().into();
                    if !self.validate_type(&param_typ) {
                        errors.push(SemError::UnknownType {
                            typ: param_typ.to_string(),
                            span: param.typ.span.clone(),
                        });
                        error_in_param_typs = true;
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
                    let out_typ = ExprType::from(output_type.clone());

                    // Allow coercion: Int -> LinExpr, Bool -> Constraint
                    let types_match = match (out_typ.clone(), body_type.clone()) {
                        (a, b) if a == b => true,
                        (ExprType::LinExpr, ExprType::Int) => true, // Coerce Int to LinExpr
                        (ExprType::Constraint, ExprType::Bool) => true, // Coerce Bool to Constraint
                        _ => false,
                    };

                    if !types_match {
                        errors.push(SemError::BodyTypeMismatch {
                            func: name.node.clone(),
                            span: body.span.clone(),
                            expected: out_typ,
                            found: body_type,
                        });
                    }
                }

                if !error_in_param_typs {
                    let args = params
                        .iter()
                        .map(|param| param.typ.node.clone().into())
                        .collect();
                    let fn_typ = FunctionType {
                        public,
                        args,
                        output: output_type.clone().into(),
                    };
                    self.register_fn(&name.node, fn_typ, name.span.clone(), type_info);
                }
            }
        }
    }

    fn expand_with_reify_statement(
        &mut self,
        constraint_name: &Spanned<String>,
        var_name: &Spanned<String>,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match self.lookup_fn(&constraint_name.node) {
            None => errors.push(SemError::UnknownIdentifer {
                identifier: constraint_name.node.clone(),
                span: constraint_name.span.clone(),
            }),
            Some(fn_type) => match fn_type.0.output {
                ExprType::Constraint => match self.lookup_var(&var_name.node) {
                    Some((_args, span_opt)) => errors.push(SemError::VariableAlreadyDefined {
                        identifier: var_name.node.clone(),
                        span: var_name.span.clone(),
                        here: span_opt,
                    }),
                    None => {
                        if let Some(suggestion) =
                            string_case::generate_suggestion_for_naming_convention(
                                &var_name.node,
                                string_case::NamingConvention::PascalCase,
                            )
                        {
                            warnings.push(SemWarning::VariableNamingConvention {
                                identifier: var_name.node.clone(),
                                span: var_name.span.clone(),
                                suggestion,
                            });
                        }
                        self.register_var(
                            &var_name.node,
                            fn_type.0.args.clone(),
                            var_name.span.clone(),
                            type_info,
                        );
                    }
                },
                _ => {
                    let expected_type = FunctionType {
                        output: ExprType::Constraint,
                        ..fn_type.0.clone()
                    };
                    errors.push(SemError::FunctionTypeMismatch {
                        identifier: constraint_name.node.clone(),
                        span: constraint_name.span.clone(),
                        expected: expected_type,
                        found: fn_type.0,
                    });
                }
            },
        }
    }
}
