use crate::ast::{Expr, Param, Span, Spanned};
use std::collections::HashMap;

mod string_case;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputType {
    Int,
    Bool,
    Object(String),
    List(Box<InputType>),
}

impl From<crate::ast::InputType> for InputType {
    fn from(value: crate::ast::InputType) -> Self {
        use crate::ast::InputType as IT;
        match value {
            IT::Bool => InputType::Bool,
            IT::Int => InputType::Int,
            IT::Object(name) => InputType::Object(name),
            IT::List(sub_typ) => InputType::List(Box::new((*sub_typ).into())),
        }
    }
}

impl std::fmt::Display for InputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputType::Bool => write!(f, "Bool"),
            InputType::Int => write!(f, "Int"),
            InputType::List(sub_type) => write!(f, "[{}]", sub_type.as_ref()),
            InputType::Object(typ) => write!(f, "{}", typ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    public: bool,
    args: ArgsType,
    output: OutputType,
}

impl std::fmt::Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args_types: Vec<_> = self.args.iter().map(|x| x.to_string()).collect();
        write!(f, "({}) -> {}", args_types.join(", "), self.output)
    }
}

pub type ArgsType = Vec<InputType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputType {
    LinExpr,
    Constraint,
}

impl From<crate::ast::OutputType> for OutputType {
    fn from(value: crate::ast::OutputType) -> Self {
        use crate::ast::OutputType as OT;
        match value {
            OT::Constraint => OutputType::Constraint,
            OT::LinExpr => OutputType::LinExpr,
        }
    }
}

impl std::fmt::Display for OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputType::LinExpr => write!(f, "LinExpr"),
            OutputType::Constraint => write!(f, "Constraint"),
        }
    }
}

