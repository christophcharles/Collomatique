//! Incompats submodule
//!
//! This module defines the relevant types to describes the schedule incompatibilities

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{IncompatId, SubjectId, WeekPatternId};
use crate::ops::AnnotatedIncompatOp;

/// Description of the schedule incompatibilities
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incompats {
    /// Incompats for subjects
    ///
    /// Each item associates an incompat id to a schedule incompatibility
    pub incompat_map: BTreeMap<IncompatId, Incompatibility>,
}

/// Description of a single schedule incompat
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incompatibility {
    /// Subject the incompatibility is linked to
    pub subject_id: SubjectId,
    /// Name of the incompatibility for clarity
    pub name: String,
    /// Slots of time when the students might not be available
    ///
    /// This is given as a weekday, a start time and a duration
    pub slots: Vec<collomatique_time::SlotWithDuration>,
    /// Number of slots to force to be free in the above list
    pub minimum_free_slots: NonZeroU32,
    /// Week pattern for the incompatibility
    ///
    /// If `None`, this means every week
    pub week_pattern_id: Option<WeekPatternId>,
}

/// Errors for schedule incompatibility operations
///
/// These errors can be returned when trying to modify [crate::Data] with an incompat op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum IncompatError {
    /// A incompat id is invalid
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(IncompatId),

    /// The incompat id already exists
    #[error("incompat id ({0:?}) already exists")]
    IncompatIdAlreadyExists(IncompatId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply incompat operations
    pub(crate) fn apply_incompat(
        &mut self,
        incompat_op: &AnnotatedIncompatOp,
    ) -> std::result::Result<AnnotatedIncompatOp, IncompatError> {
        match incompat_op {
            AnnotatedIncompatOp::Add(new_id, incompat) => {
                if self
                    .inner_data
                    .params
                    .incompats
                    .incompat_map
                    .contains_key(new_id)
                {
                    return Err(IncompatError::IncompatIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_incompat(incompat)?;

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
                    return Err(IncompatError::InvalidIncompatId(*id));
                };

                Ok(AnnotatedIncompatOp::Add(*id, old_incompat))
            }
            AnnotatedIncompatOp::Update(incompat_id, new_incompat) => {
                self.inner_data.params.validate_incompat(new_incompat)?;

                let Some(incompat) = self
                    .inner_data
                    .params
                    .incompats
                    .incompat_map
                    .get_mut(incompat_id)
                else {
                    return Err(IncompatError::InvalidIncompatId(*incompat_id));
                };

                let old_incompat = std::mem::replace(incompat, new_incompat.clone());

                Ok(AnnotatedIncompatOp::Update(*incompat_id, old_incompat))
            }
        }
    }
}
