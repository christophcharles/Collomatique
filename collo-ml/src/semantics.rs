use crate::ast::{Expr, Param, Span, Spanned};
use std::collections::HashMap;

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
}

#[derive(Debug, Error)]
pub enum SemError {
    #[error("Unknown identifier \"{identifier}\" at {span:?}")]
    UnknownIdentifer { identifier: String, span: Span },
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
    ) {
        assert!(!self.current_scope.contains_key(ident));

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
    ) {
        todo!()
    }

    fn check_constraint(
        &mut self,
        global_env: &GlobalEnv,
        constraint_expr: &crate::ast::Constraint,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
    ) {
        todo!()
    }
}

impl TypeInfo {
    pub fn new() -> Self {
        TypeInfo::default()
    }
}

impl GlobalEnv {
    pub fn expand(&mut self, file: &crate::ast::File) -> (TypeInfo, Vec<SemError>) {
        let mut type_info = TypeInfo::new();
        let mut errors = vec![];

        for statement in &file.statements {
            self.expand_with_statement(&statement.node, &mut type_info, &mut errors);
        }

        (type_info, errors)
    }

    fn expand_with_statement(
        &mut self,
        statement: &crate::ast::Statement,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
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
            ),
            crate::ast::Statement::Reify {
                docstring: _,
                constraint_name,
                var_name,
            } => self.expand_with_reify_statement(constraint_name, var_name, type_info, errors),
        }
    }

    fn expand_with_let_statement(
        &mut self,
        public: bool,
        name: &Spanned<String>,
        params: &Vec<Spanned<Param>>,
        output_type: &crate::ast::OutputType,
        body: &Spanned<Expr>,
        type_info: &mut TypeInfo,
        errors: &mut Vec<SemError>,
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
                let mut local_env = LocalEnv::new();
                let mut error_in_param_typs = false;
                for param in params {
                    let param_typ = param.node.typ.clone().into();
                    if !self.validate_type(&param_typ) {
                        errors.push(SemError::UnknownInputType {
                            typ: param_typ.to_string(),
                            span: param.span.clone(),
                        });
                        error_in_param_typs = true;
                    } else if let Some((_typ, span)) =
                        local_env.lookup_in_current_scope(&param.node.name)
                    {
                        errors.push(SemError::ParameterAlreadyDefined {
                            identifier: param.node.name.clone(),
                            span: param.span.clone(),
                            here: span,
                        });
                    } else {
                        local_env.register_identifier(
                            &param.node.name,
                            param.span.clone(),
                            param_typ,
                            type_info,
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
                        local_env.check_lin_expr(self, lin_expr, type_info, errors)
                    }
                    Expr::Constraint(constraint) => {
                        local_env.check_constraint(self, constraint, type_info, errors)
                    }
                }

                if !error_in_param_typs {
                    let args = params
                        .iter()
                        .map(|param| param.node.typ.clone().into())
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
                        expected: expected_type.into(),
                        found: fn_type.0.into(),
                    });
                }
                OutputType::Constraint => match self.lookup_var(&var_name.node) {
                    Some((_args, span_opt)) => errors.push(SemError::VariableAlreadyDefined {
                        identifier: var_name.node.clone(),
                        span: var_name.span.clone(),
                        here: span_opt,
                    }),
                    None => {
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
