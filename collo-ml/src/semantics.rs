use crate::ast::{Expr, Param, Span, Spanned};
use std::collections::{BTreeSet, HashMap};

pub mod string_case;
#[cfg(test)]
mod tests;

mod types;
pub use types::{ConcreteType, ExprType, SimpleType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub args: ArgsType,
    pub output: ExprType,
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
pub struct FunctionDesc {
    pub name_span: Span,
    pub typ: FunctionType,
    pub public: bool,
    pub used: bool,
    pub arg_names: Vec<String>,
    pub body: Spanned<crate::ast::Expr>,
    pub docstring: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDesc {
    pub args: ArgsType,
    pub span: Span,
    pub used: bool,
    pub referenced_fn: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEnv {
    defined_types: HashMap<String, ObjectFields>,
    functions: HashMap<String, FunctionDesc>,
    external_variables: HashMap<String, ArgsType>,
    internal_variables: HashMap<String, VariableDesc>,
    variable_lists: HashMap<String, VariableDesc>,
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

#[derive(Debug, Error, Clone)]
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
    pub fn validate_object_type(&self, obj_name: &str) -> bool {
        self.defined_types.contains_key(obj_name)
    }

    pub fn validate_simple_type(&self, typ: &SimpleType) -> bool {
        match typ {
            SimpleType::None => true,
            SimpleType::Bool => true,
            SimpleType::Int => true,
            SimpleType::LinExpr => true,
            SimpleType::Constraint => true,
            SimpleType::String => true,
            SimpleType::EmptyList => true,
            SimpleType::List(sub_typ) => self.validate_type(sub_typ),
            SimpleType::Object(typ_name) => self.validate_object_type(&typ_name),
        }
    }

    pub fn validate_type(&self, typ: &ExprType) -> bool {
        typ.get_variants()
            .iter()
            .all(|x| self.validate_simple_type(x))
    }

    pub fn get_functions(&self) -> &HashMap<String, FunctionDesc> {
        &self.functions
    }

    pub fn get_predefined_vars(&self) -> &HashMap<String, ArgsType> {
        &self.external_variables
    }

    pub fn get_vars(&self) -> &HashMap<String, VariableDesc> {
        &self.internal_variables
    }

    pub fn get_var_lists(&self) -> &HashMap<String, VariableDesc> {
        &self.variable_lists
    }

    pub fn get_types(&self) -> &HashMap<String, ObjectFields> {
        &self.defined_types
    }

    fn lookup_fn(&mut self, name: &str) -> Option<(FunctionType, Span)> {
        let fn_desc = self.functions.get_mut(name)?;
        fn_desc.used = true;
        Some((fn_desc.typ.clone(), fn_desc.body.span.clone()))
    }

    fn register_fn(
        &mut self,
        name: &str,
        name_span: Span,
        fn_typ: FunctionType,
        public: bool,
        arg_names: Vec<String>,
        body: Spanned<crate::ast::Expr>,
        docstring: Vec<String>,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.functions.contains_key(name));

        self.functions.insert(
            name.to_string(),
            FunctionDesc {
                name_span,
                typ: fn_typ.clone(),
                public,
                used: should_be_used_by_default(name),
                arg_names,
                body: body.clone(),
                docstring,
            },
        );

        type_info.types.insert(body.span, fn_typ.into());
    }

    fn lookup_var(&mut self, name: &str) -> Option<(ArgsType, Option<Span>)> {
        if let Some(ext_var) = self.external_variables.get(name) {
            return Some((ext_var.clone(), None));
        };

        let var_desc = self.internal_variables.get_mut(name)?;

        var_desc.used = true;

        Some((var_desc.args.clone(), Some(var_desc.span.clone())))
    }

    fn register_var(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        referenced_fn: String,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.external_variables.contains_key(name));
        assert!(!self.internal_variables.contains_key(name));

        self.internal_variables.insert(
            name.to_string(),
            VariableDesc {
                args: args_typ.clone(),
                span: span.clone(),
                used: should_be_used_by_default(name),
                referenced_fn,
            },
        );

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_var_list(&mut self, name: &str) -> Option<(ArgsType, Span)> {
        let var_desc = self.variable_lists.get_mut(name)?;

        var_desc.used = true;

        Some((var_desc.args.clone(), var_desc.span.clone()))
    }

    fn register_var_list(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        referenced_fn: String,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.variable_lists.contains_key(name));

        self.variable_lists.insert(
            name.to_string(),
            VariableDesc {
                args: args_typ.clone(),
                span: span.clone(),
                used: should_be_used_by_default(name),
                referenced_fn,
            },
        );

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_field(&self, obj_type: &str, field: &str) -> Option<ExprType> {
        self.defined_types.get(obj_type)?.get(field).cloned()
    }
}

#[derive(Debug, Error, Clone)]
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
    #[error("Multiple option markers '?' on {typ} (at {span:?}) - only one option marker '?' is allowed")]
    MultipleOptionMarkers { typ: SimpleType, span: Span },
    #[error("Type {typ} appears multiple time in the sum (at {span1:?} and {span2:?} in sum at {sum_span:?})")]
    MultipleTypeInSum {
        typ: SimpleType,
        span1: Span,
        span2: Span,
        sum_span: Span,
    },
    #[error(
        "Type {typ1} (at {span1:?}) is a subtype of {typ2} (at {span2:?} in sum at {sum_span:?})"
    )]
    SubtypeAndTypePresent {
        typ1: SimpleType,
        span1: Span,
        typ2: SimpleType,
        span2: Span,
        sum_span: Span,
    },
    #[error("Option marker '?' is forbidden on None (at {0:?})")]
    OptionMarkerOnNone(Span),
    #[error("Type {typ} at {span:?} is not a sum type of objects. This is disallowed in global collections")]
    GlobalCollectionsMustBeAListOfObjects { typ: String, span: Span },
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
    #[error("Type at {span:?}: found {found} which is not a concrete type ({context})")]
    NonConcreteType {
        span: Span,
        found: ExprType,
        context: String,
    },
    #[error("Type at {span:?}: found {found} which cannot be converted into {target}")]
    ImpossibleConversion {
        span: Span,
        found: ExprType,
        target: ConcreteType,
    },
    #[error("Local variable \"{identifier}\" at {span:?} is already defined in the same scope ({here:?})")]
    LocalIdentAlreadyDeclared {
        identifier: String,
        span: Span,
        here: Span,
    },
    #[error("Branch for match at {span:?} has a too large type ({found:?}). Maximum type is {expected:?}")]
    OverMatching {
        span: Span,
        expected: Option<ExprType>,
        found: Option<ExprType>,
    },
    #[error("Match at {span:?} does not have exhaustive checking. The case {remaining_types} is not covered")]
    NonExhaustiveMatching {
        span: Span,
        remaining_types: ExprType,
    },
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
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

