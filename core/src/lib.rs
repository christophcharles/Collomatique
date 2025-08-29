//! Core functionnality of collomatique
//!
//! This crate defines the core functionnality of collomatique in a
//! UI-agnostic way. This should allow for implementation of different
//! UIs all using the same core code.

pub mod colloscopes;
pub mod history;
pub mod state;
pub mod tools;
pub mod traits;

pub use state::{AppSession, AppState};
pub use traits::{InMemoryData, Operation};
