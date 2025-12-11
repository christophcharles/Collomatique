//! View pattern traits for DSL object access.
//!
//! This module provides a trait-based view pattern for exposing existing data structures
//! to the DSL interpreter without requiring ownership or modification of the underlying data.
//!
//! # Overview
//!
//! The solution uses three layers:
//!
//! 1. **Your existing data** - unchanged, owned by your application
//! 2. **View objects** (`ViewObject`) - lightweight, ephemeral projections of that data
//! 3. **Object IDs** (`EvalObject`) - references that the DSL uses to identify objects
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  DSL Script     │
//! │  "student.age"  │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  EvalObject     │  ← ObjectId enum
//! │  (Student ID)   │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  ViewBuilder    │  ← Constructs view from env
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  ViewObject     │  ← Student view struct
//! │  { age: i32 }   │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Your Data      │  ← Real application data
//! │  (unchanged)    │
//! └─────────────────┘
//! ```
//!
//! # Core Traits
//!
//! ## [`EvalObject`]
//!
//! The main trait representing objects accessible from the DSL. Typically implemented on an
//! enum of object IDs using `#[derive(EvalObject)]`.
//!
//! ## [`ViewObject`]
//!
//! Represents a structured view of data with accessible fields. Typically implemented on
//! structs using `#[derive(ViewObject)]`.
//!
//! ## [`ViewBuilder`]
//!
//! Connects `EvalObject` to `ViewObject` by defining how to construct view objects from
//! your environment and IDs. Must be manually implemented for each object type.
//!
//! # Usage Pattern
//!
//! ## Step 1: Define your view objects
//!
//! ```ignore
//! use collo_ml::ViewObject;
//!
//! #[derive(ViewObject, Clone)]
//! #[eval_object(ObjectId)]
//! struct Student {
//!     age: i32,
//!     enrolled: bool,
//!     room: RoomId,
//!     #[hidden]
//!     internal_id: String,  // Not accessible from DSL
//! }
//!
//! #[derive(ViewObject, Clone)]
//! #[eval_object(ObjectId)]
//! struct Room {
//!     number: i32,
//!     students: Vec<StudentId>,
//! }
//! ```
//!
//! ## Step 2: Define your object ID enum
//!
//! ```ignore
//! use collo_ml::EvalObject;
//!
//! #[derive(EvalObject)]
//! #[env(MyApplicationEnv)]
//! #[cached]  // Optional: enable caching for performance
//! enum ObjectId {
//!     Student(StudentId),
//!     Room(RoomId),
//! }
//! ```
//!
//! ## Step 3: Implement ViewBuilder for each object type
//!
//! ```ignore
//! use collo_ml::ViewBuilder;
//!
//! impl ViewBuilder<MyApplicationEnv, StudentId> for ObjectId {
//!     type Object = Student;
//!
//!     fn enumerate(env: &MyApplicationEnv) -> BTreeSet<StudentId> {
//!         // Return all student IDs from your environment
//!         env.students.keys().map(|&id| StudentId(id)).collect()
//!     }
//!
//!     fn build(env: &MyApplicationEnv, id: &StudentId) -> Option<Self::Object> {
//!         // Construct a Student view from your data
//!         let data = env.students.get(&id.0)?;
//!         Some(Student {
//!             age: data.age,
//!             enrolled: data.is_enrolled,
//!             room: RoomId(data.room_id),
//!         })
//!     }
//! }
//! ```
//!
//! # Type Information Flow
//!
//! The module uses intermediate types to handle type information across boundaries:
//!
//! - [`SimpleFieldType`]: Building blocks for field types (Int, Bool, Object, List)
//! - [`FieldType`]: May represent simple types or sum types (unions), using `TypeId` for
//!   object references
//! - These are converted to [`SimpleType`]/[`ExprType`] by `EvalObject`
//!
//! # Caching
//!
//! View objects can be expensive to construct, especially if they involve database queries
//! or complex computations. The `#[cached]` attribute enables automatic caching:
//!
//! ```ignore
//! #[derive(EvalObject)]
//! #[env(MyEnv)]
//! #[cached]  // Auto-generates ObjectIdCache struct
//! enum ObjectId { /* ... */ }
//!
//! // Or with a custom name:
//! #[derive(EvalObject)]
//! #[env(MyEnv)]
//! #[cached(MyCustomCache)]
//! enum ObjectId { /* ... */ }
//! ```
//!
//! When caching is enabled:
//! - View objects must implement `Clone`
//! - The cache is thread-local and mutable
//! - Cache is passed to `field_access` and `pretty_print` methods
//! - Objects are cached by ID and reused across multiple field accesses
//!
//! Without caching, `type Cache = ()` and the cache parameter is ignored.
//!
//! # Hidden Fields
//!
//! Fields marked with `#[hidden]` are excluded from the DSL but remain accessible in Rust:
//!
//! ```ignore
//! #[derive(ViewObject)]
//! #[eval_object(ObjectId)]
//! #[pretty("{name} is {age} years old")]
//! struct Student {
//!     age: i32,
//!     #[hidden]
//!     name: String,  // Used in pretty_print but not in DSL
//! }
//! ```
//!
//! Use cases:
//! - Internal identifiers not relevant to DSL logic
//! - Sensitive data that shouldn't be exposed
//! - Data needed for pretty printing but not computation
//!
//! # Custom Type Names
//!
//! DSL type names can differ from Rust variant names:
//!
//! ```ignore
//! #[derive(EvalObject)]
//! #[env(MyEnv)]
//! enum ObjectId {
//!     Student(StudentId),
//!     #[name("Classroom")]
//!     RoomNumber(RoomId),  // "Classroom" in DSL, RoomNumber in Rust
//! }
//! ```
//!
//! # Thread Safety
//!
//! The view pattern is designed for single-threaded use:
//! - `Env` is borrowed immutably
//! - `Cache` is borrowed mutably (preventing shared access)
//! - View objects are constructed on-demand and don't persist