pub type ObjectFields = HashMap<String, InputType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEnv {
    defined_types: HashMap<String, ObjectFields>,
    functions: HashMap<String, (FunctionType, Span)>,
    variables: HashMap<String, (ArgsType, Option<Span>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeInfo {
    types: HashMap<crate::ast::Span, GenericType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericType {
    Function(FunctionType),
    Input(InputType),
    Variable(ArgsType),
}

impl From<FunctionType> for GenericType {
    fn from(value: FunctionType) -> Self {
        GenericType::Function(value)
    }
}

impl From<InputType> for GenericType {
    fn from(value: InputType) -> Self {
        GenericType::Input(value)
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
            GenericType::Input(typ) => write!(f, "{}", typ),
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
    pub fn new(
        defined_types: HashMap<String, ObjectFields>,
        variables: HashMap<String, ArgsType>,
    ) -> Result<Self, GlobalEnvError> {
        let temp_env = GlobalEnv {
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

        Ok(temp_env)
    }

    fn validate_type(&self, typ: &InputType) -> bool {
        match typ {
            InputType::Bool => true,
            InputType::Int => true,
            InputType::List(sub_typ) => self.validate_type(sub_typ.as_ref()),
            InputType::Object(typ_name) => self.defined_types.contains_key(typ_name),
        }
    }

    pub fn lookup_fn(&self, name: &str) -> Option<(FunctionType, Span)> {
        self.functions.get(name).cloned()
    }

    fn register_fn(
        &mut self,
        name: &str,
        fn_typ: FunctionType,
        span: Span,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.functions.contains_key(name));

        self.functions
            .insert(name.to_string(), (fn_typ.clone(), span.clone()));

        type_info.types.insert(span, fn_typ.into());
    }

    pub fn lookup_var(&self, name: &str) -> Option<(ArgsType, Option<Span>)> {
        self.variables.get(name).cloned()
    }

    fn register_var(
        &mut self,
        name: &str,
        args_typ: ArgsType,
        span: Span,
        type_info: &mut TypeInfo,
    ) {
        assert!(!self.variables.contains_key(name));

        self.variables
            .insert(name.to_string(), (args_typ.clone(), Some(span.clone())));

        type_info.types.insert(span, args_typ.into());
    }

    fn lookup_field(&self, obj_type: &str, field: &str) -> Option<InputType> {
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
    #[error("Input type {typ} at {span:?} is unknown")]
    UnknownInputType { typ: String, span: Span },
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
        expected: OutputType,
        found: OutputType,
    },
    #[error("Type mismatch at {span:?}: expected {expected} but found {found} ({context})")]
    TypeMismatch {
        span: Span,
        expected: InputType,
        found: InputType,
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
        typ: InputType,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct LocalEnv {
    scopes: Vec<HashMap<String, (InputType, Span)>>,
    current_scope: HashMap<String, (InputType, Span)>,
}

impl LocalEnv {
    fn new() -> Self {
        LocalEnv::default()
    }

    fn lookup_in_current_scope(&self, ident: &str) -> Option<(InputType, Span)> {
        self.current_scope.get(ident).cloned()
    }

    fn lookup_ident(&self, ident: &str) -> Option<(InputType, Span)> {
        // We don't look in current scope as these variables are not yet accessible
        for scope in self.scopes.iter().rev() {
            let Some(val) = scope.get(ident) else {
                continue;
            };
            return Some(val.clone());
        }
        None
    }

    fn push_scope(&mut self) {
        let mut old_scope = HashMap::new();
        std::mem::swap(&mut old_scope, &mut self.current_scope);

        self.scopes.push(old_scope);
    }

    fn pop_scope(&mut self) {
        assert!(!self.scopes.is_empty());
        self.current_scope = self.scopes.pop().unwrap();
    }

    fn register_identifier(
        &mut self,
        ident: &str,
        span: Span,
        typ: InputType,
        type_info: &mut TypeInfo,
        warnings: &mut Vec<SemWarning>,
    ) {
        assert!(!self.current_scope.contains_key(ident));

        if let Some((_typ, old_span)) = self.lookup_ident(ident) {
            warnings.push(SemWarning::IdentifierShadowed {
                identifier: ident.to_string(),
                span: span.clone(),
                previous: old_span,
            });
        }

        self.current_scope
            .insert(ident.to_string(), (typ.clone(), span.clone()));
        type_info.types.insert(span, typ.into());
    }

    fn check_lin_expr(
        &mut self,
        global_env: &GlobalEnv,
        lin_expr: &crate::ast::LinExpr,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        use crate::ast::LinExpr;

        match lin_expr {
            LinExpr::Var { name, args } => {
                // Look up ILP variable
                match global_env.lookup_var(&name.node) {
                    None => {
                        errors.push(SemError::UnknownVariable {
                            var: name.node.clone(),
                            span: name.span.clone(),
                        });
                    }
                    Some((var_args, _)) => {
                        // Check argument count
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

                        // Check argument types
                        for (i, (arg, expected_type)) in
                            args.iter().zip(var_args.iter()).enumerate()
                        {
                            let arg_type = self.check_computable(
                                global_env, &arg.node, &arg.span, type_info, errors, warnings,
                            );

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
                    }
                }
            }

            LinExpr::Constant(comp) => {
                // Check the computable and ensure it's Int
                let comp_type = self.check_computable(
                    global_env, &comp.node, &comp.span, type_info, errors, warnings,
                );

                if let Some(typ) = comp_type {
                    if typ != InputType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: comp.span.clone(),
                            expected: InputType::Int,
                            found: typ,
                            context: "linear expression constant must be Int".to_string(),
                        });
                    }
                }
            }

            LinExpr::Add(left, right) | LinExpr::Sub(left, right) => {
                // Check both sides recursively
                self.check_lin_expr(global_env, &left.node, type_info, errors, warnings);
                self.check_lin_expr(global_env, &right.node, type_info, errors, warnings);
            }

            LinExpr::Mul { coeff, expr } => {
                // Check coefficient is Int
                let coeff_type = self.check_computable(
                    global_env,
                    &coeff.node,
                    &coeff.span,
                    type_info,
                    errors,
                    warnings,
                );

                if let Some(typ) = coeff_type {
                    if typ != InputType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: coeff.span.clone(),
                            expected: InputType::Int,
                            found: typ,
                            context: "coefficient in linear expression must be Int".to_string(),
                        });
                    }
                }

                // Check the linear expression
                self.check_lin_expr(global_env, &expr.node, type_info, errors, warnings);
            }

            LinExpr::Sum {
                var,
                collection,
                filter,
                body,
            } => {
                // Check the collection is valid
                let element_type = self.check_collection(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Check naming convention for loop variable
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

                // Register loop variable
                if let Some(elem_type) = element_type {
                    self.register_identifier(
                        &var.node,
                        var.span.clone(),
                        elem_type,
                        type_info,
                        warnings,
                    );
                }

                self.push_scope();

                // Check filter if present (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_computable(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if typ != InputType::Bool {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: InputType::Bool,
                                found: typ,
                                context: "sum filter must be a boolean expression".to_string(),
                            });
                        }
                    }
                }

                // Check body is a valid LinExpr
                self.check_lin_expr(global_env, &body.node, type_info, errors, warnings);

                self.pop_scope();
            }

            LinExpr::If {
                condition,
                then_expr,
                else_expr,
            } => {
                // Check condition is Bool
                let cond_type = self.check_computable(
                    global_env,
                    &condition.node,
                    &condition.span,
                    type_info,
                    errors,
                    warnings,
                );

                if let Some(typ) = cond_type {
                    if typ != InputType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: InputType::Bool,
                            found: typ,
                            context: "if condition must be a boolean expression".to_string(),
                        });
                    }
                }

                // Check both branches
                self.check_lin_expr(global_env, &then_expr.node, type_info, errors, warnings);
                self.check_lin_expr(global_env, &else_expr.node, type_info, errors, warnings);
            }

            LinExpr::FnCall { name, args } => {
                // Look up function
                match global_env.lookup_fn(&name.node) {
                    None => {
                        errors.push(SemError::UnknownIdentifer {
                            identifier: name.node.clone(),
                            span: name.span.clone(),
                        });
                    }
                    Some((fn_type, _)) => {
                        // Check it returns LinExpr
                        if fn_type.output != OutputType::LinExpr {
                            errors.push(SemError::FunctionTypeMismatch {
                                identifier: name.node.clone(),
                                span: name.span.clone(),
                                expected: FunctionType {
                                    output: OutputType::LinExpr,
                                    ..fn_type.clone()
                                },
                                found: fn_type.clone(),
                            });
                        }

                        // Check argument count
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

                        // Check argument types
                        for (i, (arg, expected_type)) in
                            args.iter().zip(fn_type.args.iter()).enumerate()
                        {
                            let arg_type = self.check_computable(
                                global_env, &arg.node, &arg.span, type_info, errors, warnings,
                            );

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
                    }
                }
            }
        }
    }

    fn check_constraint(
        &mut self,
        global_env: &GlobalEnv,
        constraint_expr: &crate::ast::Constraint,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) {
        use crate::ast::Constraint;

        match constraint_expr {
            Constraint::Comparison { left, op: _, right } => {
                // Check both sides are valid LinExpr
                self.check_lin_expr(global_env, &left.node, type_info, errors, warnings);
                self.check_lin_expr(global_env, &right.node, type_info, errors, warnings);
                // op doesn't need checking - it's just a comparison operator
            }

            Constraint::And(left, right) => {
                // Check both constraints recursively
                self.check_constraint(global_env, &left.node, type_info, errors, warnings);
                self.check_constraint(global_env, &right.node, type_info, errors, warnings);
            }

            Constraint::Forall {
                var,
                collection,
                filter,
                body,
            } => {
                // Check the collection is valid
                let element_type = self.check_collection(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Check naming convention for loop variable
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

                // Register loop variable in a new scope
                if let Some(elem_type) = element_type {
                    self.register_identifier(
                        &var.node,
                        var.span.clone(),
                        elem_type,
                        type_info,
                        warnings,
                    );
                }

                self.push_scope();

                // Check filter if present (must be Bool)
                if let Some(filter_expr) = filter {
                    let filter_type = self.check_computable(
                        global_env,
                        &filter_expr.node,
                        &filter_expr.span,
                        type_info,
                        errors,
                        warnings,
                    );

                    if let Some(typ) = filter_type {
                        if typ != InputType::Bool {
                            errors.push(SemError::TypeMismatch {
                                span: filter_expr.span.clone(),
                                expected: InputType::Bool,
                                found: typ,
                                context: "forall filter must be a boolean expression".to_string(),
                            });
                        }
                    }
                }

                // Check body constraint
                self.check_constraint(global_env, &body.node, type_info, errors, warnings);

                self.pop_scope();
            }

            Constraint::If {
                condition,
                then_expr,
                else_expr,
            } => {
                // Check condition is Bool
                let cond_type = self.check_computable(
                    global_env,
                    &condition.node,
                    &condition.span,
                    type_info,
                    errors,
                    warnings,
                );

                if let Some(typ) = cond_type {
                    if typ != InputType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: InputType::Bool,
                            found: typ,
                            context: "if condition must be a boolean expression".to_string(),
                        });
                    }
                }

                // Check both branches
                self.check_constraint(global_env, &then_expr.node, type_info, errors, warnings);
                self.check_constraint(global_env, &else_expr.node, type_info, errors, warnings);
            }

            Constraint::FnCall { name, args } => {
                // Look up function
                match global_env.lookup_fn(&name.node) {
                    None => {
                        errors.push(SemError::UnknownIdentifer {
                            identifier: name.node.clone(),
                            span: name.span.clone(),
                        });
                    }
                    Some((fn_type, _)) => {
                        // Check it returns Constraint
                        if fn_type.output != OutputType::Constraint {
                            errors.push(SemError::FunctionTypeMismatch {
                                identifier: name.node.clone(),
                                span: name.span.clone(),
                                expected: FunctionType {
                                    output: OutputType::Constraint,
                                    ..fn_type.clone()
                                },
                                found: fn_type.clone(),
                            });
                        }

                        // Check argument count
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

                        // Check argument types
                        for (i, (arg, expected_type)) in
                            args.iter().zip(fn_type.args.iter()).enumerate()
                        {
                            let arg_type = self.check_computable(
                                global_env, &arg.node, &arg.span, type_info, errors, warnings,
                            );

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
                    }
                }
            }
        }
    }

    fn check_collection(
        &mut self,
        global_env: &GlobalEnv,
        collection: &crate::ast::Collection,
        span: &Span,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<InputType> {
        use crate::ast::Collection;

        match collection {
            Collection::Global(type_name) => {
                // @[Student] - global collection of a type
                // Verify the type exists
                let obj_type = InputType::Object(type_name.node.clone());
                if !global_env.validate_type(&obj_type) {
                    errors.push(SemError::UnknownInputType {
                        typ: type_name.node.clone(),
                        span: type_name.span.clone(),
                    });
                    None
                } else {
                    // Return the element type (the object type itself)
                    Some(obj_type)
                }
            }

            Collection::Path(path) => {
                // a field that should be a list
                let path_type =
                    self.check_path(global_env, path, span, type_info, errors, warnings);

                match path_type {
                    Some(InputType::List(inner)) => {
                        // It's a list, return the element type
                        Some(*inner)
                    }
                    Some(other) => {
                        // It's not a list!
                        errors.push(SemError::TypeMismatch {
                            span: span.clone(),
                            expected: InputType::List(Box::new(other.clone())), // Placeholder
                            found: other,
                            context: "collection must be a list type".to_string(),
                        });
                        None
                    }
                    None => {
                        // Error already reported by check_path
                        None
                    }
                }
            }

            Collection::Union(left, right) => {
                // left | right - both must be collections of the same type
                let left_elem = self.check_collection(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_elem = self.check_collection(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                match (left_elem, right_elem) {
                    (Some(left_type), Some(right_type)) => {
                        if left_type == right_type {
                            Some(left_type)
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: left_type.clone(),
                                found: right_type,
                                context: "union operands must have the same element type"
                                    .to_string(),
                            });
                            Some(left_type) // Return left type as fallback
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => Some(t), // One side errored, use the other
                    (None, None) => None,                         // Both errored
                }
            }

            Collection::Inter(left, right) => {
                // left & right - both must be collections of the same type
                let left_elem = self.check_collection(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_elem = self.check_collection(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                match (left_elem, right_elem) {
                    (Some(left_type), Some(right_type)) => {
                        if left_type == right_type {
                            Some(left_type)
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: left_type.clone(),
                                found: right_type,
                                context: "intersection operands must have the same element type"
                                    .to_string(),
                            });
                            Some(left_type)
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
            }

            Collection::Diff(left, right) => {
                // left \ right - both must be collections of the same type
                let left_elem = self.check_collection(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_elem = self.check_collection(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                match (left_elem, right_elem) {
                    (Some(left_type), Some(right_type)) => {
                        if left_type == right_type {
                            Some(left_type)
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: right.span.clone(),
                                expected: left_type.clone(),
                                found: right_type,
                                context: "difference operands must have the same element type"
                                    .to_string(),
                            });
                            Some(left_type)
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
            }
        }
    }

    fn check_computable(
        &mut self,
        global_env: &GlobalEnv,
        computable: &crate::ast::Computable,
        span: &Span,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
        warnings: &mut Vec<SemWarning>,
    ) -> Option<InputType> {
        use crate::ast::Computable;

        match computable {
            Computable::Number(_) => {
                // Numbers are always Int
                Some(InputType::Int)
            }

            Computable::Path(path) => {
                // Check the path and return its type
                self.check_path(global_env, path, span, type_info, errors, warnings)
            }

            // Arithmetic operations - all require Int operands and return Int
            Computable::Add(left, right)
            | Computable::Sub(left, right)
            | Computable::Mul(left, right)
            | Computable::Div(left, right)
            | Computable::Mod(left, right) => {
                let left_type = self.check_computable(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_type = self.check_computable(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Check both are Int
                if let Some(typ) = left_type {
                    if typ != InputType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: InputType::Int,
                            found: typ,
                            context: "arithmetic operation requires Int operands".to_string(),
                        });
                    }
                }

                if let Some(typ) = right_type {
                    if typ != InputType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: InputType::Int,
                            found: typ,
                            context: "arithmetic operation requires Int operands".to_string(),
                        });
                    }
                }

                // Result is Int
                Some(InputType::Int)
            }

            // Comparison operations - operands must be same type, result is Bool
            Computable::Eq(left, right) | Computable::Ne(left, right) => {
                let left_type = self.check_computable(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_type = self.check_computable(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Check types match
                if let (Some(l), Some(r)) = (left_type, right_type) {
                    if l != r {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: l,
                            found: r,
                            context: "equality comparison requires matching types".to_string(),
                        });
                    }
                }

                // Result is Bool
                Some(InputType::Bool)
            }

            // Relational comparisons - require Int operands, result is Bool
            Computable::Lt(left, right)
            | Computable::Le(left, right)
            | Computable::Gt(left, right)
            | Computable::Ge(left, right) => {
                let left_type = self.check_computable(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_type = self.check_computable(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Check both are Int
                if let Some(typ) = left_type {
                    if typ != InputType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: InputType::Int,
                            found: typ,
                            context: "relational comparison requires Int operands".to_string(),
                        });
                    }
                }

                if let Some(typ) = right_type {
                    if typ != InputType::Int {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: InputType::Int,
                            found: typ,
                            context: "relational comparison requires Int operands".to_string(),
                        });
                    }
                }

                // Result is Bool
                Some(InputType::Bool)
            }

            // Boolean operations - require Bool operands, return Bool
            Computable::And(left, right) | Computable::Or(left, right) => {
                let left_type = self.check_computable(
                    global_env, &left.node, &left.span, type_info, errors, warnings,
                );

                let right_type = self.check_computable(
                    global_env,
                    &right.node,
                    &right.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Check both are Bool
                if let Some(typ) = left_type {
                    if typ != InputType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: left.span.clone(),
                            expected: InputType::Bool,
                            found: typ,
                            context: "boolean operation requires Bool operands".to_string(),
                        });
                    }
                }

                if let Some(typ) = right_type {
                    if typ != InputType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: right.span.clone(),
                            expected: InputType::Bool,
                            found: typ,
                            context: "boolean operation requires Bool operands".to_string(),
                        });
                    }
                }

                // Result is Bool
                Some(InputType::Bool)
            }

            Computable::Not(expr) => {
                let expr_type = self.check_computable(
                    global_env, &expr.node, &expr.span, type_info, errors, warnings,
                );

                // Check it's Bool
                if let Some(typ) = expr_type {
                    if typ != InputType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: expr.span.clone(),
                            expected: InputType::Bool,
                            found: typ,
                            context: "not operation requires Bool operand".to_string(),
                        });
                    }
                }

                // Result is Bool
                Some(InputType::Bool)
            }

            Computable::In { item, collection } => {
                // Check item type
                let item_type = self.check_computable(
                    global_env, &item.node, &item.span, type_info, errors, warnings,
                );

                // Check collection and get element type
                let elem_type = self.check_collection(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Verify item type matches collection element type
                if let (Some(item_t), Some(elem_t)) = (item_type, elem_type) {
                    if item_t != elem_t {
                        errors.push(SemError::TypeMismatch {
                            span: item.span.clone(),
                            expected: elem_t,
                            found: item_t,
                            context: "item type must match collection element type".to_string(),
                        });
                    }
                }

                // Result is Bool
                Some(InputType::Bool)
            }

            Computable::Cardinality(collection) => {
                // Check the collection is valid
                self.check_collection(
                    global_env,
                    &collection.node,
                    &collection.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Cardinality always returns Int
                Some(InputType::Int)
            }

            Computable::If {
                condition,
                then_expr,
                else_expr,
            } => {
                // Check condition is Bool
                let cond_type = self.check_computable(
                    global_env,
                    &condition.node,
                    &condition.span,
                    type_info,
                    errors,
                    warnings,
                );

                if let Some(typ) = cond_type {
                    if typ != InputType::Bool {
                        errors.push(SemError::TypeMismatch {
                            span: condition.span.clone(),
                            expected: InputType::Bool,
                            found: typ,
                            context: "if condition must be Bool".to_string(),
                        });
                    }
                }

                // Check both branches
                let then_type = self.check_computable(
                    global_env,
                    &then_expr.node,
                    &then_expr.span,
                    type_info,
                    errors,
                    warnings,
                );

                let else_type = self.check_computable(
                    global_env,
                    &else_expr.node,
                    &else_expr.span,
                    type_info,
                    errors,
                    warnings,
                );

                // Both branches must have same type
                match (then_type, else_type) {
                    (Some(then_t), Some(else_t)) => {
                        if then_t == else_t {
                            Some(then_t)
                        } else {
                            errors.push(SemError::TypeMismatch {
                                span: else_expr.span.clone(),
                                expected: then_t.clone(),
                                found: else_t,
                                context: "if branches must have the same type".to_string(),
                            });
                            Some(then_t) // Return then type as fallback
                        }
                    }
                    (Some(t), None) | (None, Some(t)) => Some(t),
                    (None, None) => None,
                }
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
    ) -> Option<InputType> {
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
                InputType::Object(type_name) => {
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
    pub fn expand(
        &mut self,
        file: &crate::ast::File,
    ) -> (TypeInfo, Vec<SemError>, Vec<SemWarning>) {
        let mut type_info = TypeInfo::new();
        let mut errors = vec![];
        let mut warnings = vec![];

        for statement in &file.statements {
            self.expand_with_statement(&statement.node, &mut type_info, &mut errors, &mut warnings);
        }

        (type_info, errors, warnings)
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
        output_type: &crate::ast::OutputType,
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
                        errors.push(SemError::UnknownInputType {
                            typ: param_typ.to_string(),
                            span: param.typ.span.clone(),
                        });
                        error_in_param_typs = true;
                    } else if let Some((_typ, span)) =
                        local_env.lookup_in_current_scope(&param.name.node)
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

                match &body.node {
                    Expr::LinExpr(_) => {
                        if *output_type != crate::ast::OutputType::LinExpr {
                            errors.push(SemError::BodyTypeMismatch {
                                func: name.node.clone(),
                                span: body.span.clone(),
                                expected: OutputType::Constraint,
                                found: OutputType::LinExpr,
                            });
                        }
                    }
                    Expr::Constraint(_) => {
                        if *output_type != crate::ast::OutputType::Constraint {
                            errors.push(SemError::BodyTypeMismatch {
                                func: name.node.clone(),
                                span: body.span.clone(),
                                expected: OutputType::LinExpr,
                                found: OutputType::Constraint,
                            });
                        }
                    }
                }

                local_env.push_scope();

                match &body.node {
                    Expr::LinExpr(lin_expr) => {
                        local_env.check_lin_expr(self, lin_expr, type_info, errors, warnings)
                    }
                    Expr::Constraint(constraint) => {
                        local_env.check_constraint(self, constraint, type_info, errors, warnings)
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
                OutputType::LinExpr => {
                    let expected_type = FunctionType {
                        output: OutputType::Constraint,
                        ..fn_type.0.clone()
                    };
                    errors.push(SemError::FunctionTypeMismatch {
                        identifier: constraint_name.node.clone(),
                        span: constraint_name.span.clone(),
                        expected: expected_type,
                        found: fn_type.0,
                    });
                }
                OutputType::Constraint => match self.lookup_var(&var_name.node) {
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
            },
        }
    }
}
