use crate::semantics::{ExprType, SimpleType};
use std::collections::BTreeSet;

/// Represents a simple (non-sum) field type in a view object.
///
/// This is an intermediate representation used as a building block for [`FieldType`], which may
/// represent sum types.
///
/// # Variants
///
/// - `Int`: An integer field (`i32`)
/// - `Bool`: A boolean field
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimpleFieldType {
    /// A none field
    None,
    /// An integer field
    Int,
    /// A boolean field
    Bool,
    /// A collection of values of the specified type
    List(FieldType),
}

impl SimpleFieldType {
    pub fn convert_to_simple_type(self) -> SimpleType {
        match self {
            SimpleFieldType::None => SimpleType::None,
            SimpleFieldType::Bool => SimpleType::Bool,
            SimpleFieldType::Int => SimpleType::Int,
            SimpleFieldType::List(typ) => SimpleType::List(typ.convert_to_expr_type()),
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
/// 2. Creates an `ExprType` with the resulting set of `SimpleType` variants
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
            !self.variants.is_empty(),
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

    pub fn convert_to_expr_type(self) -> ExprType {
        ExprType::sum(
            self.variants
                .into_iter()
                .map(|x| x.convert_to_simple_type())
                .collect::<Vec<_>>(),
        )
        .expect("There should always be at least one variant")
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
