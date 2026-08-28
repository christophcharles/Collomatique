//! Standalone group-list generation.
//!
//! This crate fills group lists before — and independently of — any
//! colloscope resolution. It is indexed by deduplicated [`GroupListSpec`]s,
//! not by subjects: subjects sharing the same student set and the same size
//! range get one list between them. The translation from document state to
//! that indexed form lives here too ([`build_generation_plan`]), and the
//! generator's output is the payload
//! `GroupListsUpdateOp::AddGeneratedGroupLists` takes.
//!
//! [`greedy_group_lists`] is the whole generator. It maximizes the
//! **collision objective**: the total partner collision probability — the
//! chance that two of a student's grouping decisions point at the same person
//! — each meeting weighted by `1 / (group size − 1)`, so a meeting in a big
//! tutorial cannot buy the right to scatter someone's colle partners. The
//! algorithm is a prefill phase that tiles whole groups out of single
//! cohorts, then one joint placement per remaining student.
//!
//! The number of groups is *not* optimized: it is the closed-form minimum
//! `⌈n / max_size⌉`, and the sizes are the balanced targets
//! ([`targets::balanced_targets`](targets)), fixed before any placement. A
//! student count the size range cannot split at all is rejected upfront by
//! [`GroupListSpec::new`], so callers must build their specs through it — the
//! config dialog does, before offering a subject for rebuild.

mod greedy;
mod mass;
mod specs;
mod targets;

pub use greedy::{greedy_group_lists, greedy_group_lists_with_log};
pub use specs::{
    GenerationPlan, GenerationPlanError, GenerationRequest, GroupListSpec, GroupListSpecError,
    KeptList, build_generation_plan, default_generation_request,
};
