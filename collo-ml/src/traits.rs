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

mod errors;
mod eval_object;
mod eval_var;
mod field_types;
mod view;

pub use errors::{FieldConversionError, TypeConversionError, VarConversionError};
pub use eval_object::EvalObject;
pub use eval_var::EvalVar;
pub use field_types::{FieldType, SimpleFieldType};
pub use view::{ViewBuilder, ViewObject};
