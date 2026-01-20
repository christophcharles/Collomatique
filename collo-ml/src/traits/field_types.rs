use super::{EvalObject, FieldConversionError};
use crate::semantics::{ExprType, SimpleType};
use std::collections::BTreeSet;

/// Represents a simple (non-sum) field type in a view object.
///
/// This is an intermediate representation used as a building block for [`FieldType`], which may
/// represent sum types. It captures field types without requiring knowledge of the DSL type names
/// for object references (those are resolved later using `TypeId`).
///
/// # Variants
///
/// - `Int`: An integer field (`i32`)
/// - `Bool`: A boolean field
/// - `Object(TypeId)`: A reference to another object, identified by its Rust type's `TypeId`
/// - `List(Box<FieldType>)`: A collection (typically `Vec`) of values - note the inner type is
///   [`FieldType`], which allows lists of sum types like `[Int | Bool]`
///
/// # Relationship to FieldType
///
/// `SimpleFieldType` is to [`FieldType`] as [`SimpleType`] is to [`ExprType`]:
/// - `SimpleFieldType`: A single, atomic field type
/// - `FieldType`: A set of `SimpleFieldType` variants (may represent a sum type)
///
/// For example:
/// - `SimpleFieldType::Int` converts to `FieldType` with one variant: `{Int}`
/// - A sum type like `Int | Bool` is represented as `FieldType` with two `SimpleFieldType` variants: `{Int, Bool}`
///
/// # Type Resolution
///
/// `Object` variants store a `TypeId` which is later mapped to DSL type names through
/// [`convert_to_simple_type`](SimpleFieldType::convert_to_simple_type), which uses
/// `EvalObject::type_id_to_name()`. This allows view objects to be defined without knowledge
/// of the complete object hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimpleFieldType {
    /// A none field
    None,
    /// An integer field
    Int,
    /// A boolean field
    Bool,
    /// A reference to another object, identified by the Rust type's TypeId
    Object(std::any::TypeId),
    /// A collection of values of the specified type
    List(FieldType),
}

impl SimpleFieldType {
    pub fn convert_to_simple_type<T: EvalObject>(self) -> Result<SimpleType, FieldConversionError> {
        match self {
            SimpleFieldType::None => Ok(SimpleType::None),
            SimpleFieldType::Bool => Ok(SimpleType::Bool),
            SimpleFieldType::Int => Ok(SimpleType::Int),
            SimpleFieldType::List(typ) => Ok(SimpleType::List(typ.convert_to_expr_type::<T>()?)),
            SimpleFieldType::Object(type_id) => {
                Ok(SimpleType::Object(T::type_id_to_name(type_id)?))
            }
        }
    }
}

impl std::fmt::Display for SimpleFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleFieldType::None => write!(f, "None"),
            SimpleFieldType::Bool => write!(f, "Bool"),
            SimpleFieldType::Int => write!(f, "Int"),
            SimpleFieldType::List(typ) => write!(f, "[{}]", typ),
            SimpleFieldType::Object(type_id) => write!(f, "Object({:?})", type_id),
        }
    }
}

/// Represents a field type, which may be a sum type.
///
/// This struct wraps a set of [`SimpleFieldType`] variants, allowing representation of both simple
/// types and sum types (unions of multiple types). It serves as an intermediate representation
/// between view objects and the DSL's [`ExprType`].
///
/// # Structure
///
/// Internally, `FieldType` contains a `BTreeSet<SimpleFieldType>`, which:
/// - Ensures uniqueness (no duplicate types in a sum)
/// - Ensures the order of the types does not matter
///
/// # Examples
///
/// ## Simple Types
///
/// ```ignore
/// // A simple Int field
/// let int_field = FieldType::simple(SimpleFieldType::Int);
///
/// // A simple list field
/// let list_field = FieldType::simple(
///     SimpleFieldType::List(FieldType::simple(SimpleFieldType::Int))
/// );
/// ```
///
/// ## Sum Types (Future)
///
/// ```ignore
/// // Int | Bool field
/// let sum_field = FieldType::sum(vec![
///     SimpleFieldType::Int,
///     SimpleFieldType::Bool,
/// ]).unwrap();
///
/// // List of (Int | Bool)
/// let list_of_sum = FieldType::simple(
///     SimpleFieldType::List(sum_field)
/// );
/// ```
///
/// # Conversion to ExprType
///
/// `FieldType` is converted to [`ExprType`] via [`convert_to_expr_type`](FieldType::convert_to_expr_type),
/// which:
/// 1. Converts each `SimpleFieldType` variant to a `SimpleType`
/// 2. Resolves object `TypeId`s to type name strings
/// 3. Creates an `ExprType` with the resulting set of `SimpleType` variants
///
/// This maintains the sum type structure through the conversion process.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldType {
    variants: BTreeSet<SimpleFieldType>,
}

impl FieldType {
    pub fn simple(typ: SimpleFieldType) -> FieldType {
        FieldType {
            variants: BTreeSet::from([typ]),
        }
    }

    pub fn sum(types: impl IntoIterator<Item = SimpleFieldType>) -> Option<Self> {
        let variants: BTreeSet<_> = types.into_iter().collect();

        if variants.is_empty() {
            return None;
        }

        Some(FieldType { variants })
    }

    pub fn is_simple(&self) -> bool {
        assert!(
            self.variants.len() >= 1,
            "FieldType should always carry at least one type"
        );
        self.variants.len() == 1
    }

    pub fn as_simple(&self) -> Option<&SimpleFieldType> {
        if !self.is_simple() {
            return None;
        }
        Some(
            self.variants
                .iter()
                .next()
                .expect("FieldType should always carry at least one type"),
        )
    }

    pub fn to_simple(self) -> Option<SimpleFieldType> {
        if !self.is_simple() {
            return None;
        }
        Some(
            self.variants
                .into_iter()
                .next()
                .expect("FieldType should always carry at least one type"),
        )
    }

    pub fn get_variants(&self) -> &BTreeSet<SimpleFieldType> {
        &self.variants
    }

    pub fn convert_to_expr_type<T: EvalObject>(self) -> Result<ExprType, FieldConversionError> {
        Ok(ExprType::sum(
            self.variants
                .into_iter()
                .map(|x| x.convert_to_simple_type::<T>())
                .collect::<Result<Vec<_>, _>>()?
                .into_iter(),
        )
        .expect("There should always be at least one variant"))
    }
}

impl From<SimpleFieldType> for FieldType {
    fn from(value: SimpleFieldType) -> Self {
        FieldType::simple(value)
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.variants.len() == 1 {
            write!(f, "{}", self.variants.iter().next().unwrap())
        } else {
            let types: Vec<_> = self.variants.iter().map(|t| t.to_string()).collect();
            write!(f, "{}", types.join(" | "))
        }
    }
}
