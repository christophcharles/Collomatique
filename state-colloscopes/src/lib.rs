//! Colloscopes state crate
//!
//! This crate implements the various concepts of [collomatique-state]
//! and the various traits for the specific case of colloscope representation.
//!

use colloscopes::ColloscopePeriod;
use ops::AnnotatedColloscopeOp;
use ops::AnnotatedExportConfigOp;
use serde::{Deserialize, Serialize};

use collomatique_state::{InMemoryData, Operation, tools};
use ops::{AnnotatedBalancingOp, AnnotatedSettingsOp};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub mod ids;
use ids::Id;
use ids::IdIssuer;
pub use ids::{
    GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekPatternId,
};
pub mod ops;
use ops::{
    AnnotatedAssignmentOp, AnnotatedGroupListOp, AnnotatedIncompatOp, AnnotatedPairingOp,
    AnnotatedPeriodOp, AnnotatedSlotOp, AnnotatedSlotPairingOp, AnnotatedStudentOp,
    AnnotatedSubjectOp, AnnotatedTeacherOp, AnnotatedWeekPatternOp,
};
pub use ops::{
    AnnotatedOp, AssignmentOp, BalancingOp, ColloscopeOp, ExportConfigOp, GroupListOp, IncompatOp,
    Op, PairingOp, PeriodOp, SettingsOp, SlotOp, SlotPairingOp, StudentOp, SubjectOp, TeacherOp,
    WeekPatternOp,
};
pub use subjects::{
    Subject, SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
};

pub mod assignments;
pub mod balancing;
pub mod colloscope_params;
pub mod colloscopes;
pub mod export_config;
pub mod group_lists;
pub mod incompats;
pub mod pairings;
pub mod periods;
pub mod settings;
pub mod slot_pairings;
pub mod slots;
pub mod soft_param;
pub mod students;
pub mod subjects;
pub mod teachers;
pub mod week_patterns;

/// Description of a person with contacts
///
/// This type is used to describe both students and teachers.
/// Each student and teacher has its own card with name and contacts.
/// There are not used for the colloscope solving process
/// but can help produce a nice colloscope output with contact info.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PersonWithContact {
    /// Surname of the person
    ///
    /// Though this field can be an empty string,
    /// it is considered mandatory internally
    pub surname: String,

    /// Firstname of the person
    ///
    /// Though this field can be an empty string,
    /// it is considered mandatory internally
    pub firstname: String,

    /// Person's telephone number
    ///
    /// This field is optional: this reflects the
    /// fact that some persons might not want to share
    /// their personal info or only some of it.
    pub tel: Option<non_empty_string::NonEmptyString>,

    /// Person's email
    ///
    /// This field is optional: this reflects the
    /// fact that some persons might not want to share
    /// their personal info or only some of it.
    pub email: Option<non_empty_string::NonEmptyString>,
}

/// Internal structure to store the data for [Data]
///
/// We have `data1 == data2` if and only if their internal
/// data is the same. This means they would lead to the same
/// file on disk. But the internal id issuer might have a different
/// state.
///
/// [InnerData] represents this actual 'on-disk' data so we can
/// directly use `derive(PartialEq, Eq)` with it. The implementation
/// of [Eq] and [PartialEq] for [Data] relies on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InnerData {
    pub params: colloscope_params::Parameters,
    pub colloscope: colloscopes::Colloscope,
    #[serde(default)]
    pub export_config: export_config::ExportConfig,
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum InnerDataError {
    #[error("Duplicate ids")]
    DuplicateIds,
    #[error("Error in paramters: {0}")]
    Params(#[from] InvariantError),
    #[error("Error in colloscope: {0}")]
    ColloscopeError(#[from] ColloscopeError),
}

impl InnerData {
    fn ids(&self) -> impl Iterator<Item = u64> {
        self.params.ids()
    }

    fn check_no_duplicate_ids(&self) -> bool {
        let mut ids_so_far = BTreeSet::new();

        for id in self.ids() {
            if !ids_so_far.insert(id) {
                return false;
            }
        }

        true
    }

    pub fn check_invariants(&self) -> Result<(), InnerDataError> {
        if !self.check_no_duplicate_ids() {
            return Err(InnerDataError::DuplicateIds);
        }

        self.params.check_invariants()?;
        self.colloscope.validate_against_params(&self.params)?;
        /*for (colloscope_id, colloscope) in &self.colloscopes.colloscope_map {
            colloscope
                .check_invariants(&self.main_params)
                .map_err(|x| InnerDataError::ColloscopeError(*colloscope_id, x))?;
        }*/

        Ok(())
    }
}

/// Complete data that can be handled in the colloscope
///
/// This [Data] structure contains all the data that can
/// be manipulated in collomatique. It contains the list
/// of students, of teachers, the various interrogations,
/// a description of constraints etc. It also contains the
/// various colloscopes that have been generated or edited.
///
/// It cannot be modified or accessed directly. To the other
/// crates, this is an opaque type.
///
/// It does not necesserally correlate exactly to the data stored
/// on disk. This is to allow versioning.
#[derive(Debug)]
pub struct Data {
    id_issuer: std::sync::Mutex<IdIssuer>,
    inner_data: InnerData,
}

impl Clone for Data {
    fn clone(&self) -> Self {
        let guard = self.id_issuer.lock().unwrap();

        let id_issuer = guard.clone();
        Data {
            id_issuer: std::sync::Mutex::new(id_issuer),
            inner_data: self.inner_data.clone(),
        }
    }
}

impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        self.inner_data == other.inner_data
    }
}

impl Eq for Data {}

use thiserror::Error;

/// Errors for students operations
///
/// These errors can be returned when trying to modify [Data] with a student op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StudentError {
    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// The student id already exists
    #[error("student id ({0:?}) already exists")]
    StudentIdAlreadyExists(StudentId),

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Some non-default assignments are still present for the student
    #[error(
        "student id {0:?} has non-default assignments for subject id {1:?} in period id ({0:?}) and cannot be removed or updated"
    )]
    StudentStillHasNonTrivialAssignments(StudentId, SubjectId, PeriodId),

    /// Student is still excluded by a group list
    #[error("student id {0:?} is still excluded by a group list {1:?}")]
    StudentIsStillExcludedByGroupList(StudentId, GroupListId),

    /// Student is still referenced by a pre-filled group list
    #[error("student id {0:?} is still referenced by a pre-filled group list {1:?}")]
    StudentIsStillReferencedByPrefilledGroupList(StudentId, GroupListId),

    /// Student is referenced in a colloscope group list
    #[error("student id {0:?} is referenced in a colloscope group list ({1:?})")]
    StudentIsReferencedInColloscopeGroupList(StudentId, GroupListId),

    /// Student still has per-student settings
    #[error("student id {0:?} still has per-student settings")]
    StudentStillHasSettings(StudentId),
}

/// Errors for periods operations
///
/// These errors can be returned when trying to modify [Data] with a period op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PeriodError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// The period id already exists
    #[error("period id ({0:?}) already exists")]
    PeriodIdAlreadyExists(PeriodId),

    /// The period is referenced by a subject
    #[error("period id ({0:?}) is referenced by subject {1:?}")]
    PeriodIsReferencedBySubject(PeriodId, SubjectId),

    /// The period is referenced by a student
    #[error("period id ({0:?}) is referenced by student {1:?}")]
    PeriodIsReferencedByStudent(PeriodId, StudentId),

    /// Some non-default assignments are still present for the period
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed"
    )]
    PeriodStillHasNonTrivialAssignments(PeriodId, SubjectId),

    /// Some non-default group list association are still present for the period
    #[error("period id ({0:?}) has non-default group list associations and cannot be removed")]
    PeriodStillHasNonTrivialGroupListAssociation(PeriodId),

    /// Period is not empty in colloscope
    #[error("period id ({0:?}) is not empty in colloscope")]
    NotEmptyPeriodInColloscope(PeriodId),

    /// A week pattern is not trivial on the period to be cut
    #[error("week pattern {1:?} is not trivial for the period {0:?}")]
    NonTrivialWeekPattern(PeriodId, WeekPatternId),

    /// The slot in colloscope is incompatible with the new period
    #[error("slot {0:?} in colloscope is not compatible with the new period")]
    NotCompatibleSlotInColloscope(SlotId),

    /// The period is referenced by a pairing rule
    #[error("period id ({0:?}) is referenced by pairing rule {1:?}")]
    PeriodIsReferencedByPairingRule(PeriodId, PairingRuleId),

    /// The period is referenced by a slot pairing rule
    #[error("period id ({0:?}) is referenced by slot pairing rule {1:?}")]
    PeriodIsReferencedBySlotPairingRule(PeriodId, SlotPairingRuleId),
}

