//! Group lists submodule
//!
//! This module defines the relevant types to describes the lists of groups

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::References;

use crate::Table;
use crate::colloscopes;
use crate::group_lists;
use crate::ids::{GroupListId, PeriodId, SlotId, StudentId, SubjectId};
use crate::non_empty_range::NonEmptyRangeInclusive;
use crate::ops::AnnotatedGroupListOp;

/// Description of the group lists
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupLists {
    /// Group lists
    ///
    /// Each item associates a group list id to an actual group list
    pub group_list_map: Table<GroupListId, GroupList>,

    /// Associations between subjects and group lists
    ///
    /// A sparse junction table keyed by `(period, subject)`: a pair is present
    /// exactly when a group list has been associated to that subject on that
    /// period. Absent means no association.
    pub subjects_associations: Table<(PeriodId, SubjectId), GroupListId>,
}

/// Description of a single group list
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default, References)]
pub struct GroupList {
    /// parameters for the group list
    pub params: GroupListParameters,
    /// Filling strategy for the group list
    #[fk]
    pub filling: GroupListFilling,
}

/// Filling strategy for a group list
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupListFilling {
    /// Groups are filled manually with prefilled students
    Prefilled { groups: Vec<PrefilledGroup> },
    /// Groups are filled automatically, except for excluded students
    Automatic {
        excluded_students: BTreeSet<StudentId>,
    },
}

impl Default for GroupListFilling {
    fn default() -> Self {
        GroupListFilling::Automatic {
            excluded_students: BTreeSet::new(),
        }
    }
}

// Irregular shape: the referenced students live inside the enum variants, so the
// `#[derive(References)]` field-walk on `GroupList` cannot reach them. This manual
// impl bridges the gap and composes with that derive through its generic `K` bound
// (the derive only requires `GroupListFilling: References<K>`). The visit order
// matches the hand-written walker it replaces: prefilled groups in `Vec` order
// then students in set order, or excluded students in set order.
impl<K: From<StudentId>> References<K> for GroupListFilling {
    fn for_each_ref(&self, f: &mut dyn FnMut(K)) {
        match self {
            GroupListFilling::Prefilled { groups } => {
                for group in groups {
                    for &student_id in &group.students {
                        f(K::from(student_id));
                    }
                }
            }
            GroupListFilling::Automatic { excluded_students } => {
                for &student_id in excluded_students {
                    f(K::from(student_id));
                }
            }
        }
    }
}

impl GroupListFilling {
    /// Returns true if the filling is prefilled
    pub fn is_prefilled(&self) -> bool {
        matches!(self, GroupListFilling::Prefilled { .. })
    }

    /// Returns the excluded students (empty set for Prefilled variant)
    pub fn excluded_students(&self) -> &BTreeSet<StudentId> {
        match self {
            GroupListFilling::Automatic { excluded_students } => excluded_students,
            GroupListFilling::Prefilled { .. } => {
                static EMPTY: std::sync::LazyLock<BTreeSet<StudentId>> =
                    std::sync::LazyLock::new(BTreeSet::new);
                &EMPTY
            }
        }
    }

    /// Checks that no student appears twice in the groups (for Prefilled variant)
    pub fn check_duplicated_student(&self) -> bool {
        match self {
            GroupListFilling::Prefilled { groups } => {
                let mut students_so_far = BTreeSet::new();
                for group in groups {
                    for student in &group.students {
                        if !students_so_far.insert(*student) {
                            return false;
                        }
                    }
                }
                true
            }
            GroupListFilling::Automatic { .. } => true,
        }
    }