use crate::semantics::{CompleteType, SimpleType};
use crate::{eval::ExprValue, ExprType};
use collomatique_ilp::UsableData;
use std::collections::{BTreeSet, HashMap};

/// The main trait for objects in the DSL evaluation system.
///
/// This trait represents objects that can be accessed and manipulated by the DSL interpreter.
/// Objects are identified by IDs and provide access to their fields through an environment reference.
///
/// # Design Philosophy
///
/// `EvalObject` is designed to work with existing data structures without requiring ownership.
/// It acts as a bridge between your application's data model and the DSL interpreter, using
/// a view pattern where objects are constructed on-demand from an environment.
///
/// # Associated Types
///
/// - `Env`: The environment type that holds the actual data. This is typically a reference
///   to your application's data structures.
/// - `Cache`: An optional cache type for storing constructed view objects. Use `()` for no caching,
///   or a custom cache struct for performance optimization.
///
/// # Implementation
///
/// This trait is typically implemented via the `#[derive(EvalObject)]` macro on an enum of object IDs:
///
/// ```ignore
/// #[derive(EvalObject)]
/// #[env(MyEnv)]
/// enum ObjectId {
///     Student(StudentId),
///     Room(RoomId),
/// }
/// ```
///
/// With optional caching:
///
/// ```ignore
/// #[derive(EvalObject)]
/// #[env(MyEnv)]
/// #[cached]  // or #[cached(CustomCacheName)]
/// enum ObjectId {
///     Student(StudentId),
///     Room(RoomId),
/// }
/// ```
///
/// # Examples
///
/// ```ignore
/// // Get all objects of a specific type
/// let students = ObjectId::objects_with_typ(&env, "Student");
///
/// // Access a field on an object
/// let mut cache = MyCache::default();
/// let age = student_obj.field_access(&env, &mut cache, "age");
///
/// // Get the type name of an object
/// let type_name = student_obj.typ_name(&env);
/// ```
pub trait EvalObject: UsableData {
    /// The environment type that provides access to the underlying data.
    type Env;

    /// The cache type for storing constructed view objects.
    ///
    /// Use `()` for no caching, or a custom struct implementing `Default` for caching.
    /// The cache is automatically managed by the interpreter and passed to methods that need it.
    type Cache: Default;

    /// Returns all objects of a given type name.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data
    /// * `name` - The DSL type name (e.g., "Student", "Room")
    ///
    /// # Returns
    ///
    /// A set of all object IDs that match the given type name. Returns an empty set
    /// if the type name is not recognized.
    fn objects_with_typ(env: &Self::Env, name: &str) -> BTreeSet<Self>;

