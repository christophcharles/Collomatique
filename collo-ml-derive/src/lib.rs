//! Procedural macros for the collo-ml DSL view pattern.
//!
//! This crate provides derive macros that automatically generate the boilerplate code needed
//! to expose your application's data to the DSL interpreter using the view pattern.
//!
//! # Available Macros
//!
//! - [`ViewObject`] - Derives field schema and accessor methods for view structs
//! - [`EvalObject`] - Derives the complete `EvalObject` trait implementation for ID enums
//!
//! # Overview
//!
//! The view pattern requires three components:
//!
//! 1. **View objects** - Structs representing data accessible from the DSL
//! 2. **Object ID enum** - An enum of all object ID types
//! 3. **View builders** - Manual implementations connecting IDs to view objects
//!
//! These macros automate components 1 and 2, leaving only the view builders to implement manually.
//!
//! # Quick Start
//!
//! ```ignore
//! use collo_ml_derive::{ViewObject, EvalObject};
//!
//! // 1. Define view objects
//! #[derive(ViewObject, Clone)]
//! #[eval_object(ObjectId)]
//! struct Student {
//!     age: i32,
//!     enrolled: bool,
//! }
//!
//! // 2. Define the object ID enum
//! #[derive(EvalObject)]
//! #[env(MyEnv)]
//! enum ObjectId {
//!     Student(StudentId),
//! }
//!
//! // 3. Implement ViewBuilder manually (not shown)
//! ```
//!
//! # Features
//!
//! ## Hidden Fields
//!
//! Fields marked `#[hidden]` are excluded from the DSL but remain accessible in Rust:
//!
//! ```ignore
//! #[derive(ViewObject)]
//! #[eval_object(ObjectId)]
//! struct Student {
//!     age: i32,
//!     #[hidden]
//!     internal_id: String,  // Not accessible from DSL
//! }
//! ```
//!
//! ## Pretty Printing
//!
//! Custom formatting for debug output and user-facing display:
//!
//! ```ignore
//! #[derive(ViewObject)]
//! #[eval_object(ObjectId)]
//! #[pretty("{name} (age {age})")]
//! struct Student {
//!     age: i32,
//!     #[hidden]
//!     name: String,
//! }
//! ```
//!
//! ## Caching
//!
//! Automatic caching of constructed view objects for performance:
//!
//! ```ignore
//! #[derive(EvalObject)]
//! #[env(MyEnv)]
//! #[cached]  // Auto-generates ObjectIdCache
//! enum ObjectId {
//!     Student(StudentId),
//! }
//! ```
//!
//! ## Custom Type Names
//!
//! DSL type names can differ from Rust variant names:
//!
//! ```ignore
//! #[derive(EvalObject)]
//! #[env(MyEnv)]
//! enum ObjectId {
//!     #[name("Classroom")]
//!     RoomNumber(RoomId),
//! }
//! ```

use proc_macro::TokenStream;

mod eval_object;
mod eval_var;
mod view_object;

