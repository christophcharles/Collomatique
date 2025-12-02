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
//! - [`FieldType`]: Intermediate representation of field types in view objects, using `TypeId`
//!   for object references
//! - [`FieldValue`]: Intermediate representation of field values, preserving type info for
//!   empty collections
//! - These are converted to [`ExprType`] and
//!   [`ExprValue`] by `EvalObject`
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

use crate::eval::ExprValue;
use crate::semantics::ExprType;
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
    /// # Arguemnts
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
    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>>;

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

/// Represents the type of a field in a view object.
///
/// This is an intermediate representation used between view objects and the final DSL type system.
/// It captures field types without requiring knowledge of the DSL type names for object references
/// (those are resolved later using `TypeId`).
///
/// # Variants
///
/// - `Int`: An integer field (`i32`)
/// - `Bool`: A boolean field
/// - `Object(TypeId)`: A reference to another object, identified by its Rust type's `TypeId`
/// - `List(Box<FieldType>)`: A collection (typically `Vec`) of values of the inner type
///
/// # Type Resolution
///
/// `Object` variants store a `TypeId` which is later mapped to DSL type names in a [ExprType] by the
/// [EvalObject] implementation. This allows view objects to be defined without knowledge
/// of the complete object hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldType {
    /// An integer field
    Int,
    /// A boolean field
    Bool,
    /// A reference to another object, identified by the Rust type's TypeId
    Object(std::any::TypeId),
    /// A collection of values of the specified type
    List(Box<FieldType>),
}

impl FieldType {
    pub fn convert_to_expr_type<T: EvalObject>(self) -> Result<ExprType, FieldConversionError> {
        match self {
            FieldType::Bool => Ok(ExprType::Bool),
            FieldType::Int => Ok(ExprType::Int),
            FieldType::List(typ) => Ok(ExprType::List(Box::new(typ.convert_to_expr_type::<T>()?))),
            FieldType::Object(type_id) => Ok(ExprType::Object(T::type_id_to_name(type_id)?)),
        }
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Bool => write!(f, "Bool"),
            FieldType::Int => write!(f, "Int"),
            FieldType::List(typ) => write!(f, "[{}]", typ),
            FieldType::Object(type_id) => write!(f, "Object({:?})", type_id),
        }
    }
}

/// Represents the value of a field from a view object.
///
/// This is an intermediate representation between view objects and the final [`ExprValue`] type.
/// Unlike [`ExprValue`], `FieldValue` includes type information for lists, which is necessary
/// to handle empty collections correctly.
///
/// # Type Parameters
///
/// * `T` - The `EvalObject` type that this value belongs to
///
/// # Variants
///
/// - `Int(i32)`: An integer value
/// - `Bool(bool)`: A boolean value
/// - `Object(T)`: A reference to another object
/// - `List(FieldType, Vec<FieldValue<T>>)`: A collection with its element type
///
/// # Conversion to ExprValue
///
/// `FieldValue` is converted to [ExprValue] by the [EvalObject] implementation, which resolves
/// `FieldType::Object(TypeId)` to `ExprType::Object(String)` using the type name mapping.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldValue<T: EvalObject> {
    /// An integer value
    Int(i32),
    /// A boolean value
    Bool(bool),
    /// A reference to another object
    Object(T),
    /// A collection of values with type information
    ///
    /// The `FieldType` describes the type of elements in the collection, which is essential
    /// for handling empty collections where the type cannot be inferred from the elements.
    List(FieldType, Vec<FieldValue<T>>),
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
    /// `Some(FieldValue)` if the field exists, `None` otherwise.
    /// Fields marked with `#[hidden]` return `None` when accessed.
    fn get_field(&self, field: &str) -> Option<FieldValue<Self::EvalObject>>;

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
/// # Type Parameter
///
/// - `T`: The `EvalObject` type that provides access to the underlying data via its environment
///
/// # Implementation
///
/// This trait is typically implemented via the `#[derive(EvalVar)]` macro on an enum:
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
/// The macro generates an implementation like:
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
/// # Usage Example
///
/// ```ignore
/// // Define variables
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
/// if let Some(value) = <Var as EvalVar<ObjectId>>::fix(&var) {
///     // This variable should be fixed to `value`
/// }
///
/// // Get the schema
/// let schema = Var::field_schema();
/// ```
///
/// # Variable Enumeration Strategy
///
/// The `vars()` method generates all valid combinations by taking the cartesian product of:
///
/// - **i32 parameters**: All values in the specified range (e.g., `0..10` → 10 values)
/// - **bool parameters**: Both `true` and `false` (2 values)
/// - **Object parameters**: All objects of that type from `T::objects_with_typ(env, type_name)`
///
/// For example:
/// ```ignore
/// TimeSlot {
///     #[range(0..7)]
///     day: i32,     // range 0..7  → 7 values
///     #[range(8..18)]
///     hour: i32,    // range 8..18 → 10 values
/// }
/// // Total: 7 × 10 = 70 variables
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
///
/// Return `None` if the variable is free to take any value, or `Some(value)` to fix it.
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
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data needed to enumerate objects
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
    /// - **i32**: Iterates through the range specified in `#[range(start..end)]`
    /// - **bool**: Enumerates `[false, true]`
    /// - **Objects**: Calls `T::objects_with_typ(env, type_name)` and converts to the ID type
    ///
    /// # Example
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
    /// # Returns
    ///
    /// * `None` - This variable instance is free to take any value (it's a decision variable)
    /// * `Some(value)` - This variable instance should be fixed to `value`
    ///
    /// # Default Behavior (Generated by Macro)
    ///
    /// The macro generates a `fix()` implementation that returns `Some(default_fix)` if any
    /// `i32` parameter is outside its specified range, and `None` otherwise. This prevents
    /// invalid variable instances from being created.
    ///
    /// The default fix value is:
    /// - `0.0` by default
    /// - Can be overridden with `#[default_fix(value)]` on the enum or variant
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[default_fix(0.0)]
    /// enum Var {
    ///     TimeSlot {
    ///         #[range(0..7)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let valid_var = Var::TimeSlot { day: 3, hour: 10 };
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&valid_var), None);  // Free variable
    ///
    /// let invalid_var = Var::TimeSlot { day: 10, hour: 10 };  // day out of range
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&invalid_var), Some(0.0));  // Fixed to 0.0
    /// ```
    ///
    fn fix(&self, env: &T::Env) -> Option<f64>;
}