    /// Converts a TypeId to an object nmae
    ///
    /// # Arguments
    ///
    /// * `type_id` - rust TypeID object to be converted into an object name
    ///
    /// # Returns
    ///
    /// The corresponding object name if the `EvalObject`` is used for evaluation.
    /// Returns `None` if `type_id` is an object that cannot be represented with this `EvalObject`.
    fn type_id_to_name(field_typ: std::any::TypeId) -> Result<String, FieldConversionError>;

    /// Returns the DSL type name of this object.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment (may not be needed but provided for consistency)
    ///
    /// # Returns
    ///
    /// The type name as it appears in the DSL (e.g., "Student", "Room").
    fn typ_name(&self, env: &Self::Env) -> String;

    /// Accesses a field on this object and returns its value.
    ///
    /// This method constructs a view object from the environment (or retrieves it from cache),
    /// then accesses the requested field.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data
    /// * `cache` - Mutable reference to the cache for storing/retrieving view objects
    /// * `field` - The name of the field to access
    ///
    /// # Returns
    ///
    /// `Some(ExprValue)` if the object and field exist, `None` otherwise.
    fn field_access(
        &self,
        env: &Self::Env,
        cache: &mut Self::Cache,
        field: &str,
    ) -> Option<ExprValue<Self>>;

    /// Returns the schema for all object types in the DSL.
    ///
    /// This provides type information for semantic analysis and validation before execution.
    ///
    /// # Returns
    ///
    /// A nested map where:
    /// - Outer keys are DSL type names (e.g., "Student")
    /// - Inner maps associate field names with their types
    fn type_schemas() -> HashMap<String, HashMap<String, CompleteType>>;

    /// Returns a human-readable string representation of this object.
    ///
    /// This is used for debugging, logging, and user-facing output. The default implementation
    /// returns `None`, indicating no pretty printing is available.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment (may be needed to access additional data)
    /// * `cache` - Mutable reference to the cache (view object may be cached during pretty printing)
    ///
    /// # Returns
    ///
    /// `Some(String)` with a formatted representation, or `None` if not implemented.
    fn pretty_print(&self, _env: &Self::Env, _cache: &mut Self::Cache) -> Option<String> {
        None
    }
}

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
            SimpleFieldType::Bool => Ok(SimpleType::Bool),
            SimpleFieldType::Int => Ok(SimpleType::Int),
            SimpleFieldType::List(typ) => {
                Ok(SimpleType::List(Some(typ.convert_to_expr_type::<T>()?)))
            }
            SimpleFieldType::Object(type_id) => {
                Ok(SimpleType::Object(T::type_id_to_name(type_id)?))
            }
        }
    }
}

impl std::fmt::Display for SimpleFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

/// Represents a view of an object that can be accessed by the DSL.
///
/// View objects are ephemeral representations of data from the environment. They don't own
/// the data but provide a structured interface for accessing it. View objects are typically
/// constructed on-demand by `ViewBuilder` and may be cached for performance.
///
/// # Design
///
/// View objects serve as an adapter layer between your application's data model and the DSL.
/// They:
/// - Define which fields are accessible from the DSL
/// - Specify the type of each field
/// - Provide field access without exposing the underlying data structure
///
/// # Implementation
///
/// This trait is typically implemented via the `#[derive(ViewObject)]` macro:
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// struct Student {
///     age: i32,
///     enrolled: bool,
///     room: RoomId,
///     #[hidden]
///     internal_data: String,  // Not accessible from DSL
/// }
/// ```
///
/// With pretty printing:
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// #[pretty("{name} (age {age})")]
/// struct Student {
///     age: i32,
///     #[hidden]
///     name: String,
/// }
/// ```
///
/// # Requirements for Caching
///
/// If the corresponding `EvalObject` uses caching (via `#[cached]`), the view object
/// must implement `Clone`. This is automatically checked by the compiler.
pub trait ViewObject {
    /// The `EvalObject` type this view belongs to.
    type EvalObject: EvalObject;

    /// Returns the schema describing all fields in this view object.
    ///
    /// Fields marked with `#[hidden]` are excluded from the schema.
    ///
    /// # Returns
    ///
    /// A map from field names to their types.
    fn field_schema() -> HashMap<String, FieldType>;

