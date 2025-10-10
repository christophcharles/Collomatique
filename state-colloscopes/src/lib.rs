//! Colloscopes state crate
//!
//! This crate implements the various concepts of [collomatique-state]
//! and the various traits for the specific case of colloscope representation.
//!

use serde::{Deserialize, Serialize};

use assignments::Assignments;
use collomatique_state::{tools, InMemoryData, Operation};
use colloscope_params::ColloscopeIdMaps;
use colloscope_params::Parameters;
use group_lists::GroupLists;
use incompats::Incompats;
use ops::AnnotatedSettingsOp;
use periods::Periods;
use rules::Rules;
use slots::Slots;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use students::Students;
use subjects::{Subjects, SubjectsExternalData};
use teachers::Teachers;
use week_patterns::WeekPatterns;

pub mod ids;
use ids::Id;
use ids::IdIssuer;
pub use ids::{
    ColloscopeGroupListId, ColloscopeId, ColloscopeIncompatId, ColloscopePeriodId,
    ColloscopeRuleId, ColloscopeSlotId, ColloscopeStudentId, ColloscopeSubjectId,
    ColloscopeTeacherId, ColloscopeWeekPatternId, GroupListId, IncompatId, PeriodId, RuleId,
    SlotId, StudentId, SubjectId, TeacherId, WeekPatternId,
};
pub mod ops;
use ops::{
    AnnotatedAssignmentOp, AnnotatedColloscopeOp, AnnotatedGroupListOp, AnnotatedIncompatOp,
    AnnotatedPeriodOp, AnnotatedRuleOp, AnnotatedSlotOp, AnnotatedStudentOp, AnnotatedSubjectOp,
    AnnotatedTeacherOp, AnnotatedWeekPatternOp,
};
pub use ops::{
    AnnotatedOp, AssignmentOp, ColloscopeOp, GroupListOp, IncompatOp, Op, PeriodOp, RuleOp,
    SettingsOp, SlotOp, StudentOp, SubjectOp, TeacherOp, WeekPatternOp,
};
pub use subjects::{
    Subject, SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
};

pub mod assignments;
pub mod colloscope_params;
pub mod colloscopes;
pub mod group_lists;
pub mod incompats;
pub mod periods;
pub mod rules;
pub mod settings;
pub mod slots;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerData {
    pub main_params: colloscope_params::Parameters<
        PeriodId,
        SubjectId,
        TeacherId,
        StudentId,
        WeekPatternId,
        SlotId,
        IncompatId,
        GroupListId,
        RuleId,
    >,
    pub colloscopes: colloscopes::Colloscopes,
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
pub enum StudentError<StudentId: Id, PeriodId: Id, SubjectId: Id, GroupListId: Id> {
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

    /// Student is referenced in a colloscope id map
    #[error(
        "student id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed"
    )]
    StudentIsReferencedInColloscopeIdMaps(StudentId, ColloscopeId),
}

/// Errors for periods operations
///
/// These errors can be returned when trying to modify [Data] with a period op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PeriodError<PeriodId: Id, SubjectId: Id, StudentId: Id, RuleId: Id> {
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

    /// The period is referenced by a rule
    #[error("period id ({0:?}) is referenced by rule {1:?}")]
    PeriodIsReferencedByRule(PeriodId, RuleId),

    /// Period is referenced in a colloscope id map
    #[error("period id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed")]
    PeriodIsReferencedInColloscopeIdMaps(PeriodId, ColloscopeId),
}

/// Errors for subject operations
///
/// These errors can be returned when trying to modify [Data] with a subject op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SubjectError<SubjectId: Id, PeriodId: Id, TeacherId: Id, IncompatId: Id, GroupListId: Id> {
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

    /// Subject is referenced in a colloscope id map
    #[error(
        "subject id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed"
    )]
    SubjectIsReferencedInColloscopeIdMaps(SubjectId, ColloscopeId),
}

/// Errors for teacher operations
///
/// These errors can be returned when trying to modify [Data] with a teacher op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum TeacherError<TeacherId: Id, SubjectId: Id, SlotId: Id> {
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

    /// Teacher is referenced in a colloscope id map
    #[error(
        "teacher id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed"
    )]
    TeacherIsReferencedInColloscopeIdMaps(TeacherId, ColloscopeId),
}

/// Errors for assignment operations
///
/// These errors can be returned when trying to modify [Data] with a assignment op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentError<PeriodId: Id, SubjectId: Id, StudentId: Id> {
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
pub enum WeekPatternError<WeekPatternId: Id, SlotId: Id, IncompatId: Id> {
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

    /// Week pattern is referenced in a colloscope id map
    #[error(
        "week pattern id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed"
    )]
    WeekPatternIsReferencedInColloscopeIdMaps(WeekPatternId, ColloscopeId),
}

/// Errors for interrogation slot operations
///
/// These errors can be returned when trying to modify [Data] with a slot op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotError<SlotId: Id, SubjectId: Id, TeacherId: Id, WeekPatternId: Id, RuleId: Id> {
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

    /// The slot is referenced by a rule
    #[error("Slot id ({0:?}) is referenced by rule {1:?}")]
    SlotIsReferencedByRule(SlotId, RuleId),

    /// Slot is referenced in a colloscope id map
    #[error("slot id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed")]
    SlotIsReferencedInColloscopeIdMaps(SlotId, ColloscopeId),
}

/// Errors for schedule incompatibility operations
///
/// These errors can be returned when trying to modify [Data] with an incompat op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum IncompatError<IncompatId: Id, SubjectId: Id, WeekPatternId: Id> {
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

    /// Incompat is referenced in a colloscope id map
    #[error(
        "incompat id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed"
    )]
    IncompatIsReferencedInColloscopeIdMaps(IncompatId, ColloscopeId),
}

/// Errors for group list operations
///
/// These errors can be returned when trying to modify [Data] with a group list op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GroupListError<GroupListId: Id, StudentId: Id, SubjectId: Id, PeriodId: Id> {
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

    /// empty group count range
    #[error("group_count range is empty")]
    GroupCountRangeIsEmpty,

    /// students per group range is empty
    #[error("students_per_group range is empty")]
    StudentsPerGroupRangeIsEmpty,

    /// student is both excluded and associated to a group
    #[error("Student id {0:?} is both excluded and included in prefilled groups")]
    StudentBothIncludedAndExcluded(StudentId),

    /// cannot remove group list as there are still prefilled groups
    #[error("Group list still has prefilled groups and cannot be removed")]
    RemainingPrefilledGroups,

    /// students appear multiple times in prefilled groups
    #[error("Some students appear multiple times in prefilled groups")]
    DuplicatedStudentInPrefilledGroups,

    /// cannot remove group list as there are still associated subjects
    #[error("Group list still is associated to subjects and cannot be removed")]
    RemainingAssociatedSubjects,

    /// GroupList is referenced in a colloscope id map
    #[error(
        "group list id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed"
    )]
    GroupListIsReferencedInColloscopeIdMaps(GroupListId, ColloscopeId),
}

/// Errors for rules operations
///
/// These errors can be returned when trying to modify [Data] with a rule op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RuleError<RuleId: Id, PeriodId: Id, SlotId: Id> {
    /// rule id is invalid
    #[error("invalid rule id ({0:?})")]
    InvalidRuleId(RuleId),

    /// The rule id already exists
    #[error("rule id ({0:?}) already exists")]
    RuleIdAlreadyExists(RuleId),

    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// slot id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// Rule is referenced in a colloscope id map
    #[error("rule id {0:?} is referenced in a colloscope ({1:?}) id maps and cannot be removed")]
    RuleIsReferencedInColloscopeIdMaps(RuleId, ColloscopeId),
}