    /// Iterates over all students in prefilled groups (empty for Automatic)
    pub fn iter_students(&self) -> impl Iterator<Item = StudentId> + '_ {
        match self {
            GroupListFilling::Prefilled { groups } => {
                Some(groups.iter().flat_map(|g| g.students.iter().copied()))
            }
            GroupListFilling::Automatic { .. } => None,
        }
        .into_iter()
        .flatten()
    }

    /// Removes a student from prefilled groups (returns true if found)
    pub fn remove_student(&mut self, student_id: StudentId) -> bool {
        match self {
            GroupListFilling::Prefilled { groups } => {
                for group in groups {
                    if group.students.remove(&student_id) {
                        return true;
                    }
                }
                false
            }
            GroupListFilling::Automatic { .. } => false,
        }
    }

    /// Returns true if the student is in a prefilled group
    pub fn contains_student(&self, student_id: StudentId) -> bool {
        self.find_student_group(student_id).is_some()
    }

    /// Finds the group number of a student (for Prefilled variant)
    pub fn find_student_group(&self, student_id: StudentId) -> Option<usize> {
        match self {
            GroupListFilling::Prefilled { groups } => {
                for (num, group) in groups.iter().enumerate() {
                    if group.students.contains(&student_id) {
                        return Some(num);
                    }
                }
                None
            }
            GroupListFilling::Automatic { .. } => None,
        }
    }

    /// Returns the number of groups (for Prefilled variant, 0 for Automatic)
    pub fn groups_len(&self) -> usize {
        match self {
            GroupListFilling::Prefilled { groups } => groups.len(),
            GroupListFilling::Automatic { .. } => 0,
        }
    }
}

/// Prefilled groups for a single group list
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefilledGroup {
    /// Students set
    ///
    /// Set of students that are in the group
    pub students: BTreeSet<StudentId>,
}

/// Parameters for a single group list
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListParameters {
    /// Name for the list
    pub name: String,
    /// Range of possible count of students per group
    pub students_per_group: NonEmptyRangeInclusive<NonZeroU32>,
    /// Group names (length determines max group count, None = unnamed group)
    pub group_names: Vec<Option<non_empty_string::NonEmptyString>>,
}

impl Default for GroupListParameters {
    fn default() -> Self {
        GroupListParameters {
            name: "Liste".into(),
            students_per_group: NonEmptyRangeInclusive::new(
                NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            )
            .expect("statically non-empty"),
            group_names: vec![None; 16], // 16 unnamed groups (typical for a class of 48 with 3 students per group)
        }
    }
}

impl GroupList {
    /// Checks whether the group list is prefilled
    ///
    /// Returns true if filling is Prefilled variant
    pub fn is_prefilled(&self) -> bool {
        self.filling.is_prefilled()
    }

    /// Returns the set of students that are not already in a prefilled group
    pub fn students(&self, students: &BTreeSet<StudentId>) -> BTreeSet<StudentId> {
        match &self.filling {
            GroupListFilling::Automatic { excluded_students } => {
                students.difference(excluded_students).copied().collect()
            }
            GroupListFilling::Prefilled { groups } => groups
                .iter()
                .flat_map(|g| g.students.iter().copied())
                .collect(),
        }
    }
}

/// Errors for group list operations
///
/// These errors can be returned when trying to modify [crate::Data] with a group list op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GroupListError {
    /// group list id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    /// The group list id already exists
    #[error("group list id ({0:?}) already exists")]
    GroupListIdAlreadyExists(GroupListId),

    /// student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// subject does not have interrogations
    #[error("subject id ({0:?}) has no interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(SubjectId, PeriodId),

    /// cannot remove group list as it still has a filling (prefilled or automatic with exclusions)
    #[error("Group list still has a filling and cannot be removed")]
    RemainingFilling,

    /// students appear multiple times in prefilled groups
    #[error("Some students appear multiple times in prefilled groups")]
    DuplicatedStudentInPrefilledGroups,

    /// cannot remove group list as there are still associated subjects
    #[error("Group list still is associated to subjects and cannot be removed")]
    RemainingAssociatedSubjects,

    /// Group list is not empty in colloscope
    #[error("group list id {0:?} in colloscope is not empty")]
    NotEmptyGroupListInColloscope(GroupListId),

    /// Group list in colloscope not compatible with new parameters
    #[error("group list id {0:?} in colloscope is not compatible with the given parameters")]
    NotCompatibleGroupListInColloscope(GroupListId),

    /// The subject has non-empty slots associated to the old group list with invalid numbers
    #[error(
        "subject {0:?} in colloscope has non-empty slots (slot {2:?}) in period {1:?} with invalid group number"
    )]
    InvalidGroupInSubjectSlotInColloscope(SubjectId, PeriodId, SlotId),

    /// Prefilled groups count does not match group_names count
    #[error("prefilled groups count ({actual}) does not match group names count ({expected})")]
    PrefillGroupCountMismatch { expected: usize, actual: usize },

    /// Cannot reduce group count when last groups have students
    #[error(
        "cannot reduce group count: groups to be removed still have students (ops layer should clean first)"
    )]
    NonEmptyGroupsWhenReducing,

    /// Cannot set prefilling: colloscope group list has students assigned
    #[error("Cannot set prefilling: colloscope group list {0:?} has students assigned")]
    NonEmptyColloscopeGroupListWhenPrefilling(GroupListId),
}

