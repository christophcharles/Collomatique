//! Collomatique-core
//!
//! This create contains the GUI independant code. This avoids duplicating effort
//! and regroup various parts that are useful for various interfaces.
//!
//! In practice, this contains the actual "elementary" operations that should be
//! presented to a user, it also contains RPC logic annd it contains the Python
//! interface code.
//!

pub mod ops;
pub mod python;
pub mod rpc;
pub mod solver;
