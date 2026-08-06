//! Standalone group-list generation model.
//!
//! This crate builds an ILP model that prefills group lists before — and
//! independently of — any colloscope resolution. The full design is the
//! retired roadmap, pinned at
//! `git show 5556784b:docs/plans/auto_group_lists_plan.md`. The model is
//! indexed by deduplicated [`GroupListSpec`]s, not by subjects: the
//! translation between document state and model lives here too
//! ([`build_generation_plan`] on the way in, [`build_group_lists`] on the
//! way out).
//!
//! The model is complete (end of phase B): the base `StudentInGroup`
//! binaries — the assignment matrix itself, one variable per (list,
//! student, group) — the `SharedPair` extras of piece 7, the shape constraints
//! of piece 8 (one group per student, min and max size), the stability
//! objective of piece 9
//! (minimize the globally shared student pairs, with a configurable weight
//! — [`ObjectiveWeights`], piece 11), and the inclusion-based incremental
//! epochs of piece 10 ([`build_incremental_epochs`]), which callers feed to
//! the solver so the inclusion-minimal lists are built first and the larger
//! lists align with them.
//!
//! The number of groups is *not* optimized: it is the closed-form minimum
//! `⌈n / max_size⌉` ([`vars::VarEnv::group_count`]), imposed on the model.
//! A student count the size range cannot split at all is rejected upfront
//! by [`GroupListSpec::new`], so callers must build their specs through it
//! — the config dialog does, before offering a subject for rebuild.

mod builder;
mod constraints;
mod convert;
mod extras;
mod incremental;
mod objective;
mod specs;
mod types;
pub mod vars;

pub use builder::{build_model, build_model_with_log};
pub use convert::build_group_lists;
pub use incremental::build_incremental_epochs;
pub use objective::ObjectiveWeights;
pub use specs::{
    GenerationPlan, GenerationPlanError, GenerationRequest, GroupListSpec, GroupListSpecError,
    RangeSource, build_generation_plan,
};
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::{GroupListIdx, SizeClassIdx, Var};

pub type GroupListsModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;