/// Precondition errors of the forced group-list ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence, the
/// empty-first protocol guards ([Self::RemainingFilling],
/// [Self::NonEmptyGroupsWhenReducing]), the dual-listed prefill-count boundary
/// ([Self::PrefillGroupCountMismatch], kept per Appendix D.3), and the
/// `AssignToSubject` coordinate-existence checks
/// ([Self::InvalidSubjectId] / [Self::InvalidPeriodId] / [Self::InvalidGroupListId]).
/// `validate_group_list*`, the Remove/Update/SetFilling scans and the
/// `AssignToSubject` semantic guards are stripped. Variants copied verbatim
/// from [GroupListError].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GroupListPrecheckError {
    /// group list id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    /// The group list id already exists
    #[error("group list id ({0:?}) already exists")]
    GroupListIdAlreadyExists(GroupListId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// cannot remove group list as it still has a filling (prefilled or automatic with exclusions)
    #[error("Group list still has a filling and cannot be removed")]
    RemainingFilling,

    /// Prefilled groups count does not match group_names count
    #[error("prefilled groups count ({actual}) does not match group names count ({expected})")]
    PrefillGroupCountMismatch { expected: usize, actual: usize },

    /// Cannot reduce group count when last groups have students
    #[error(
        "cannot reduce group count: groups to be removed still have students (ops layer should clean first)"
    )]
    NonEmptyGroupsWhenReducing,
}