/// Errors for colloscopes operations
///
/// These errors can be returned when trying to modify [Data] with a colloscope op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ColloscopeError {
    /// an internal id already exists in another colloscope
    #[error("duplicate internal id with respect to another colloscope")]
    DuplicateInternalId(u64),

    /// an internal id already exists in global parameters
    #[error("duplicate internal id with respect to global parameters")]
    InternalIdAlreadyInMainParams(u64),

    /// colloscope id is invalid
    #[error("invalid colloscope id ({0:?})")]
    InvalidColloscopeId(ColloscopeId),

    /// The colloscope id already exists
    #[error("colloscope id ({0:?}) already exists")]
    ColloscopeIdAlreadyExists(ColloscopeId),

    /// Student original id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// Period original id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Subject original id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// Teacher original id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// Week pattern original id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// Slot original id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// Incompat original id is invalid
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(IncompatId),

    /// Group list original id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    /// Rule original id is invalid
    #[error("invalid rule id ({0:?})")]
    InvalidRuleId(RuleId),

    /// Student colloscope id is invalid
    #[error("invalid colloscope student id ({0:?})")]
    InvalidColloscopeStudentId(ColloscopeStudentId),

    /// Period colloscope id is invalid
    #[error("invalid colloscope period id ({0:?})")]
    InvalidColloscopePeriodId(ColloscopePeriodId),

    /// Subject colloscope id is invalid
    #[error("invalid colloscope subject id ({0:?})")]
    InvalidColloscopeSubjectId(ColloscopeSubjectId),

    /// Teacher colloscope id is invalid
    #[error("invalid colloscope teacher id ({0:?})")]
    InvalidColloscopeTeacherId(ColloscopeTeacherId),

    /// Week pattern colloscope id is invalid
    #[error("invalid colloscope week pattern id ({0:?})")]
    InvalidColloscopeWeekPatternId(ColloscopeWeekPatternId),

    /// Slot colloscope id is invalid
    #[error("invalid colloscope slot id ({0:?})")]
    InvalidColloscopeSlotId(ColloscopeSlotId),

    /// Incompat colloscope id is invalid
    #[error("invalid colloscope incompat id ({0:?})")]
    InvalidColloscopeIncompatId(ColloscopeIncompatId),

    /// Group list colloscope id is invalid
    #[error("invalid colloscope group list id ({0:?})")]
    InvalidColloscopeGroupListId(ColloscopeGroupListId),

    /// Rule colloscope id is invalid
    #[error("invalid colloscope rule id ({0:?})")]
    InvalidColloscopeRuleId(ColloscopeRuleId),

    #[error(transparent)]
    InvariantErrorInParameters(#[from] InvariantError),

    #[error("Wrong period count")]
    WrongPeriodCountInColloscopeData,

    #[error("Wrong group list count")]
    WrongGroupListCountInColloscopeData,

    #[error("Wrong subject count in period")]
    WrongSubjectCountInPeriodInColloscopeData(ColloscopePeriodId),

    #[error("Wrong slot count for subject in period")]
    WrongSlotCountForSubjectInPeriodInColloscopeData(ColloscopePeriodId, ColloscopeSubjectId),

    #[error("Wrong interrogation count for slot in period")]
    WrongInterrogationCountForSlotInPeriodInColloscopeData(ColloscopePeriodId, ColloscopeSlotId),

    #[error("Interrogation on non-interrogation week")]
    InterrogationOnNonInterrogationWeek(ColloscopePeriodId, ColloscopeSlotId, usize),

    #[error("Missing interrogation on interrogation week")]
    MissingInterrogationOnInterrogationWeek(ColloscopePeriodId, ColloscopeSlotId, usize),

    #[error("Invalid group number in interrogation")]
    InvalidGroupNumInInterrogation(ColloscopePeriodId, ColloscopeSlotId, usize),

    #[error("excluded student in group list")]
    ExcludedStudentInGroupList(ColloscopeGroupListId, ColloscopeStudentId),

    #[error("wrong student count in group list")]
    WrongStudentCountInGroupList(ColloscopeGroupListId),

    #[error("Invalid group number for student")]
    InvalidGroupNumForStudentInGroupList(ColloscopeGroupListId, ColloscopeStudentId),
}

