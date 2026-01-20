use super::FieldConversionError;
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
