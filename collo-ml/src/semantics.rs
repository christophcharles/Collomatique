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

pub type ArgsType = Vec<InputType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputType {
    LinExpr,
    Constraint,
}

pub type ObjectFields = HashMap<String, InputType>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEnv {
    defined_types: HashMap<String, ObjectFields>,
    functions: HashMap<String, FunctionType>,
    variables: HashMap<String, ArgsType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    global: GlobalEnv,
    scopes: Vec<HashMap<String, InputType>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    types: HashMap<crate::ast::Span, InputType>,
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvError {
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
    ) -> Result<Self, EnvError> {
        let temp_env = GlobalEnv {
            defined_types,
            functions: HashMap::new(),
            variables,
        };

        for (object_type, field_desc) in &temp_env.defined_types {
            for (field, typ) in field_desc {
                if !temp_env.validate_type(typ) {
                    return Err(EnvError::UnknownTypeInField {
                        object_type: object_type.clone(),
                        field: field.clone(),
                        unknown_type: typ.to_string(),
                    });
                }
            }
        }

        for (var, args) in &temp_env.variables {
            for (param, typ) in args.iter().enumerate() {
                if !temp_env.validate_type(typ) {
                    return Err(EnvError::UnknownTypeForVariableArg {
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
}
