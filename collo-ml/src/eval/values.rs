//! Expression values for evaluation.
//!
//! This module defines the runtime value types:
//! - `ExprValue`: The main enum representing evaluated expressions
//! - `CustomValue`: Data for custom type values
//! - `NoObject`: A placeholder object type for tests without objects
//! - `NoObjectEnv`: Environment for NoObject

use derivative::Derivative;

use super::database::DatabaseHandle;
use super::variables::{ConstraintWithOrigin, IlpVar, Origin};
use crate::database::DatabaseConnection;
use crate::semantics::{ConcreteType, ExprType, SimpleType};
use crate::traits::{EvalObject, FieldConversionError};
use collomatique_ilp::{Constraint, LinExpr};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject", feature_allow_slow_enum = "true"),
    Ord(bound = "T: EvalObject", feature_allow_slow_enum = "true")
)]
pub enum ExprValue<T: EvalObject, D: DatabaseConnection> {
    None,
    Int(i32),
    Bool(bool),
    LinExpr(LinExpr<IlpVar<T, D>>),
    Constraint(Vec<ConstraintWithOrigin<T, D>>),
    String(String),
    List(Vec<Arc<ExprValue<T, D>>>),
    Tuple(Vec<Arc<ExprValue<T, D>>>),
    Struct(BTreeMap<String, Arc<ExprValue<T, D>>>),
    Custom(CustomValue<T, D>),
    Database(DatabaseHandle<D>),
    #[doc(hidden)]
    _Phantom(std::convert::Infallible, PhantomData<T>),
}

/// Data for custom type values.
///
/// The `content` field uses `Arc<ExprValue>` to provide both recursion-breaking
/// indirection and cheap cloning.
#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject"),
    PartialOrd(bound = "T: EvalObject"),
    Ord(bound = "T: EvalObject")
)]
pub struct CustomValue<T: EvalObject, D: DatabaseConnection> {
    /// The module where this type is defined
    pub module: String,
    /// The root type name (e.g., "Result" or "MyType")
    pub type_name: String,
    /// The variant name if this is an enum variant (e.g., Some("Ok") for Result::Ok)
    pub variant: Option<String>,
    pub content: Arc<ExprValue<T, D>>,
}

impl<T: EvalObject, D: DatabaseConnection> std::fmt::Display for ExprValue<T, D> {
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
            ExprValue::Database(db) => write!(f, "{}", db),
            ExprValue::_Phantom(..) => unreachable!(),
        }
    }
}

impl<T: EvalObject, D: DatabaseConnection> From<i32> for ExprValue<T, D> {
    fn from(value: i32) -> Self {
        ExprValue::Int(value)
    }
}

impl<T: EvalObject, D: DatabaseConnection> From<bool> for ExprValue<T, D> {
    fn from(value: bool) -> Self {
        ExprValue::Bool(value)
    }
}

impl<T: EvalObject, D: DatabaseConnection> From<LinExpr<IlpVar<T, D>>> for ExprValue<T, D> {
    fn from(value: LinExpr<IlpVar<T, D>>) -> Self {
        ExprValue::LinExpr(value)
    }
}

impl<T: EvalObject, D: DatabaseConnection> From<Constraint<IlpVar<T, D>>> for ExprValue<T, D> {
    fn from(value: Constraint<IlpVar<T, D>>) -> Self {
        ExprValue::Constraint(Vec::from([ConstraintWithOrigin {
            constraint: value,
            origin: None,
        }]))
    }
}

impl<T: EvalObject, D: DatabaseConnection> From<ConstraintWithOrigin<T, D>> for ExprValue<T, D> {
    fn from(value: ConstraintWithOrigin<T, D>) -> Self {
        ExprValue::Constraint(Vec::from([value]))
    }
}

