use super::{EvalObject, FieldType};
use crate::eval::ExprValue;
use std::collections::{BTreeSet, HashMap};

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
