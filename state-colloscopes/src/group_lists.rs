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
///
/// Sealed: the fields are private and every value is built through
/// [`GroupList::new`], which enforces the two value-internal invariants (the
/// prefilled group count matches the group-name count, and no student appears
/// in two prefilled groups). State-dependent facts (student existence) stay
/// with the checker/walker as dangling FKs. Serialized exactly like the raw
/// `{ params, filling }` pair via [`RawGroupList`]; deserializing an
/// inconsistent pair is a hard error (the [`NonEmptyRangeInclusive`] precedent).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References)]
#[serde(try_from = "RawGroupList", into = "RawGroupList")]
pub struct GroupList {
    /// parameters for the group list
    params: GroupListParameters,
    /// Filling strategy for the group list
    #[fk]
    filling: GroupListFilling,
}

/// Private serde mirror of [`GroupList`]: the transparent `{ params, filling }`
/// pair. Deserialization funnels through [`GroupList::new`] (honest decode);
/// serialization is the plain field dump, so the wire format is byte-identical
/// to the pre-sealing struct.
#[derive(Serialize, Deserialize)]
struct RawGroupList {
    params: GroupListParameters,
    filling: GroupListFilling,
}

impl From<GroupList> for RawGroupList {
    fn from(group_list: GroupList) -> Self {
        RawGroupList {
            params: group_list.params,
            filling: group_list.filling,
        }
    }
}

impl TryFrom<RawGroupList> for GroupList {
    type Error = GroupListBuildError;
    fn try_from(raw: RawGroupList) -> Result<Self, GroupListBuildError> {
        GroupList::new(raw.params, raw.filling)
    }
}

impl Default for GroupList {
    fn default() -> Self {
        // Default params paired with the automatic filling — always internally
        // consistent (no prefilled groups to count or deduplicate).
        GroupList {
            params: GroupListParameters::default(),
            filling: GroupListFilling::default(),
        }
    }
}

/// Value-internal build failures of [`GroupList::new`]. These describe a
/// self-contradictory `(params, filling)` pair, independent of any state.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GroupListBuildError {
    /// Prefilled groups count does not match `group_names` count
    #[error("prefilled group count ({actual}) does not match the group name count ({expected})")]
    PrefillGroupCountMismatch { expected: usize, actual: usize },
    /// A student appears in two prefilled groups
    #[error("student {0:?} appears in two prefilled groups")]
    DuplicatedStudentInPrefilledGroups(StudentId),
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
    /// Builds a group list, enforcing the two value-internal invariants: a
    /// prefilled filling must have exactly as many groups as `group_names`, and
    /// no student may appear in two prefilled groups. Student *existence* is a
    /// state-dependent fact and stays with the checker.
    pub fn new(
        params: GroupListParameters,
        filling: GroupListFilling,
    ) -> Result<Self, GroupListBuildError> {
        if let GroupListFilling::Prefilled { groups } = &filling {
            if groups.len() != params.group_names.len() {
                return Err(GroupListBuildError::PrefillGroupCountMismatch {
                    expected: params.group_names.len(),
                    actual: groups.len(),
                });
            }
            let mut seen = BTreeSet::new();
            for group in groups {
                for &student_id in &group.students {
                    if !seen.insert(student_id) {
                        return Err(GroupListBuildError::DuplicatedStudentInPrefilledGroups(
                            student_id,
                        ));
                    }
                }
            }
        }
        Ok(GroupList { params, filling })
    }

    /// Read access to the parameters.
    pub fn params(&self) -> &GroupListParameters {
        &self.params
    }

    /// Read access to the filling strategy.
    pub fn filling(&self) -> &GroupListFilling {
        &self.filling
    }

    /// Consumes the group list, yielding its `(params, filling)` pair.
    pub fn into_parts(self) -> (GroupListParameters, GroupListFilling) {
        (self.params, self.filling)
    }

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

    /// Cannot set prefilling: colloscope group list has students assigned
    #[error("Cannot set prefilling: colloscope group list {0:?} has students assigned")]
    NonEmptyColloscopeGroupListWhenPrefilling(GroupListId),
}