impl crate::Data {
    /// Used internally
    ///
    /// Checks that every group number assigned in the interrogations of
    /// the given subject on the given period is strictly below
    /// `first_forbidden_group_number`
    fn check_interrogations_group_bound(
        &self,
        period_id: PeriodId,
        subject_id: SubjectId,
        first_forbidden_group_number: u32,
    ) -> std::result::Result<(), GroupListError> {
        let Some(subject_slots) = self.inner_data.params.slots.slots_for_subject(subject_id) else {
            // No slots: no interrogation can reference a group number
            return Ok(());
        };
        let periods = &self.inner_data.params.periods;
        for (slot_id, _slot) in subject_slots {
            for (week, assigned_groups) in
                self.inner_data.colloscope.interrogations_for_slot(*slot_id)
            {
                if periods.week_position(week).map(|(p, _pos)| p) != Some(period_id) {
                    continue;
                }
                if assigned_groups
                    .iter()
                    .any(|group| *group >= first_forbidden_group_number)
                {
                    return Err(GroupListError::InvalidGroupInSubjectSlotInColloscope(
                        subject_id, period_id, *slot_id,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Used internally
    ///
    /// Apply group list operations
    pub(crate) fn apply_group_list(
        &mut self,
        group_list_op: &AnnotatedGroupListOp,
    ) -> std::result::Result<AnnotatedGroupListOp, GroupListError> {
        match group_list_op {
            AnnotatedGroupListOp::Add(new_id, params, filling) => {
                if self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .contains(new_id)
                {
                    return Err(GroupListError::GroupListIdAlreadyExists(*new_id));
                };
                let new_group_list = group_lists::GroupList {
                    params: params.clone(),
                    filling: filling.clone(),
                };

                self.inner_data
                    .params
                    .validate_group_list(&new_group_list)?;

                self.inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*new_id, new_group_list);

                // No colloscope row is seeded: a group list's placements are a
                // sparse table row created lazily on the first non-empty write
                // (an absent row is an empty placement map).

                Ok(AnnotatedGroupListOp::Remove(*new_id))
            }
            AnnotatedGroupListOp::Remove(id) => {
                let Some(old_group_list) =
                    self.inner_data.params.group_lists.group_list_map.get(id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*id));
                };

                // Check filling is empty before removal
                match &old_group_list.filling {
                    group_lists::GroupListFilling::Prefilled { groups } => {
                        if groups.iter().any(|g| !g.students.is_empty()) {
                            return Err(GroupListError::RemainingFilling);
                        }
                    }
                    group_lists::GroupListFilling::Automatic { excluded_students } => {
                        if !excluded_students.is_empty() {
                            return Err(GroupListError::RemainingFilling);
                        }
                        // Canonical-absent surface: a non-empty placement row
                        // blocks removal.
                        if self.inner_data.colloscope.group_list(*id).is_some() {
                            return Err(GroupListError::NotEmptyGroupListInColloscope(*id));
                        }
                    }
                }

                for group_list_id in self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .values()
                {
                    if *group_list_id == *id {
                        return Err(GroupListError::RemainingAssociatedSubjects);
                    }
                }

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .remove(id)
                    .expect("Group list ID was checked above");
                // The removal guard above rejected the op while any placement
                // row survived, so there is no colloscope row to drop.

                Ok(AnnotatedGroupListOp::Add(
                    *id,
                    old_group_list.params,
                    old_group_list.filling,
                ))
            }
            AnnotatedGroupListOp::Update(group_list_id, new_params) => {
                let Some(old_group_list) = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };

                // Only validate colloscope placements for non-prefilled group
                // lists. Canonical-absent surface: an empty (or absent) list is
                // trivially compatible.
                if !old_group_list.is_prefilled()
                    && let Some(placements) = self.inner_data.colloscope.group_list(*group_list_id)
                    && colloscopes::validate_group_list_placements(
                        *group_list_id,
                        placements,
                        new_params,
                        &old_group_list.filling,
                        &self.inner_data.params.students,
                    )
                    .is_err()
                {
                    return Err(GroupListError::NotCompatibleGroupListInColloscope(
                        *group_list_id,
                    ));
                }

                // The interrogations of every subject associated with this
                // list must stay within the new group count
                let first_forbidden_group_number = new_params.group_names.len() as u32;
                for ((period_id, subject_id), associated_list) in self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                {
                    if associated_list == group_list_id {
                        self.check_interrogations_group_bound(
                            period_id,
                            subject_id,
                            first_forbidden_group_number,
                        )?;
                    }
                }

                // Atomically adjust filling when size changes
                let new_filling = match &old_group_list.filling {
                    group_lists::GroupListFilling::Automatic { excluded_students } => {
                        group_lists::GroupListFilling::Automatic {
                            excluded_students: excluded_students.clone(),
                        }
                    }
                    group_lists::GroupListFilling::Prefilled { groups: old_groups } => {
                        let old_count = old_group_list.params.group_names.len();
                        let new_count = new_params.group_names.len();

                        if new_count < old_count {
                            // Reducing groups: check last groups are empty
                            for group in old_groups.iter().skip(new_count) {
                                if !group.students.is_empty() {
                                    return Err(GroupListError::NonEmptyGroupsWhenReducing);
                                }
                            }
                            // Truncate to new size
                            group_lists::GroupListFilling::Prefilled {
                                groups: old_groups[..new_count].to_vec(),
                            }
                        } else if new_count > old_count {
                            // Increasing groups: extend with empty groups
                            let mut new_groups = old_groups.clone();
                            for _ in old_count..new_count {
                                new_groups.push(group_lists::PrefilledGroup::default());
                            }
                            group_lists::GroupListFilling::Prefilled { groups: new_groups }
                        } else {
                            // Same size: keep as is
                            group_lists::GroupListFilling::Prefilled {
                                groups: old_groups.clone(),
                            }
                        }
                    }
                };

                let new_group_list = group_lists::GroupList {
                    params: new_params.clone(),
                    filling: new_filling,
                };

                self.inner_data
                    .params
                    .validate_group_list(&new_group_list)?;

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*group_list_id, new_group_list)
                    .expect("Group list ID was validated above");

                Ok(AnnotatedGroupListOp::Update(
                    *group_list_id,
                    old_group_list.params,
                ))
            }
            AnnotatedGroupListOp::SetFilling(group_list_id, filling) => {
                let Some(old_group_list) = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };

                // Check that prefilled groups count matches group_names count
                if let group_lists::GroupListFilling::Prefilled { groups } = filling {
                    let expected = old_group_list.params.group_names.len();
                    let actual = groups.len();
                    if actual != expected {
                        return Err(GroupListError::PrefillGroupCountMismatch { expected, actual });
                    }
                }

                // Handle colloscope group list based on prefill transition
                let was_prefilled = old_group_list.is_prefilled();
                let will_be_prefilled = filling.is_prefilled();

                if !was_prefilled && will_be_prefilled {
                    // Transitioning to prefilled: the colloscope must hold no
                    // placements. Canonical-absent surface: a present row means
                    // non-empty placements (and an absent row needs no cleanup).
                    if self
                        .inner_data
                        .colloscope
                        .group_list(*group_list_id)
                        .is_some()
                    {
                        return Err(GroupListError::NonEmptyColloscopeGroupListWhenPrefilling(
                            *group_list_id,
                        ));
                    }
                } else if was_prefilled && !will_be_prefilled {
                    // Transitioning from prefilled: nothing to seed — the empty
                    // placement map is simply an absent sparse row.
                } else if !was_prefilled
                    && !will_be_prefilled
                    && let Some(placements) = self.inner_data.colloscope.group_list(*group_list_id)
                {
                    // Staying automatic: the colloscope placements persist, so
                    // the new exclusions must be compatible with the placed
                    // students.
                    if colloscopes::validate_group_list_placements(
                        *group_list_id,
                        placements,
                        &old_group_list.params,
                        filling,
                        &self.inner_data.params.students,
                    )
                    .is_err()
                    {
                        return Err(GroupListError::NotCompatibleGroupListInColloscope(
                            *group_list_id,
                        ));
                    }
                }

                let new_group_list = group_lists::GroupList {
                    params: old_group_list.params.clone(),
                    filling: filling.clone(),
                };

                self.inner_data
                    .params
                    .validate_group_list(&new_group_list)?;

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*group_list_id, new_group_list)
                    .expect("Group list ID was validated above");

                Ok(AnnotatedGroupListOp::SetFilling(
                    *group_list_id,
                    old_group_list.filling,
                ))
            }
            AnnotatedGroupListOp::AssignToSubject(period_id, subject_id, group_list_id) => {
                let Some(subject) = self.inner_data.params.subjects.find_subject(*subject_id)
                else {
                    return Err(GroupListError::InvalidSubjectId(*subject_id));
                };
                if subject.parameters.interrogation_parameters.is_none() {
                    return Err(GroupListError::SubjectHasNoInterrogation(*subject_id));
                }
                if subject.excluded_periods.contains(period_id) {
                    return Err(GroupListError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                }
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(GroupListError::InvalidPeriodId(*period_id));
                }

                let first_forbidden_group_number = match group_list_id {
                    Some(id) => {
                        let Some(group_list) =
                            self.inner_data.params.group_lists.group_list_map.get(id)
                        else {
                            return Err(GroupListError::InvalidGroupListId(*id));
                        };
                        group_list.params.group_names.len() as u32
                    }
                    None => 0,
                };

                self.check_interrogations_group_bound(
                    *period_id,
                    *subject_id,
                    first_forbidden_group_number,
                )?;

                let associations = &mut self.inner_data.params.group_lists.subjects_associations;

                let old_group_list_id = match group_list_id {
                    Some(id) => associations.insert((*period_id, *subject_id), *id),
                    None => associations.remove(&(*period_id, *subject_id)),
                };

                Ok(AnnotatedGroupListOp::AssignToSubject(
                    *period_id,
                    *subject_id,
                    old_group_list_id,
                ))
            }
        }
    }
}