/// Errors for subject operations
///
/// These errors can be returned when trying to modify [Data] with a subject op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SubjectError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The subject id already exists
    #[error("subject id ({0:?}) already exists")]
    SubjectIdAlreadyExists(SubjectId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// A reference period is invalid
    #[error("Referenced period id {0:?} is invalid")]
    InvalidPeriodId(PeriodId),

    /// Invalid parameters : students per group
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,

    /// Invalid parameters : groups per interrogation
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,

    /// Invalid parameters : week block has empty range for interrogation count
    #[error("Interrogation count range should allow at least one value")]
    InterrogationCountRangeIsEmpty,

    /// Some non-default assignments are still present for the subject
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed or updated"
    )]
    SubjectStillHasNonTrivialAssignments(PeriodId, SubjectId),

    /// Some teachers still are associated to the subject
    #[error("teacher id ({0:?}) is associated to the subject id {1:?}")]
    SubjectStillHasAssociatedTeachers(TeacherId, SubjectId),

    /// The subject is referenced by a slot
    #[error("subject id ({0:?}) is referenced by slots")]
    SubjectStillHasAssociatedSlots(SubjectId),

    /// The subject is referenced by a schedule incompatibility
    #[error("subject id ({0:?}) is referenced by the incompat id {1:?}")]
    SubjectStillHasAssociatedIncompats(SubjectId, IncompatId),

    /// The subject is associated to a group list
    #[error("subject id ({0:?}) is associated to group list id {1:?} for period {2:?}")]
    SubjectStillHasAssociatedGroupList(SubjectId, GroupListId, PeriodId),

    /// The subject has filled slots in colloscope
    #[error("subject id {0:?} has a least one non-empty slot {1:?} in colloscope")]
    SubjectStillHasNonEmptySlotInColloscope(SubjectId, SlotId),

    /// The subject still has balancing options
    #[error("subject id {0:?} still has balancing options")]
    SubjectStillHasBalancingOptions(SubjectId),

    /// The subject is referenced by a pairing rule
    #[error("subject id ({0:?}) is referenced by pairing rule {1:?}")]
    SubjectIsReferencedByPairingRule(SubjectId, PairingRuleId),
}

/// Errors for teacher operations
///
/// These errors can be returned when trying to modify [Data] with a teacher op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum TeacherError {
    /// A teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// The teacher id already exists
    #[error("teacher id ({0:?}) already exists")]
    TeacherIdAlreadyExists(TeacherId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The selected subject does not have interrogations
    #[error("Subject id ({0:?}) corresponds to a subject without interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// The teacher is referenced by a slot
    #[error("teacher id ({0:?}) is referenced by a slot ({1:?})")]
    TeacherStillHasAssociatedSlots(TeacherId, SlotId),

    /// The teacher is referenced by slots for a bad subject
    #[error("teacher id ({0:?}) gives interrogation in a now forbidden subject ({1:?})")]
    TeacherStillHasAssociatedSlotsInSubject(TeacherId, SubjectId),
}

/// Errors for assignment operations
///
/// These errors can be returned when trying to modify [Data] with a assignment op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(SubjectId, PeriodId),

    /// Student is not present on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    StudentIsNotPresentOnPeriod(StudentId, PeriodId),
}

/// Errors for week pattern operations
///
/// These errors can be returned when trying to modify [Data] with a week pattern op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPatternError {
    /// A week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// The week pattern id already exists
    #[error("week pattern id ({0:?}) already exists")]
    WeekPatternIdAlreadyExists(WeekPatternId),

    /// The week pattern is referenced by a slot
    #[error("week pattern id ({0:?}) is referenced by a slot ({1:?})")]
    WeekPatternStillHasAssociatedSlots(WeekPatternId, SlotId),

    /// The week pattern is referenced by a schedule incompatibility
    #[error("week pattern id ({0:?}) is referenced by an incompat ({1:?})")]
    WeekPatternStillHasAssociatedIncompat(WeekPatternId, IncompatId),

    /// The week pattern does not have the right length
    #[error("week pattern does not have the right length")]
    BadWeekPatternLength,

    /// The slot in colloscope is incompatible with the new week pattern
    #[error("slot {0:?} in colloscope is not compatible with the new week pattern")]
    NotCompatibleSlotInColloscope(SlotId),
}

/// Errors for interrogation slot operations
///
/// These errors can be returned when trying to modify [Data] with a slot op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotError {
    /// A slot id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// The slot id already exists
    #[error("slot id ({0:?}) already exists")]
    SlotIdAlreadyExists(SlotId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// The previous slot given is not for the same subject
    #[error("Slot {0:?} to be previous slot is not for subject {1:?}")]
    PreviousSlotIsNotInRightSubject(SlotId, SubjectId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// subject has no interrogations
    #[error("subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// Provided teacher does not teach in the corresponding subject
    #[error("Provided teacher ({0:?}) does not teach in subject ({1:?})")]
    TeacherDoesNotTeachInSubject(TeacherId, SubjectId),

    /// Slot overlaps with next day
    #[error("The slot start time is too late and the slot overlaps with the next day")]
    SlotOverlapsWithNextDay,

    /// The slot is not empty in colloscope
    #[error("slot {0:?} in colloscope is not empty for period {1:?}")]
    NotEmptySlotInColloscope(SlotId, PeriodId),

    /// The slot in colloscope is incomaptible with the new week pattern
    #[error("slot {0:?} in colloscope is not compatible with the new week pattern {1:?}")]
    NotCompatibleSlotInColloscope(SlotId, Option<WeekPatternId>),

    /// The slot is referenced by a slot pairing rule
    #[error("slot id ({0:?}) is referenced by a slot pairing rule ({1:?})")]
    SlotIsReferencedBySlotPairingRule(SlotId, SlotPairingRuleId),
}

/// Errors for schedule incompatibility operations
///
/// These errors can be returned when trying to modify [Data] with an incompat op.
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

/// Errors for group list operations
///
/// These errors can be returned when trying to modify [Data] with a group list op.
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

    /// students per group range is empty
    #[error("students_per_group range is empty")]
    StudentsPerGroupRangeIsEmpty,

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

/// Errors for settings operations
///
/// These errors can be returned when trying to modify [Data] with a settings op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SettingsError {
    /// student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),
}

/// Errors for balancing operations
///
/// These errors can be returned when trying to modify [Data] with a balancing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BalancingError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),
    /// Subject does not have interrogations
    #[error("subject id ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(SubjectId),
}

/// Errors for pairing rule operations
///
/// These errors can be returned when trying to modify [Data] with a pairing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PairingError {
    /// A pairing rule id is invalid
    #[error("invalid pairing rule id ({0:?})")]
    InvalidPairingRuleId(PairingRuleId),

    /// The pairing rule id already exists
    #[error("pairing rule id ({0:?}) already exists")]
    PairingRuleIdAlreadyExists(PairingRuleId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Antecedent and consequent subjects are the same
    #[error("antecedent and consequent subjects are the same ({0:?})")]
    SameSubjectInBothParts(SubjectId),
}

/// Errors for slot pairing rule operations
///
/// These errors can be returned when trying to modify [Data] with a slot pairing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotPairingError {
    #[error("invalid slot pairing rule id ({0:?})")]
    InvalidSlotPairingRuleId(SlotPairingRuleId),
    #[error("slot pairing rule id ({0:?}) already exists")]
    SlotPairingRuleIdAlreadyExists(SlotPairingRuleId),
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),
    #[error("same slot in both parts ({0:?})")]
    SameSlotInBothParts(SlotId),
    #[error("slots {0:?} and {1:?} do not belong to the same subject")]
    SlotsNotInSameSubject(SlotId, SlotId),
}

/// Errors for export configuration operations
///
/// These errors can be returned when trying to modify [Data] with an export config op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ExportConfigError {}

/// Errors for colloscopes operations
///
/// These errors can be returned when trying to modify [Data] with a colloscope op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ColloscopeError {
    /// Student original id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// Period original id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Slot original id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// Group list original id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    #[error("Wrong period count")]
    WrongPeriodCountInColloscopeData,

    #[error("Wrong group list count")]
    WrongGroupListCountInColloscopeData,

    #[error("Wrong slot count in period")]
    WrongSlotCountInPeriodInColloscopeData(PeriodId),

    #[error("Wrong interrogation count for slot in period")]
    WrongInterrogationCountForSlotInPeriodInColloscopeData(PeriodId, SlotId),

    #[error("Interrogation on non-interrogation week")]
    InterrogationOnNonInterrogationWeek(PeriodId, SlotId, usize),

    #[error("Missing interrogation on interrogation week")]
    MissingInterrogationOnInterrogationWeek(PeriodId, SlotId, usize),

    #[error("Invalid group number in interrogation")]
    InvalidGroupNumInInterrogation(PeriodId, SlotId, usize),

    #[error("excluded student in group list")]
    ExcludedStudentInGroupList(GroupListId, StudentId),

    #[error("Invalid group number for student")]
    InvalidGroupNumForStudentInGroupList(GroupListId, StudentId),

    #[error("Invalid week number in period")]
    InvalidWeekNumberInPeriod(PeriodId, usize),

    #[error("No interrogation for the given week in period and slot")]
    NoInterrogationOnWeek(PeriodId, SlotId, usize),

    #[error("Prefilled group list {0:?} should not be in colloscope")]
    PrefilledGroupListInColloscope(GroupListId),

    #[error("Non-prefilled group list {0:?} is missing from colloscope")]
    MissingNonPrefilledGroupList(GroupListId),
}

