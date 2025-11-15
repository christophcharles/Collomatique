use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputType {
    Int,
    Bool,
    Object(String),
    List(Box<InputType>),
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

use crate::ast::Span;

#[derive(Debug, Error)]
pub enum SemError {
    #[error("Unknown identifier \"{identifier}\" at {span:?}")]
    UnknownIdentifer { identifier: String, span: Span },
    #[error("Identifier type mismatch: \"{identifier}\" at {span:?} has type {found} but type {expected} expected.")]
    IdentifierTypeMismatch {
        identifier: String,
        span: Span,
        expected: GenericType,
        found: GenericType,
    },
    #[error("Variable {identifier} at {span:?} is already defined ({here:?})")]
    VariableAlreadyDefined {
        identifier: String,
        span: Span,
        here: Option<Span>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct LocalEnv {
    scopes: Vec<HashMap<String, InputType>>,
}

impl LocalEnv {
    fn new() -> Self {
        LocalEnv::default()
    }
}

impl TypeInfo {
    pub fn new() -> Self {
        TypeInfo::default()
    }
}

impl GlobalEnv {
    pub fn expand(&mut self, file: &crate::ast::File) -> Result<TypeInfo, SemError> {
        let mut type_info = TypeInfo::new();

        for statement in &file.statements {
            self.expand_with_statement(&statement.node, &mut type_info)?;
        }

        Ok(type_info)
    }

    fn expand_with_statement(
        &mut self,
        statement: &crate::ast::Statement,
        type_info: &mut TypeInfo,
    ) -> Result<(), SemError> {
        match statement {
            crate::ast::Statement::Let {
                docstring,
                public,
                name,
                params,
                output_type,
                body,
            } => {
                todo!()
            }

            crate::ast::Statement::Reify {
                docstring: _,
                constraint_name,
                var_name,
            } => match self.lookup_fn(&constraint_name.node) {
                None => {
                    return Err(SemError::UnknownIdentifer {
                        identifier: constraint_name.node.clone(),
                        span: constraint_name.span.clone(),
                    })
                }
                Some(fn_type) => match fn_type.0.output {
                    OutputType::LinExpr => {
                        let expected_type = FunctionType {
                            output: OutputType::Constraint,
                            ..fn_type.0.clone()
                        };
                        return Err(SemError::IdentifierTypeMismatch {
                            identifier: constraint_name.node.clone(),
                            span: constraint_name.span.clone(),
                            expected: expected_type.into(),
                            found: fn_type.0.into(),
                        });
                    }
                    OutputType::Constraint => match self.lookup_var(&var_name.node) {
                        Some((_args, span_opt)) => {
                            return Err(SemError::VariableAlreadyDefined {
                                identifier: var_name.node.clone(),
                                span: var_name.span.clone(),
                                here: span_opt,
                            })
                        }
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
            },
        }
        Ok(())
    }
}
