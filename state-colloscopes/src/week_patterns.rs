//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{ContentOrd, References};

use crate::Table;
use crate::ids::{WeekId, WeekPatternId};
use crate::ops::AnnotatedWeekPatternOp;

/// Description of the week patterns
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct WeekPatterns {
    /// Week patterns
    ///
    /// Each item associates a single ID with the set of weeks it disables.
    pub week_pattern_map: Table<WeekPatternId, WeekPattern>,
}

/// Description of a week pattern
///
/// A pattern is stored as the *exception set* of the weeks it disables; every
/// week not listed is active. This is the sparse dual of the historical
/// positional bitmask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, ContentOrd)]
pub struct WeekPattern {
    /// Name of the week pattern for identification
    pub name: String,
    /// Weeks the pattern *disables*. Absent = active (the trivial value).
    ///
    /// May reference non-interrogation weeks: the bit is preserved regardless
    /// of the week's `interrogations` flag (byte-stability, decision 12). The
    /// merged activity of a week is `week.interrogations ∧ ¬excluded`.
    #[fk]
    pub excluded_weeks: BTreeSet<WeekId>,
}

impl WeekPatterns {
    /// The single definition of "a slot can carry an interrogation on `week`":
    /// the week runs interrogations and is not excluded by the given pattern (or
    /// there is no pattern). Homed here so consumers holding only a `Weeks` +
    /// `WeekPatterns` pair — e.g. the gtk4 colloscope grid — can call it;
    /// [`super::colloscope_params::Parameters::is_week_active`] delegates to it.
    ///
    /// Returns `false` for a dangling week id; a dangling pattern id is treated
    /// as "no exclusion". Both are bugs on validated data.
    pub fn is_week_active(
        &self,
        weeks: &super::weeks::Weeks,
        week: WeekId,
        pattern: Option<WeekPatternId>,
    ) -> bool {
        let Some(week_desc) = weeks.find_week(week) else {
            return false;
        };
        week_desc.interrogations
            && pattern.is_none_or(|p| {
                self.week_pattern_map
                    .get(&p)
                    .is_none_or(|wp| !wp.excluded_weeks.contains(&week))
            })
    }
}

/// Precondition errors of the forced week-pattern ops — the carve-out subset
/// (step-3 survey Table 2). Only no-clobber and op-target existence survive;
/// `validate_week_pattern` and the reference scans are stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPatternPrecheckError {
    /// A week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// The week pattern id already exists
    #[error("week pattern id ({0:?}) already exists")]
    WeekPatternIdAlreadyExists(WeekPatternId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a week-pattern op: carve-out guards kept (returned
    /// as [WeekPatternPrecheckError]), invariant guards stripped (step-3 survey
    /// Table 1). May leave the state invalid; the caller owns checking and
    /// rollback.
    pub(crate) fn force_apply_week_pattern(
        &mut self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<AnnotatedWeekPatternOp, WeekPatternPrecheckError> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, week_pattern) => {
                if self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains(new_id)
                {
                    return Err(WeekPatternPrecheckError::WeekPatternIdAlreadyExists(
                        *new_id,
                    ));
                }

                // stripped: validate_week_pattern

                self.inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .insert(*new_id, week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Remove(*new_id))
            }
            AnnotatedWeekPatternOp::Remove(id) => {
                // stripped: slot-reference / incompat-reference scans
                let Some(old_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .remove(id)
                else {
                    return Err(WeekPatternPrecheckError::InvalidWeekPatternId(*id));
                };

                Ok(AnnotatedWeekPatternOp::Add(*id, old_week_pattern))
            }
            AnnotatedWeekPatternOp::Update(id, new_week_pattern) => {
                // stripped: validate_week_pattern + the colloscope silencing guard
                let Some(current_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                else {
                    return Err(WeekPatternPrecheckError::InvalidWeekPatternId(*id));
                };

                let old_week_pattern =
                    std::mem::replace(current_week_pattern, new_week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Update(*id, old_week_pattern))
            }
        }
    }
}
