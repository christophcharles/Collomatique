//! Expression values for evaluation.
//!
//! This module defines the runtime value types:
//! - `ExprValue`: The main enum representing evaluated expressions
//! - `CustomValue`: Data for custom type values
//! - `NoObject`: A placeholder object type for tests without objects
//! - `NoObjectEnv`: Environment for NoObject

use super::variables::{ConstraintWithOrigin, IlpVar, Origin};
use crate::semantics::{ConcreteType, ExprType, SimpleType};
use crate::traits::{EvalObject, FieldConversionError};
use collomatique_ilp::{Constraint, LinExpr};
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    Tuple(Vec<ExprValue<T>>),
    Struct(BTreeMap<String, ExprValue<T>>),
    Custom(Box<CustomValue<T>>),
}

/// Data for custom type values (boxed to keep ExprValue enum small)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomValue<T: EvalObject> {
    /// The module where this type is defined
    pub module: String,
    /// The root type name (e.g., "Result" or "MyType")
    pub type_name: String,
    /// The variant name if this is an enum variant (e.g., Some("Ok") for Result::Ok)
    pub variant: Option<String>,
    pub content: ExprValue<T>,
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
            ExprValue::Tuple(elements) => {
                let strs: Vec<_> = elements.iter().map(|x| x.to_string()).collect();
                write!(f, "({})", strs.join(", "))
            }
            ExprValue::Struct(fields) => {
                let field_strs: Vec<_> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", field_strs.join(", "))
            }
            ExprValue::Custom(custom) => match &custom.variant {
                None => write!(
                    f,
                    "{}::{}({})",
                    custom.module, custom.type_name, custom.content
                ),
                Some(v) => write!(
                    f,
                    "{}::{}::{}({})",
                    custom.module, custom.type_name, v, custom.content
                ),
            },
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
            ExprValue::Tuple(elements) => {
                ExprValue::Tuple(elements.iter().map(|x| x.with_origin(origin)).collect())
            }
            ExprValue::Struct(fields) => ExprValue::Struct(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.with_origin(origin)))
                    .collect(),
            ),
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

    pub fn is_tuple(&self) -> bool {
        matches!(self, Self::Tuple(_))
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
            // Tuples must match element-wise
            Self::Tuple(elements) => {
                for variant in target.get_variants() {
                    let SimpleType::Tuple(target_elems) = variant else {
                        continue;
                    };
                    if elements.len() != target_elems.len() {
                        continue;
                    }
                    if elements
                        .iter()
                        .zip(target_elems.iter())
                        .all(|(e, t)| e.fits_in_typ(env, t))
                    {
                        return true;
                    }
                }
                false
            }
            // Structs must match field-wise
            Self::Struct(fields) => {
                for variant in target.get_variants() {
                    let SimpleType::Struct(target_fields) = variant else {
                        continue;
                    };
                    if fields.len() != target_fields.len() {
                        continue;
                    }
                    if !fields.keys().all(|k| target_fields.contains_key(k)) {
                        continue;
                    }
                    if fields.iter().all(|(k, v)| {
                        target_fields
                            .get(k)
                            .map(|t| v.fits_in_typ(env, t))
                            .unwrap_or(false)
                    }) {
                        return true;
                    }
                }
                false
            }
            // Custom values only fit in Custom types with the same name
            // Also handles subtype relationship: Custom(Root, Some(Variant)) fits in Custom(Root, None)
            Self::Custom(custom) => {
                // Check for exact match
                if target.get_variants().contains(&SimpleType::Custom(
                    custom.module.clone(),
                    custom.type_name.clone(),
                    custom.variant.clone(),
                )) {
                    return true;
                }
                // Check if this variant fits in the root enum type (subtype relationship)
                if custom.variant.is_some() {
                    target.get_variants().contains(&SimpleType::Custom(
                        custom.module.clone(),
                        custom.type_name.clone(),
                        None,
                    ))
                } else {
                    false
                }
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
            // Custom type conversions - semantic analysis has validated these
            // Enum variant can convert to root enum type (subtype relationship)
            (
                Self::Custom(custom),
                SimpleType::Custom(target_module, target_root, target_variant),
            ) => {
                custom.module == *target_module
                    && custom.type_name == *target_root
                    && (custom.variant == *target_variant || target_variant.is_none())
            }
            // Custom to underlying type - semantic analysis has validated this is allowed
            // The actual conversion happens by unwrapping and converting the content
            (Self::Custom(custom), target_typ) => {
                custom.content.can_convert_to(env, target)
                    || matches!(target_typ, SimpleType::String)
                // Everything converts to String
            }
            // Value to Custom type - semantic analysis has validated this
            // At runtime, we always allow wrapping if semantic check passed
            (_, SimpleType::Custom(_, _, _)) => {
                // Semantic analysis has validated this conversion is legal
                // At runtime, we trust that validation
                true
            }
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
            // Tuples: element-wise conversion
            (Self::Tuple(elements), SimpleType::Tuple(target_elems)) => {
                if elements.len() != target_elems.len() {
                    return false;
                }
                elements.iter().zip(target_elems.iter()).all(|(e, t)| {
                    let t_concrete = t
                        .as_simple()
                        .expect("Type should be concrete")
                        .clone()
                        .into_concrete()
                        .expect("Type should be concrete");
                    e.can_convert_to(env, &t_concrete)
                })
            }
            // Structs: field-wise conversion
            (Self::Struct(fields), SimpleType::Struct(target_fields)) => {
                if fields.len() != target_fields.len() {
                    return false;
                }
                if !fields.keys().all(|k| target_fields.contains_key(k)) {
                    return false;
                }
                fields.iter().all(|(k, v)| {
                    target_fields
                        .get(k)
                        .map(|t| {
                            let t_concrete = t
                                .as_simple()
                                .expect("Type should be concrete")
                                .clone()
                                .into_concrete()
                                .expect("Type should be concrete");
                            v.can_convert_to(env, &t_concrete)
                        })
                        .unwrap_or(false)
                })
            }
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
            // Tuple conversion: element-wise
            (Self::Tuple(elements), SimpleType::Tuple(target_elems)) => {
                let converted: Option<Vec<_>> = elements
                    .into_iter()
                    .zip(target_elems.iter())
                    .map(|(e, t)| {
                        let t_concrete = t
                            .as_simple()
                            .expect("Type should be concrete")
                            .clone()
                            .into_concrete()
                            .expect("Type should be concrete");
                        e.convert_to(env, cache, &t_concrete)
                    })
                    .collect();
                Self::Tuple(converted?)
            }
            // Structs: field-wise conversion
            (Self::Struct(fields), SimpleType::Struct(target_fields)) => {
                let converted: Option<BTreeMap<_, _>> = fields
                    .into_iter()
                    .map(|(k, v)| {
                        let target_type = target_fields.get(&k)?;
                        let t_concrete = target_type
                            .as_simple()
                            .expect("Type should be concrete")
                            .clone()
                            .into_concrete()
                            .expect("Type should be concrete");
                        let converted_v = v.convert_to(env, cache, &t_concrete)?;
                        Some((k, converted_v))
                    })
                    .collect();
                Self::Struct(converted?)
            }
            // Custom type conversions
            // Converting TO a Custom type: wrap the value
            (value, SimpleType::Custom(module, type_name, variant)) => {
                Self::Custom(Box::new(CustomValue {
                    module: module.clone(),
                    type_name: type_name.clone(),
                    variant: variant.clone(),
                    content: value,
                }))
            }
            // Converting FROM a Custom type: unwrap and convert the content
            (Self::Custom(custom), target_typ) => {
                // Recursively convert the inner content to the target type
                let inner_target = target_typ
                    .clone()
                    .into_concrete()
                    .expect("Should be concrete");
                custom
                    .content
                    .clone()
                    .convert_to(env, cache, &inner_target)?
            }
            // Assume can_convert_to is correct so we just have the default behavior: return the current value
            (orig, _) => orig,
        })
    }

    pub(crate) fn convert_to_string(&self, env: &T::Env, cache: &mut T::Cache) -> String {
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
            Self::Tuple(elements) => {
                let inners: Vec<_> = elements
                    .iter()
                    .map(|x| x.convert_to_string(env, cache))
                    .collect();
                format!("({})", inners.join(", "))
            }
            Self::Struct(fields) => {
                let inners: Vec<_> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.convert_to_string(env, cache)))
                    .collect();
                format!("{{{}}}", inners.join(", "))
            }
            Self::Custom(custom) => match &custom.variant {
                None => format!(
                    "{}({})",
                    custom.type_name,
                    custom.content.convert_to_string(env, cache)
                ),
                Some(v) => format!(
                    "{}::{}({})",
                    custom.type_name,
                    v,
                    custom.content.convert_to_string(env, cache)
                ),
            },
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
