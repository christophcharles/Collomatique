//! State crate
//!
//! This crate defines the everything needed to maintain
//! the state of an editor in a UI-agnostic way.
//! This should allow for implementation of different
//! UIs all using the same state code.

pub mod history;
pub mod ids;
pub mod state;
pub mod tables;
#[cfg(test)]
mod test_utils;
pub mod tools;
pub mod traits;

pub use ids::Id;
pub use state::{AppSession, AppState};
pub use tables::{OrderedTable, Table};
pub use traits::{Description, InMemoryData, Operation};