    /// Accesses a field by name and returns its value.
    ///
    /// # Arguments
    ///
    /// * `field` - The name of the field to access
    ///
    /// # Returns
    ///
    /// `Some(ExprValue)` if the field exists, `None` otherwise.
    /// Fields marked with `#[hidden]` return `None` when accessed.
    fn get_field(&self, field: &str) -> Option<ExprValue<Self::EvalObject>>;

    /// Returns a human-readable string representation of this view object.
    ///
    /// This can be customized using the `#[pretty("...")]` attribute on the struct.
    /// The default implementation returns `None`.
    ///
    /// # Returns
    ///
    /// `Some(String)` with a formatted representation, or `None` if not implemented.
    fn pretty_print(&self) -> Option<String> {
        None
    }
}

/// Builds view objects from an environment and object IDs.
///
/// This trait connects the `EvalObject` enum to specific view object types, defining how
/// to construct view objects from the underlying data.
///
/// # Type Parameters
///
/// * `Env` - The environment type containing the data
/// * `Id` - The ID type for this specific object kind (e.g., `StudentId`, `RoomId`)
///
/// # Implementation
///
/// You must manually implement this trait for each object type in your system:
///
/// ```ignore
/// impl ViewBuilder<MyEnv, StudentId> for ObjectId {
///     type Object = Student;
///
///     fn enumerate(env: &MyEnv) -> BTreeSet<StudentId> {
///         env.students.keys().copied().collect()
///     }
///
///     fn build(env: &MyEnv, id: &StudentId) -> Option<Self::Object> {
///         let data = env.students.get(&id.0)?;
///         Some(Student {
///             age: data.age,
///             enrolled: data.enrolled,
///             room: RoomId(data.room_id),
///         })
///     }
/// }
/// ```
///
/// # Relationship to Other Traits
///
/// - The `EvalObject` derive macro generates code that calls these methods
/// - `build()` is called on-demand when fields are accessed
/// - `enumerate()` is called when getting all objects of a type
pub trait ViewBuilder<Env, Id> {
    /// The view object type constructed by this builder.
    type Object: ViewObject;

    /// Constructs a view object for the given ID.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data
    /// * `id` - The ID of the object to construct
    ///
    /// # Returns
    ///
    /// `Some(Object)` if the object exists in the environment, `None` otherwise.
    fn build(env: &Env, id: &Id) -> Option<Self::Object>;

    /// Returns all IDs of this object type that exist in the environment.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data
    ///
    /// # Returns
    ///
    /// A set of all IDs for this object type.
    fn enumerate(env: &Env) -> BTreeSet<Id>;
}

use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum FieldConversionError {
    #[error("Cannot convert value: unknown TypeId")]
    UnknownTypeId(std::any::TypeId),
}

/// Error used in TryFrom auto-impl for converting between types
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TypeConversionError {
    #[error("Cannot convert value: type not compatible with eval object")]
    BadType,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum VarConversionError {
    #[error("Cannot convert variable: unknown name \"{0}\"")]
    Unknown(String),
    #[error("Cannot convert variable: wrong parameter count for {name}. Expected {expected} got {found}")]
    WrongParameterCount {
        name: String,
        expected: usize,
        found: usize,
    },
    #[error(
        "Cannot convert variable {name}: parameter {param} has wrong type. Expected {expected}"
    )]
    WrongParameterType {
        name: String,
        param: usize,
        expected: FieldType,
    },
}