/// Derives the `ViewObject` trait for a struct.
///
/// This macro generates the field schema, field accessor, and optional pretty-print
/// implementation for a view object struct. View objects represent data that can be
/// accessed from the DSL.
///
/// # Required Attributes
///
/// - `#[eval_object(Type)]` - Specifies which `EvalObject` enum this view belongs to
///
/// # Optional Attributes
///
/// ## On the struct:
///
/// - `#[pretty("format")]` - Provides a format string for pretty printing
///
/// ## On fields:
///
/// - `#[hidden]` - Excludes the field from DSL access while keeping it in the struct
///
/// # Supported Field Types
///
/// - `i32` - Integer values
/// - `bool` - Boolean values
/// - `IdType` - Object references (must implement `Into<EvalObject>`)
/// - `Vec<T>` - Collections of any supported type (including nested collections)
///
/// # Requirements
///
/// - The struct must have named fields (tuple structs are not supported)
/// - If the corresponding `EvalObject` uses `#[cached]`, the struct must implement `Clone`
/// - Object reference types must implement `Into` for the specified `EvalObject` type
///   If you use types that appear in the enums of the EvalObject, this is auto-generated.
///
/// # Examples
///
/// ## Basic usage
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// struct Student {
///     age: i32,
///     enrolled: bool,
///     room: RoomId,
/// }
/// ```
///
/// ## With hidden fields
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// struct Student {
///     age: i32,
///     #[hidden]
///     internal_id: String,  // Not accessible from DSL
/// }
/// ```
///
/// ## With pretty printing
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// #[pretty("{name} is {age} years old")]
/// struct Student {
///     age: i32,
///     #[hidden]
///     name: String,  // Can still be used in format string
/// }
/// ```
///
/// ## With collections
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// struct Teacher {
///     name: String,
///     students: Vec<StudentId>,
///     grades: Vec<i32>,
/// }
/// ```
///
/// ## With nested collections
///
/// ```ignore
/// #[derive(ViewObject)]
/// #[eval_object(ObjectId)]
/// struct Course {
///     student_groups: Vec<Vec<StudentId>>,
/// }
/// ```
///
/// # Generated Code
///
/// The macro generates:
///
/// ```ignore
/// impl ::collo_ml::ViewObject for Student {
///     type EvalObject = ObjectId;
///     
///     fn field_schema() -> HashMap<String, FieldType> {
///         // Maps field names to their types
///     }
///     
///     fn get_field(&self, field: &str) -> Option<FieldValue<ObjectId>> {
///         // Match on field name and return value
///     }
///     
///     fn pretty_print(&self) -> Option<String> {
///         // Generated from #[pretty] or returns None
///     }
/// }
/// ```
///
/// # Format String Syntax
///
/// The `#[pretty("...")]` attribute supports standard Rust format string syntax:
///
/// - `{field}` - Simple field substitution
/// - `{field:?}` - Debug formatting
/// - `{field:width}` - Width specifier
/// - Multiple occurrences of the same field are allowed
///
/// All fields referenced in the format string must exist in the struct (including hidden fields).
///
/// # Panics
///
/// The macro panics at compile time if:
/// - The `#[eval_object(...)]` attribute is missing
/// - Applied to something other than a struct with named fields
/// - A format string references a non-existent field
/// - An unsupported type is used in a field
#[proc_macro_derive(ViewObject, attributes(eval_object, hidden, pretty))]
pub fn derive_view_object(input: TokenStream) -> TokenStream {
    view_object::derive(input)
}

