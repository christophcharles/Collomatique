//! Collomatique-core
//! ---
//!
//! This crate contains the main logic of Collomatique for solving colloscopes related problems.
//! The goal for this crate is to fulfill the role of a translator. It takes a description
//! of a colloscope (or at least the various constraints of a colloscope) and works with
//! [collomatique_solver] to be able to represent the problem and solve it using [collomatique_ilp].
//! It builds on [collomatique_solver::generics] and [collomatique_solver::solver] and uses its interfaces
//! to build a *nice-enough* interface for automatic solving of colloscopes.
//!
//! This crate however does not expose a user-friendly interface. The reason is, to
//! make the translation algorithm as thin as possible, and its verification as easy as
//! possible, I strive to make the colloscopes constraints and the actual colloscopes
//! representation the least redundant possible.
//!
//! Also to keep this part lean, a lot of information is not represented as it is not
//! needed to build the constraint system. For instance, the name of the students or
//! the name of the teachers are not stored in the structures of this modules. Students
//! and teachers are represented with numbers and that's it. It is the job of other crates
//! from collomatique to provide necessary utilities to make working the algorithm
//! somewhat pleasant.
//! 
//! The colloscope logic itself is found in the module [base].

pub mod colloscopes_draft;

pub mod base;
pub mod constraints;