/// Precondition errors of the forced group-list ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence, and the
/// `AssignToSubject` coordinate-existence checks
/// ([Self::InvalidSubjectId] / [Self::InvalidPeriodId] / [Self::InvalidGroupListId]).
/// With the op payload consolidated to a whole sealed [GroupList], the
/// empty-first protocol guards and the prefill-count boundary have no place
/// left in the surface (the shape invariants hold by construction);
/// `validate_group_list*`, the Remove/Update scans and the `AssignToSubject`
/// semantic guards are stripped. Variants copied verbatim from [GroupListError].
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
        let weeks = &self.inner_data.params.weeks;
        for (slot_id, _slot) in subject_slots {
            for (week, assigned_groups) in
                self.inner_data.colloscope.interrogations_for_slot(*slot_id)
            {
                if weeks.week_position(week).map(|(p, _pos)| p) != Some(period_id) {
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
            AnnotatedGroupListOp::Add(new_id, new_group_list) => {
                if self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .contains(new_id)
                {
                    return Err(GroupListError::GroupListIdAlreadyExists(*new_id));
                };

                // The value is a sealed `GroupList`: its value-internal invariants
                // hold by construction. Only the state-dependent fact (student
                // existence) is checked here.
                self.inner_data.params.validate_group_list(new_group_list)?;

                self.inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*new_id, new_group_list.clone());

                // No colloscope row is seeded: a group list's placements are a
                // sparse table row created lazily on the first non-empty write
                // (an absent row is an empty placement map).

                Ok(AnnotatedGroupListOp::Remove(*new_id))
            }
            AnnotatedGroupListOp::Remove(id) => {
                if !self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .contains(id)
                {
                    return Err(GroupListError::InvalidGroupListId(*id));
                }

                // The whole value (filling included) is removed atomically and
                // carried in the reverse `Add`, so a non-empty filling no longer
                // blocks removal. Canonical-absent surface: a non-empty placement
                // row would be left dangling, so it still blocks (a placement row
                // is never valid for a prefilled list either).
                if self.inner_data.colloscope.group_list(*id).is_some() {
                    return Err(GroupListError::NotEmptyGroupListInColloscope(*id));
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

                Ok(AnnotatedGroupListOp::Add(*id, old_group_list))
            }
            AnnotatedGroupListOp::Update(group_list_id, new_group_list) => {
                let Some(old_group_list) = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };

                // The new value is a sealed, complete `GroupList`: only the
                // state-dependent fact (student existence) needs checking.
                self.inner_data.params.validate_group_list(new_group_list)?;

                // Merged colloscope guard set, driven by the prefill transition.
                // Canonical-absent surface: an empty (or absent) placement row is
                // trivially compatible. A prefilled list never owns a placement
                // row, so `(true, _)` has nothing to check.
                match (old_group_list.is_prefilled(), new_group_list.is_prefilled()) {
                    (false, false) => {
                        // Staying automatic: existing placements must stay
                        // compatible with the complete new pair.
                        if let Some(placements) =
                            self.inner_data.colloscope.group_list(*group_list_id)
                            && colloscopes::validate_group_list_placements(
                                *group_list_id,
                                placements,
                                new_group_list.params(),
                                new_group_list.filling(),
                                &self.inner_data.params.students,
                            )
                            .is_err()
                        {
                            return Err(GroupListError::NotCompatibleGroupListInColloscope(
                                *group_list_id,
                            ));
                        }
                    }
                    (false, true) => {
                        // Turning prefilled: the colloscope must hold no
                        // placements (a present row means non-empty placements).
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
                    }
                    (true, _) => {}
                }

                // The interrogations of every subject associated with this
                // list must stay within the new group count.
                let first_forbidden_group_number = new_group_list.params().group_names.len() as u32;
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

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*group_list_id, new_group_list.clone())
                    .expect("Group list ID was validated above");

                Ok(AnnotatedGroupListOp::Update(*group_list_id, old_group_list))
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
                        group_list.params().group_names.len() as u32
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

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_group_list]: carve-out guards kept (returned as
    /// [GroupListPrecheckError] — no-clobber, target existence, the empty-first
    /// protocol guards `RemainingFilling` / `NonEmptyGroupsWhenReducing`, the
    /// dual-listed `PrefillGroupCountMismatch`, and the `AssignToSubject`
    /// coordinate existence), invariant guards stripped (step-3 survey Table 1).
    /// May leave the state invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_group_list(
        &mut self,
        group_list_op: &AnnotatedGroupListOp,
    ) -> std::result::Result<AnnotatedGroupListOp, GroupListPrecheckError> {
        match group_list_op {
            AnnotatedGroupListOp::Add(new_id, new_group_list) => {
                if self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .contains(new_id)
                {
                    return Err(GroupListPrecheckError::GroupListIdAlreadyExists(*new_id));
                };
                // force inserts the value verbatim: no student-existence check
                // (fixes nothing). The payload is a sealed `GroupList`, so its
                // value-internal invariants already hold by construction.
                self.inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*new_id, new_group_list.clone());

                Ok(AnnotatedGroupListOp::Remove(*new_id))
            }
            AnnotatedGroupListOp::Remove(id) => {
                // Target existence only: the whole value is removed atomically
                // and carried in the reverse `Add`.
                if !self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .contains(id)
                {
                    return Err(GroupListPrecheckError::InvalidGroupListId(*id));
                }

                // stripped: NotEmptyGroupListInColloscope + RemainingAssociatedSubjects scans

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .remove(id)
                    .expect("Group list ID was checked above");

                Ok(AnnotatedGroupListOp::Add(*id, old_group_list))
            }
            AnnotatedGroupListOp::Update(group_list_id, new_group_list) => {
                // Target existence only; the sealed value replaces the old one
                // verbatim.
                if !self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .contains(group_list_id)
                {
                    return Err(GroupListPrecheckError::InvalidGroupListId(*group_list_id));
                }

                // stripped: colloscope placement compat guard + interrogation
                // group-bound scan + validate_group_list

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .insert(*group_list_id, new_group_list.clone())
                    .expect("Group list ID was checked above");

                Ok(AnnotatedGroupListOp::Update(*group_list_id, old_group_list))
            }
            AnnotatedGroupListOp::AssignToSubject(period_id, subject_id, group_list_id) => {
                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_none()
                {
                    return Err(GroupListPrecheckError::InvalidSubjectId(*subject_id));
                }
                // stripped: SubjectHasNoInterrogation + SubjectDoesNotRunOnPeriod
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(GroupListPrecheckError::InvalidPeriodId(*period_id));
                }

                // Keep the group-list existence carve-out; the bound value it also
                // computed was only used by the stripped group-bound scan.
                if let Some(id) = group_list_id
                    && !self
                        .inner_data
                        .params
                        .group_lists
                        .group_list_map
                        .contains(id)
                {
                    return Err(GroupListPrecheckError::InvalidGroupListId(*id));
                }

                // stripped: check_interrogations_group_bound

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