/// Derives the `EvalObject` trait for an enum of object IDs.
///
/// This macro generates the complete `EvalObject` trait implementation, including field access,
/// type enumeration, schema generation, and optional caching support. The enum represents all
/// possible object types accessible from the DSL.
///
/// # Required Attributes
///
/// - `#[env(Type)]` - Specifies the environment type containing your data
///
/// # Optional Attributes
///
/// ## On the enum:
///
/// - `#[cached]` - Enables caching with auto-generated cache name `{EnumName}Cache`
/// - `#[cached(Name)]` - Enables caching with custom cache struct name
///
/// ## On variants:
///
/// - `#[name("TypeName")]` - Overrides the DSL type name (default is variant name)
///
/// # Requirements
///
/// - Must be applied to an enum
/// - Each variant must have exactly one unnamed field (the ID type)
/// - For each variant, `ViewBuilder<Env, IdType>` must be implemented on the enum
/// - If `#[cached]` is used, all corresponding `ViewObject` types must implement `Clone`
/// - All ID types must be `Clone` and comparable
///
/// # Examples
///
/// ## Basic usage
///
/// ```ignore
/// #[derive(EvalObject)]
/// #[env(MyApplicationEnv)]
/// enum ObjectId {
///     Student(StudentId),
///     Room(RoomId),
/// }
/// ```
///
/// ## With caching (auto-named)
///
/// ```ignore
/// #[derive(EvalObject)]
/// #[env(MyEnv)]
/// #[cached]  // Generates ObjectIdCache struct
/// enum ObjectId {
///     Student(StudentId),
///     Room(RoomId),
/// }
/// ```
///
/// ## With caching (custom name)
///
/// ```ignore
/// #[derive(EvalObject)]
/// #[env(MyEnv)]
/// #[cached(MyCustomCache)]
/// enum ObjectId {
///     Student(StudentId),
///     Room(RoomId),
/// }
/// ```
///
/// ## With custom type names
///
/// ```ignore
/// #[derive(EvalObject)]
/// #[env(MyEnv)]
/// enum ObjectId {
///     Student(StudentId),
///     #[name("Classroom")]
///     RoomNumber(RoomId),  // "Classroom" in DSL, RoomNumber in Rust
/// }
/// ```
///
/// # Generated Code
///
/// ## Without Caching
///
/// ```ignore
/// impl ::collo_ml::EvalObject for ObjectId {
///     type Env = MyEnv;
///     type Cache = ();
///     
///     fn objects_with_typ(env: &Self::Env, name: &str) -> BTreeSet<Self> {
///         // Dispatch to ViewBuilder::enumerate based on type name
///     }
///     
///     fn typ_name(&self, env: &Self::Env) -> String {
///         // Return type name based on variant
///     }
///     
///     fn field_access(&self, env: &Self::Env, cache: &mut Self::Cache, field: &str)
///         -> Option<ExprValue<Self>>
///     {
///         // Build view object and access field
///     }
///     
///     fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
///         // Generate schema for all types
///     }
///     
///     fn pretty_print(&self, env: &Self::Env, cache: &mut Self::Cache) -> Option<String> {
///         // Build view object and call its pretty_print
///     }
/// }
///
/// // Also generates From<IdType> for ObjectId for each variant
/// impl From<StudentId> for ObjectId { /* ... */ }
/// impl From<RoomId> for ObjectId { /* ... */ }
/// ```
///
/// ## With Caching
///
/// Additionally generates:
///
/// ```ignore
/// pub struct ObjectIdCache {
///     student_cache: BTreeMap<StudentId, Student>,
///     room_cache: BTreeMap<RoomId, Room>,
/// }
///
/// impl Default for ObjectIdCache {
///     fn default() -> Self {
///         Self {
///             student_cache: BTreeMap::new(),
///             room_cache: BTreeMap::new(),
///         }
///     }
/// }
/// ```
///
/// The cached implementation of `field_access` and `pretty_print` checks the cache first
/// before calling `ViewBuilder::build`.
///
/// # Cache Behavior
///
/// When caching is enabled:
/// - View objects are built once per ID and cached
/// - The cache is checked before every `ViewBuilder::build` call
/// - Objects are stored by ID in a `BTreeMap`
/// - The cache is passed mutably to allow updates
/// - Each object type has its own cache field
///
/// # Type Name Resolution
///
/// The macro handles type name resolution for object references:
/// - View objects use `TypeId` for object references in their schema
/// - The macro generates code to map `TypeId` to DSL type names
/// - This mapping is based on the enum variants and their `#[name]` attributes
///
/// # Panics
///
/// The macro panics at compile time if:
/// - The `#[env(...)]` attribute is missing
/// - Applied to something other than an enum
/// - A variant has zero fields or multiple fields
/// - A variant has named fields instead of unnamed fields
#[proc_macro_derive(EvalObject, attributes(env, cached, name))]
pub fn derive_eval_object(input: TokenStream) -> TokenStream {
    eval_object::derive(input)
}

