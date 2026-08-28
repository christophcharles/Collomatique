//! Incompats submodule
//!
//! This module defines the relevant types to describes the schedule incompatibilities

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::partial_order::vec_subsequence;
use collomatique_state::{ContentOrd, Join, References};

use crate::Table;
use crate::ids::{IncompatId, NewId, SubjectId, WeekPatternId};
use crate::ops::AnnotatedIncompatOp;

/// Description of the schedule incompatibilities
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Incompats {
    /// Incompats for subjects
    ///
    /// Each item associates an incompat id to a schedule incompatibility
    pub incompat_map: Table<IncompatId, Incompatibility>,
}

/// Description of a single schedule incompat
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
#[join(error = NewId)]
pub struct Incompatibility {
    /// Subject the incompatibility is linked to
    ///
    /// Deliberately, this subject is *not* required to run interrogations of
    /// its own — unlike the subject references held by teachers, slots,
    /// balancing and group-list associations. This lets students be declared
    /// in a subject purely so that an incompatibility can block slots for them
    /// (the subject's own schedule creates the unavailability) without the
    /// subject having colles. Neither invariant checker enforces a
    /// "has interrogations" predicate on this edge, on purpose.
    #[fk(name = subject)]
    pub subject_id: SubjectId,
    /// Name of the incompatibility for clarity
    pub name: String,
    /// Slots of time when the students might not be available
    ///
    /// This is given as a weekday, a start time and a duration
    // Value-borne identity: a time window *is* its value, nothing points at
    // it by position, so dropping one anywhere in the list is removing
    // content (subsequence). The helper is needed because
    // `SlotWithDuration` is foreign and so carries no `ContentIdentity` for
    // the `Vec` blanket to use.
    #[ord(with = vec_subsequence)]
    pub slots: Vec<collomatique_time::SlotWithDuration>,
    /// Number of slots to force to be free in the above list
    pub minimum_free_slots: NonZeroU32,
    /// Week pattern for the incompatibility
    ///
    /// If `None`, this means every week
    #[fk(name = week_pattern)]
    pub week_pattern_id: Option<WeekPatternId>,
}

// The container's half of the dense renumbering walk (see [crate::compact]).
// The two methods must visit exactly the same id occurrences. The `slots` of an
// incompatibility are time windows, not ids.
impl Incompats {
    pub(crate) fn collect_ids(&self, ids: &mut std::collections::BTreeSet<u64>) {
        use crate::ids::Id as _;
        for (incompat_id, incompat) in self.incompat_map.iter() {
            ids.insert(incompat_id.inner());
            ids.insert(incompat.subject_id.inner());
            if let Some(week_pattern_id) = incompat.week_pattern_id {
                ids.insert(week_pattern_id.inner());
            }
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        Incompats {
            incompat_map: self
                .incompat_map
                .into_iter()
                .map(|(incompat_id, incompat)| {
                    let Incompatibility {
                        subject_id,
                        name,
                        slots,
                        minimum_free_slots,
                        week_pattern_id,
                    } = incompat;
                    (
                        remap(map, incompat_id),
                        Incompatibility {
                            subject_id: remap(map, subject_id),
                            name,
                            slots,
                            minimum_free_slots,
                            week_pattern_id: week_pattern_id.map(|id| remap(map, id)),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// Precondition errors of the forced incompat ops — the carve-out subset. Only
/// no-clobber and op-target existence survive; `validate_incompat` is stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum IncompatPrecheckError {
    /// A incompat id is invalid
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(IncompatId),

    /// The incompat id already exists
    #[error("incompat id ({0:?}) already exists")]
    IncompatIdAlreadyExists(IncompatId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies an incompat op: carve-out guards kept (returned as
    /// [IncompatPrecheckError]), invariant guards stripped. May leave the state
    /// invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_incompat(
        &mut self,
        incompat_op: &AnnotatedIncompatOp,
    ) -> std::result::Result<AnnotatedIncompatOp, IncompatPrecheckError> {
        match incompat_op {
            AnnotatedIncompatOp::Add(new_id, incompat) => {
                if self
                    .inner_data
                    .params
                    .incompats
                    .incompat_map
                    .contains(new_id)
                {
                    return Err(IncompatPrecheckError::IncompatIdAlreadyExists(*new_id));
                }
                // stripped: validate_incompat

                self.inner_data
                    .params
                    .incompats
                    .incompat_map
                    .insert(*new_id, incompat.clone());

                Ok(AnnotatedIncompatOp::Remove(*new_id))
            }
            AnnotatedIncompatOp::Remove(id) => {
                let Some(old_incompat) = self.inner_data.params.incompats.incompat_map.remove(id)
                else {
                    return Err(IncompatPrecheckError::InvalidIncompatId(*id));
                };

                Ok(AnnotatedIncompatOp::Add(*id, old_incompat))
            }
            AnnotatedIncompatOp::Update(incompat_id, new_incompat) => {
                // stripped: validate_incompat

                let Some(incompat) = self
                    .inner_data
                    .params
                    .incompats
                    .incompat_map
                    .get_mut(incompat_id)
                else {
                    return Err(IncompatPrecheckError::InvalidIncompatId(*incompat_id));
                };

                let old_incompat = std::mem::replace(incompat, new_incompat.clone());

                Ok(AnnotatedIncompatOp::Update(*incompat_id, old_incompat))
            }
        }
    }
}
