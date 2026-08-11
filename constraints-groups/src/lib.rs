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
//! The model holds the base `StudentInGroup` binaries — the assignment matrix
//! itself, one variable per (list, student, group), and the only variable of
//! the model — the `SharedPair` extras of piece 7 and the `RefGroupInGroup`
//! extras that count how many pieces a list breaks a reference group into, the
//! shape constraints of piece 8 (one group per student, min and max size), the
//! two-term stability objective of piece 9 (minimize the shared student pairs
//! *and* the shattering of the template, with configurable weights —
//! [`ObjectiveWeights`], piece 11), and the incremental epochs
//! ([`build_incremental_epochs`]), which callers feed to the solver.
//!
//! The epochs stagger the solve. Strict inclusion of the specs' student sets
//! (piece 10) orders them into *levels*, so the inclusion-minimal lists are
//! built first and every larger list aligns with the ones already fixed. A
//! level is not solved in one go, though: every spec gets an epoch of its own,
//! and inside a level the least entangled lists — those sharing the fewest
//! students with the rest of the level — run before the more entangled ones,
//! smaller before larger on a tie (pieces 12 and 12bis). See
//! [`build_incremental_epochs`] for the two passes that build them.
//!
//! The *template* ([`GenerationPlan::ghost`]) is a grouping of every student
//! at the canonical group size, which the objective asks the real lists to
//! reuse. It is plan data, computed by clustering ([`ghost`]), not something
//! the solver decides.
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
pub mod ghost;
mod incremental;
mod objective;
mod specs;
mod types;
pub mod vars;

pub use builder::{build_model, build_model_with_log};
pub use convert::build_group_lists;
pub use ghost::GhostGrouping;
pub use incremental::build_incremental_epochs;
pub use objective::ObjectiveWeights;
pub use specs::{
    GenerationPlan, GenerationPlanError, GenerationRequest, GroupListSpec, GroupListSpecError,
    RangeSource, build_generation_plan,
};
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::{GroupListIdx, RefGroupIdx, SizeClassIdx, Var};

pub type GroupListsModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;