/// Derives the `EvalVar` trait for an enum of ILP variables.
///
/// This macro generates the complete `EvalVar<T>` trait implementation, including field schema,
/// variable enumeration, and fix value computation. The enum represents all possible variable
/// types in your Integer Linear Programming problem.
///
/// # Key Design
///
/// The trait is **generic over the `EvalObject` type**, allowing the same variable definitions
/// to work with different data sources. When using environment-dependent expressions, specify
/// `#[env(EnvType)]` to enable access to environment fields.
///
/// # Optional Attributes
///
/// ## On the enum:
///
/// - `#[env(EnvType)]` - Specifies the concrete environment type (required for env-dependent expressions)
/// - `#[fix_with(value)]` - Sets the default fix value for out-of-range variables (default: 0.0)
///
/// ## On variants:
///
/// - `#[name("VarName")]` - Overrides the DSL variable name (default is variant name)
/// - `#[var(expr)]` - Sets the variable type (default: `Variable::binary()`)
/// - `#[fix_with(expr)]` - Expression returning `f64`, evaluated when any field is out of range
/// - `#[defer_fix(expr)]` - Expression returning `Option<f64>`, replaces all range checking
///
/// ## On fields:
///
/// - `#[range(expr)]` - **Required for `i32` fields**. Expression returning a range of i32
///
/// # Supported Field Types
///
/// - `i32` - **Must** have `#[range(...)]` attribute
/// - `bool` - Automatically enumerates `true` and `false`
/// - Object ID types - Enumerated via `EvalObject::objects_with_typ()`
///
/// # Requirements
///
/// - Must be applied to an enum
/// - All `i32` fields must have `#[range(...)]` attribute
/// - For object types, `IdType: TryFrom<T>` must be satisfied
/// - The enum must implement standard derives: `Clone, PartialEq, Eq, PartialOrd, Ord`
///
/// # Examples
///
/// ## Basic usage (fully generic)
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
/// ## With custom DSL names
///
/// ```ignore
/// #[derive(EvalVar)]
/// enum Var {
///     #[name("SiG")]  // Called "SiG" in DSL scripts
///     StudentInGroup(StudentId, GroupId),
/// }
/// ```
///
/// ## With dynamic ranges from environment
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]  // Required for env access
/// #[fix_with(0.0)]
/// enum Var {
///     StudentInWeek {
///         student: StudentId,
///         #[range(0..env.max_week)]  // Range depends on environment!
///         week: i32,
///     },
/// }
/// ```
///
/// ## With dynamic fix values based on environment
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..env.last_hour)]  // Dynamic range
///         hour: i32,
///     },
/// }
/// ```
///
/// ## With fix values based on field values (named fields)
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
///     WorkSlot {
///         #[range(0..5)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,  // Can reference 'hour' in fix_with
///     },
/// }
/// ```
///
/// ## With fix values based on field values (unnamed fields)
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     // Use v0, v1, etc. for unnamed fields
///     #[fix_with(if *v1 >= 12 && *v1 < 14 { 2.0 } else { 0.0 })]
///     TimeSlot(
///         #[range(0..7)] i32,
///         #[range(8..18)] i32,
///     ),
/// }
/// ```
///
/// ## With defer_fix for complex logic
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
/// impl Var {
///     fn check_availability(
///         env: &DynamicEnv,
///         student: &StudentId,
///         week: &i32,
///     ) -> Option<f64> {
///         let env = env.as_ref();
///         if env.is_student_absent(student, *week) {
///             Some(10.0)  // High penalty for unavailable slots
///         } else {
///             None
///         }
///     }
/// }
/// ```
///
/// ## With inline defer_fix expression
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[defer_fix({
///         if *week >= 3 {
///             Some(5.0)
///         } else {
///             None
///         }
///     })]
///     LateWeekTask {
///         task: TaskId,
///         #[range(0..env.max_week)]
///         week: i32,
///     },
/// }
/// ```
///
/// ## With custom variable types
///
/// ```ignore
/// #[derive(EvalVar)]
/// enum Var {
///     #[var(Variable::binary())]
///     IsSelected(TaskId),
///     
///     #[var(Variable::integer())]
///     Count(#[range(0..100)] i32),
///     
///     #[var(Variable::continuous().min(0.0).max(1.0))]
///     Proportion(ProjectId),
/// }
/// ```
///
/// ## With boolean fields
///
/// ```ignore
/// #[derive(EvalVar)]
/// enum Var {
///     TaskEnabled {
///         task: TaskId,
///         is_enabled: bool,  // No range needed - automatically [false, true]
///     },
/// }
/// ```
///
/// # Generated Code
///
/// ## Without #[env(...)] - Fully Generic
///
/// ```ignore
/// impl<__T: EvalObject> EvalVar<__T> for Var
/// where
///     StudentId: TryFrom<__T, Error = TypeConversionError>,
///     GroupId: TryFrom<__T, Error = TypeConversionError>,
/// {
///     fn field_schema() -> HashMap<String, Vec<FieldType>> {
///         // Maps DSL names to parameter types
///     }
///     
///     fn vars(env: &__T::Env) -> Result<BTreeMap<Self, Variable>, TypeId> {
///         // Generates cartesian product of all parameter combinations
///         // Returns Err(type_id) if an object type is unknown
///     }
///     
///     fn fix(&self, env: &__T::Env) -> Option<f64> {
///         // Returns Some(fix_value) if any i32 is out of range
///     }
/// }
/// ```
///
/// ## With #[env(EnvType)] - Environment-Specific
///
/// ```ignore
/// impl<__T> EvalVar<__T> for Var
/// where
///     __T: EvalObject<Env = DynamicEnv>,  // Constrained to specific env!
///     StudentId: TryFrom<__T, Error = TypeConversionError>,
///     GroupId: TryFrom<__T, Error = TypeConversionError>,
/// {
///     fn field_schema() -> HashMap<String, Vec<FieldType>> {
///         // Maps DSL names to parameter types
///     }
///     
///     fn vars(env: &DynamicEnv) -> Result<BTreeMap<Self, Variable>, TypeId> {
///         // env is concrete &DynamicEnv - can access env.max_week!
///         // Generates cartesian product of all parameter combinations
///         // Variables with defer_fix returning Some are excluded
///     }
///     
///     fn fix(&self, env: &DynamicEnv) -> Option<f64> {
///         // env is concrete &DynamicEnv - can access env.lunch_mandatory!
///         // Evaluates fix_with or defer_fix expressions
///     }
/// }
/// ```
///
/// ## TryFrom Implementation
///
/// ```ignore
/// // Also generates generic TryFrom for converting from DSL representation
/// impl<__T: EvalObject> TryFrom<&ExternVar<__T>> for Var
/// where
///     StudentId: TryFrom<__T>,
///     GroupId: TryFrom<__T>,
/// {
///     type Error = VarConversionError;
///     
///     fn try_from(value: &ExternVar<__T>) -> Result<Self, Self::Error> {
///         // Matches DSL name and converts parameters
///     }
/// }
/// ```
///
/// # Usage with Different EvalObjects
///
/// ## Generic Variables (no #[env])
///
/// The fully generic design allows using the same `Var` with different data sources:
///
/// ```ignore
/// // Production use with real database
/// let vars = <Var as EvalVar<ProductionObjectId>>::vars(&production_env)?;
///
/// // Testing with mock data
/// let vars = <Var as EvalVar<MockObjectId>>::vars(&test_env)?;
/// ```
///
/// ## Environment-Specific Variables (with #[env])
///
/// When using `#[env(EnvType)]`, the variable works with any `EvalObject` that has that environment:
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum DynamicVar {
///     #[range(0..env.max_week)]
///     StudentInWeek {
///         student: StudentId,
///         week: i32,
///     },
/// }
///
/// // Both ObjectId types use DynamicEnv
/// #[derive(EvalObject)]
/// #[env(DynamicEnv)]
/// enum ObjectId { /* ... */ }
///
/// #[derive(EvalObject)]
/// #[env(DynamicEnv)]
/// enum MockObjectId { /* ... */ }
///
/// // DynamicVar works with BOTH!
/// let vars1 = <DynamicVar as EvalVar<ObjectId>>::vars(&env)?;
/// let vars2 = <DynamicVar as EvalVar<MockObjectId>>::vars(&env)?;
/// ```
///
/// # Variable Enumeration
///
/// The `vars()` method generates all valid variable combinations:
///
/// - **i32**: Iterates through the specified range `start..end` (exclusive end)
///   - Ranges can be static: `#[range(0..10)]`
///   - Or dynamic from environment: `#[range(0..env.max_week)]`
/// - **bool**: Iterates through `[false, true]`
/// - **Objects**: Calls `T::objects_with_typ(env, type_name)` and converts to the ID type
///
/// **Filtering with defer_fix**: Variables where `defer_fix` returns `Some(_)` are automatically
/// excluded from `vars()`.
///
/// For example:
/// ```ignore
/// // Static ranges
/// StudentInGroup(StudentId, GroupId)
/// // With 3 students and 5 groups → generates 15 variables
///
/// TimeSlot { day: i32, hour: i32 }  // #[range(0..7)] and #[range(8..18)]
/// // 7 days × 10 hours → generates 70 variables
///
/// // Dynamic ranges
/// #[env(DynamicEnv)]
/// StudentInWeek { student: StudentId, week: i32 }  // #[range(0..env.max_week)]
/// // With 3 students and env.max_week = 4 → generates 12 variables
/// // With env.max_week = 2 → generates 6 variables
/// ```
///
/// # Fix Values
///
/// The `fix()` method returns a value for variables that should be fixed in the ILP solver:
///
/// ## With fix_with
///
/// - Returns `None` if all parameters are within their valid ranges
/// - Returns `Some(fix_value)` if any `i32` parameter is out of range
/// - The `fix_value` can be:
///   - Static: `#[fix_with(1.0)]`
///   - Based on environment: `#[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]`
///   - Based on field values (named): `#[fix_with(if *hour >= 12 { 2.0 } else { 0.0 })]`
///   - Based on field values (unnamed): `#[fix_with(if *v0 > 10 { 1.0 } else { 0.0 })]`
///
/// ## With defer_fix
///
/// - Completely replaces range checking
/// - Returns `Option<f64>` directly from the expression
/// - Can call custom functions: `#[defer_fix(Self::check(env, student, week))]`
/// - Can use inline logic: `#[defer_fix({ if *week >= 3 { Some(5.0) } else { None } })]`
/// - Variables where `defer_fix` returns `Some(_)` are excluded from `vars()`
///
/// ## Priority
///
/// When a field is out of range:
/// 1. If `defer_fix` is present → return `defer_fix` result
/// 2. Otherwise → return `fix_with` value (variant-level or enum-level default)
///
/// Useful for handling edge cases or enforcing constraints.
///
/// # Error Handling
///
/// ## Compile-time Errors
///
/// The macro panics at compile time if:
/// - Applied to non-enum types
/// - An `i32` field lacks `#[range(...)]` attribute
/// - A `bool` or object field has `#[range(...)]` attribute
/// - A `Vec` type is used (lists cannot be enumerated)
/// - Both `#[fix_with(...)]` and `#[defer_fix(...)]` are present on the same variant
/// - `#[env(...)]` is missing but expressions reference `env` (compile error in generated code)
///
/// ## Runtime Errors
///
/// `vars()` returns `Err(type_id)` if an object type's name cannot be resolved via
/// `T::type_id_to_name()`. This typically indicates a mismatch between your variable
/// definition and your `EvalObject` implementation.
///
/// `TryFrom` returns `VarConversionError` if:
/// - The DSL variable name is unknown
/// - Parameter count doesn't match
/// - A parameter has the wrong type
///
/// ## Range Expression Errors
///
/// Range expressions (e.g., `#[range(0..env.max_week)]`) are evaluated at runtime:
/// - If the expression panics, `vars()` will panic
/// - If the expression produces an invalid range (start > end), behavior is defined by Rust's Range
/// - Empty ranges (start == end) generate zero variables
///
/// # Integration with ColloML
///
/// Variables work seamlessly with ColloML constraint scripts:
///
/// ```ignore
/// // Rust definition
/// #[derive(EvalVar)]
/// enum Var {
///     #[name("SiG")]
///     StudentInGroup(StudentId, GroupId),
/// }
///
/// // ColloML script
/// pub let constraints() -> [Constraint] = [
///     sum g in @[Group] { $SiG(s, g) } === 1
///     for s in @[Student]
/// ];
/// ```
///
/// # The #[env(...)] Attribute
///
/// ## When to Use
///
/// Use `#[env(EnvType)]` when your variable definitions need to access environment fields:
///
/// - **Dynamic ranges**: `#[range(0..env.max_week)]`
/// - **Environment-dependent fix values**: `#[fix_with(if env.flag { 1.0 } else { 0.0 })]`
/// - **Complex environment checks**: `#[defer_fix(env.check_availability(...))]`
///
/// Without `#[env(...)]`, the environment type is generic (`&__T::Env`), preventing field access.
///
/// ## How It Works
///
/// ```ignore
/// // Without #[env] - Fully generic
/// #[derive(EvalVar)]
/// enum StaticVar {
///     #[range(0..10)]  // Static range - works with any EvalObject
///     Slot { slot: i32 },
/// }
/// // Generated: impl<__T: EvalObject> EvalVar<__T> for StaticVar
///
/// // With #[env] - Environment-specific
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum DynamicVar {
///     #[range(0..env.max_week)]  // Can access env.max_week
///     Slot { slot: i32 },
/// }
/// // Generated: impl<__T: EvalObject<Env = DynamicEnv>> EvalVar<__T> for DynamicVar
/// ```
///
/// # Variable Type Expressions
///
/// The `#[var(...)]` attribute accepts any expression that evaluates to `Variable`:
///
/// ```ignore
/// #[var(Variable::binary())]                    // Binary variable (0 or 1)
/// #[var(Variable::integer())]                   // Integer variable
/// #[var(Variable::continuous())]                // Continuous variable
/// #[var(Variable::integer().min(0).max(100))]   // With bounds
/// ```
#[proc_macro_derive(EvalVar, attributes(name, var, range, fix_with, defer_fix, env))]
pub fn derive_eval_var(input: TokenStream) -> TokenStream {
    eval_var::derive(input)
}