impl<T: EvalObject, D: DatabaseConnection> ExprValue<T, D> {
    pub fn with_origin(&self, origin: &Origin<T, D>) -> ExprValue<T, D> {
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
            ExprValue::List(list) => ExprValue::List(
                list.iter()
                    .map(|x| Arc::new(x.with_origin(origin)))
                    .collect(),
            ),
            ExprValue::Tuple(elements) => ExprValue::Tuple(
                elements
                    .iter()
                    .map(|x| Arc::new(x.with_origin(origin)))
                    .collect(),
            ),
            ExprValue::Struct(fields) => ExprValue::Struct(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), Arc::new(v.with_origin(origin))))
                    .collect(),
            ),
            ExprValue::Custom(custom) => ExprValue::Custom(CustomValue {
                module: custom.module.clone(),
                type_name: custom.type_name.clone(),
                variant: custom.variant.clone(),
                content: Arc::new(custom.content.with_origin(origin)),
            }),
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

    pub fn fits_in_typ(&self, target: &ExprType) -> bool {
        match self {
            // for non-list, it is just of matter of checking that the typ is in the sum
            Self::None => target.get_variants().contains(&SimpleType::None),
            Self::Int(_) => target.get_variants().contains(&SimpleType::Int),
            Self::Bool(_) => target.get_variants().contains(&SimpleType::Bool),
            Self::LinExpr(_) => target.get_variants().contains(&SimpleType::LinExpr),
            Self::Constraint(_) => target.get_variants().contains(&SimpleType::Constraint),
            Self::String(_) => target.get_variants().contains(&SimpleType::String),
            // if we have an empty list, we just need to check that ExprType is a list
            Self::List(list) if list.is_empty() => target.has_list(),
            // if not empty, we have to check recursively for all list types in the sum
            Self::List(list) => {
                for variant in target.get_variants() {
                    let SimpleType::List(inner_typ) = variant else {
                        continue;
                    };

                    if list.iter().all(|x| x.fits_in_typ(inner_typ)) {
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
                        .all(|(e, t)| e.fits_in_typ(t))
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
                            .map(|t| v.fits_in_typ(t))
                            .unwrap_or(false)
                    }) {
                        return true;
                    }
                }
                false
            }
            Self::Database(db) => target.get_variants().iter().any(|v| {
                if let SimpleType::DatabaseSchema(declared_schema) = v {
                    db.matches_schema(declared_schema)
                } else {
                    false
                }
            }),
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
            Self::_Phantom(..) => unreachable!(),
        }
    }

    pub fn can_convert_to(&self, target: &ConcreteType) -> bool {
        match (self, target.inner()) {
            // Can always convert to its own type
            (Self::None, SimpleType::None) => true,
            (Self::Int(_), SimpleType::Int) => true,
            (Self::Bool(_), SimpleType::Bool) => true,
            (Self::LinExpr(_), SimpleType::LinExpr) => true,
            (Self::Constraint(_), SimpleType::Constraint) => true,
            (Self::String(_), SimpleType::String) => true,
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
                custom.content.can_convert_to(target) || matches!(target_typ, SimpleType::String)
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
                list.iter().all(|x| x.can_convert_to(&concrete_inner))
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
                    e.can_convert_to(&t_concrete)
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
                            v.can_convert_to(&t_concrete)
                        })
                        .unwrap_or(false)
                })
            }
            // Database can convert to DatabaseSchema if schema matches
            (Self::Database(db), SimpleType::DatabaseSchema(declared_schema)) => {
                db.matches_schema(declared_schema)
            }
            // Everything else forbidden
            _ => false,
        }
    }

    pub unsafe fn convert_to_unchecked(
        &self,
        env: &T::Env,
        cache: &mut T::Cache,
        target: &SimpleType,
    ) -> ExprValue<T, D> {
        match (self, target) {
            // This should also work for empty lists as the iterator will be empty
            (Self::List(list), SimpleType::List(inner_typ)) => {
                let inner_target = inner_typ
                    .as_simple()
                    .expect("Inner list target type should have already been checked");
                Self::List(
                    list.iter()
                        .map(|x| {
                            Arc::new(unsafe { x.convert_to_unchecked(env, cache, inner_target) })
                        })
                        .collect(),
                )
            }
            (Self::Int(val), SimpleType::LinExpr) => Self::LinExpr(LinExpr::constant(*val as f64)),
            // Conversion to string
            (Self::String(v), SimpleType::String) => Self::String(v.clone()),
            (v, SimpleType::String) => Self::String(v.convert_to_string(env, cache)),
            // Tuple conversion: element-wise
            (Self::Tuple(elements), SimpleType::Tuple(target_elems)) => {
                let converted = elements
                    .iter()
                    .zip(target_elems.iter())
                    .map(|(e, t)| {
                        let target_type = t.as_simple().expect("Type should be concrete");
                        Arc::new(unsafe { e.convert_to_unchecked(env, cache, target_type) })
                    })
                    .collect();
                Self::Tuple(converted)
            }
            // Structs: field-wise conversion
            (Self::Struct(fields), SimpleType::Struct(target_fields)) => {
                let converted = fields
                    .iter()
                    .map(|(k, v)| {
                        let target_type = target_fields.get(k).expect("Field should exist");
                        let inner_target =
                            target_type.as_simple().expect("Type should be concrete");
                        let converted_v =
                            Arc::new(unsafe { v.convert_to_unchecked(env, cache, inner_target) });
                        (k.clone(), converted_v)
                    })
                    .collect();
                Self::Struct(converted)
            }
            // Custom type conversions
            // Converting TO a Custom type: wrap the value
            (value, SimpleType::Custom(module, type_name, variant)) => Self::Custom(CustomValue {
                module: module.clone(),
                type_name: type_name.clone(),
                variant: variant.clone(),
                content: Arc::new(value.clone()),
            }),
            // Converting FROM a Custom type: unwrap and convert the content
            (Self::Custom(custom), target_typ) => {
                // Recursively convert the inner content to the target type
                unsafe { custom.content.convert_to_unchecked(env, cache, target_typ) }
            }
            // Assume can_convert_to is correct so we just have the default behavior: return the current value
            (_, _) => self.clone(),
        }
    }

    pub fn convert_to(
        &self,
        env: &T::Env,
        cache: &mut T::Cache,
        target: &ConcreteType,
    ) -> Option<ExprValue<T, D>> {
        if !self.can_convert_to(target) {
            return None;
        }

        Some(unsafe { self.convert_to_unchecked(env, cache, target.inner()) })
    }

    pub(crate) fn convert_to_string(&self, env: &T::Env, cache: &mut T::Cache) -> String {
        match self {
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

    fn typ_name(&self) -> String {
        panic!("No object is defined for NoObject")
    }

    fn type_id_to_name(type_id: std::any::TypeId) -> Result<String, FieldConversionError> {
        Err(FieldConversionError::UnknownTypeId(type_id))
    }

    fn field_access<D: DatabaseConnection>(
        &self,
        _env: &Self::Env,
        _cache: &mut Self::Cache,
        _field: &str,
    ) -> Option<ExprValue<Self, D>> {
        None
    }

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        HashMap::new()
    }
}
