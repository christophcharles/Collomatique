//! Standalone group-list generation model.
//!
//! This crate builds an ILP model that prefills group lists before — and
//! independently of — any colloscope resolution. See
//! `docs/plans/auto_group_lists_plan.md` for the full design. The model is
//! indexed by deduplicated [`GroupListSpec`]s, not by subjects: the
//! translation between document state and model lives here too
//! ([`build_generation_plan`] on the way in, [`build_group_lists`] on the
//! way out).
//!
//! The model is complete (end of phase B): the base `StudentGroup`
//! variables, the reified extras of piece 7, the shape constraints of
//! piece 8 (max size, conditional min size, ascending fill order), the
//! stability objective of piece 9 (minimize non-empty groups plus globally
//! shared student pairs, equal weights), and the inclusion-based
//! incremental epochs of piece 10 ([`build_incremental_epochs`]), which
//! callers feed to the solver so the inclusion-minimal lists are built
//! first and the larger lists align with them.

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
pub use specs::{
    GenerationPlan, GenerationPlanError, GenerationRequest, GroupListSpec, build_generation_plan,
};
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::{GroupListIdx, Var};

pub type GroupListsModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;
