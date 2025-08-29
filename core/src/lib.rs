//! Collomatique-core
//! ---
//!
//! This crate contains the main logic of Collomatique for solving colloscopes.
//! The goal for this crate is to fulfill the role of a translator. It takes a description
//! of a colloscope (or at least the various constraints of a colloscope) and returns
//! an ILP problem as described by the crate [collomatique-ilp].
//!
//! Similarly, it can translate a solution of an ILP problem into the description of
//! an actual colloscope and conversly, it can take the description of a colloscope
//! and turn it into an ILP configuration. This is useful to check in real time if
//! a colloscope satisfies all the constraints and helps when constructing a colloscope
//! incrementally.
//!
//! This crate however does not expose a user-friendly interface. The reason is, to
//! make the translation algorithm as thin as possible, and its verification as easy as
//! possible, I strive to make the colloscopes constraints and the actual colloscopes
//! representation the least redundant possible.
//!
//! It is the job of other crates to provide necessary utilities to make working the algorithm
//! somewhat pleasant.
//!
