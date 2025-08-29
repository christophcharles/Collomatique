//! State crate
//!
//! This crate defines the everything needed to maintain
//! the state of an editor in a UI-agnostic way.
//! This should allow for implementation of different
//! UIs all using the same state code.

pub mod history;
pub mod state;
pub mod tools;
pub mod traits;

pub use state::{AppSession, AppState};
pub use traits::{InMemoryData, Operation};
