//! State crate
//!
//! This crate defines the everything needed to maintain
//! the state of an editor in a UI-agnostic way.
//! This should allow for implementation of different
//! UIs all using the same state code.

pub mod history;
pub mod ids;
pub mod join;
pub mod refs;
pub mod state;
pub mod tables;
#[cfg(test)]
mod test_utils;
pub mod tools;
pub mod traits;

/// Re-export the derive macros so users can write `#[derive(EntityId)]`,
/// `#[derive(References)]` and `#[derive(Join)]` after
/// `use collomatique_state::{EntityId, References, Join}`.
#[cfg(feature = "derive")]
pub use collomatique_state_derive::{EntityId, Join, References};

pub use ids::Id;
pub use join::{Join, Joinable, Lookup};
pub use refs::References;
pub use state::{AppSession, AppState};
pub use tables::{OrderedTable, Table};
pub use traits::{Description, InMemoryData, Operation};