/// Errors for colloscopes modification
///
/// These errors can be returned when trying to modify [Data].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error(transparent)]
    Student(#[from] StudentError),
    #[error(transparent)]
    Period(#[from] PeriodError),
    #[error(transparent)]
    Subject(#[from] SubjectError),
    #[error(transparent)]
    Teacher(#[from] TeacherError),
    #[error(transparent)]
    Assignment(#[from] AssignmentError),
    #[error(transparent)]
    WeekPattern(#[from] WeekPatternError),
    #[error(transparent)]
    Slot(#[from] SlotError),
    #[error(transparent)]
    Incompat(#[from] IncompatError),
    #[error(transparent)]
    GroupList(#[from] GroupListError),
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error(transparent)]
    Pairing(#[from] PairingError),
    #[error(transparent)]
    SlotPairing(#[from] SlotPairingError),
    #[error(transparent)]
    Balancing(#[from] BalancingError),
    #[error(transparent)]
    Colloscope(#[from] ColloscopeError),
    #[error(transparent)]
    ExportConfig(#[from] ExportConfigError),
    #[error(transparent)]
    GlobalUpdate(#[from] InnerDataError),
}

/// Errors for IDs
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FromDataError {
    #[error(transparent)]
    IdError(#[from] tools::IdError),
    #[error("Invalid ID")]
    InvalidId,
    #[error("Inconsistent assignments")]
    InconsistentAssignments,
    #[error("Error in slots data")]
    InconsistentSlots,
    #[error("Inconsistent group lists")]
    InconsistentGroupLists,
}

/// Errors for IDs
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FromInnerDataError {
    #[error(transparent)]
    IdError(#[from] tools::IdError),
    #[error(transparent)]
    InnerDataError(#[from] InnerDataError),
}

/// Potential new id returned by annotation
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewId {
    StudentId(StudentId),
    PeriodId(PeriodId),
    SubjectId(SubjectId),
    TeacherId(TeacherId),
    WeekPatternId(WeekPatternId),
    SlotId(SlotId),
    IncompatId(IncompatId),
    GroupListId(GroupListId),
    PairingRuleId(PairingRuleId),
    SlotPairingRuleId(SlotPairingRuleId),
}

impl From<StudentId> for NewId {
    fn from(value: StudentId) -> Self {
        NewId::StudentId(value)
    }
}

impl From<PeriodId> for NewId {
    fn from(value: PeriodId) -> Self {
        NewId::PeriodId(value)
    }
}

impl From<SubjectId> for NewId {
    fn from(value: SubjectId) -> Self {
        NewId::SubjectId(value)
    }
}

impl From<TeacherId> for NewId {
    fn from(value: TeacherId) -> Self {
        NewId::TeacherId(value)
    }
}

impl From<WeekPatternId> for NewId {
    fn from(value: WeekPatternId) -> Self {
        NewId::WeekPatternId(value)
    }
}

impl From<SlotId> for NewId {
    fn from(value: SlotId) -> Self {
        NewId::SlotId(value)
    }
}

impl From<IncompatId> for NewId {
    fn from(value: IncompatId) -> Self {
        NewId::IncompatId(value)
    }
}

impl From<GroupListId> for NewId {
    fn from(value: GroupListId) -> Self {
        NewId::GroupListId(value)
    }
}

impl From<PairingRuleId> for NewId {
    fn from(value: PairingRuleId) -> Self {
        NewId::PairingRuleId(value)
    }
}

impl From<SlotPairingRuleId> for NewId {
    fn from(value: SlotPairingRuleId) -> Self {
        NewId::SlotPairingRuleId(value)
    }
}

/// Errors for students operations
///
/// These errors can be returned when trying to modify [Data] with a student op.
#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum InvariantError {
    #[error("duplicated id")]
    DuplicatedId,
    #[error("invalid subject")]
    InvalidSubject,
    #[error("invalid teacher")]
    InvalidTeacher,
    #[error("invalid student")]
    InvalidStudent,
    #[error("invalid period id in assignments")]
    InvalidPeriodIdInAssignements,
    #[error("invalid subject id in assignments")]
    InvalidSubjectIdInAssignments,
    #[error("invalid student id in assignments")]
    InvalidStudentIdInAssignments,
    #[error("student assigned but not present")]
    AssignedStudentNotPresentForPeriod,
    #[error("wrong number of subjects in a period for assignments")]
    WrongSubjectCountInAssignments,
    #[error("wrong number of subjects in slots")]
    WrongSubjectCountInSlots,
    #[error("invalid slot")]
    InvalidSlot,
    #[error("invalid incompat")]
    InvalidIncompat,
    #[error("invalid group list")]
    InvalidGroupList,
    #[error("wrong number of periods in subject associations for group lists")]
    WrongPeriodCountInSubjectAssociationsForGroupLists,
    #[error("invalid group list id in subject associations")]
    InvalidGroupListIdInSubjectAssociations,
    #[error("invalid subject id in subject associations")]
    InvalidSubjectIdInSubjectAssociations,
    #[error("subject association given but subject does not have interrogations")]
    SubjectAssociationForSubjectWithoutInterrogations,
    #[error("subject association given but subject does not run on given period")]
    SubjectAssociationForSubjectNotRunningOnPeriod,
    #[error("invalid student id in settings")]
    InvalidStudentIdInSettings,
    #[error("week pattern is invalid")]
    InvalidWeekPattern,
    #[error("invalid subject id in balancing")]
    InvalidSubjectIdInBalancing,
    #[error("balancing options given for subject without interrogations")]
    BalancingForSubjectWithoutInterrogations,
    #[error("invalid pairing rule")]
    InvalidPairingRule,
    #[error("invalid slot pairing rule")]
    InvalidSlotPairingRule,
}

impl InMemoryData for Data {
    type OriginalOperation = Op;
    type AnnotatedOperation = AnnotatedOp;
    type NewInfo = Option<NewId>;
    type Error = Error;

    fn annotate(&self, op: Op) -> (AnnotatedOp, Option<NewId>) {
        let mut guard = self.id_issuer.lock().unwrap();
        AnnotatedOp::annotate(op, &mut guard)
    }

    fn apply(
        &mut self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error> {
        let backward = match op {
            AnnotatedOp::Student(student_op) => {
                AnnotatedOp::Student(self.apply_student(student_op)?)
            }
            AnnotatedOp::Period(period_op) => AnnotatedOp::Period(self.apply_period(period_op)?),
            AnnotatedOp::Subject(subject_op) => {
                AnnotatedOp::Subject(self.apply_subject(subject_op)?)
            }
            AnnotatedOp::Teacher(teacher_op) => {
                AnnotatedOp::Teacher(self.apply_teacher(teacher_op)?)
            }
            AnnotatedOp::Assignment(assignment_op) => {
                AnnotatedOp::Assignment(self.apply_assignment(assignment_op)?)
            }
            AnnotatedOp::WeekPattern(week_pattern_op) => {
                AnnotatedOp::WeekPattern(self.apply_week_pattern(week_pattern_op)?)
            }
            AnnotatedOp::Slot(slot_op) => AnnotatedOp::Slot(self.apply_slot(slot_op)?),
            AnnotatedOp::Incompat(incompat_op) => {
                AnnotatedOp::Incompat(self.apply_incompat(incompat_op)?)
            }
            AnnotatedOp::Pairing(pairing_op) => {
                AnnotatedOp::Pairing(self.apply_pairing(pairing_op)?)
            }
            AnnotatedOp::SlotPairing(slot_pairing_op) => {
                AnnotatedOp::SlotPairing(self.apply_slot_pairing(slot_pairing_op)?)
            }
            AnnotatedOp::GroupList(group_list_op) => {
                AnnotatedOp::GroupList(self.apply_group_list(group_list_op)?)
            }
            AnnotatedOp::Settings(settings_op) => {
                AnnotatedOp::Settings(self.apply_settings(settings_op)?)
            }
            AnnotatedOp::Balancing(balancing_op) => {
                AnnotatedOp::Balancing(self.apply_balancing(balancing_op)?)
            }
            AnnotatedOp::Colloscope(colloscope_op) => {
                AnnotatedOp::Colloscope(self.apply_colloscope(colloscope_op)?)
            }
            AnnotatedOp::ExportConfig(export_config_op) => {
                AnnotatedOp::ExportConfig(self.apply_export_config(export_config_op)?)
            }
            AnnotatedOp::GlobalUpdate(new_inner_data) => {
                new_inner_data.check_invariants()?;
                let old = std::mem::replace(&mut self.inner_data, new_inner_data.clone());
                AnnotatedOp::GlobalUpdate(old)
            }
        };
        self.check_invariants();
        Ok(backward)
    }
}

impl Data {
    /// USED INTERNALLY
    ///
    /// Checks all the invariants of data
    fn check_invariants(&self) {
        let max_id = self.inner_data.ids().max();

        if let Some(id) = max_id {
            let guard = self.id_issuer.lock().expect("No error on lock");
            if id >= guard.get_internal_counter() {
                panic!("IdIssuer internal counter is not greater than all internal ids");
            }
        }

        self.inner_data
            .check_invariants()
            .expect("Invariants should be valid in Data");
    }
}

impl Default for Data {
    fn default() -> Self {
        Self::new()
    }
}

impl Data {
    /// Create a new [Data]
    ///
    /// This [Data] is basically empty and corresponds to the
    /// state of a new file
    pub fn new() -> Data {
        Self::from_inner_data(InnerData::default()).expect("Default data should be valid")
    }

    /// Create a new [Data] from existing data
    ///
    /// This will check the consistency of the data
    /// and will also do some internal checks, so this might fail.
    pub fn from_inner_data(inner_data: InnerData) -> Result<Data, FromInnerDataError> {
        inner_data.check_invariants()?;

        let id_issuer = IdIssuer::new(inner_data.ids())?;

        let data = Data {
            id_issuer: std::sync::Mutex::new(id_issuer),
            inner_data,
        };

        data.check_invariants();

        Ok(data)
    }

    /// Returns a non-mutable reference to internal data
    ///
    /// Elementary ops allow the edition of data. But between two ops
    /// you can inspect the current data via this function
    pub fn get_inner_data(&self) -> &InnerData {
        &self.inner_data
    }

    /// Used internally
    ///
    /// Apply student operations
    fn apply_student(
        &mut self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<AnnotatedStudentOp, StudentError> {
        match student_op {
            AnnotatedStudentOp::Add(new_id, student) => {
                if self
                    .inner_data
                    .params
                    .students
                    .student_map
                    .contains_key(new_id)
                {
                    return Err(StudentError::StudentIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_student(student)?;

                self.inner_data
                    .params
                    .students
                    .student_map
                    .insert(*new_id, student.clone());

                Ok(AnnotatedStudentOp::Remove(*new_id))
            }
            AnnotatedStudentOp::Remove(id) => {
                let Some(current_student) = self.inner_data.params.students.student_map.get(id)
                else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (group_list_id, group_list) in &self.inner_data.colloscope.group_lists {
                    if group_list.groups_for_students.contains_key(id) {
                        return Err(StudentError::StudentIsReferencedInColloscopeGroupList(
                            *id,
                            *group_list_id,
                        ));
                    }
                }

                for (group_list_id, group_list) in
                    &self.inner_data.params.group_lists.group_list_map
                {
                    if group_list.filling.excluded_students().contains(id) {
                        return Err(StudentError::StudentIsStillExcludedByGroupList(
                            *id,
                            *group_list_id,
                        ));
                    }
                    if group_list.filling.contains_student(*id) {
                        return Err(StudentError::StudentIsStillReferencedByPrefilledGroupList(
                            *id,
                            *group_list_id,
                        ));
                    }
                }

                for (period_id, period_assignments) in
                    &self.inner_data.params.assignments.period_map
                {
                    if current_student.excluded_periods.contains(period_id) {
                        continue;
                    }
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(id) {
                            return Err(StudentError::StudentStillHasNonTrivialAssignments(
                                *id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                if self.inner_data.params.settings.students.contains_key(id) {
                    return Err(StudentError::StudentStillHasSettings(*id));
                }

                let old_student = self
                    .inner_data
                    .params
                    .students
                    .student_map
                    .remove(id)
                    .expect("Student ID was checked above");

                Ok(AnnotatedStudentOp::Add(*id, old_student))
            }
            AnnotatedStudentOp::Update(id, new_student) => {
                self.inner_data.params.validate_student(new_student)?;
                let Some(current_student) = self.inner_data.params.students.student_map.get_mut(id)
                else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (period_id, period_assignments) in
                    &self.inner_data.params.assignments.period_map
                {
                    if current_student.excluded_periods.contains(period_id)
                        || !new_student.excluded_periods.contains(period_id)
                    {
                        continue;
                    }
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(id) {
                            return Err(StudentError::StudentStillHasNonTrivialAssignments(
                                *id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                let old_student = std::mem::replace(current_student, new_student.clone());

                Ok(AnnotatedStudentOp::Update(*id, old_student))
            }
        }
    }

    /// Used internally
    ///
    /// Apply period operations
    fn apply_period(
        &mut self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<AnnotatedPeriodOp, PeriodError> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(new_date) => {
                let old_date = std::mem::replace(
                    &mut self.inner_data.params.periods.first_week,
                    new_date.clone(),
                );
                Ok(AnnotatedPeriodOp::ChangeStartDate(old_date))
            }
            AnnotatedPeriodOp::AddFront(period_id, desc) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert(0, (*period_id, desc.clone()));
                self.inner_data.params.assignments.period_map.insert(
                    *period_id,
                    assignments::PeriodAssignments {
                        subject_map: self
                            .inner_data
                            .params
                            .subjects
                            .ordered_subject_list
                            .iter()
                            .map(|(subject_id, _subject)| (*subject_id, BTreeSet::new()))
                            .collect(),
                    },
                );
                self.inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .insert(*period_id, BTreeMap::new());
                for week_pattern in self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .values_mut()
                {
                    week_pattern.add_weeks(0, desc.len());
                }
                self.inner_data.colloscope.period_map.insert(
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(&self.inner_data.params, *period_id),
                );
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id, desc) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                let Some((position, new_first_week)) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position_and_total_number_of_weeks(*after_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*after_id));
                };

                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert(position + 1, (*period_id, desc.clone()));
                self.inner_data.params.assignments.period_map.insert(
                    *period_id,
                    assignments::PeriodAssignments {
                        subject_map: self
                            .inner_data
                            .params
                            .subjects
                            .ordered_subject_list
                            .iter()
                            .map(|(subject_id, _subject)| (*subject_id, BTreeSet::new()))
                            .collect(),
                    },
                );
                self.inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .insert(*period_id, BTreeMap::new());
                for week_pattern in self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .values_mut()
                {
                    week_pattern.add_weeks(new_first_week, desc.len());
                }
                self.inner_data.colloscope.period_map.insert(
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(&self.inner_data.params, *period_id),
                );
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some((position, first_week)) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                let colloscope_period = self
                    .inner_data
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");

                if !colloscope_period.is_empty() {
                    return Err(PeriodError::NotEmptyPeriodInColloscope(*period_id));
                }

                let week_count = self.inner_data.params.periods.ordered_period_list[position]
                    .1
                    .len();

                for (week_pattern_id, week_pattern) in
                    &self.inner_data.params.week_patterns.week_pattern_map
                {
                    if !week_pattern.can_remove_weeks(first_week, week_count) {
                        return Err(PeriodError::NonTrivialWeekPattern(
                            *period_id,
                            *week_pattern_id,
                        ));
                    }
                }

                for (subject_id, subject) in &self.inner_data.params.subjects.ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySubject(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                for (student_id, student) in &self.inner_data.params.students.student_map {
                    if student.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByStudent(
                            *period_id,
                            *student_id,
                        ));
                    }
                }

                for (rule_id, rule) in &self.inner_data.params.pairings.pairing_rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByPairingRule(
                            *period_id, *rule_id,
                        ));
                    }
                }

                for (rule_id, rule) in &self.inner_data.params.slot_pairings.slot_pairing_rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySlotPairingRule(
                            *period_id, *rule_id,
                        ));
                    }
                }

                let period_assignments = self
                    .inner_data
                    .params
                    .assignments
                    .period_map
                    .get(period_id)
                    .expect("At this point, period id should be valid");
                for (subject_id, assigned_students) in &period_assignments.subject_map {
                    if !assigned_students.is_empty() {
                        return Err(PeriodError::PeriodStillHasNonTrivialAssignments(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                let subject_map = self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .get(period_id)
                    .expect("Period id should be valid at this point");
                if !subject_map.is_empty() {
                    return Err(PeriodError::PeriodStillHasNonTrivialGroupListAssociation(
                        *period_id,
                    ));
                }

                let previous_id = (position > 0)
                    .then(|| self.inner_data.params.periods.ordered_period_list[position - 1].0);

                let (_, old_desc) = self
                    .inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .remove(position);
                self.inner_data
                    .params
                    .assignments
                    .period_map
                    .remove(period_id);
                self.inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .remove(period_id);
                for week_pattern in self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .values_mut()
                {
                    week_pattern.remove_weeks(first_week, week_count);
                }
                self.inner_data.colloscope.period_map.remove(period_id);

                Ok(match previous_id {
                    None => AnnotatedPeriodOp::AddFront(*period_id, old_desc),
                    Some(prev) => AnnotatedPeriodOp::AddAfter(*period_id, prev, old_desc),
                })
            }
            AnnotatedPeriodOp::Update(period_id, desc) => {
                let Some((position, first_week)) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                let period = &self.inner_data.params.periods.ordered_period_list[position].1;
                let old_length = period.len();
                if desc.len() < old_length {
                    for (week_pattern_id, week_pattern) in
                        &self.inner_data.params.week_patterns.week_pattern_map
                    {
                        if !week_pattern
                            .can_remove_weeks(first_week + desc.len(), old_length - desc.len())
                        {
                            return Err(PeriodError::NonTrivialWeekPattern(
                                *period_id,
                                *week_pattern_id,
                            ));
                        }
                    }
                }
                let colloscope_period = self
                    .inner_data
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");
                for (slot_id, collo_slot) in &colloscope_period.slot_map {
                    let slot = self
                        .inner_data
                        .params
                        .slots
                        .find_slot(*slot_id)
                        .expect("Slot ID should be valid");
                    let new_pattern = slot.build_pattern_for_new_period(
                        desc,
                        first_week,
                        &self.inner_data.params.week_patterns,
                    );

                    if !collo_slot.check_empty_on_removed_weeks(&new_pattern) {
                        return Err(PeriodError::NotCompatibleSlotInColloscope(*slot_id));
                    }
                }

                let old_desc = std::mem::replace(
                    &mut self.inner_data.params.periods.ordered_period_list[position].1,
                    desc.clone(),
                );
                if desc.len() > old_length {
                    let first_week_to_add = first_week + old_length;
                    for week_pattern in self
                        .inner_data
                        .params
                        .week_patterns
                        .week_pattern_map
                        .values_mut()
                    {
                        week_pattern.add_weeks(first_week_to_add, desc.len() - old_length);
                    }
                } else if desc.len() < old_length {
                    let first_week_to_remove = first_week + desc.len();
                    for week_pattern in self
                        .inner_data
                        .params
                        .week_patterns
                        .week_pattern_map
                        .values_mut()
                    {
                        week_pattern.remove_weeks(first_week_to_remove, old_length - desc.len());
                    }
                }
                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, _slot) in &subject_slots.ordered_slots {
                        self.inner_data
                            .colloscope
                            .update_slot_to_match_week_pattern(*slot_id, &self.inner_data.params);
                    }
                }

                Ok(AnnotatedPeriodOp::Update(*period_id, old_desc))
            }
        }
    }

    /// Used internally
    ///
    /// Apply period operations
    fn apply_subject(
        &mut self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<AnnotatedSubjectOp, SubjectError> {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, params) => {
                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectError::SubjectIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_subject(params)?;

                let position = match after_id {
                    Some(id) => {
                        self.inner_data
                            .params
                            .subjects
                            .find_subject_position(*id)
                            .ok_or(SubjectError::InvalidSubjectId(*id))?
                            + 1
                    }
                    None => 0,
                };

                self.inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .insert(position, (*new_id, params.clone()));
                if params.parameters.interrogation_parameters.is_some() {
                    self.inner_data.params.slots.subject_map.insert(
                        *new_id,
                        slots::SubjectSlots {
                            ordered_slots: vec![],
                        },
                    );
                }
                for (period_id, _period) in &self.inner_data.params.periods.ordered_period_list {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .params
                        .assignments
                        .period_map
                        .get_mut(period_id)
                        .expect("Every period should appear in assignments");

                    period_assignment
                        .subject_map
                        .insert(*new_id, BTreeSet::new());
                }

                Ok(AnnotatedSubjectOp::Remove(*new_id))
            }
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                if *new_pos >= self.inner_data.params.subjects.ordered_subject_list.len() {
                    return Err(SubjectError::PositionOutOfBounds(
                        *new_pos,
                        self.inner_data.params.subjects.ordered_subject_list.len(),
                    ));
                }
                let Some(old_pos) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let data = self
                    .inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .remove(old_pos);
                self.inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .insert(*new_pos, data);
                Ok(AnnotatedSubjectOp::ChangePosition(*id, old_pos))
            }
            AnnotatedSubjectOp::Remove(id) => {
                let Some(position) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                if self.inner_data.params.balancing.subjects.contains_key(id) {
                    return Err(SubjectError::SubjectStillHasBalancingOptions(*id));
                }

                for (rule_id, rule) in &self.inner_data.params.pairings.pairing_rule_map {
                    if rule.antecedent.subject_id == *id || rule.consequent.subject_id == *id {
                        return Err(SubjectError::SubjectIsReferencedByPairingRule(
                            *id, *rule_id,
                        ));
                    }
                }

                for (period_id, subject_map) in
                    &self.inner_data.params.group_lists.subjects_associations
                {
                    if let Some(group_list_id) = subject_map.get(id) {
                        return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                            *id,
                            *group_list_id,
                            *period_id,
                        ));
                    }
                }

                if let Some(subject_slots) = self.inner_data.params.slots.subject_map.get(id)
                    && !subject_slots.ordered_slots.is_empty()
                {
                    return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                }

                for (teacher_id, teacher) in &self.inner_data.params.teachers.teacher_map {
                    if teacher.subjects.contains(id) {
                        return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                            *teacher_id,
                            *id,
                        ));
                    }
                }

                for (incompat_id, incompat) in &self.inner_data.params.incompats.incompat_map {
                    if incompat.subject_id == *id {
                        return Err(SubjectError::SubjectStillHasAssociatedIncompats(
                            *id,
                            *incompat_id,
                        ));
                    }
                }

                let params = &self.inner_data.params.subjects.ordered_subject_list[position].1;
                for (period_id, _period) in &self.inner_data.params.periods.ordered_period_list {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .params
                        .assignments
                        .period_map
                        .get(period_id)
                        .expect("Every period should appear in assignments");

                    let assigned_students = period_assignment
                        .subject_map
                        .get(id)
                        .expect("Subject should appear in assignments for relevant periods");

                    if !assigned_students.is_empty() {
                        return Err(SubjectError::SubjectStillHasNonTrivialAssignments(
                            *period_id, *id,
                        ));
                    }
                }

                let previous_id = (position > 0)
                    .then(|| self.inner_data.params.subjects.ordered_subject_list[position - 1].0);

                let (_, params) = self
                    .inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .remove(position);
                self.inner_data.params.slots.subject_map.remove(id);
                for (period_id, _period) in &self.inner_data.params.periods.ordered_period_list {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .params
                        .assignments
                        .period_map
                        .get_mut(period_id)
                        .expect("Every period should appear in assignments");

                    period_assignment.subject_map.remove(id);
                }

                Ok(AnnotatedSubjectOp::AddAfter(*id, previous_id, params))
            }
            AnnotatedSubjectOp::Update(id, new_params) => {
                self.inner_data.params.validate_subject(new_params)?;
                let Some(position) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let old_params = self.inner_data.params.subjects.ordered_subject_list[position]
                    .1
                    .clone();

                if old_params.parameters.interrogation_parameters.is_some()
                    && new_params.parameters.interrogation_parameters.is_none()
                {
                    if self.inner_data.params.balancing.subjects.contains_key(id) {
                        return Err(SubjectError::SubjectStillHasBalancingOptions(*id));
                    }

                    // The new subject does not have interrogations, let's check that no teacher has been assigned to it
                    for (teacher_id, teacher) in &self.inner_data.params.teachers.teacher_map {
                        if teacher.subjects.contains(id) {
                            return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                                *teacher_id,
                                *id,
                            ));
                        }
                    }

                    // Also, we should not have a corresponding group list
                    for (period_id, subject_map) in
                        &self.inner_data.params.group_lists.subjects_associations
                    {
                        if let Some(group_list_id) = subject_map.get(id) {
                            return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                                *id,
                                *group_list_id,
                                *period_id,
                            ));
                        }
                    }

                    // Let's also check that we don't have corresponding interrogations
                    let subject_slots = self
                        .inner_data
                        .params
                        .slots
                        .subject_map
                        .get(id)
                        .expect("Subject should have a slot list at this point");

                    if !subject_slots.ordered_slots.is_empty() {
                        return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                    }
                }

                for (period_id, _period) in &self.inner_data.params.periods.ordered_period_list {
                    // If the period was excluded before, there is no structure to check
                    // and if the period is not excluded now, the structure will be fine anyway
                    if old_params.excluded_periods.contains(period_id)
                        || !new_params.excluded_periods.contains(period_id)
                    {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .params
                        .assignments
                        .period_map
                        .get(period_id)
                        .expect("Every period should appear in assignments");

                    let assigned_students = period_assignment
                        .subject_map
                        .get(id)
                        .expect("Subject should appear in assignments for relevant periods");

                    if !assigned_students.is_empty() {
                        return Err(SubjectError::SubjectStillHasNonTrivialAssignments(
                            *period_id, *id,
                        ));
                    }

                    let subject_map = self
                        .inner_data
                        .params
                        .group_lists
                        .subjects_associations
                        .get(period_id)
                        .expect("Period id should be valid at this point");

                    if let Some(group_list_id) = subject_map.get(id) {
                        return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                            *id,
                            *group_list_id,
                            *period_id,
                        ));
                    }

                    // Check if there are non-empty slots in colloscope for the subject
                    if let Some(subject_slots) = self.inner_data.params.slots.subject_map.get(id) {
                        let colloscope_period = self
                            .inner_data
                            .colloscope
                            .period_map
                            .get(period_id)
                            .expect("Period ID should be valid at this point");

                        for (slot_id, _slot) in &subject_slots.ordered_slots {
                            let Some(collo_slot) = colloscope_period.slot_map.get(slot_id) else {
                                continue;
                            };
                            if !collo_slot.is_empty() {
                                return Err(SubjectError::SubjectStillHasNonEmptySlotInColloscope(
                                    *id, *slot_id,
                                ));
                            }
                        }
                    }
                }

                self.inner_data.params.subjects.ordered_subject_list[position].1 =
                    new_params.clone();
                if new_params.parameters.interrogation_parameters.is_some()
                    != old_params.parameters.interrogation_parameters.is_some()
                {
                    if new_params.parameters.interrogation_parameters.is_some() {
                        // We don't need to update the colloscope in this case: no slots have been added so far
                        self.inner_data.params.slots.subject_map.insert(
                            *id,
                            slots::SubjectSlots {
                                ordered_slots: vec![],
                            },
                        );
                    } else {
                        // We don't need to update the colloscope in this case: all slots have already been removed
                        self.inner_data.params.slots.subject_map.remove(id);
                    }
                }

                // Let's update the colloscope.
                // However, if there are no interrogations, then we don't have slots to update
                if new_params.parameters.interrogation_parameters.is_some() {
                    let subject_slots = self
                        .inner_data
                        .params
                        .slots
                        .subject_map
                        .get(id)
                        .expect("Subject should have a slot list at this point");

                    for (period_id, collo_period) in &mut self.inner_data.colloscope.period_map {
                        // Only change in period status should be considered
                        if old_params.excluded_periods.contains(period_id)
                            == new_params.excluded_periods.contains(period_id)
                        {
                            continue;
                        }

                        if old_params.excluded_periods.contains(period_id) {
                            // The period was excluded but is not anymore
                            for (slot_id, _slot) in &subject_slots.ordered_slots {
                                collo_period.slot_map.insert(
                                    *slot_id,
                                    colloscopes::ColloscopeSlot::new_empty_from_params(
                                        &self.inner_data.params,
                                        *period_id,
                                        *slot_id,
                                    ),
                                );
                            }
                        } else {
                            // The period was included but will now be excluded
                            for (slot_id, _slot) in &subject_slots.ordered_slots {
                                collo_period.slot_map.remove(slot_id);
                            }
                        }
                    }
                }

                for (period_id, _period) in &self.inner_data.params.periods.ordered_period_list {
                    // Only change in period status should be considered
                    if old_params.excluded_periods.contains(period_id)
                        == new_params.excluded_periods.contains(period_id)
                    {
                        continue;
                    }

                    if old_params.excluded_periods.contains(period_id) {
                        // The period was excluded but is not anymore
                        let period_assignment = self
                            .inner_data
                            .params
                            .assignments
                            .period_map
                            .get_mut(period_id)
                            .expect("Every period should appear in assignments");

                        period_assignment.subject_map.insert(*id, BTreeSet::new());
                    } else {
                        // The period was included but will now be excluded
                        let period_assignment = self
                            .inner_data
                            .params
                            .assignments
                            .period_map
                            .get_mut(period_id)
                            .expect("Every period should appear in assignments");

                        period_assignment.subject_map.remove(id);
                    }
                }

                Ok(AnnotatedSubjectOp::Update(*id, old_params))
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_teacher(
        &mut self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<AnnotatedTeacherOp, TeacherError> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, teacher) => {
                if self
                    .inner_data
                    .params
                    .teachers
                    .teacher_map
                    .contains_key(new_id)
                {
                    return Err(TeacherError::TeacherIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_teacher(teacher)?;

                self.inner_data
                    .params
                    .teachers
                    .teacher_map
                    .insert(*new_id, teacher.clone());

                Ok(AnnotatedTeacherOp::Remove(*new_id))
            }
            AnnotatedTeacherOp::Remove(id) => {
                if !self.inner_data.params.teachers.teacher_map.contains_key(id) {
                    return Err(TeacherError::InvalidTeacherId(*id));
                }

                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if *id == slot.teacher_id {
                            return Err(TeacherError::TeacherStillHasAssociatedSlots(
                                *id, *slot_id,
                            ));
                        }
                    }
                }

                let old_teacher = self
                    .inner_data
                    .params
                    .teachers
                    .teacher_map
                    .remove(id)
                    .expect("Teacher ID was checked above");

                Ok(AnnotatedTeacherOp::Add(*id, old_teacher))
            }
            AnnotatedTeacherOp::Update(id, new_teacher) => {
                self.inner_data.params.validate_teacher(new_teacher)?;
                let Some(current_teacher) = self.inner_data.params.teachers.teacher_map.get_mut(id)
                else {
                    return Err(TeacherError::InvalidTeacherId(*id));
                };

                for (subject_id, subject_slots) in &self.inner_data.params.slots.subject_map {
                    if new_teacher.subjects.contains(subject_id) {
                        continue;
                    }
                    for (_slot_id, slot) in &subject_slots.ordered_slots {
                        if *id == slot.teacher_id {
                            return Err(TeacherError::TeacherStillHasAssociatedSlotsInSubject(
                                *id,
                                *subject_id,
                            ));
                        }
                    }
                }

                let old_teacher = std::mem::replace(current_teacher, new_teacher.clone());

                Ok(AnnotatedTeacherOp::Update(*id, old_teacher))
            }
        }
    }

    /// Used internally
    ///
    /// Apply assignment operations
    fn apply_assignment(
        &mut self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<AnnotatedAssignmentOp, AssignmentError> {
        match assignment_op {
            AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, status) => {
                let Some(period_assignments) = self
                    .inner_data
                    .params
                    .assignments
                    .period_map
                    .get_mut(period_id)
                else {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                };

                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AssignmentError::InvalidSubjectId(*subject_id));
                }

                let Some(assigned_students) = period_assignments.subject_map.get_mut(subject_id)
                else {
                    return Err(AssignmentError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                };

                let Some(student_desc) =
                    self.inner_data.params.students.student_map.get(student_id)
                else {
                    return Err(AssignmentError::InvalidStudentId(*student_id));
                };

                if student_desc.excluded_periods.contains(period_id) {
                    return Err(AssignmentError::StudentIsNotPresentOnPeriod(
                        *student_id,
                        *period_id,
                    ));
                }

                let previous_status = assigned_students.contains(student_id);

                if *status {
                    assigned_students.insert(*student_id);
                } else {
                    assigned_students.remove(student_id);
                }

                Ok(AnnotatedAssignmentOp::Assign(
                    *period_id,
                    *student_id,
                    *subject_id,
                    previous_status,
                ))
            }
        }
    }

    /// Used internally
    ///
    /// Apply week pattern operations
    fn apply_week_pattern(
        &mut self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<AnnotatedWeekPatternOp, WeekPatternError> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, week_pattern) => {
                if self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains_key(new_id)
                {
                    return Err(WeekPatternError::WeekPatternIdAlreadyExists(*new_id));
                }

                self.inner_data.params.validate_week_pattern(week_pattern)?;

                self.inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .insert(*new_id, week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Remove(*new_id))
            }
            AnnotatedWeekPatternOp::Remove(id) => {
                if !self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains_key(id)
                {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                }

                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if let Some(week_pattern_id) = &slot.week_pattern
                            && *id == *week_pattern_id
                        {
                            return Err(WeekPatternError::WeekPatternStillHasAssociatedSlots(
                                *id, *slot_id,
                            ));
                        }
                    }
                }

                for (incompat_id, incompat) in &self.inner_data.params.incompats.incompat_map {
                    if let Some(week_pattern_id) = &incompat.week_pattern_id
                        && *id == *week_pattern_id
                    {
                        return Err(WeekPatternError::WeekPatternStillHasAssociatedIncompat(
                            *id,
                            *incompat_id,
                        ));
                    }
                }

                let old_week_pattern = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .remove(id)
                    .expect("Week pattern ID was checked above");

                Ok(AnnotatedWeekPatternOp::Add(*id, old_week_pattern))
            }
            AnnotatedWeekPatternOp::Update(id, new_week_pattern) => {
                self.inner_data
                    .params
                    .validate_week_pattern(new_week_pattern)?;
                let new_merged_pattern = self
                    .inner_data
                    .params
                    .merge_pattern(&new_week_pattern.weeks);

                let Some(current_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                };

                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.week_pattern != Some(*id) {
                            continue;
                        }

                        if !self.inner_data.colloscope.check_empty_on_removed_weeks(
                            *slot_id,
                            &self.inner_data.params.periods,
                            &new_merged_pattern,
                        ) {
                            return Err(WeekPatternError::NotCompatibleSlotInColloscope(*slot_id));
                        }
                    }
                }

                let old_week_pattern =
                    std::mem::replace(current_week_pattern, new_week_pattern.clone());
                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.week_pattern != Some(*id) {
                            continue;
                        }

                        self.inner_data.colloscope.update_slot_for_week_pattern(
                            *slot_id,
                            &self.inner_data.params.periods,
                            &new_merged_pattern,
                        );
                    }
                }

                Ok(AnnotatedWeekPatternOp::Update(*id, old_week_pattern))
            }
        }
    }

    /// Used internally
    ///
    /// Apply slot operations
    fn apply_slot(
        &mut self,
        slot_op: &AnnotatedSlotOp,
    ) -> std::result::Result<AnnotatedSlotOp, SlotError> {
        match slot_op {
            AnnotatedSlotOp::AddAfter(new_id, subject_id, after_id, slot) => {
                if self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*new_id)
                    .is_some()
                {
                    return Err(SlotError::SlotIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_slot(slot, *subject_id)?;

                let position = match after_id {
                    Some(id) => {
                        let (sub_id, after_pos) = self
                            .inner_data
                            .params
                            .slots
                            .find_slot_subject_and_position(*id)
                            .ok_or(SlotError::InvalidSlotId(*id))?;
                        if sub_id != *subject_id {
                            return Err(SlotError::PreviousSlotIsNotInRightSubject(
                                *id,
                                *subject_id,
                            ));
                        }

                        after_pos + 1
                    }
                    None => 0,
                };

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(subject_id)
                    .ok_or(SlotError::SubjectHasNoInterrogation(*subject_id))?;

                subject_slots
                    .ordered_slots
                    .insert(position, (*new_id, slot.clone()));

                let subject = self
                    .inner_data
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .expect("Subject ID should be valid at this point");
                for (period_id, period) in &mut self.inner_data.colloscope.period_map {
                    if subject.excluded_periods.contains(period_id) {
                        continue;
                    }

                    period.slot_map.insert(
                        *new_id,
                        colloscopes::ColloscopeSlot::new_empty_from_params(
                            &self.inner_data.params,
                            *period_id,
                            *new_id,
                        ),
                    );
                }

                Ok(AnnotatedSlotOp::Remove(*new_id))
            }
            AnnotatedSlotOp::ChangePosition(id, new_pos) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");

                if *new_pos >= subject_slots.ordered_slots.len() {
                    return Err(SlotError::PositionOutOfBounds(
                        *new_pos,
                        subject_slots.ordered_slots.len(),
                    ));
                }

                let data = subject_slots.ordered_slots.remove(old_pos);
                subject_slots.ordered_slots.insert(*new_pos, data);

                Ok(AnnotatedSlotOp::ChangePosition(*id, old_pos))
            }
            AnnotatedSlotOp::Remove(id) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                for (period_id, collo_period) in &self.inner_data.colloscope.period_map {
                    let Some(collo_slot) = collo_period.slot_map.get(id) else {
                        continue;
                    };

                    if !collo_slot.is_empty() {
                        return Err(SlotError::NotEmptySlotInColloscope(*id, *period_id));
                    }
                }

                for (rule_id, rule) in &self.inner_data.params.slot_pairings.slot_pairing_rule_map {
                    if rule.antecedent.slot_id == *id || rule.consequent.slot_id == *id {
                        return Err(SlotError::SlotIsReferencedBySlotPairingRule(*id, *rule_id));
                    }
                }

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");
                let previous_id = (old_pos > 0).then(|| subject_slots.ordered_slots[old_pos - 1].0);
                let (_, old_slot) = subject_slots.ordered_slots.remove(old_pos);
                for collo_period in self.inner_data.colloscope.period_map.values_mut() {
                    // The slot might not be in period but this won't raise an error
                    collo_period.slot_map.remove(id);
                }

                Ok(AnnotatedSlotOp::AddAfter(
                    *id,
                    subject_id,
                    previous_id,
                    old_slot,
                ))
            }
            AnnotatedSlotOp::Update(slot_id, new_slot) => {
                let Some((subject_id, position)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(*slot_id));
                };

                self.inner_data.params.validate_slot(new_slot, subject_id)?;
                let pattern = self
                    .inner_data
                    .params
                    .get_merged_pattern(new_slot.week_pattern);
                if !self.inner_data.colloscope.check_empty_on_removed_weeks(
                    *slot_id,
                    &self.inner_data.params.periods,
                    &pattern[..],
                ) {
                    return Err(SlotError::NotCompatibleSlotInColloscope(
                        *slot_id,
                        new_slot.week_pattern,
                    ));
                }

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");

                let old_slot = std::mem::replace(
                    &mut subject_slots.ordered_slots[position].1,
                    new_slot.clone(),
                );
                self.inner_data.colloscope.update_slot_for_week_pattern(
                    *slot_id,
                    &self.inner_data.params.periods,
                    &pattern[..],
                );

                Ok(AnnotatedSlotOp::Update(*slot_id, old_slot))
            }
        }
    }

    /// Used internally
    ///
    /// Apply incompat operations
    fn apply_incompat(
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

    /// Used internally
    ///
    /// Apply pairing rule operations
    fn apply_pairing(
        &mut self,
        pairing_op: &AnnotatedPairingOp,
    ) -> std::result::Result<AnnotatedPairingOp, PairingError> {
        match pairing_op {
            AnnotatedPairingOp::Add(new_id, rule) => {
                if self
                    .inner_data
                    .params
                    .pairings
                    .pairing_rule_map
                    .contains_key(new_id)
                {
                    return Err(PairingError::PairingRuleIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_pairing_rule(rule)?;

                self.inner_data
                    .params
                    .pairings
                    .pairing_rule_map
                    .insert(*new_id, rule.clone());

                Ok(AnnotatedPairingOp::Remove(*new_id))
            }
            AnnotatedPairingOp::Remove(id) => {
                let Some(old_rule) = self.inner_data.params.pairings.pairing_rule_map.remove(id)
                else {
                    return Err(PairingError::InvalidPairingRuleId(*id));
                };

                Ok(AnnotatedPairingOp::Add(*id, old_rule))
            }
            AnnotatedPairingOp::Update(id, new_rule) => {
                self.inner_data.params.validate_pairing_rule(new_rule)?;

                let Some(rule) = self.inner_data.params.pairings.pairing_rule_map.get_mut(id)
                else {
                    return Err(PairingError::InvalidPairingRuleId(*id));
                };

                let old_rule = std::mem::replace(rule, new_rule.clone());

                Ok(AnnotatedPairingOp::Update(*id, old_rule))
            }
        }
    }

    fn apply_slot_pairing(
        &mut self,
        slot_pairing_op: &AnnotatedSlotPairingOp,
    ) -> Result<AnnotatedSlotPairingOp, SlotPairingError> {
        let backward = match slot_pairing_op {
            AnnotatedSlotPairingOp::Add(new_id, rule) => {
                if self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .contains_key(new_id)
                {
                    return Err(SlotPairingError::SlotPairingRuleIdAlreadyExists(*new_id));
                }

                self.inner_data.params.validate_slot_pairing_rule(rule)?;

                self.inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .insert(*new_id, rule.clone());

                AnnotatedSlotPairingOp::Remove(*new_id)
            }
            AnnotatedSlotPairingOp::Remove(id) => {
                let Some(old_rule) = self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .remove(id)
                else {
                    return Err(SlotPairingError::InvalidSlotPairingRuleId(*id));
                };

                AnnotatedSlotPairingOp::Add(*id, old_rule)
            }
            AnnotatedSlotPairingOp::Update(id, new_rule) => {
                self.inner_data
                    .params
                    .validate_slot_pairing_rule(new_rule)?;

                let Some(rule) = self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .get_mut(id)
                else {
                    return Err(SlotPairingError::InvalidSlotPairingRuleId(*id));
                };

                let old_rule = std::mem::replace(rule, new_rule.clone());

                AnnotatedSlotPairingOp::Update(*id, old_rule)
            }
        };
        Ok(backward)
    }

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
        let collo_period = self
            .inner_data
            .colloscope
            .period_map
            .get(&period_id)
            .expect("Period ID should be valid at this point");
        let Some(subject_slots) = self.inner_data.params.slots.subject_map.get(&subject_id) else {
            // No slots: no interrogation can reference a group number
            return Ok(());
        };
        for (slot_id, _slot) in &subject_slots.ordered_slots {
            let collo_slot = collo_period
                .slot_map
                .get(slot_id)
                .expect("Subject should run on given period");

            for interrogation in collo_slot.interrogations.iter().flatten() {
                for group in &interrogation.assigned_groups {
                    if *group >= first_forbidden_group_number {
                        return Err(GroupListError::InvalidGroupInSubjectSlotInColloscope(
                            subject_id, period_id, *slot_id,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Used internally
    ///
    /// Apply group list operations
    fn apply_group_list(
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
                    .contains_key(new_id)
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

                // Only non-prefilled group lists have a colloscope entry
                // (mirrors the Remove logic)
                if !filling.is_prefilled() {
                    self.inner_data
                        .colloscope
                        .group_lists
                        .insert(*new_id, colloscopes::ColloscopeGroupList::new_empty());
                }

                Ok(AnnotatedGroupListOp::Remove(*new_id))
            }
            AnnotatedGroupListOp::Remove(id) => {
                let Some(old_group_list) =
                    self.inner_data.params.group_lists.group_list_map.get(id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*id));
                };
                let was_prefilled = old_group_list.is_prefilled();

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
                        let collo_group_list = self
                            .inner_data
                            .colloscope
                            .group_lists
                            .get(id)
                            .expect("Non-prefilled group list should have colloscope entry");
                        if !collo_group_list.is_empty() {
                            return Err(GroupListError::NotEmptyGroupListInColloscope(*id));
                        }
                    }
                }

                for subject_map in self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .values()
                {
                    for group_list_id in subject_map.values() {
                        if *group_list_id == *id {
                            return Err(GroupListError::RemainingAssociatedSubjects);
                        }
                    }
                }

                let old_group_list = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .remove(id)
                    .expect("Group list ID was checked above");
                if !was_prefilled {
                    self.inner_data.colloscope.group_lists.remove(id);
                }

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

                // Only validate colloscope entry for non-prefilled group lists
                if !old_group_list.is_prefilled() {
                    let collo_group_list = self
                        .inner_data
                        .colloscope
                        .group_lists
                        .get(group_list_id)
                        .expect("Non-prefilled group list should have colloscope entry");
                    if collo_group_list
                        .validate_against_params(
                            *group_list_id,
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
                }

                // The interrogations of every subject associated with this
                // list must stay within the new group count
                let first_forbidden_group_number = new_params.group_names.len() as u32;
                for (period_id, subject_map) in
                    &self.inner_data.params.group_lists.subjects_associations
                {
                    for (subject_id, associated_list) in subject_map {
                        if associated_list == group_list_id {
                            self.check_interrogations_group_bound(
                                *period_id,
                                *subject_id,
                                first_forbidden_group_number,
                            )?;
                        }
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
                    // Transitioning to prefilled: check colloscope is empty, then remove entry
                    let collo_group_list = self
                        .inner_data
                        .colloscope
                        .group_lists
                        .get(group_list_id)
                        .expect("Non-prefilled group list should have colloscope entry");
                    if !collo_group_list.groups_for_students.is_empty() {
                        return Err(GroupListError::NonEmptyColloscopeGroupListWhenPrefilling(
                            *group_list_id,
                        ));
                    }
                    self.inner_data.colloscope.group_lists.remove(group_list_id);
                } else if was_prefilled && !will_be_prefilled {
                    // Transitioning from prefilled: add empty colloscope entry
                    self.inner_data.colloscope.group_lists.insert(
                        *group_list_id,
                        colloscopes::ColloscopeGroupList::new_empty(),
                    );
                } else if !was_prefilled && !will_be_prefilled {
                    // Staying automatic: the colloscope entry persists, so the
                    // new exclusions must be compatible with the placed students
                    let collo_group_list = self
                        .inner_data
                        .colloscope
                        .group_lists
                        .get(group_list_id)
                        .expect("Non-prefilled group list should have colloscope entry");
                    if collo_group_list
                        .validate_against_params(
                            *group_list_id,
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
                if !self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .contains_key(period_id)
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

                let subject_map = self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .get_mut(period_id)
                    .expect("Period ID was just checked");

                let old_group_list_id = match group_list_id {
                    Some(id) => subject_map.insert(*subject_id, *id),
                    None => subject_map.remove(subject_id),
                };

                Ok(AnnotatedGroupListOp::AssignToSubject(
                    *period_id,
                    *subject_id,
                    old_group_list_id,
                ))
            }
        }
    }

    /// Used internally
    ///
    /// Apply settings operations
    fn apply_settings(
        &mut self,
        settings_op: &AnnotatedSettingsOp,
    ) -> std::result::Result<AnnotatedSettingsOp, SettingsError> {
        match settings_op {
            AnnotatedSettingsOp::Update(new_settings) => {
                self.inner_data.params.validate_settings(new_settings)?;
                let old_settings =
                    std::mem::replace(&mut self.inner_data.params.settings, new_settings.clone());
                Ok(AnnotatedSettingsOp::Update(old_settings))
            }
        }
    }

    /// Used internally
    ///
    /// Apply balancing operations
    fn apply_balancing(
        &mut self,
        balancing_op: &AnnotatedBalancingOp,
    ) -> std::result::Result<AnnotatedBalancingOp, BalancingError> {
        match balancing_op {
            AnnotatedBalancingOp::Update(new_balancing) => {
                self.inner_data.params.validate_balancing(new_balancing)?;
                let old_balancing =
                    std::mem::replace(&mut self.inner_data.params.balancing, new_balancing.clone());
                Ok(AnnotatedBalancingOp::Update(old_balancing))
            }
        }
    }

    /// Used internally
    ///
    /// Apply colloscope operations
    fn apply_colloscope(
        &mut self,
        colloscope_op: &AnnotatedColloscopeOp,
    ) -> std::result::Result<AnnotatedColloscopeOp, ColloscopeError> {
        match colloscope_op {
            AnnotatedColloscopeOp::UpdateGroupList(group_list_id, group_list) => {
                let Some(params_group_list) = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
                };

                group_list.validate_against_params(
                    *group_list_id,
                    &params_group_list.params,
                    &params_group_list.filling,
                    &self.inner_data.params.students,
                )?;

                // Prefilled group lists have a params entry but no colloscope
                // entry: the op must be rejected, not insert one.
                if !self
                    .inner_data
                    .colloscope
                    .group_lists
                    .contains_key(group_list_id)
                {
                    return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
                }

                let old_group_list = self
                    .inner_data
                    .colloscope
                    .group_lists
                    .insert(*group_list_id, group_list.clone())
                    .expect("Entry presence was checked above");

                Ok(AnnotatedColloscopeOp::UpdateGroupList(
                    *group_list_id,
                    old_group_list,
                ))
            }
            AnnotatedColloscopeOp::UpdateInterrogation(
                period_id,
                slot_id,
                week_in_period,
                new_interrogation,
            ) => {
                new_interrogation.validate_against_params(
                    *period_id,
                    *slot_id,
                    *week_in_period,
                    &self.inner_data.params,
                )?;

                let Some(period) = self.inner_data.colloscope.period_map.get_mut(period_id) else {
                    return Err(ColloscopeError::InvalidPeriodId(*period_id));
                };

                let Some(slot) = period.slot_map.get_mut(slot_id) else {
                    return Err(ColloscopeError::InvalidSlotId(*slot_id));
                };

                let Some(interrogation_opt) = slot.interrogations.get_mut(*week_in_period) else {
                    return Err(ColloscopeError::InvalidWeekNumberInPeriod(
                        *period_id,
                        *week_in_period,
                    ));
                };

                let Some(interrogation) = interrogation_opt else {
                    return Err(ColloscopeError::NoInterrogationOnWeek(
                        *period_id,
                        *slot_id,
                        *week_in_period,
                    ));
                };

                let old_interrogation = std::mem::replace(interrogation, new_interrogation.clone());

                Ok(AnnotatedColloscopeOp::UpdateInterrogation(
                    *period_id,
                    *slot_id,
                    *week_in_period,
                    old_interrogation,
                ))
            }
        }
    }

    /// Used internally
    ///
    /// Apply export configuration operations
    fn apply_export_config(
        &mut self,
        export_config_op: &AnnotatedExportConfigOp,
    ) -> std::result::Result<AnnotatedExportConfigOp, ExportConfigError> {
        let backward = match export_config_op {
            AnnotatedExportConfigOp::UpdateGlobalConfig(v) => {
                let old = std::mem::replace(&mut self.inner_data.export_config.global, v.clone());
                AnnotatedExportConfigOp::UpdateGlobalConfig(old)
            }
            AnnotatedExportConfigOp::UpdateColloscopeEnabled(v) => {
                let old =
                    std::mem::replace(&mut self.inner_data.export_config.colloscope_enabled, *v);
                AnnotatedExportConfigOp::UpdateColloscopeEnabled(old)
            }
            AnnotatedExportConfigOp::UpdateAllGroupsEnabled(v) => {
                let old =
                    std::mem::replace(&mut self.inner_data.export_config.all_groups_enabled, *v);
                AnnotatedExportConfigOp::UpdateAllGroupsEnabled(old)
            }
            AnnotatedExportConfigOp::UpdatePrefilledGroupsEnabled(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.prefilled_groups_enabled,
                    *v,
                );
                AnnotatedExportConfigOp::UpdatePrefilledGroupsEnabled(old)
            }
            AnnotatedExportConfigOp::UpdateAutomaticGroupsEnabled(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.automatic_groups_enabled,
                    *v,
                );
                AnnotatedExportConfigOp::UpdateAutomaticGroupsEnabled(old)
            }
            AnnotatedExportConfigOp::UpdatePerGroupListEnabled(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.per_group_list_enabled,
                    *v,
                );
                AnnotatedExportConfigOp::UpdatePerGroupListEnabled(old)
            }
            AnnotatedExportConfigOp::UpdateColloscopeConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.colloscope_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdateColloscopeConfig(old)
            }
            AnnotatedExportConfigOp::UpdateAllGroupsConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.all_groups_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdateAllGroupsConfig(old)
            }
            AnnotatedExportConfigOp::UpdatePrefilledGroupsConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.prefilled_groups_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdatePrefilledGroupsConfig(old)
            }
            AnnotatedExportConfigOp::UpdateAutomaticGroupsConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.automatic_groups_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdateAutomaticGroupsConfig(old)
            }
            AnnotatedExportConfigOp::UpdatePerGroupListConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.per_group_list_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdatePerGroupListConfig(old)
            }
        };
        Ok(backward)
    }
}
