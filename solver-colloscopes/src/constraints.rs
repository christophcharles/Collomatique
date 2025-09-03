//! Constraints module
//!
//! This module defines all the necessary constraints to define a colloscope.
//!
//! The main constraints are described in the module [main]. They define what a colloscope
//! is but do not add anything extra.
//!
//! Other possible constraints are defined in the various other submodules and allow
//! a finer description of your colloscope problem.

pub mod group_count;
pub mod groups_per_slots;
pub mod students_per_groups;
