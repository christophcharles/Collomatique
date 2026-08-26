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
//! [`ObjectiveWeights`], piece 11).
//!
//! The crate used to number *incremental epochs* over the specs (pieces 10, 12
//! and 12bis), so the solve could be staggered along strict inclusion of the
//! specs' student sets. Nothing seeds this model that way any more — the
//! greedy below supplies the initial solution — so the epochs are gone; the
//! pinned roadmap still describes them.
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
//!
//! The crate also hosts the *greedy* generator ([`greedy_group_lists`]),
//! which is the primary path: it reads the same [`GenerationPlan`] and
//! returns the same output as [`build_group_lists`], in negligible time,
//! maximizing a partner-concentration objective the ILP's per-pair step term
//! cannot express. The ILP above is the optional polish. See
//! `docs/plans/greedy_roadmap.md` and `docs/plans/greedy_algorithm.md`.

mod builder;
mod constraints;
mod convert;
mod extras;
pub mod ghost;
mod greedy;
mod objective;
mod specs;
mod types;
pub mod vars;

pub use builder::{build_model, build_model_with_log};
pub use convert::{build_group_lists, group_lists_to_warm_start};
pub use ghost::GhostGrouping;
pub use greedy::{greedy_group_lists, greedy_group_lists_with_log};
pub use objective::ObjectiveWeights;
pub use specs::{
    GenerationPlan, GenerationPlanError, GenerationRequest, GroupListSpec, GroupListSpecError,
    KeptList, RangeSource, build_generation_plan,
};
pub use types::{ConstraintDesc, ExtraVarName};
pub use vars::{GroupListIdx, RefGroupIdx, SizeClassIdx, Var};

pub type GroupListsModel = collomatique_ilp_modeler::Model<Var, ExtraVarName, ConstraintDesc>;