fn ident_can_be_unused(ident: &str) -> bool {
    assert!(ident.len() > 0);

    ident.chars().next().unwrap() == '_'
}

fn should_be_used_by_default(ident: &str) -> bool {
    ident_can_be_unused(ident)
}

fn ident_can_be_shadowed(ident: &str) -> bool {
    ident_can_be_unused(ident)
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
    ) -> Result<(), SemError> {
        if let Some((_, old_ident_span, _)) = self.pending_scope.get(ident) {
            return Err(SemError::LocalIdentAlreadyDeclared {
                identifier: ident.to_string(),
                span,
                here: old_ident_span.clone(),
            });
        }

        if let Some((_typ, old_span)) = self.lookup_ident(ident) {
            if !ident_can_be_shadowed(ident) {
                warnings.push(SemWarning::IdentifierShadowed {
                    identifier: ident.to_string(),
                    span: span.clone(),
                    previous: old_span,
                });
            }
        }

        self.pending_scope.insert(
            ident.to_string(),
            (typ.clone(), span.clone(), should_be_used_by_default(ident)),
        );
        type_info.types.insert(span, typ.into());

        Ok(())
    }

    fn check_expr(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &crate::ast::Expr,
        span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        let result = self.check_expr_internal(
            global_env, expr, span, type_info, expr_types, errors, warnings,
        );
        if let Some(typ) = &result {
            expr_types.insert(span.clone(), typ.clone());
        }
        result
    }

    fn check_expr_internal(
        &mut self,
        global_env: &mut GlobalEnv,
        expr: &crate::ast::Expr,
        global_span: &Span,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        use crate::ast::Expr;

        match expr {
            // ========== Literals ==========
            Expr::None => Some(SimpleType::None.into()),
            Expr::Number(_) => Some(SimpleType::Int.into()),
            Expr::Boolean(_) => Some(SimpleType::Bool.into()),
            Expr::StringLiteral(_) => Some(SimpleType::String.into()),

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
                .check_path(
                    global_env, &object, segments, type_info, expr_types, errors, warnings,
                )
                .map(|x| x.into()),

            // ========== Into construct ==========
            Expr::TypeConversion { expr, typ } => {
                // Check the inner expression
                let expr_type = self.check_expr(
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                );

                // Convert the declared type
                let target_type = match ExprType::try_from(typ.clone()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return expr_type; // Fallback to inferred type
                    }
                };

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

                // Validate that the target type is concrete for type conversion
                if !target_type.is_concrete() {
                    errors.push(SemError::NonConcreteType {
                        span: typ.span.clone(),
                        found: target_type,
                        context: "Type conversion requires a concrete target type".to_string(),
                    });
                    return expr_type; // Fallback to inferred type
                }
                let concrete_target = target_type.to_simple().unwrap().into_concrete().unwrap();

                if let Some(inferred) = expr_type {
                    // Check if the inferred type can convert to the target type
                    if !inferred.can_convert_to(&concrete_target) {
                        // Error: can't convert
                        errors.push(SemError::ImpossibleConversion {
                            span: expr.span.clone(),
                            found: inferred,
                            target: concrete_target.clone(),
                        });
                    }
                }
                Some(concrete_target.into_inner().into()) // Propagate concrete target in all cases
            }
            // ========== As construct ==========
            Expr::ExplicitType { expr, typ } => {
                // Check the inner expression
                let expr_type = self.check_expr(
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                );

                // Convert the declared type
                let target_type = match ExprType::try_from(typ.clone()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return expr_type; // Fallback to inferred type
                    }
                };

                // Validate that the target type is actually valid
                if !global_env.validate_type(&target_type) {
                    errors.push(SemError::UnknownType {
                        typ: target_type.to_string(),
                        span: typ.span.clone(),
                    });
                    return expr_type; // Fallback to inferred type
                }

                if let Some(inferred) = expr_type {
                    // Check if the inferred type can convert to the target type
                    if !inferred.is_subtype_of(&target_type) {
                        // Error: can't convert
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: inferred,
                            found: target_type.clone(),
                            context: "Type coercion can only be done to super-types".into(),
                        });
                    }
                }
                Some(target_type) // Propagate target in all cases
            }

            // ========== Arithmetic Operations ==========
            // Int + Int -> Int
            // LinExpr + Int -> LinExpr (auto convert Int to LinExpr)
            // Int + LinExpr -> LinExpr (auto convert Int to LinExpr)
            // LinExpr + LinExpr -> LinExpr
            // [Type] + [Type] -> [Type]
            Expr::Add(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (None, None) => None,
                    (Some(t), None) |
                    (None, Some(t)) => {
                        if t.is_int() || t.is_lin_expr() || t.is_list() {
                            Some(t.clone())
                        } else if t.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap()) {
                            Some(SimpleType::LinExpr.into())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context:
                                    "addition/concatenation requires Int or LinExpr or List"
                                        .to_string(),
                            });
                            None
                        }
                    }
                    (Some(l), Some(r)) => {
                        l.cross_check(
                            &r,
                            errors,
                            |v1,v2| match (v1,v2) {
                                (SimpleType::Int, SimpleType::Int) => Ok(SimpleType::Int),
                                (SimpleType::LinExpr, SimpleType::Int) |
                                (SimpleType::Int, SimpleType::LinExpr) |
                                (SimpleType::LinExpr, SimpleType::LinExpr) => Ok(SimpleType::LinExpr),
                                (SimpleType::EmptyList, SimpleType::EmptyList) => Ok(SimpleType::EmptyList),
                                (SimpleType::List(inner), SimpleType::EmptyList) |
                                (SimpleType::EmptyList, SimpleType::List(inner)) => Ok(
                                    SimpleType::List(inner.clone())
                                ),
                                (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                                    Ok(SimpleType::List(inner1.unify_with(inner2)))
                                }
                                (a,b) => {
                                    let is_a_valid = a.is_arithmetic() || a.is_list();
                                    let is_b_valid = b.is_arithmetic() || b.is_list();
                                    let span = if is_a_valid {
                                        right.span.clone()
                                    } else {
                                        left.span.clone()
                                    };
                                    let expected = if is_a_valid {
                                        a.clone()
                                    } else if is_b_valid {
                                        b.clone()
                                    } else {
                                        SimpleType::Int
                                    };
                                    let found = if is_a_valid {
                                        b.clone()
                                    } else {
                                        a.clone()
                                    };
                                    Err(SemError::TypeMismatch {
                                        span,
                                        expected: expected.into(),
                                        found: found.into(),
                                        context: format!(
                                            "addition/concatenation requires Int, LinExpr or List, got {} and {}",
                                            a, b
                                        ),
                                    })
                                }
                            }
                        )
                    }
                }
            }
            // Int - Int -> Int
            // LinExpr - Int -> LinExpr (auto convert Int to LinExpr)
            // Int - LinExpr -> LinExpr (auto convert Int to LinExpr)
            // LinExpr - LinExpr -> LinExpr
            // [Type1] - [Type2] -> [Type1] if Type1 and Type2 overlap
            Expr::Sub(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (None, None) => None,
                    (Some(t), None) |
                    (None, Some(t)) => {
                        if t.is_int() || t.is_lin_expr() || t.is_list() {
                            Some(t.clone())
                        } else if t.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap()) {
                            Some(SimpleType::LinExpr.into())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context:
                                    "substraction/difference requires Int or LinExpr or List"
                                        .to_string(),
                            });
                            None
                        }
                    }
                    (Some(l), Some(r)) => {
                        l.cross_check(
                            &r,
                            errors,
                            |v1,v2| match (v1,v2) {
                                (SimpleType::Int, SimpleType::Int) => Ok(SimpleType::Int),
                                (SimpleType::LinExpr, SimpleType::Int) |
                                (SimpleType::Int, SimpleType::LinExpr) |
                                (SimpleType::LinExpr, SimpleType::LinExpr) => Ok(SimpleType::LinExpr),
                                (SimpleType::EmptyList, _) => Err(SemError::TypeMismatch {
                                    span: left.span.clone(),
                                    expected: SimpleType::List(SimpleType::Int.into()).into(),
                                    found: SimpleType::EmptyList.into(),
                                    context: format!(
                                        "Cannot remove elements from empty list",
                                    ),
                                }),
                                (SimpleType::List(_inner), SimpleType::EmptyList) => Err(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::List(SimpleType::Int.into()).into(),
                                    found: SimpleType::EmptyList.into(),
                                    context: format!(
                                        "Removing empty list is a no-op",
                                    ),
                                }),
                                (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                                    if inner1.overlaps_with(inner2) {
                                        Ok(SimpleType::List(inner1.clone()))
                                    } else {
                                        Err(SemError::TypeMismatch {
                                            span: right.span.clone(),
                                            expected: inner1.clone(),
                                            found: inner2.clone(),
                                            context: format!(
                                                "Types must overlap in set difference",
                                            ),
                                        })
                                    }
                                }
                                (a,b) => {
                                    let is_a_valid = a.is_arithmetic() || a.is_list();
                                    let is_b_valid = b.is_arithmetic() || b.is_list();
                                    let span = if is_a_valid {
                                        right.span.clone()
                                    } else {
                                        left.span.clone()
                                    };
                                    let expected = if is_a_valid {
                                        a.clone()
                                    } else if is_b_valid {
                                        b.clone()
                                    } else {
                                        SimpleType::Int
                                    };
                                    let found = if is_a_valid {
                                        b.clone()
                                    } else {
                                        a.clone()
                                    };
                                    Err(SemError::TypeMismatch {
                                        span,
                                        expected: expected.into(),
                                        found: found.into(),
                                        context: format!(
                                            "subtraction/difference requires Int, LinExpr or List, got {} and {}",
                                            a, b
                                        ),
                                    })
                                }
                            }
                        )
                    }
                }
            }
            // Unary negation - for LinExpr and Int
            Expr::Neg(term) => {
                let term_type = self.check_expr(
                    global_env, &term.node, &term.span, type_info, expr_types, errors, warnings,
                );

                match term_type.clone() {
                    Some(t) if t.is_arithmetic() => Some(t),
                    Some(t) => {
                        let span = term.span.clone();
                        errors.push(SemError::TypeMismatch {
                            span,
                            expected: SimpleType::Int.into(),
                            found: t.clone(),
                            context: "negation requires Int or LinExpr".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }
            // Multiplication: Int * Int -> Int, Int * LinExpr -> LinExpr, LinExpr * Int -> LinExpr
            // But NOT LinExpr * LinExpr (non-linear!)
            Expr::Mul(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (None, None) => None,
                    (Some(t), None) | (None, Some(t)) => {
                        if t.is_int() || t.is_lin_expr() {
                            Some(t.clone())
                        } else if t.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap()) {
                            Some(SimpleType::LinExpr.into())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context: "multiplication requires Int or LinExpr".to_string(),
                            });
                            None
                        }
                    }
                    (Some(l), Some(r)) => l.cross_check(&r, errors, |v1, v2| match (v1, v2) {
                        (SimpleType::Int, SimpleType::Int) => Ok(SimpleType::Int),
                        (SimpleType::LinExpr, SimpleType::Int)
                        | (SimpleType::Int, SimpleType::LinExpr) => Ok(SimpleType::LinExpr),
                        (SimpleType::LinExpr, SimpleType::LinExpr) => Err(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: SimpleType::LinExpr.into(),
                            context: "cannot multiply two linear expressions (non-linear)"
                                .to_string(),
                        }),
                        (a, b) => {
                            let is_a_valid = a.is_arithmetic();
                            let is_b_valid = b.is_arithmetic();
                            let span = if is_a_valid {
                                right.span.clone()
                            } else {
                                left.span.clone()
                            };
                            let expected = if is_a_valid {
                                a.clone()
                            } else if is_b_valid {
                                b.clone()
                            } else {
                                SimpleType::Int
                            };
                            let found = if is_a_valid { b.clone() } else { a.clone() };
                            Err(SemError::TypeMismatch {
                                span,
                                expected: expected.into(),
                                found: found.into(),
                                context: format!(
                                    "multiplication requires Int or LinExpr, got {} and {}",
                                    a, b
                                ),
                            })
                        }
                    }),
                }
            }
            // Division and modulo: Int // Int -> Int, Int % Int -> Int
            // These are NOT allowed on LinExpr
            Expr::Div(left, right) | Expr::Mod(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match (left_type, right_type) {
                    (Some(l), Some(r)) => {
                        // Check if both are Int
                        let l_ok = l.is_int();
                        let r_ok = r.is_int();

                        if !l_ok {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: l.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                        }
                        if !r_ok {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: r.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                        }

                        if l_ok || r_ok {
                            Some(SimpleType::Int.into())
                        } else {
                            None
                        }
                    }
                    (Some(t), None) => {
                        if !t.is_int() {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                            None
                        } else {
                            Some(SimpleType::Int.into())
                        }
                    }
                    (None, Some(t)) => {
                        if !t.is_int() {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: SimpleType::Int.into(),
                                found: t.clone(),
                                context: "division/modulo requires Int operands".to_string(),
                            });
                            None
                        } else {
                            Some(SimpleType::Int.into())
                        }
                    }
                    (None, None) => None,
                }
            }

            // ========== Constraints operators ==========
            Expr::ConstraintEq(left, right)
            | Expr::ConstraintLe(left, right)
            | Expr::ConstraintGe(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                if let (Some(l), Some(r)) = (left_type, right_type) {
                    // Check if both can convert to LinExpr
                    let l_ok = l.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap());
                    let r_ok = r.can_convert_to(&SimpleType::LinExpr.into_concrete().unwrap());

                    if !l_ok {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::LinExpr.into(),
                            found: l,
                            context: "constraint operator requires LinExpr or Int operands"
                                .to_string(),
                        });
                    }
                    if !r_ok {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: SimpleType::LinExpr.into(),
                            found: r,
                            context: "constraint operator requires LinExpr or Int operands"
                                .to_string(),
                        });
                    }
                }

                // Always return Constraint (even on error, intent is clear)
                Some(SimpleType::Constraint.into())
            }

            // ========== Comparison Operations ==========
            Expr::Eq(left, right) | Expr::Ne(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                if let (Some(l), Some(r)) = (left_type, right_type) {
                    if !l.overlaps_with(&r) {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l.clone(),
                            found: r.clone(),
                            context: "equality can never happens with incompatible types"
                                .to_string(),
                        });
                    }
                }
                Some(SimpleType::Bool.into())
            }

            // Relational: Int < Int -> Bool
            Expr::Le(left, right)
            | Expr::Ge(left, right)
            | Expr::Lt(left, right)
            | Expr::Gt(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                if let (Some(l), Some(r)) = (left_type, right_type) {
                    // Check if both can coerce to Int
                    let l_ok = l.can_convert_to(&SimpleType::Int.into_concrete().unwrap());
                    let r_ok = r.can_convert_to(&SimpleType::Int.into_concrete().unwrap());

                    if !l_ok {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: l,
                            context: "relational comparison requires Int operands".to_string(),
                        });
                    }
                    if !r_ok {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: r,
                            context: "relational comparison requires Int operands".to_string(),
                        });
                    }
                }
                Some(SimpleType::Bool.into())
            }

            // ========== Boolean Operations ==========
            // Bool and Bool -> Bool, Constraint and Constraint -> Constraint
            Expr::And(left, right) | Expr::Or(left, right) => {
                let left_type = self.check_expr(
                    global_env, &left.node, &left.span, type_info, expr_types, errors, warnings,
                );
                let right_type = self.check_expr(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match (&left_type, &right_type) {
                    (Some(l), Some(r)) => {
                        if l.is_bool() {
                            if r.is_bool() {
                                Some(SimpleType::Bool.into())
                            } else {
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::Bool.into(),
                                    found: r.clone(),
                                    context: "and/or requires Bool or Constraint operands"
                                        .to_string(),
                                });
                                None
                            }
                        } else if l.is_constraint() {
                            if r.is_constraint() {
                                Some(SimpleType::Constraint.into())
                            } else {
                                errors.push(SemError::TypeMismatch {
                                    span: right.span.clone(),
                                    expected: SimpleType::Constraint.into(),
                                    found: r.clone(),
                                    context: "and/or requires Bool or Constraint operands"
                                        .to_string(),
                                });
                                None
                            }
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: left.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: l.clone(),
                                context: "and/or requires Bool or Constraint operands".to_string(),
                            });
                            None
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => {
                        if t.is_bool() || t.is_constraint() {
                            Some(t.clone())
                        } else {
                            let span = if left_type.is_some() {
                                left.span.clone()
                            } else {
                                right.span.clone()
                            };
                            errors.push(SemError::TypeMismatch {
                                span,
                                expected: SimpleType::Bool.into(),
                                found: t.clone(),
                                context: "and/or requires Bool or Constraint operands".to_string(),
                            });
                            None
                        }
                    }
                    (None, None) => None,
                }
            }

            Expr::Not(expr) => {
                let expr_type = self.check_expr(
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                );

                match expr_type {
                    Some(typ) if typ.is_bool() => Some(SimpleType::Bool.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: SimpleType::Bool.into(),
                            found: typ.clone(),
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
                let item_type = self.check_expr(
                    global_env, &item.node, &item.span, type_info, expr_types, errors, warnings,
                );
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match coll_type {
                    Some(a) if a.is_list() => {
                        let elem_t_opt = a.get_inner_list_type();
                        if let Some(elem_t) = elem_t_opt {
                            // The list might not be empty so we check the inner type
                            if let Some(item_t) = item_type {
                                if !item_t.overlaps_with(&elem_t) {
                                    errors.push(SemError::TypeMismatch {
                                        span: item.span.clone(),
                                        expected: elem_t.into(),
                                        found: item_t,
                                        context: "item type must match collection element type"
                                            .to_string(),
                                    });
                                }
                            }
                        }
                    }
                    Some(t) => {
                        // Not a list at all
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "membership test requires a list".to_string(),
                        });
                    }
                    None => {
                        // Collection failed to type-check
                    }
                }

                // Always returns Bool
                Some(SimpleType::Bool.into())
            }

            // ========== Quantifiers ==========
            Expr::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

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
                match coll_type {
                    Some(a) if a.is_empty_list() => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(),
                            found: a.clone(),
                            context: "forall collection inner type must be known (use 'as' for explicit typing)".to_string(),
                        });
                        return None; // Return early
                    }
                    Some(a) if a.is_list() => {
                        let elem_t = a
                            .get_inner_list_type()
                            .expect("The list should not be empty at this point");
                        // Register the loop variable with the element type
                        if let Err(e) = self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        ) {
                            errors.push(e);
                            return None;
                        }
                    }

                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "forall collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None, // Return early
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_expr(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        expr_types,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if !typ.is_bool() {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "forall filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be Constraint or Bool)
                let body_type = self.check_expr(
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
                );

                self.pop_scope(warnings);

                match body_type {
                    Some(typ) if typ.is_constraint() => Some(SimpleType::Constraint.into()),
                    Some(typ) if typ.is_bool() => Some(SimpleType::Bool.into()),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: SimpleType::Constraint.into(),
                            found: typ,
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
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

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
                match coll_type {
                    Some(a) if a.is_empty_list() => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(),
                            found: a.clone(),
                            context:
                                "sum collection inner type must be known (use 'as' for explicit typing)"
                                    .to_string(),
                        });
                        return None; // Return early
                    }
                    Some(a) if a.is_list() => {
                        let elem_t = a
                            .get_inner_list_type()
                            .expect("List should not be empty at this point");
                        // Register the loop variable with the element type
                        if let Err(e) = self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            elem_t,
                            type_info,
                            warnings,
                        ) {
                            errors.push(e);
                            return None;
                        }
                    }
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "sum collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None, // Return early
                }

                self.push_scope();

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_expr(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        expr_types,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if !typ.is_bool() {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "sum filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check body (must be arithmetic: Int or LinExpr)
                let body_type = self.check_expr(
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
                );

                self.pop_scope(warnings);

                match body_type {
                    Some(typ) if typ.is_arithmetic() || typ.is_list() => Some(typ),
                    Some(typ) => {
                        errors.push(SemError::TypeMismatch {
                            span: body.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: typ,
                            context: "sum body must be Int or LinExpr or list".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }

            Expr::Fold {
                var,
                collection,
                accumulator,
                init_value,
                filter,
                body,
                reversed: _,
            } => {
                let coll_type = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                let acc_type = self.check_expr(
                    global_env,
                    &init_value.node,
                    &init_value.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                // Check naming conventions
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

                if let Some(suggestion) = string_case::generate_suggestion_for_naming_convention(
                    &accumulator.node,
                    string_case::NamingConvention::SnakeCase,
                ) {
                    warnings.push(SemWarning::ParameterNamingConvention {
                        identifier: accumulator.node.clone(),
                        span: accumulator.span.clone(),
                        suggestion,
                    });
                }

                // Extract type info for elements in the collection
                let elem_t = match coll_type {
                    Some(a) if a.is_empty_list() => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(SimpleType::Int.into()).into(),
                            found: a.clone(),
                            context: "fold|rfold collection inner type must be known (use 'as' for explicit typing)".to_string(),
                        });
                        return None; // Return early
                    }
                    Some(a) if a.is_list() => a
                        .get_inner_list_type()
                        .expect("List should not be empty at this point"),
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "fold|rfold collection must be a list".to_string(),
                        });
                        return None; // Return early
                    }
                    None => return None, // Return early
                };

                let mut current_acc_type = match acc_type {
                    Some(t) => t,
                    None => return None,
                };
                let mut has_to_iterate = true;
                while has_to_iterate {
                    // Register the loop variable with the element type
                    if let Err(e) = self.register_identifier(
                        &var.node,
                        var.span.clone(),
                        elem_t.clone(),
                        type_info,
                        warnings,
                    ) {
                        errors.push(e);
                    }

                    // Register the accumulator variable with the current computed type
                    if let Err(e) = self.register_identifier(
                        &accumulator.node,
                        accumulator.span.clone(),
                        current_acc_type.clone(),
                        type_info,
                        warnings,
                    ) {
                        errors.push(e);
                    }

                    self.push_scope();

                    // Check filter (must be Bool)
                    if let Some(filter_expr) = filter {
                        let filter_type = self.check_expr(
                            global_env,
                            &filter_expr.node,
                            &filter_expr.span,
                            type_info,
                            expr_types,
                            errors,
                            warnings,
                        );

                        if let Some(typ) = filter_type {
                            if !typ.is_bool() {
                                errors.push(SemError::TypeMismatch {
                                    span: filter_expr.span.clone(),
                                    expected: SimpleType::Bool.into(),
                                    found: typ,
                                    context: "fold|rfold filter must be Bool".to_string(),
                                });
                            }
                        }
                    }

                    // Check body (must match accumulator)
                    let body_type = self.check_expr(
                        global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
                    );

                    self.pop_scope(warnings);

                    match body_type {
                        Some(typ) => {
                            has_to_iterate = !typ.is_subtype_of(&current_acc_type);
                            if has_to_iterate {
                                current_acc_type = current_acc_type.unify_with(&typ);
                            }
                        }
                        None => {
                            // This will end the loop and return the last known type
                            // for the accumulator
                            has_to_iterate = false;
                        }
                    }
                }

                Some(current_acc_type)
            }

            // ========== If Expression ==========
            Expr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_type = self.check_expr(
                    global_env,
                    &condition.node,
                    &condition.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                if let Some(typ) = cond_type {
                    if !typ.is_bool() {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: SimpleType::Bool.into(),
                            found: typ,
                            context: "if condition must be Bool".to_string(),
                        });
                    }
                }

                let then_type = self.check_expr(
                    global_env,
                    &then_expr.node,
                    &then_expr.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );
                let else_type = self.check_expr(
                    global_env,
                    &else_expr.node,
                    &else_expr.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                match (then_type, else_type) {
                    (Some(t), Some(e)) => Some(t.unify_with(&e)),
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
            }
            Expr::Match {
                match_expr,
                branches,
            } => {
                let Some(expr_type) = self.check_expr(
                    global_env,
                    &match_expr.node,
                    &match_expr.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                ) else {
                    // Cannot type check anything, propagate underspecified type
                    return None;
                };

                let mut output = Option::<ExprType>::None;
                let mut current_type = Some(expr_type);

                for branch in branches {
                    let as_type = if let Some(bt) = &branch.as_typ {
                        match ExprType::try_from(bt.clone()) {
                            Ok(t) => {
                                if !global_env.validate_type(&t) {
                                    errors.push(SemError::UnknownType {
                                        typ: t.to_string(),
                                        span: bt.span.clone(),
                                    });
                                    continue;
                                }
                                Some(t)
                            }
                            Err(e) => {
                                errors.push(e);
                                continue; // Can't evaluate branch
                            }
                        }
                    } else {
                        None
                    };

                    let into_type = if let Some(it) = &branch.into_typ {
                        match ExprType::try_from(it.clone()) {
                            Ok(t) => {
                                if !global_env.validate_type(&t) {
                                    errors.push(SemError::UnknownType {
                                        typ: t.to_string(),
                                        span: it.span.clone(),
                                    });
                                    // We could ignore the branch but we try to continue without type conversion
                                    None
                                } else if !t.is_concrete() {
                                    errors.push(SemError::NonConcreteType {
                                        span: it.span.clone(),
                                        found: t,
                                        context: "Type conversion requires a concrete target type"
                                            .to_string(),
                                    });
                                    None
                                } else {
                                    let concrete_target =
                                        t.to_simple().unwrap().into_concrete().unwrap();
                                    Some(concrete_target)
                                }
                            }
                            Err(e) => {
                                errors.push(e);
                                // Probably can't evaluate branch, but continue anyway without type conversion
                                None
                            }
                        }
                    } else {
                        None
                    };

                    let bad_branch_typ = match &current_type {
                        Some(typ) => match &as_type {
                            Some(b_typ) => !b_typ.is_subtype_of(typ),
                            None => false,
                        },
                        None => true,
                    };

                    if bad_branch_typ {
                        errors.push(SemError::OverMatching {
                            span: match &branch.as_typ {
                                Some(t) => t.span.clone(),
                                None => branch.ident.span.clone(),
                            },
                            expected: current_type.clone(),
                            found: as_type.clone(),
                        });
                    }

                    let actual_branch_typ_opt = match as_type {
                        Some(typ) => Some(typ),
                        None => current_type.clone(),
                    };

                    if let Some(actual_branch_typ) = actual_branch_typ_opt {
                        let typ_in_branch = if let Some(concrete_target) = into_type {
                            if !actual_branch_typ.can_convert_to(&concrete_target) {
                                // Error: can't convert
                                errors.push(SemError::ImpossibleConversion {
                                    span: match_expr.span.clone(),
                                    found: actual_branch_typ.clone(),
                                    target: concrete_target.clone(),
                                });
                                actual_branch_typ.clone()
                            } else {
                                concrete_target.into_inner().into()
                            }
                        } else {
                            actual_branch_typ.clone()
                        };

                        if let Err(e) = self.register_identifier(
                            &branch.ident.node,
                            branch.ident.span.clone(),
                            typ_in_branch,
                            type_info,
                            warnings,
                        ) {
                            panic!("There should be no other identifier in the current scope. But got: {:?}", e);
                        }

                        self.push_scope();

                        if let Some(filter_expr) = &branch.filter {
                            let filter_type = self.check_expr(
                                global_env,
                                &filter_expr.node,
                                &filter_expr.span,
                                type_info,
                                expr_types,
                                errors,
                                warnings,
                            );

                            if let Some(typ) = filter_type {
                                if !typ.is_bool() {
                                    errors.push(SemError::TypeMismatch {
                                        span: filter_expr.span.clone(),
                                        expected: SimpleType::Bool.into(),
                                        found: typ,
                                        context: "where filter must be Bool".to_string(),
                                    });
                                }
                            }
                        }

                        let body_typ = self.check_expr(
                            global_env,
                            &branch.body.node,
                            &branch.body.span,
                            type_info,
                            expr_types,
                            errors,
                            warnings,
                        );

                        self.pop_scope(warnings);

                        // Update output type
                        if let Some(typ) = body_typ {
                            output = Some(match output {
                                Some(x) => x.unify_with(&typ),
                                None => typ,
                            });
                        }

                        // Update remaining type
                        if branch.filter.is_none() {
                            if let Some(typ) = current_type {
                                current_type = typ.substract(&actual_branch_typ);
                            }
                        }
                    }
                }

                if let Some(remaining_types) = current_type {
                    errors.push(SemError::NonExhaustiveMatching {
                        span: global_span.clone(),
                        remaining_types,
                    });
                }

                output
            }

            // ========== ILP Variables ==========
            Expr::VarCall { name, args } => {
                match global_env.lookup_var(&name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                        Some(SimpleType::LinExpr.into()) // Syntax indicates LinExpr intent
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
                            let arg_type = self.check_expr(
                                global_env, &arg.node, &arg.span, type_info, expr_types, errors,
                                warnings,
                            );

                            if let Some(found_type) = arg_type {
                                if !found_type.is_subtype_of(expected_type) {
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

                        Some(SimpleType::LinExpr.into())
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
                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
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
                            let arg_type = self.check_expr(
                                global_env, &arg.node, &arg.span, type_info, expr_types, errors,
                                warnings,
                            );

                            if let Some(found_type) = arg_type {
                                if !found_type.is_subtype_of(expected_type) {
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

                        Some(SimpleType::List(SimpleType::LinExpr.into()).into())
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
                        let arg_type = self.check_expr(
                            global_env, &arg.node, &arg.span, type_info, expr_types, errors,
                            warnings,
                        );

                        if let Some(found_type) = arg_type {
                            if !found_type.is_subtype_of(expected_type) {
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
                let typ = match ExprType::try_from(type_name.clone()) {
                    Ok(t) => t,
                    Err(e) => {
                        errors.push(e);
                        return None;
                    }
                };
                if !global_env.validate_type(&typ) {
                    errors.push(SemError::UnknownType {
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else if !typ.is_sum_of_objects() {
                    errors.push(SemError::GlobalCollectionsMustBeAListOfObjects {
                        typ: typ.to_string(),
                        span: type_name.span.clone(),
                    });
                    None
                } else {
                    Some(SimpleType::List(typ).into())
                }
            }

            Expr::ListLiteral { elements } => {
                if elements.is_empty() {
                    return Some(SimpleType::EmptyList.into());
                }

                // Check all elements and unify their types
                let mut unified_type = self.check_expr(
                    global_env,
                    &elements[0].node,
                    &elements[0].span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

                for item in &elements[1..] {
                    let item_type = self.check_expr(
                        global_env, &item.node, &item.span, type_info, expr_types, errors, warnings,
                    );

                    match (unified_type.clone(), item_type) {
                        (Some(u), Some(i)) => {
                            unified_type = Some(u.unify_with(&i));
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

                match unified_type {
                    Some(t) => Some(SimpleType::List(t).into()),
                    None => None, // Inner None does not imply [<unknown>] because this is reserved for empty lists
                }
            }

            Expr::ListRange { start, end } => {
                let start_type = self.check_expr(
                    global_env,
                    &start.node,
                    &start.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );
                let end_type = self.check_expr(
                    global_env, &end.node, &end.span, type_info, expr_types, errors, warnings,
                );

                if let (Some(s), Some(e)) = (start_type, end_type) {
                    // Check if both can coerce to Int
                    let s_ok = s.is_int();
                    let e_ok = e.is_int();

                    if !s_ok {
                        errors.push(SemError::TypeMismatch {
                            span: start.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: s,
                            context: "list range requires Int limits".to_string(),
                        });
                    }
                    if !e_ok {
                        errors.push(SemError::TypeMismatch {
                            span: end.span.clone(),
                            expected: SimpleType::Int.into(),
                            found: e,
                            context: "list range requires Int limits".to_string(),
                        });
                    }
                }
                // Always return [Int] (even on error, intent is clear)
                Some(SimpleType::List(SimpleType::Int.into()).into())
            }

            Expr::ListComprehension {
                body: expr,
                vars_and_collections,
                filter,
            } => {
                for (i, (var, collection)) in vars_and_collections.iter().enumerate() {
                    let mut typ_error = false;

                    let coll_type = self.check_expr(
                        global_env,
                        &collection.node,
                        &collection.span,
                        type_info,
                        expr_types,
                        errors,
                        warnings,
                    );

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
                    match coll_type {
                        Some(a) if a.is_empty_list() => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: SimpleType::List(SimpleType::Int.into()).into(),
                                found: a.clone(),
                                context: "list comprehension collection inner type must be known (use 'as' for explicit typing)".to_string(),
                            });
                            typ_error = true;
                        }
                        Some(a) if a.is_list() => {
                            let elem_t = a
                                .get_inner_list_type()
                                .expect("List should not be empty at this point");
                            // Register the loop variable with the element type
                            if let Err(e) = self.register_identifier(
                                &var.node,
                                var.span.clone(),
                                elem_t,
                                type_info,
                                warnings,
                            ) {
                                errors.push(e);
                                typ_error = true;
                            }
                        }
                        Some(t) => {
                            errors.push(SemError::TypeMismatch {
                                span: collection.span.clone(),
                                expected: SimpleType::List(t.clone()).into(),
                                found: t,
                                context: "list comprehension collection must be a list".to_string(),
                            });
                            typ_error = true;
                        }
                        None => typ_error = true,
                    }

                    if typ_error {
                        for _j in 0..i {
                            let mut ignored_warnings = vec![];
                            self.pop_scope(&mut ignored_warnings);
                        }
                        return None;
                    }

                    self.push_scope();
                }

                // Check filter (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_expr(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        expr_types,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if !typ.is_bool() {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: SimpleType::Bool.into(),
                                found: typ,
                                context: "list comprehension filter must be Bool".to_string(),
                            });
                        }
                    }
                }

                // Check the output expression - this determines the result element type
                let elem_type = self.check_expr(
                    global_env, &expr.node, &expr.span, type_info, expr_types, errors, warnings,
                );

                for (_var, _collection) in vars_and_collections {
                    self.pop_scope(warnings);
                }

                match elem_type {
                    Some(t) => Some(SimpleType::List(t).into()),
                    None => None, // Inner None does not imply [<unknown>] because this is reserved for empty lists
                }
            }

            // ========== Cardinality ==========
            Expr::Cardinality(collection) => {
                let elem_t = self.check_expr(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );
                match elem_t {
                    Some(t) if t.is_list() => (),
                    None => (),
                    Some(t) => {
                        errors.push(SemError::TypeMismatch {
                            span: collection.span.clone(),
                            expected: SimpleType::List(t.clone()).into(),
                            found: t,
                            context: "cardinality is always computed on a collection".to_string(),
                        });
                    }
                }
                Some(SimpleType::Int.into()) // Cardinality always gives an Int
            }

            // ========== Let construct ==========
            Expr::Let { var, value, body } => {
                let value_type = self.check_expr(
                    global_env,
                    &value.node,
                    &value.span,
                    type_info,
                    expr_types,
                    errors,
                    warnings,
                );

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
                match value_type {
                    Some(typ) => {
                        if let Err(e) = self.register_identifier(
                            &var.node,
                            var.span.clone(),
                            typ,
                            type_info,
                            warnings,
                        ) {
                            errors.push(e);
                            return None;
                        }
                    }
                    None => return None,
                }

                self.push_scope();

                let body_type = self.check_expr(
                    global_env, &body.node, &body.span, type_info, expr_types, errors, warnings,
                );

                self.pop_scope(warnings);

                body_type
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
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<ExprType> {
        assert!(!segments.is_empty(), "Path must have at least one segment");

        // First segment can be an expression
        let mut current_type = self.check_expr(
            global_env,
            &object.node,
            &object.span,
            type_info,
            expr_types,
            errors,
            warnings,
        )?;

        // Follow the path through fields
        for segment in segments {
            let mut variants = BTreeSet::new();
            for variant in current_type.get_variants() {
                match variant {
                    a if a.is_object() => {
                        let type_name = a.get_inner_object_type().unwrap();
                        // Look up the field in this object type
                        match global_env.lookup_field(type_name, &segment.node) {
                            Some(field_type) => {
                                variants.extend(field_type.into_variants());
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
                            typ: current_type.clone().into(),
                            field: segment.node.clone(),
                            span: segment.span.clone(),
                        });
                        return None;
                    }
                }
            }
            current_type = ExprType::sum(variants).expect("There should be at least one variant");
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
    ) -> Result<
        (
            Self,
            TypeInfo,
            HashMap<Span, ExprType>,
            Vec<SemError>,
            Vec<SemWarning>,
        ),
        GlobalEnvError,
    > {
        let mut temp_env = GlobalEnv {
            defined_types,
            functions: HashMap::new(),
            external_variables: variables
                .into_iter()
                .map(|(var_name, args_type)| (var_name, args_type))
                .collect(),
            internal_variables: HashMap::new(),
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

        for (var, args) in &temp_env.external_variables {
            for (param, typ) in args.iter().enumerate() {
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
        let mut expr_types = HashMap::new();
        let mut errors = vec![];
        let mut warnings = vec![];

        for statement in &file.statements {
            temp_env.expand_with_statement(
                &statement.node,
                &mut type_info,
                &mut expr_types,
                &mut errors,
                &mut warnings,
            );
        }

        temp_env.check_unused_fn(&mut warnings);
        temp_env.check_unused_var(&mut warnings);

        Ok((temp_env, type_info, expr_types, errors, warnings))
    }

    fn check_unused_fn(&self, warnings: &mut Vec<SemWarning>) {
        for (name, fn_desc) in &self.functions {
            if !fn_desc.public && !fn_desc.used {
                warnings.push(SemWarning::UnusedFunction {
                    identifier: name.clone(),
                    span: fn_desc.body.span.clone(),
                });
            }
        }
    }

    fn check_unused_var(&self, warnings: &mut Vec<SemWarning>) {
        for (name, var_desc) in &self.internal_variables {
            if !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: name.clone(),
                    span: var_desc.span.clone(),
                });
            }
        }

        for (name, var_desc) in &self.variable_lists {
            if !var_desc.used {
                warnings.push(SemWarning::UnusedVariable {
                    identifier: name.clone(),
                    span: var_desc.span.clone(),
                });
            }
        }
    }

    fn expand_with_statement(
        &mut self,
        statement: &crate::ast::Statement,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        match statement {
            crate::ast::Statement::Let {
                docstring,
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
                docstring,
                type_info,
                expr_types,
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
        docstring: &Vec<String>,
        type_info: &mut TypeInfo,
        expr_types: &mut HashMap<Span, ExprType>,
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
                let mut params_typ = vec![];
                for param in params {
                    match ExprType::try_from(param.typ.clone()) {
                        Err(e) => {
                            errors.push(e);
                            error_in_typs = true;
                        }
                        Ok(param_typ) => {
                            params_typ.push(param_typ.clone());
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
                                if let Err(e) = local_env.register_identifier(
                                    &param.name.node,
                                    param.name.span.clone(),
                                    param_typ,
                                    type_info,
                                    warnings,
                                ) {
                                    errors.push(e);
                                }
                            }
                        }
                    }
                }

                local_env.push_scope();
                let body_type_opt = local_env.check_expr(
                    self, &body.node, &body.span, type_info, expr_types, errors, warnings,
                );
                local_env.pop_scope(warnings);

                match ExprType::try_from(output_type.clone()) {
                    Err(e) => {
                        errors.push(e);
                    }
                    Ok(out_typ) => {
                        if !self.validate_type(&out_typ) {
                            errors.push(SemError::UnknownType {
                                typ: out_typ.to_string(),
                                span: output_type.span.clone(),
                            });
                        } else {
                            if let Some(body_type) = body_type_opt {
                                // Allow subtyping
                                let types_match = match (out_typ.clone(), body_type.clone()) {
                                    (a, b) if b.is_subtype_of(&a) => true,
                                    _ => false,
                                };

                                if !types_match {
                                    errors.push(SemError::BodyTypeMismatch {
                                        func: name.node.clone(),
                                        span: body.span.clone(),
                                        expected: out_typ.clone(),
                                        found: body_type,
                                    });
                                }
                            }
                        }

                        if !error_in_typs {
                            let fn_typ = FunctionType {
                                args: params_typ,
                                output: out_typ,
                            };
                            self.register_fn(
                                &name.node,
                                name.span.clone(),
                                fn_typ,
                                public,
                                params.iter().map(|x| x.name.node.clone()).collect(),
                                body.clone(),
                                docstring.clone(),
                                type_info,
                            );
                        }
                    }
                };
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
                let needed_output_type = ExprType::simple(if var_list {
                    SimpleType::List(SimpleType::Constraint.into())
                } else {
                    SimpleType::Constraint
                });
                let correct_type = fn_type.0.output == needed_output_type;
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
                                constraint_name.node.clone(),
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
                                constraint_name.node.clone(),
                                type_info,
                            );
                        }
                    }
                }
            }
        }
    }
}
