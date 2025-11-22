//! Constraints module
//!
//! This module defines all the necessary constraints to define a colloscope.
//!
//! Each constraint is in its own module.
//! Some constraints are needed for all colloscopes. Some are extra contraints.
//! Refer to each documentation.

pub mod group_count;
pub mod groups_per_slots;
pub mod incompat_for_single_week;
pub mod one_interrogation_at_a_time;
pub mod sealed_groups;
pub mod strict_limits;
pub mod students_per_groups;
pub mod students_per_groups_for_subject;