/// Errors for colloscopes modification
///
/// These errors can be returned when trying to modify [Data].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error(transparent)]
    Student(#[from] StudentError<StudentId, PeriodId, SubjectId, GroupListId>),
    #[error(transparent)]
    Period(#[from] PeriodError<PeriodId, SubjectId, StudentId, RuleId>),
    #[error(transparent)]
    Subject(#[from] SubjectError<SubjectId, PeriodId, TeacherId, IncompatId, GroupListId>),
    #[error(transparent)]
    Teacher(#[from] TeacherError<TeacherId, SubjectId, SlotId>),
    #[error(transparent)]
    Assignment(#[from] AssignmentError<PeriodId, SubjectId, StudentId>),
    #[error(transparent)]
    WeekPattern(#[from] WeekPatternError<WeekPatternId, SlotId, IncompatId>),
    #[error(transparent)]
    Slot(#[from] SlotError<SlotId, SubjectId, TeacherId, WeekPatternId, RuleId>),
    #[error(transparent)]
    Incompat(#[from] IncompatError<IncompatId, SubjectId, WeekPatternId>),
    #[error(transparent)]
    GroupList(#[from] GroupListError<GroupListId, StudentId, SubjectId, PeriodId>),
    #[error(transparent)]
    Rule(#[from] RuleError<RuleId, PeriodId, SlotId>),
    #[error(transparent)]
    Colloscope(#[from] ColloscopeError),
}

/// Errors for IDs
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FromDataError {
    #[error(transparent)]
    IdError(#[from] tools::IdError),
    #[error("Inconsistent assignments")]
    InconsistentAssignments,
    #[error("Error in slots data")]
    InconsistentSlots,
    #[error("Inconsistent group lists")]
    InconsistentGroupLists,
    #[error("Inconsistent rules")]
    InconsistentRules,
}

/// Potential new id returned by annotation
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewId {
    StudentId(StudentId),
    PeriodId(PeriodId),
    SubjectId(SubjectId),
    TeacherId(TeacherId),
    WeekPatternId(WeekPatternId),
    SlotId(SlotId),
    IncompatId(IncompatId),
    GroupListId(GroupListId),
    RuleId(RuleId),
    ColloscopeId(ColloscopeId),
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

impl From<RuleId> for NewId {
    fn from(value: RuleId) -> Self {
        NewId::RuleId(value)
    }
}

impl From<ColloscopeId> for NewId {
    fn from(value: ColloscopeId) -> Self {
        NewId::ColloscopeId(value)
    }
}

/// Errors for students operations
///
/// These errors can be returned when trying to modify [Data] with a student op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
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
    #[error("invalid rule")]
    InvalidRule,
}

impl InMemoryData for Data {
    type OriginalOperation = Op;
    type AnnotatedOperation = AnnotatedOp;
    type NewInfo = Option<NewId>;
    type Error = Error;

    fn annotate(&self, op: Op) -> (AnnotatedOp, Option<NewId>) {
        let mut guard = self.id_issuer.lock().unwrap();
        AnnotatedOp::annotate(op, &mut *guard)
    }

    fn build_rev_with_current_state(
        &self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error> {
        match op {
            AnnotatedOp::Student(student_op) => {
                Ok(AnnotatedOp::Student(self.build_rev_student(student_op)?))
            }
            AnnotatedOp::Period(period_op) => {
                Ok(AnnotatedOp::Period(self.build_rev_period(period_op)?))
            }
            AnnotatedOp::Subject(subject_op) => {
                Ok(AnnotatedOp::Subject(self.build_rev_subject(subject_op)?))
            }
            AnnotatedOp::Teacher(teacher_op) => {
                Ok(AnnotatedOp::Teacher(self.build_rev_teacher(teacher_op)?))
            }
            AnnotatedOp::Assignment(assignment_op) => Ok(AnnotatedOp::Assignment(
                self.build_rev_assignment(assignment_op)?,
            )),
            AnnotatedOp::WeekPattern(week_pattern_op) => Ok(AnnotatedOp::WeekPattern(
                self.build_rev_week_pattern(week_pattern_op)?,
            )),
            AnnotatedOp::Slot(slot_op) => Ok(AnnotatedOp::Slot(self.build_rev_slot(slot_op)?)),
            AnnotatedOp::Incompat(incompat_op) => {
                Ok(AnnotatedOp::Incompat(self.build_rev_incompat(incompat_op)?))
            }
            AnnotatedOp::GroupList(group_list_op) => Ok(AnnotatedOp::GroupList(
                self.build_rev_group_list(group_list_op)?,
            )),
            AnnotatedOp::Rule(rule_op) => Ok(AnnotatedOp::Rule(self.build_rev_rule(rule_op)?)),
            AnnotatedOp::Settings(settings_op) => {
                Ok(AnnotatedOp::Settings(self.build_rev_settings(settings_op)))
            }
            AnnotatedOp::Colloscopes(colloscope_op) => Ok(AnnotatedOp::Colloscopes(
                self.build_rev_colloscope(colloscope_op)?,
            )),
        }
    }

    fn apply(&mut self, op: &Self::AnnotatedOperation) -> std::result::Result<(), Self::Error> {
        match op {
            AnnotatedOp::Student(student_op) => self.apply_student(student_op)?,
            AnnotatedOp::Period(period_op) => self.apply_period(period_op)?,
            AnnotatedOp::Subject(subject_op) => self.apply_subject(subject_op)?,
            AnnotatedOp::Teacher(teacher_op) => self.apply_teacher(teacher_op)?,
            AnnotatedOp::Assignment(assignment_op) => self.apply_assignment(assignment_op)?,
            AnnotatedOp::WeekPattern(week_pattern_op) => {
                self.apply_week_pattern(week_pattern_op)?
            }
            AnnotatedOp::Slot(slot_op) => self.apply_slot(slot_op)?,
            AnnotatedOp::Incompat(incompat_op) => self.apply_incompat(incompat_op)?,
            AnnotatedOp::GroupList(group_list_op) => self.apply_group_list(group_list_op)?,
            AnnotatedOp::Rule(rule_op) => self.apply_rule(rule_op)?,
            AnnotatedOp::Settings(settings_op) => self.apply_settings(settings_op),
            AnnotatedOp::Colloscopes(colloscope_op) => self.apply_colloscope(colloscope_op)?,
        }
        self.check_invariants();
        Ok(())
    }
}

impl Data {
    fn ids(&self) -> impl Iterator<Item = u64> {
        self.inner_data.main_params.ids().chain(
            self.inner_data
                .colloscopes
                .colloscope_map
                .iter()
                .flat_map(|(colloscope_id, _colloscope)| [colloscope_id.inner()].into_iter()),
        )
    }

    /// USED INTERNALLY
    ///
    /// Checks that there are no duplicate ids in data
    ///
    /// Even ids for different type of data should be different
    fn check_no_duplicate_ids(&self) {
        let mut ids_so_far = BTreeSet::new();

        for id in self.ids() {
            assert!(ids_so_far.insert(id));
        }
    }

    /// USED INTERNALLY
    ///
    /// Checks all the invariants of data
    fn check_invariants(&self) {
        self.check_no_duplicate_ids();

        self.inner_data
            .main_params
            .check_invariants()
            .expect("Invariant error in main params");
        for (_colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
            colloscope
                .check_invariants(&self.inner_data.main_params)
                .expect("Invariant error in colloscope");
        }
    }
}

impl Data {
    /// Create a new [Data]
    ///
    /// This [Data] is basically empty and corresponds to the
    /// state of a new file
    pub fn new() -> Data {
        Self::from_data(colloscope_params::ColloscopeParametersExternalData::default())
            .expect("Default data should be valid")
    }

    /// Create a new [Data] from existing data
    ///
    /// This will check the consistency of the data
    /// and will also do some internal checks, so this might fail.
    pub fn from_data(
        main_params: colloscope_params::ColloscopeParametersExternalData,
    ) -> Result<Data, FromDataError> {
        let id_issuer = IdIssuer::new(main_params.ids())?;

        main_params.validate()?;

        // Ids have been validated
        let main_params = unsafe { colloscope_params::Parameters::from_external_data(main_params) };
        let colloscopes = colloscopes::Colloscopes::default();

        let data = Data {
            id_issuer: std::sync::Mutex::new(id_issuer),
            inner_data: InnerData {
                main_params,
                colloscopes,
            },
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

    pub fn copy_main_params(
        &self,
    ) -> (
        Parameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
        ColloscopeIdMaps<
            PeriodId,
            SubjectId,
            TeacherId,
            StudentId,
            WeekPatternId,
            SlotId,
            IncompatId,
            GroupListId,
            RuleId,
        >,
    ) {
        let mut guard = self.id_issuer.lock().unwrap();
        self.inner_data.main_params.duplicate(&mut *guard)
    }

    /// Used internally
    ///
    /// Apply student operations
    fn apply_student(
        &mut self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<(), StudentError<StudentId, PeriodId, SubjectId, GroupListId>> {
        match student_op {
            AnnotatedStudentOp::Add(new_id, student) => {
                if self
                    .inner_data
                    .main_params
                    .students
                    .student_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(StudentError::StudentIdAlreadyExists(*new_id));
                }
                self.inner_data.main_params.validate_student(student)?;

                self.inner_data
                    .main_params
                    .students
                    .student_map
                    .insert(*new_id, student.clone());

                Ok(())
            }
            AnnotatedStudentOp::Remove(id) => {
                let Some(current_student) =
                    self.inner_data.main_params.students.student_map.get(id)
                else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.students.contains_key(id) {
                        return Err(StudentError::StudentIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                for (group_list_id, group_list) in
                    &self.inner_data.main_params.group_lists.group_list_map
                {
                    if group_list.params.excluded_students.contains(id) {
                        return Err(StudentError::StudentIsStillExcludedByGroupList(
                            *id,
                            *group_list_id,
                        ));
                    }
                    if group_list.prefilled_groups.contains_student(*id) {
                        return Err(StudentError::StudentIsStillReferencedByPrefilledGroupList(
                            *id,
                            *group_list_id,
                        ));
                    }
                }

                for (period_id, period_assignments) in
                    &self.inner_data.main_params.assignments.period_map
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

                self.inner_data.main_params.students.student_map.remove(id);

                Ok(())
            }
            AnnotatedStudentOp::Update(id, new_student) => {
                self.inner_data.main_params.validate_student(new_student)?;
                let Some(current_student) =
                    self.inner_data.main_params.students.student_map.get_mut(id)
                else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (period_id, period_assignments) in
                    &self.inner_data.main_params.assignments.period_map
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

                *current_student = new_student.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply period operations
    fn apply_period(
        &mut self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<(), PeriodError<PeriodId, SubjectId, StudentId, RuleId>> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(new_date) => {
                self.inner_data.main_params.periods.first_week = new_date.clone();
                Ok(())
            }
            AnnotatedPeriodOp::AddFront(period_id, desc) => {
                if self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                self.inner_data
                    .main_params
                    .periods
                    .ordered_period_list
                    .insert(0, (*period_id, desc.clone()));
                self.inner_data.main_params.assignments.period_map.insert(
                    *period_id,
                    assignments::PeriodAssignments {
                        subject_map: self
                            .inner_data
                            .main_params
                            .subjects
                            .ordered_subject_list
                            .iter()
                            .map(|(subject_id, _subject)| (*subject_id, BTreeSet::new()))
                            .collect(),
                    },
                );
                self.inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                    .insert(*period_id, BTreeMap::new());
                Ok(())
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id, desc) => {
                if self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                let Some(position) = self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*after_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*after_id));
                };

                self.inner_data
                    .main_params
                    .periods
                    .ordered_period_list
                    .insert(position + 1, (*period_id, desc.clone()));
                self.inner_data.main_params.assignments.period_map.insert(
                    *period_id,
                    assignments::PeriodAssignments {
                        subject_map: self
                            .inner_data
                            .main_params
                            .subjects
                            .ordered_subject_list
                            .iter()
                            .map(|(subject_id, _subject)| (*subject_id, BTreeSet::new()))
                            .collect(),
                    },
                );
                self.inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                    .insert(*period_id, BTreeMap::new());
                Ok(())
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.periods.contains_key(period_id) {
                        return Err(PeriodError::PeriodIsReferencedInColloscopeIdMaps(
                            *period_id,
                            *colloscope_id,
                        ));
                    }
                }

                for (subject_id, subject) in
                    &self.inner_data.main_params.subjects.ordered_subject_list
                {
                    if subject.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySubject(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                for (rule_id, rule) in &self.inner_data.main_params.rules.rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByRule(*period_id, *rule_id));
                    }
                }

                for (student_id, student) in &self.inner_data.main_params.students.student_map {
                    if student.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByStudent(
                            *period_id,
                            *student_id,
                        ));
                    }
                }

                let period_assignments = self
                    .inner_data
                    .main_params
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
                    .main_params
                    .group_lists
                    .subjects_associations
                    .get(period_id)
                    .expect("Period id should be valid at this point");
                if !subject_map.is_empty() {
                    return Err(PeriodError::PeriodStillHasNonTrivialGroupListAssociation(
                        *period_id,
                    ));
                }

                self.inner_data
                    .main_params
                    .periods
                    .ordered_period_list
                    .remove(position);
                self.inner_data
                    .main_params
                    .assignments
                    .period_map
                    .remove(period_id);
                self.inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                    .remove(period_id);

                Ok(())
            }
            AnnotatedPeriodOp::Update(period_id, desc) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                self.inner_data.main_params.periods.ordered_period_list[position].1 = desc.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply period operations
    fn apply_subject(
        &mut self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<
        (),
        SubjectError<SubjectId, PeriodId, TeacherId, IncompatId, GroupListId>,
    > {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, params) => {
                if self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectError::SubjectIdAlreadyExists(*new_id));
                }
                self.inner_data.main_params.validate_subject(params)?;

                let position = match after_id {
                    Some(id) => {
                        self.inner_data
                            .main_params
                            .subjects
                            .find_subject_position(*id)
                            .ok_or(SubjectError::InvalidSubjectId(*id))?
                            + 1
                    }
                    None => 0,
                };

                self.inner_data
                    .main_params
                    .subjects
                    .ordered_subject_list
                    .insert(position, (*new_id, params.clone()));
                if params.parameters.interrogation_parameters.is_some() {
                    self.inner_data.main_params.slots.subject_map.insert(
                        *new_id,
                        slots::SubjectSlots {
                            ordered_slots: vec![],
                        },
                    );
                }
                for (period_id, _period) in &self.inner_data.main_params.periods.ordered_period_list
                {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .main_params
                        .assignments
                        .period_map
                        .get_mut(period_id)
                        .expect("Every period should appear in assignments");

                    period_assignment
                        .subject_map
                        .insert(*new_id, BTreeSet::new());
                }

                Ok(())
            }
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                if *new_pos
                    >= self
                        .inner_data
                        .main_params
                        .subjects
                        .ordered_subject_list
                        .len()
                {
                    return Err(SubjectError::PositionOutOfBounds(
                        *new_pos,
                        self.inner_data
                            .main_params
                            .subjects
                            .ordered_subject_list
                            .len(),
                    ));
                }
                let Some(old_pos) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let data = self
                    .inner_data
                    .main_params
                    .subjects
                    .ordered_subject_list
                    .remove(old_pos);
                self.inner_data
                    .main_params
                    .subjects
                    .ordered_subject_list
                    .insert(*new_pos, data);
                Ok(())
            }
            AnnotatedSubjectOp::Remove(id) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.subjects.contains_key(id) {
                        return Err(SubjectError::SubjectIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                for (period_id, subject_map) in &self
                    .inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                {
                    if let Some(group_list_id) = subject_map.get(id) {
                        return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                            *id,
                            *group_list_id,
                            *period_id,
                        ));
                    }
                }

                if let Some(subject_slots) = self.inner_data.main_params.slots.subject_map.get(id) {
                    if !subject_slots.ordered_slots.is_empty() {
                        return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                    }
                }

                for (teacher_id, teacher) in &self.inner_data.main_params.teachers.teacher_map {
                    if teacher.subjects.contains(id) {
                        return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                            *teacher_id,
                            *id,
                        ));
                    }
                }

                for (incompat_id, incompat) in &self.inner_data.main_params.incompats.incompat_map {
                    if incompat.subject_id == *id {
                        return Err(SubjectError::SubjectStillHasAssociatedIncompats(
                            *id,
                            *incompat_id,
                        ));
                    }
                }

                let params = &self.inner_data.main_params.subjects.ordered_subject_list[position].1;
                for (period_id, _period) in &self.inner_data.main_params.periods.ordered_period_list
                {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .main_params
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

                let (_, params) = self
                    .inner_data
                    .main_params
                    .subjects
                    .ordered_subject_list
                    .remove(position);
                self.inner_data.main_params.slots.subject_map.remove(id);
                for (period_id, _period) in &self.inner_data.main_params.periods.ordered_period_list
                {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .main_params
                        .assignments
                        .period_map
                        .get_mut(period_id)
                        .expect("Every period should appear in assignments");

                    period_assignment.subject_map.remove(id);
                }

                Ok(())
            }
            AnnotatedSubjectOp::Update(id, new_params) => {
                self.inner_data.main_params.validate_subject(new_params)?;
                let Some(position) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let old_params = self.inner_data.main_params.subjects.ordered_subject_list
                    [position]
                    .1
                    .clone();

                if old_params.parameters.interrogation_parameters.is_some()
                    && new_params.parameters.interrogation_parameters.is_none()
                {
                    // The new subject does not have interrogations, let's check that no teacher has been assigned to it
                    for (teacher_id, teacher) in &self.inner_data.main_params.teachers.teacher_map {
                        if teacher.subjects.contains(id) {
                            return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                                *teacher_id,
                                *id,
                            ));
                        }
                    }

                    // Also, we should not have a corresponding group list
                    for (period_id, subject_map) in &self
                        .inner_data
                        .main_params
                        .group_lists
                        .subjects_associations
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
                        .main_params
                        .slots
                        .subject_map
                        .get(id)
                        .expect("Subject should have a slot list at this point");

                    if !subject_slots.ordered_slots.is_empty() {
                        return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                    }
                }

                for (period_id, _period) in &self.inner_data.main_params.periods.ordered_period_list
                {
                    // If the period was excluded before, there is no structure to check
                    // and if the period is not excluded now, the structure will be fine anyway
                    if old_params.excluded_periods.contains(period_id)
                        || !new_params.excluded_periods.contains(period_id)
                    {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .main_params
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
                        .main_params
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
                }

                self.inner_data.main_params.subjects.ordered_subject_list[position].1 =
                    new_params.clone();
                if new_params.parameters.interrogation_parameters.is_some()
                    != old_params.parameters.interrogation_parameters.is_some()
                {
                    if new_params.parameters.interrogation_parameters.is_some() {
                        self.inner_data.main_params.slots.subject_map.insert(
                            *id,
                            slots::SubjectSlots {
                                ordered_slots: vec![],
                            },
                        );
                    } else {
                        self.inner_data.main_params.slots.subject_map.remove(id);
                    }
                }

                for (period_id, _period) in &self.inner_data.main_params.periods.ordered_period_list
                {
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
                            .main_params
                            .assignments
                            .period_map
                            .get_mut(period_id)
                            .expect("Every period should appear in assignments");

                        period_assignment.subject_map.insert(*id, BTreeSet::new());
                    } else {
                        // The period was included but will now be excluded
                        let period_assignment = self
                            .inner_data
                            .main_params
                            .assignments
                            .period_map
                            .get_mut(period_id)
                            .expect("Every period should appear in assignments");

                        period_assignment.subject_map.remove(id);
                    }
                }

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_teacher(
        &mut self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<(), TeacherError<TeacherId, SubjectId, SlotId>> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, teacher) => {
                if self
                    .inner_data
                    .main_params
                    .teachers
                    .teacher_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(TeacherError::TeacherIdAlreadyExists(*new_id));
                }
                self.inner_data.main_params.validate_teacher(teacher)?;

                self.inner_data
                    .main_params
                    .teachers
                    .teacher_map
                    .insert(*new_id, teacher.clone());

                Ok(())
            }
            AnnotatedTeacherOp::Remove(id) => {
                if !self
                    .inner_data
                    .main_params
                    .teachers
                    .teacher_map
                    .contains_key(id)
                {
                    return Err(TeacherError::InvalidTeacherId(*id));
                }

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.teachers.contains_key(id) {
                        return Err(TeacherError::TeacherIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                for (_subject_id, subject_slots) in &self.inner_data.main_params.slots.subject_map {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if *id == slot.teacher_id {
                            return Err(TeacherError::TeacherStillHasAssociatedSlots(
                                *id, *slot_id,
                            ));
                        }
                    }
                }

                self.inner_data.main_params.teachers.teacher_map.remove(id);

                Ok(())
            }
            AnnotatedTeacherOp::Update(id, new_teacher) => {
                self.inner_data.main_params.validate_teacher(new_teacher)?;
                let Some(current_teacher) =
                    self.inner_data.main_params.teachers.teacher_map.get_mut(id)
                else {
                    return Err(TeacherError::InvalidTeacherId(*id));
                };

                for (subject_id, subject_slots) in &self.inner_data.main_params.slots.subject_map {
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

                *current_teacher = new_teacher.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_assignment(
        &mut self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<(), AssignmentError<PeriodId, SubjectId, StudentId>> {
        match assignment_op {
            AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, status) => {
                let Some(period_assignments) = self
                    .inner_data
                    .main_params
                    .assignments
                    .period_map
                    .get_mut(period_id)
                else {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                };

                if self
                    .inner_data
                    .main_params
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

                let Some(student_desc) = self
                    .inner_data
                    .main_params
                    .students
                    .student_map
                    .get(student_id)
                else {
                    return Err(AssignmentError::InvalidStudentId(*student_id));
                };

                if student_desc.excluded_periods.contains(period_id) {
                    return Err(AssignmentError::StudentIsNotPresentOnPeriod(
                        *student_id,
                        *period_id,
                    ));
                }

                if *status {
                    assigned_students.insert(*student_id);
                } else {
                    assigned_students.remove(student_id);
                }

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_week_pattern(
        &mut self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<(), WeekPatternError<WeekPatternId, SlotId, IncompatId>> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, week_pattern) => {
                if self
                    .inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(WeekPatternError::WeekPatternIdAlreadyExists(*new_id));
                }

                self.inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .insert(*new_id, week_pattern.clone());

                Ok(())
            }
            AnnotatedWeekPatternOp::Remove(id) => {
                if !self
                    .inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .contains_key(id)
                {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                }

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.week_patterns.contains_key(id) {
                        return Err(WeekPatternError::WeekPatternIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                for (_subject_id, subject_slots) in &self.inner_data.main_params.slots.subject_map {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if let Some(week_pattern_id) = &slot.week_pattern {
                            if *id == *week_pattern_id {
                                return Err(WeekPatternError::WeekPatternStillHasAssociatedSlots(
                                    *id, *slot_id,
                                ));
                            }
                        }
                    }
                }

                for (incompat_id, incompat) in &self.inner_data.main_params.incompats.incompat_map {
                    if let Some(week_pattern_id) = &incompat.week_pattern_id {
                        if *id == *week_pattern_id {
                            return Err(WeekPatternError::WeekPatternStillHasAssociatedIncompat(
                                *id,
                                *incompat_id,
                            ));
                        }
                    }
                }

                self.inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .remove(id);

                Ok(())
            }
            AnnotatedWeekPatternOp::Update(id, new_week_pattern) => {
                let Some(current_week_pattern) = self
                    .inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                };

                *current_week_pattern = new_week_pattern.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply slot operations
    fn apply_slot(
        &mut self,
        slot_op: &AnnotatedSlotOp,
    ) -> std::result::Result<(), SlotError<SlotId, SubjectId, TeacherId, WeekPatternId, RuleId>>
    {
        match slot_op {
            AnnotatedSlotOp::AddAfter(new_id, subject_id, after_id, slot) => {
                if self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*new_id)
                    .is_some()
                {
                    return Err(SlotError::SlotIdAlreadyExists(*new_id));
                }
                self.inner_data
                    .main_params
                    .validate_slot(slot, *subject_id)?;

                let position = match after_id {
                    Some(id) => {
                        let (sub_id, after_pos) = self
                            .inner_data
                            .main_params
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
                    .main_params
                    .slots
                    .subject_map
                    .get_mut(subject_id)
                    .ok_or(SlotError::SubjectHasNoInterrogation(*subject_id))?;

                subject_slots
                    .ordered_slots
                    .insert(position, (*new_id, slot.clone()));

                Ok(())
            }
            AnnotatedSlotOp::ChangePosition(id, new_pos) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                let subject_slots = self
                    .inner_data
                    .main_params
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

                Ok(())
            }
            AnnotatedSlotOp::Remove(id) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.slots.contains_key(id) {
                        return Err(SlotError::SlotIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                for (rule_id, rule) in &self.inner_data.main_params.rules.rule_map {
                    if rule.desc.references_slot(*id) {
                        return Err(SlotError::SlotIsReferencedByRule(*id, *rule_id));
                    }
                }

                let subject_slots = self
                    .inner_data
                    .main_params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");

                subject_slots.ordered_slots.remove(old_pos);

                Ok(())
            }
            AnnotatedSlotOp::Update(slot_id, new_slot) => {
                let Some((subject_id, position)) = self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(*slot_id));
                };

                self.inner_data
                    .main_params
                    .validate_slot(new_slot, subject_id)?;

                let subject_slots = self
                    .inner_data
                    .main_params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");

                subject_slots.ordered_slots[position].1 = new_slot.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply incompat operations
    fn apply_incompat(
        &mut self,
        incompat_op: &AnnotatedIncompatOp,
    ) -> std::result::Result<(), IncompatError<IncompatId, SubjectId, WeekPatternId>> {
        match incompat_op {
            AnnotatedIncompatOp::Add(new_id, incompat) => {
                if self
                    .inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .contains_key(new_id)
                {
                    return Err(IncompatError::IncompatIdAlreadyExists(*new_id));
                }
                self.inner_data.main_params.validate_incompat(incompat)?;

                self.inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .insert(*new_id, incompat.clone());

                Ok(())
            }
            AnnotatedIncompatOp::Remove(id) => {
                if !self
                    .inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .contains_key(id)
                {
                    return Err(IncompatError::InvalidIncompatId(*id));
                }

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.incompats.contains_key(id) {
                        return Err(IncompatError::IncompatIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                self.inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .remove(id);

                Ok(())
            }
            AnnotatedIncompatOp::Update(incompat_id, new_incompat) => {
                self.inner_data
                    .main_params
                    .validate_incompat(new_incompat)?;

                let Some(incompat) = self
                    .inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .get_mut(incompat_id)
                else {
                    return Err(IncompatError::InvalidIncompatId(*incompat_id));
                };

                *incompat = new_incompat.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply group list operations
    fn apply_group_list(
        &mut self,
        group_list_op: &AnnotatedGroupListOp,
    ) -> std::result::Result<(), GroupListError<GroupListId, StudentId, SubjectId, PeriodId>> {
        match group_list_op {
            AnnotatedGroupListOp::Add(new_id, params) => {
                if self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .contains_key(new_id)
                {
                    return Err(GroupListError::GroupListIdAlreadyExists(*new_id));
                };
                let new_group_list = group_lists::GroupList {
                    params: params.clone(),
                    prefilled_groups: group_lists::GroupListPrefilledGroups::default(),
                };

                self.inner_data
                    .main_params
                    .validate_group_list(&new_group_list)?;

                self.inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .insert(*new_id, new_group_list);

                Ok(())
            }
            AnnotatedGroupListOp::Remove(id) => {
                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.group_lists.contains_key(id) {
                        return Err(GroupListError::GroupListIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                let Some(old_group_list) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .get(id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*id));
                };
                if !old_group_list.prefilled_groups.is_empty() {
                    return Err(GroupListError::RemainingPrefilledGroups);
                }

                for (_period_id, subject_map) in &self
                    .inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                {
                    for (_subject_id, group_list_id) in subject_map {
                        if *group_list_id == *id {
                            return Err(GroupListError::RemainingAssociatedSubjects);
                        }
                    }
                }

                self.inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .remove(id);

                Ok(())
            }
            AnnotatedGroupListOp::Update(group_list_id, new_params) => {
                let Some(old_group_list) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };
                let new_group_list = group_lists::GroupList {
                    params: new_params.clone(),
                    prefilled_groups: old_group_list.prefilled_groups.clone(),
                };

                self.inner_data
                    .main_params
                    .validate_group_list(&new_group_list)?;

                self.inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .insert(*group_list_id, new_group_list);

                Ok(())
            }
            AnnotatedGroupListOp::PreFill(group_list_id, prefilled_groups) => {
                let Some(old_group_list) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };
                let new_group_list = group_lists::GroupList {
                    params: old_group_list.params.clone(),
                    prefilled_groups: prefilled_groups.clone(),
                };

                self.inner_data
                    .main_params
                    .validate_group_list(&new_group_list)?;

                self.inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .insert(*group_list_id, new_group_list);

                Ok(())
            }
            AnnotatedGroupListOp::AssignToSubject(period_id, subject_id, group_list_id) => {
                let Some(subject) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject(*subject_id)
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
                let Some(subject_map) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                    .get_mut(period_id)
                else {
                    return Err(GroupListError::InvalidPeriodId(*period_id));
                };

                match group_list_id {
                    Some(id) => {
                        if !self
                            .inner_data
                            .main_params
                            .group_lists
                            .group_list_map
                            .contains_key(id)
                        {
                            return Err(GroupListError::InvalidGroupListId(*id));
                        };
                        subject_map.insert(*subject_id, *id);
                    }
                    None => {
                        subject_map.remove(subject_id);
                    }
                }

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply rule operations
    fn apply_rule(
        &mut self,
        rule_op: &AnnotatedRuleOp,
    ) -> std::result::Result<(), RuleError<RuleId, PeriodId, SlotId>> {
        match rule_op {
            AnnotatedRuleOp::Add(new_id, rule) => {
                if self
                    .inner_data
                    .main_params
                    .rules
                    .rule_map
                    .contains_key(new_id)
                {
                    return Err(RuleError::RuleIdAlreadyExists(*new_id));
                };

                self.inner_data.main_params.validate_rule(rule)?;

                self.inner_data
                    .main_params
                    .rules
                    .rule_map
                    .insert(*new_id, rule.clone());

                Ok(())
            }
            AnnotatedRuleOp::Remove(id) => {
                if !self.inner_data.main_params.rules.rule_map.contains_key(id) {
                    return Err(RuleError::InvalidRuleId(*id));
                }

                for (colloscope_id, colloscope) in &self.inner_data.colloscopes.colloscope_map {
                    if colloscope.id_maps.rules.contains_key(id) {
                        return Err(RuleError::RuleIsReferencedInColloscopeIdMaps(
                            *id,
                            *colloscope_id,
                        ));
                    }
                }

                self.inner_data.main_params.rules.rule_map.remove(id);

                Ok(())
            }
            AnnotatedRuleOp::Update(id, rule) => {
                if !self.inner_data.main_params.rules.rule_map.contains_key(id) {
                    return Err(RuleError::InvalidRuleId(*id));
                }

                self.inner_data.main_params.validate_rule(rule)?;

                self.inner_data
                    .main_params
                    .rules
                    .rule_map
                    .insert(*id, rule.clone());

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply settings operations
    fn apply_settings(&mut self, settings_op: &AnnotatedSettingsOp) {
        match settings_op {
            AnnotatedSettingsOp::Update(new_settings) => {
                self.inner_data.main_params.settings = new_settings.clone();
            }
        }
    }

    /// Used internally
    ///
    /// Apply colloscope operations
    fn apply_colloscope(
        &mut self,
        colloscope_op: &AnnotatedColloscopeOp,
    ) -> std::result::Result<(), ColloscopeError> {
        match colloscope_op {
            AnnotatedColloscopeOp::Add(new_id, colloscope) => {
                if self
                    .inner_data
                    .colloscopes
                    .colloscope_map
                    .contains_key(new_id)
                {
                    return Err(ColloscopeError::ColloscopeIdAlreadyExists(*new_id));
                };

                let ids: BTreeSet<_> = colloscope.params.ids().collect();

                for (_collo_id, collo) in &self.inner_data.colloscopes.colloscope_map {
                    for id in collo.params.ids() {
                        if ids.contains(&id) {
                            return Err(ColloscopeError::DuplicateInternalId(id));
                        }
                    }
                }

                for id in self.inner_data.main_params.ids() {
                    if ids.contains(&id) {
                        return Err(ColloscopeError::InternalIdAlreadyInMainParams(id));
                    }
                }

                colloscope.check_invariants(&self.inner_data.main_params)?;

                self.inner_data
                    .colloscopes
                    .colloscope_map
                    .insert(*new_id, colloscope.clone());

                Ok(())
            }
            AnnotatedColloscopeOp::Update(colloscope_id, colloscope) => {
                if !self
                    .inner_data
                    .colloscopes
                    .colloscope_map
                    .contains_key(colloscope_id)
                {
                    return Err(ColloscopeError::InvalidColloscopeId(*colloscope_id));
                }

                let ids: BTreeSet<_> = colloscope.params.ids().collect();

                for (collo_id, collo) in &self.inner_data.colloscopes.colloscope_map {
                    if collo_id == colloscope_id {
                        continue;
                    }
                    for id in collo.params.ids() {
                        if ids.contains(&id) {
                            return Err(ColloscopeError::DuplicateInternalId(id));
                        }
                    }
                }

                for id in self.inner_data.main_params.ids() {
                    if ids.contains(&id) {
                        return Err(ColloscopeError::InternalIdAlreadyInMainParams(id));
                    }
                }

                colloscope.check_invariants(&self.inner_data.main_params)?;

                self.inner_data
                    .colloscopes
                    .colloscope_map
                    .insert(*colloscope_id, colloscope.clone());

                Ok(())
            }
            AnnotatedColloscopeOp::Remove(id) => {
                if !self.inner_data.colloscopes.colloscope_map.contains_key(id) {
                    return Err(ColloscopeError::InvalidColloscopeId(*id));
                }

                self.inner_data.colloscopes.colloscope_map.remove(id);

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a student operation
    fn build_rev_student(
        &self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<
        AnnotatedStudentOp,
        StudentError<StudentId, PeriodId, SubjectId, GroupListId>,
    > {
        match student_op {
            AnnotatedStudentOp::Add(student_id, _student) => {
                if self
                    .inner_data
                    .main_params
                    .students
                    .student_map
                    .contains_key(student_id)
                {
                    return Err(StudentError::StudentIdAlreadyExists(student_id.clone()));
                }

                Ok(AnnotatedStudentOp::Remove(student_id.clone()))
            }
            AnnotatedStudentOp::Remove(student_id) => {
                let Some(old_student) = self
                    .inner_data
                    .main_params
                    .students
                    .student_map
                    .get(&student_id)
                    .cloned()
                else {
                    return Err(StudentError::InvalidStudentId(student_id.clone()));
                };

                Ok(AnnotatedStudentOp::Add(student_id.clone(), old_student))
            }
            AnnotatedStudentOp::Update(student_id, _student) => {
                let Some(old_student) = self
                    .inner_data
                    .main_params
                    .students
                    .student_map
                    .get(&student_id)
                    .cloned()
                else {
                    return Err(StudentError::InvalidStudentId(student_id.clone()));
                };

                Ok(AnnotatedStudentOp::Update(student_id.clone(), old_student))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a period operation
    fn build_rev_period(
        &self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<AnnotatedPeriodOp, PeriodError<PeriodId, SubjectId, StudentId, RuleId>>
    {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(_new_date) => {
                Ok(AnnotatedPeriodOp::ChangeStartDate(
                    self.inner_data.main_params.periods.first_week.clone(),
                ))
            }
            AnnotatedPeriodOp::AddFront(new_id, _desc) => {
                if self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*new_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(new_id.clone()));
                }

                Ok(AnnotatedPeriodOp::Remove(new_id.clone()))
            }
            AnnotatedPeriodOp::AddAfter(new_id, after_id, _desc) => {
                if self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*new_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(new_id.clone()));
                }

                let Some(_after_position) = self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*after_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(after_id.clone()));
                };

                Ok(AnnotatedPeriodOp::Remove(new_id.clone()))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(period_id.clone()));
                };

                let old_desc = self.inner_data.main_params.periods.ordered_period_list[position]
                    .1
                    .clone();

                Ok(if position == 0 {
                    AnnotatedPeriodOp::AddFront(period_id.clone(), old_desc)
                } else {
                    let previous_id =
                        self.inner_data.main_params.periods.ordered_period_list[position - 1].0;
                    AnnotatedPeriodOp::AddAfter(period_id.clone(), previous_id.clone(), old_desc)
                })
            }
            AnnotatedPeriodOp::Update(period_id, _desc) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(period_id.clone()));
                };

                let old_desc = self.inner_data.main_params.periods.ordered_period_list[position]
                    .1
                    .clone();

                Ok(AnnotatedPeriodOp::Update(period_id.clone(), old_desc))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a subject operation
    fn build_rev_subject(
        &self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<
        AnnotatedSubjectOp,
        SubjectError<SubjectId, PeriodId, TeacherId, IncompatId, GroupListId>,
    > {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, _params) => {
                if self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectError::SubjectIdAlreadyExists(new_id.clone()));
                }

                if let Some(id) = after_id {
                    if self
                        .inner_data
                        .main_params
                        .subjects
                        .find_subject_position(*id)
                        .is_none()
                    {
                        return Err(SubjectError::InvalidSubjectId(id.clone()));
                    }
                }

                Ok(AnnotatedSubjectOp::Remove(new_id.clone()))
            }
            AnnotatedSubjectOp::Remove(subject_id) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*subject_id)
                else {
                    return Err(SubjectError::InvalidSubjectId(subject_id.clone()));
                };

                let old_params = self.inner_data.main_params.subjects.ordered_subject_list
                    [position]
                    .1
                    .clone();

                Ok(AnnotatedSubjectOp::AddAfter(
                    *subject_id,
                    if position == 0 {
                        None
                    } else {
                        Some(
                            self.inner_data.main_params.subjects.ordered_subject_list[position - 1]
                                .0,
                        )
                    },
                    old_params.into(),
                ))
            }
            AnnotatedSubjectOp::Update(subject_id, _new_params) => {
                let Some(position) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*subject_id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*subject_id));
                };

                let old_params = self.inner_data.main_params.subjects.ordered_subject_list
                    [position]
                    .1
                    .clone();

                Ok(AnnotatedSubjectOp::Update(*subject_id, old_params.into()))
            }
            AnnotatedSubjectOp::ChangePosition(subject_id, _new_pos) => {
                let Some(old_pos) = self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*subject_id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*subject_id));
                };

                Ok(AnnotatedSubjectOp::ChangePosition(*subject_id, old_pos))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a teacher operation
    fn build_rev_teacher(
        &self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<AnnotatedTeacherOp, TeacherError<TeacherId, SubjectId, SlotId>> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, _teacher) => {
                if self
                    .inner_data
                    .main_params
                    .teachers
                    .teacher_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(TeacherError::TeacherIdAlreadyExists(new_id.clone()));
                }

                Ok(AnnotatedTeacherOp::Remove(new_id.clone()))
            }
            AnnotatedTeacherOp::Remove(teacher_id) => {
                let Some(old_teacher) = self
                    .inner_data
                    .main_params
                    .teachers
                    .teacher_map
                    .get(teacher_id)
                else {
                    return Err(TeacherError::InvalidTeacherId(teacher_id.clone()));
                };

                Ok(AnnotatedTeacherOp::Add(*teacher_id, old_teacher.clone()))
            }
            AnnotatedTeacherOp::Update(teacher_id, _new_teacher) => {
                let Some(old_teacher) = self
                    .inner_data
                    .main_params
                    .teachers
                    .teacher_map
                    .get(teacher_id)
                else {
                    return Err(TeacherError::InvalidTeacherId(teacher_id.clone()));
                };

                Ok(AnnotatedTeacherOp::Update(*teacher_id, old_teacher.clone()))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of an assignment operation
    fn build_rev_assignment(
        &self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<AnnotatedAssignmentOp, AssignmentError<PeriodId, SubjectId, StudentId>>
    {
        match assignment_op {
            AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, _status) => {
                let Some(period_assignments) = self
                    .inner_data
                    .main_params
                    .assignments
                    .period_map
                    .get(period_id)
                else {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                };

                if self
                    .inner_data
                    .main_params
                    .subjects
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AssignmentError::InvalidSubjectId(*subject_id));
                }

                let Some(assigned_students) = period_assignments.subject_map.get(subject_id) else {
                    return Err(AssignmentError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                };

                let Some(student_desc) = self
                    .inner_data
                    .main_params
                    .students
                    .student_map
                    .get(student_id)
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
    /// Builds reverse of a week pattern operation
    fn build_rev_week_pattern(
        &self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<
        AnnotatedWeekPatternOp,
        WeekPatternError<WeekPatternId, SlotId, IncompatId>,
    > {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, _week_pattern) => {
                if self
                    .inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(WeekPatternError::WeekPatternIdAlreadyExists(new_id.clone()));
                }

                Ok(AnnotatedWeekPatternOp::Remove(new_id.clone()))
            }
            AnnotatedWeekPatternOp::Remove(week_pattern_id) => {
                let Some(old_week_pattern) = self
                    .inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(
                        week_pattern_id.clone(),
                    ));
                };

                Ok(AnnotatedWeekPatternOp::Add(
                    *week_pattern_id,
                    old_week_pattern.clone(),
                ))
            }
            AnnotatedWeekPatternOp::Update(week_pattern_id, _new_week_pattern) => {
                let Some(old_week_pattern) = self
                    .inner_data
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(
                        week_pattern_id.clone(),
                    ));
                };

                Ok(AnnotatedWeekPatternOp::Update(
                    *week_pattern_id,
                    old_week_pattern.clone(),
                ))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a slot operation
    fn build_rev_slot(
        &self,
        slot_op: &AnnotatedSlotOp,
    ) -> std::result::Result<
        AnnotatedSlotOp,
        SlotError<SlotId, SubjectId, TeacherId, WeekPatternId, RuleId>,
    > {
        match slot_op {
            AnnotatedSlotOp::AddAfter(new_id, _subject_id, after_id, _slot) => {
                if self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*new_id)
                    .is_some()
                {
                    return Err(SlotError::SlotIdAlreadyExists(new_id.clone()));
                }

                if let Some(id) = after_id {
                    if self
                        .inner_data
                        .main_params
                        .slots
                        .find_slot_subject_and_position(*id)
                        .is_none()
                    {
                        return Err(SlotError::InvalidSlotId(id.clone()));
                    }
                }

                Ok(AnnotatedSlotOp::Remove(new_id.clone()))
            }
            AnnotatedSlotOp::Remove(slot_id) => {
                let Some((subject_id, position)) = self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(slot_id.clone()));
                };

                let subject_slots = self
                    .inner_data
                    .main_params
                    .slots
                    .subject_map
                    .get(&subject_id)
                    .expect("Subject id should be valid");

                let old_slot = subject_slots.ordered_slots[position].1.clone();

                let previous_id = if position == 0 {
                    None
                } else {
                    Some(subject_slots.ordered_slots[position - 1].0)
                };

                Ok(AnnotatedSlotOp::AddAfter(
                    *slot_id,
                    subject_id,
                    previous_id,
                    old_slot,
                ))
            }
            AnnotatedSlotOp::Update(slot_id, _new_slot) => {
                let Some((subject_id, position)) = self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(*slot_id));
                };

                let subject_slots = self
                    .inner_data
                    .main_params
                    .slots
                    .subject_map
                    .get(&subject_id)
                    .expect("Subject id should be valid");

                let old_slot = subject_slots.ordered_slots[position].1.clone();

                Ok(AnnotatedSlotOp::Update(*slot_id, old_slot))
            }
            AnnotatedSlotOp::ChangePosition(slot_id, _new_pos) => {
                let Some((_subject_id, old_pos)) = self
                    .inner_data
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(*slot_id));
                };

                Ok(AnnotatedSlotOp::ChangePosition(*slot_id, old_pos))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a schedule incompat operation
    fn build_rev_incompat(
        &self,
        incompat_op: &AnnotatedIncompatOp,
    ) -> std::result::Result<AnnotatedIncompatOp, IncompatError<IncompatId, SubjectId, WeekPatternId>>
    {
        match incompat_op {
            AnnotatedIncompatOp::Add(new_id, _incompat) => {
                Ok(AnnotatedIncompatOp::Remove(new_id.clone()))
            }
            AnnotatedIncompatOp::Remove(incompat_id) => {
                let Some(old_incompat) = self
                    .inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .get(incompat_id)
                else {
                    return Err(IncompatError::InvalidIncompatId(*incompat_id));
                };

                Ok(AnnotatedIncompatOp::Add(*incompat_id, old_incompat.clone()))
            }
            AnnotatedIncompatOp::Update(incompat_id, _new_incompat) => {
                let Some(old_incompat) = self
                    .inner_data
                    .main_params
                    .incompats
                    .incompat_map
                    .get(incompat_id)
                else {
                    return Err(IncompatError::InvalidIncompatId(*incompat_id));
                };

                Ok(AnnotatedIncompatOp::Update(
                    *incompat_id,
                    old_incompat.clone(),
                ))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a group list operation
    fn build_rev_group_list(
        &self,
        group_list_op: &AnnotatedGroupListOp,
    ) -> std::result::Result<
        AnnotatedGroupListOp,
        GroupListError<GroupListId, StudentId, SubjectId, PeriodId>,
    > {
        match group_list_op {
            AnnotatedGroupListOp::Add(new_id, _params) => {
                Ok(AnnotatedGroupListOp::Remove(new_id.clone()))
            }
            AnnotatedGroupListOp::Remove(group_list_id) => {
                let Some(old_group_list) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };

                if !old_group_list.prefilled_groups.is_empty() {
                    return Err(GroupListError::RemainingPrefilledGroups);
                }

                Ok(AnnotatedGroupListOp::Add(
                    *group_list_id,
                    old_group_list.params.clone(),
                ))
            }
            AnnotatedGroupListOp::Update(group_list_id, _new_params) => {
                let Some(old_group_list) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };

                Ok(AnnotatedGroupListOp::Update(
                    *group_list_id,
                    old_group_list.params.clone(),
                ))
            }
            AnnotatedGroupListOp::PreFill(group_list_id, _prefilled_groups) => {
                let Some(old_group_list) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(GroupListError::InvalidGroupListId(*group_list_id));
                };

                Ok(AnnotatedGroupListOp::PreFill(
                    *group_list_id,
                    old_group_list.prefilled_groups.clone(),
                ))
            }
            AnnotatedGroupListOp::AssignToSubject(period_id, subject_id, _group_list_id) => {
                let Some(subject_map) = self
                    .inner_data
                    .main_params
                    .group_lists
                    .subjects_associations
                    .get(period_id)
                else {
                    return Err(GroupListError::InvalidPeriodId(*period_id));
                };
                let old_group_list_id = subject_map.get(subject_id).cloned();
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
    /// Builds reverse of a rule operation
    fn build_rev_rule(
        &self,
        rule_op: &AnnotatedRuleOp,
    ) -> std::result::Result<AnnotatedRuleOp, RuleError<RuleId, PeriodId, SlotId>> {
        match rule_op {
            AnnotatedRuleOp::Add(new_id, _rule) => Ok(AnnotatedRuleOp::Remove(new_id.clone())),
            AnnotatedRuleOp::Remove(rule_id) => {
                let Some(old_rule) = self.inner_data.main_params.rules.rule_map.get(rule_id) else {
                    return Err(RuleError::InvalidRuleId(*rule_id));
                };

                Ok(AnnotatedRuleOp::Add(*rule_id, old_rule.clone()))
            }
            AnnotatedRuleOp::Update(rule_id, _new_rule) => {
                let Some(old_rule) = self.inner_data.main_params.rules.rule_map.get(rule_id) else {
                    return Err(RuleError::InvalidRuleId(*rule_id));
                };

                Ok(AnnotatedRuleOp::Update(*rule_id, old_rule.clone()))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a settings operation
    fn build_rev_settings(&self, settings_op: &AnnotatedSettingsOp) -> AnnotatedSettingsOp {
        match settings_op {
            AnnotatedSettingsOp::Update(_new_settings) => {
                let old_settings = self.inner_data.main_params.settings.clone();
                AnnotatedSettingsOp::Update(old_settings)
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a colloscope operation
    fn build_rev_colloscope(
        &self,
        colloscope_op: &AnnotatedColloscopeOp,
    ) -> std::result::Result<AnnotatedColloscopeOp, ColloscopeError> {
        match colloscope_op {
            AnnotatedColloscopeOp::Add(new_id, _name) => {
                Ok(AnnotatedColloscopeOp::Remove(new_id.clone()))
            }
            AnnotatedColloscopeOp::Update(colloscope_id, _colloscope) => {
                let Some(old_colloscope) = self
                    .inner_data
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return Err(ColloscopeError::InvalidColloscopeId(*colloscope_id));
                };

                Ok(AnnotatedColloscopeOp::Update(
                    *colloscope_id,
                    old_colloscope.clone(),
                ))
            }
            AnnotatedColloscopeOp::Remove(colloscope_id) => {
                let Some(old_colloscope) = self
                    .inner_data
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return Err(ColloscopeError::InvalidColloscopeId(*colloscope_id));
                };

                Ok(AnnotatedColloscopeOp::Add(
                    *colloscope_id,
                    old_colloscope.clone(),
                ))
            }
        }
    }
}