/// Represents variables in an Integer Linear Programming problem.
///
/// This trait defines the interface for enumerating and working with ILP variables that are
/// parameterized by data from the environment. Variables can have parameters like object IDs,
/// integers, or booleans, and the trait provides methods for:
///
/// - Describing the schema of variable parameters
/// - Enumerating all valid variable instances
/// - Determining if a variable should be fixed to a specific value
///
/// # Design Philosophy
///
/// `EvalVar` is **generic over `T: EvalObject`**, allowing the same variable definitions to work
/// with different data sources. This enables:
///
/// - Testing with mock data while using production data in deployment
/// - Reusing variable definitions across different problem instances
/// - Clear separation between variable structure and data access
///
/// # Dynamic Features
///
/// Variables can have dynamic behavior based on the environment:
///
/// - **Dynamic ranges**: `#[range(0..env.max_week)]` - Range depends on environment state
/// - **Dynamic fix values**: `#[fix_with(if env.flag { 1.0 } else { 0.5 })]` - Fix value depends on environment
/// - **Field-aware fixes**: `#[fix_with(if hour >= 12 { 2.0 } else { 0.0 })]` - Fix value depends on field values
/// - **Complex logic**: `#[defer_fix(Self::check(env, params...))]` - Custom fix logic
///
/// To enable environment field access, use `#[env(EnvType)]` on the enum. This constrains the
/// implementation to work with any `EvalObject` that has that environment type, while remaining
/// generic over the specific `EvalObject`.
///
/// # Type Parameter
///
/// - `T`: The `EvalObject` type that provides access to the underlying data via its environment
///
/// # Implementation
///
/// This trait is typically implemented via the `#[derive(EvalVar)]` macro on an enum:
///
/// ## Static Variables (Fully Generic)
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
/// enum Var {
///     StudentInGroup(StudentId, GroupId),
///     
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,
///     },
/// }
/// ```
///
/// The macro generates:
///
/// ```ignore
/// impl<T: EvalObject> EvalVar<T> for Var
/// where
///     StudentId: TryFrom<T, Error = TypeConversionError>,
///     GroupId: TryFrom<T, Error = TypeConversionError>,
/// {
///     fn field_schema() -> HashMap<String, Vec<FieldType>> { /* ... */ }
///     fn vars(env: &T::Env) -> Result<BTreeMap<Self, Variable>, TypeId> { /* ... */ }
///     fn fix(&self, env: &T::Env) -> Option<f64> { /* ... */ }
/// }
/// ```
///
/// ## Dynamic Variables (Environment-Specific)
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
/// #[env(DynamicEnv)]  // Required for env field access
/// #[fix_with(0.0)]
/// enum DynamicVar {
///     StudentInWeek {
///         student: StudentId,
///         #[range(0..env.max_week)]  // Dynamic range
///         week: i32,
///     },
///     
///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]  // Dynamic fix value
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..env.last_hour)]  // Dynamic range
///         hour: i32,
///     },
/// }
/// ```
///
/// The macro generates:
///
/// ```ignore
/// impl<T> EvalVar<T> for DynamicVar
/// where
///     T: EvalObject<Env = DynamicEnv>,  // Constrained to specific Env!
///     StudentId: TryFrom<T, Error = TypeConversionError>,
/// {
///     fn field_schema() -> HashMap<String, Vec<FieldType>> { /* ... */ }
///     fn vars(env: &DynamicEnv) -> Result<BTreeMap<Self, Variable>, TypeId> {
///         // env is concrete &DynamicEnv - can access env.max_week!
///         // Variables with defer_fix returning Some are excluded
///     }
///     fn fix(&self, env: &DynamicEnv) -> Option<f64> {
///         // env is concrete &DynamicEnv - can evaluate dynamic expressions!
///     }
/// }
/// ```
///
/// # Usage Example
///
/// ## Static Variables
///
/// ```ignore
/// // Define variables with static ranges
/// #[derive(EvalVar)]
/// enum Var {
///     StudentInGroup(StudentId, GroupId),
/// }
///
/// // Create environment
/// let env = MyEnv::new();
///
/// // Generate all variables with a specific EvalObject type
/// let vars = <Var as EvalVar<ObjectId>>::vars(&env)?;
///
/// // Check if a specific variable should be fixed
/// let var = Var::StudentInGroup(StudentId(0), GroupId(1));
/// if let Some(value) = <Var as EvalVar<ObjectId>>::fix(&var, &env) {
///     // This variable should be fixed to `value`
/// }
///
/// // Get the schema
/// let schema = Var::field_schema();
/// ```
///
/// ## Dynamic Variables
///
/// ```ignore
/// // Define variables with environment-dependent behavior
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// #[fix_with(0.0)]
/// enum DynamicVar {
///     #[range(0..env.max_week)]
///     StudentInWeek {
///         student: StudentId,
///         week: i32,
///     },
///     
///     #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..env.last_hour)]
///         hour: i32,
///     },
/// }
///
/// // Create environment with dynamic configuration
/// let env = DynamicEnv { max_week: 4, last_hour: 17, /* ... */ };
///
/// // Works with any EvalObject that has Env = DynamicEnv
/// let vars = <DynamicVar as EvalVar<ObjectId>>::var(&env)?;
/// let vars = <DynamicVar as EvalVar<MockObjectId>>::var(&env)?;  // Also works
///
/// // Fix values depend on environment and field values
/// let var = DynamicVar::TimeSlot { day: 3, hour: 13 };
/// // Returns Some(2.0) because hour is in lunch range (12-14)
/// let fix = <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env);
/// ```
///
/// # Variable Enumeration Strategy
///
/// The `vars()` method generates all valid combinations by taking the cartesian product of:
///
/// - **i32 parameters**: All values in the specified range (e.g., `0..10` -> 10 values)
///   - Ranges can be static: `#[range(0..10)]`
///   - Or dynamic from environment: `#[range(0..env.max_week)]`
/// - **bool parameters**: Both `true` and `false` (2 values)
/// - **Object parameters**: All objects of that type from `T::objects_with_typ(env, type_name)`
///
/// **Filtering with defer_fix**: Variables where `#[defer_fix(...)]` returns `Some(_)` are
/// automatically excluded from `vars()` for performance.
///
/// For example:
/// ```ignore
/// // Static ranges
/// TimeSlot {
///     #[range(0..7)]
///     day: i32,     // range 0..7  -> 7 values
///     #[range(8..18)]
///     hour: i32,    // range 8..18 -> 10 values
/// }
/// // Total: 7 × 10 = 70 variables
///
/// // Dynamic ranges
/// #[env(DynamicEnv)]
/// StudentInWeek {
///     student: StudentId,
///     #[range(0..env.max_week)]  // Range depends on env!
///     week: i32,
/// }
/// // With 3 students and env.max_week = 4: 3 × 4 = 12 variables
/// // With env.max_week = 2: 3 × 2 = 6 variables
/// ```
///
/// # Fix Values
///
/// The `fix()` method allows specifying that certain variable instances should be fixed to
/// specific values in the ILP solver. This is useful for:
///
/// - Enforcing constraints (e.g., "this combination is impossible")
/// - Handling boundary cases
/// - Pre-solving parts of the problem
/// - Dynamic business rules (e.g., "no scheduling during lunch if mandatory")
///
/// Return `None` if the variable is free to take any value, or `Some(value)` to fix it.
///
/// ## Fix Mechanisms
///
/// There are two ways to specify fix values:
///
/// ### 1. `#[fix_with(expr)]` - Returns `f64`
///
/// Evaluated when any field is out of its range. The expression can reference:
/// - `env` - The environment (if `#[env(EnvType)]` is specified)
/// - Field names (for named fields) or `v0`, `v1`, etc. (for unnamed fields)
///
/// ```ignore
/// #[env(DynamicEnv)]
/// enum Var {
///     // Static value
///     #[fix_with(1.0)]
///     SimpleSlot { #[range(0..10)] slot: i32 },
///     
///     // Based on environment
///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
///     TimeSlot { #[range(0..7)] day: i32, #[range(8..18)] hour: i32 },
///     
///     // Based on field values (named)
///     #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
///     WorkSlot { #[range(0..5)] day: i32, #[range(8..18)] hour: i32 },
///     
///     // Based on field values (unnamed)
///     #[fix_with(if *v1 >= 12 { 2.0 } else { 0.0 })]
///     Slot(#[range(0..7)] i32, #[range(8..18)] i32),
/// }
/// ```
///
/// ### 2. `#[defer_fix(expr)]` - Returns `Option<f64>`
///
/// Completely replaces range checking. Useful for complex logic that can't be expressed
/// with simple range checks. Variables where this returns `Some(_)` are excluded from `vars()`.
///
/// ```ignore
/// #[env(DynamicEnv)]
/// enum Var {
///     // Call custom function
///     #[defer_fix(Self::check_availability(env, student, week))]
///     StudentAvailable {
///         student: StudentId,
///         #[range(0..env.max_week)]
///         week: i32,
///     },
///     
///     // Inline expression
///     #[defer_fix({ if *week >= 3 { Some(5.0) } else { None } })]
///     LateWeek { #[range(0..10)] week: i32 },
/// }
///
/// impl Var {
///     fn check_availability<T: EvalObject>(
///         env: &T::Env,
///         student: &StudentId,
///         week: &i32,
///     ) -> Option<f64>
///     where
///         T::Env: AsRef<DynamicEnv>,
///     {
///         let env = env.as_ref();
///         if env.is_student_absent(student, *week) {
///             Some(10.0)  // Penalty for unavailable
///         } else {
///             None
///         }
///     }
/// }
/// ```
///
/// # Integration with ColloML
///
/// Variables defined with `EvalVar` can be referenced in ColloML constraint scripts using
/// the `$` syntax:
///
/// ```ignore
/// // Rust
/// #[derive(EvalVar)]
/// enum Var {
///     #[name("SiG")]
///     StudentInGroup(StudentId, GroupId),
/// }
///
/// // ColloML
/// pub let one_group_per_student() -> [Constraint] = [
///     sum g in @[Group] { $SiG(s, g) } === 1
///     for s in @[Student]
/// ];
/// ```
///
/// # Thread Safety
///
/// Like the view pattern, `EvalVar` is designed for single-threaded use:
/// - `vars()` takes an immutable reference to the environment
/// - The generated variables are independent and can be used concurrently if needed
/// - Type `T` and `Self` must be `Send` if variables need to cross thread boundaries
pub trait EvalVar<T: EvalObject>: UsableData {
    /// Returns the schema describing all variable types and their parameters.
    ///
    /// This method provides type information for each variable variant, mapping the DSL name
    /// to a list of parameter types. This schema is used for semantic analysis and validation
    /// before code generation.
    ///
    /// # Returns
    ///
    /// A map where:
    /// - Keys are DSL variable names (affected by `#[name("...")]` attributes)
    /// - Values are vectors of parameter types in declaration order
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// enum Var {
    ///     #[name("SiG")]
    ///     StudentInGroup(StudentId, GroupId),
    ///     
    ///     TimeSlot {
    ///         #[range(0..7)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let schema = Var::field_schema();
    /// // Returns:
    /// // {
    /// //     "SiG": [FieldType::Object(TypeId::of::<StudentId>()),
    /// //             FieldType::Object(TypeId::of::<GroupId>())],
    /// //     "TimeSlot": [FieldType::Int, FieldType::Int],
    /// // }
    /// ```
    fn field_schema() -> HashMap<String, Vec<FieldType>>;
    /// Generates all valid variable instances by enumerating parameter combinations.
    ///
    /// This method creates the cartesian product of all possible parameter values, using the
    /// environment to enumerate object instances. Each generated variable is associated with
    /// its `Variable` type (binary, integer, continuous, etc.).
    ///
    /// **Important**: Variables with `#[defer_fix(...)]` that return `Some(_)` are automatically
    /// excluded from the result.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data needed to enumerate objects
    ///   - For generic variables (no `#[env]`): type is `&T::Env`
    ///   - For environment-specific variables (with `#[env(EnvType)]`): type is `&EnvType`
    ///
    /// # Returns
    ///
    /// * `Ok(BTreeMap<Self, Variable>)` - Map of all variable instances to their variable types
    /// * `Err(TypeId)` - If an object type cannot be resolved via `T::type_id_to_name()`
    ///
    /// The error case typically indicates a mismatch between the variable definition and the
    /// `EvalObject` implementation.
    ///
    /// # Enumeration Strategy
    ///
    /// For each parameter type:
    ///
    /// - **i32**: Iterates through the range specified in `#[range(expr)]`
    ///   - Can be static: `#[range(0..10)]`
    ///   - Or dynamic: `#[range(0..env.max_week)]`
    /// - **bool**: Enumerates `[false, true]`
    /// - **Objects**: Calls `T::objects_with_typ(env, type_name)` and converts to the ID type
    ///
    /// # Example
    ///
    /// ## Static Variables
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// enum Var {
    ///     StudentInGroup(StudentId, GroupId),
    ///     WeekUsed(#[range(0..3)] i32),
    /// }
    ///
    /// let env = MyEnv { /* ... */ };
    /// let vars = <Var as EvalVar<ObjectId>>::vars(&env)?;
    ///
    /// // If there are 5 students and 3 groups:
    /// // - StudentInGroup generates: 5 × 3 = 15 variables
    /// // - WeekUsed generates: 3 variables
    /// // Total: 18 variables
    /// ```
    ///
    /// ## Dynamic Variables
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum DynamicVar {
    ///     StudentInWeek {
    ///         student: StudentId,
    ///         #[range(0..env.max_week)]  // Dynamic range
    ///         week: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv { max_week: 4, /* ... */ };
    /// let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&env)?;
    ///
    /// // With 3 students and max_week = 4:
    /// // - StudentInWeek generates: 3 × 4 = 12 variables
    ///
    /// // Change environment
    /// env.max_week = 2;
    /// let vars2 = <DynamicVar as EvalVar<ObjectId>>::vars(&env)?;
    /// // Now generates: 3 × 2 = 6 variables
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// The cartesian product can grow quickly with multiple parameters:
    /// - 10 students × 5 groups × 7 days = 350 variables
    /// - 100 tasks × 20 slots × 2 bools = 4000 variables
    ///
    /// Be mindful of the total number of combinations when designing your variables.
    fn vars(
        env: &T::Env,
    ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>;
    /// Returns a fixed value for this variable instance, if it should be fixed.
    ///
    /// This method allows specifying that certain variable instances should be fixed to
    /// specific values in the ILP solver, rather than being decision variables. This is
    /// useful for handling boundary cases or invalid states. The goal is mainly to catch
    /// variables that are invalid but that the type system of ColloML cannot forbid.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment
    ///   - For generic variables (no `#[env]`): type is `&T::Env`
    ///   - For environment-specific variables (with `#[env(EnvType)]`): type is `&EnvType`
    ///
    /// # Returns
    ///
    /// * `None` - This variable instance is free to take any value (it's a decision variable)
    /// * `Some(value)` - This variable instance should be fixed to `value`
    ///
    /// # Generated Behavior
    ///
    /// The macro generates a `fix()` implementation based on the attributes:
    ///
    /// ## With `#[fix_with(expr)]`
    ///
    /// Returns `Some(expr)` if any `i32` parameter is outside its specified range, and `None`
    /// otherwise. The expression can reference:
    /// - `env` - The environment (if `#[env(EnvType)]` is used)
    /// - Field names (for named fields) or `v0`, `v1`, etc. (for unnamed fields)
    ///
    /// Default fix value is:
    /// - `0.0` by default
    /// - Can be overridden with `#[fix_with(value)]` on the enum
    /// - Can be overridden with `#[fix_with(value)]` on the variant
    ///
    /// ## With `#[defer_fix(expr)]`
    ///
    /// Returns the result of evaluating `expr` directly, completely replacing range checking.
    /// The expression must return `Option<f64>`.
    ///
    /// # Examples
    ///
    /// ## Basic Range Checking
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[fix_with(0.0)]
    /// enum Var {
    ///     TimeSlot {
    ///         #[range(0..7)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let env = MyEnv::new();
    ///
    /// let valid_var = Var::TimeSlot { day: 3, hour: 10 };
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&valid_var, &env), None);  // Free variable
    ///
    /// let invalid_var = Var::TimeSlot { day: 10, hour: 10 };  // day out of range
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&invalid_var, &env), Some(0.0));  // Fixed
    /// ```
    ///
    /// ## Dynamic Fix Based on Environment
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum Var {
    ///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
    ///     TimeSlot {
    ///         #[range(0..7)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv { lunch_mandatory: true, /* ... */ };
    /// let var = Var::TimeSlot { day: 10, hour: 10 };  // Out of range
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&var, &env), Some(1.0));  // Uses env.lunch_mandatory
    /// ```
    ///
    /// ## Dynamic Fix Based on Field Values
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum Var {
    ///     #[fix_with(if hour >= 12 && hour < 14 { 2.0 } else { 0.0 })]
    ///     WorkSlot {
    ///         #[range(0..5)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv::new();
    /// let var = Var::WorkSlot { day: 10, hour: 13 };  // day out of range, hour in lunch
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&var, &env), Some(2.0));  // Lunch hour penalty
    /// ```
    ///
    /// ## Complex Logic with defer_fix
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum Var {
    ///     #[defer_fix(Self::check_availability(env, student, week))]
    ///     StudentAvailable {
    ///         student: StudentId,
    ///         #[range(0..env.max_week)]
    ///         week: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv::new();
    /// let var = Var::StudentAvailable {
    ///     student: StudentId(0),
    ///     week: 1,
    /// };
    ///
    /// // Returns Some(10.0) if student is absent that week, None otherwise
    /// let fix_value = <Var as EvalVar<ObjectId>>::fix(&var, &env);
    /// ```
    ///
    fn fix(&self, env: &T::Env) -> Option<f64>;
}
