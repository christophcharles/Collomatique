//! Ops module
//!
//! This modules defines all the operations (that means atomic modification)
//! we can do on colloscopes data
//!
//! The main type is [Op] which defines all possible modification operations
//! that can be done on the data.
//!
//! [AnnotatedOp] is the corresponding annotated type. See [collomatique_state::history]
//! for a full discussion of annotation.

use super::*;

/// Operation enumeration
///
/// This is the list of all possible operations on [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Operation on the student list
    Student(StudentOp),
    /// Operation on periods
    Period(PeriodOp),
    /// Operation on weeks
    Week(WeekOp),
    /// Operation on the subjects
    Subject(SubjectOp),
    /// Operation on the teachers
    Teacher(TeacherOp),
    /// Operation on assignments
    Assignment(AssignmentOp),
    /// Operation on week patterns
    WeekPattern(WeekPatternOp),
    /// Operation on slots
    Slot(SlotOp),
    /// Operation on incompatibilities
    Incompat(IncompatOp),
    /// Operation on group lists
    GroupList(GroupListOp),
    /// Operation on settings
    Settings(SettingsOp),
    /// Operation on pairings
    Pairing(PairingOp),
    /// Operation on slot pairings
    SlotPairing(SlotPairingOp),
    /// Operation on balancing
    Balancing(BalancingOp),
    /// Operation on colloscopes
    Colloscope(ColloscopeOp),
    /// Operation on export configuration
    ExportConfig(ExportConfigOp),
    /// Global update of all data at once
    GlobalUpdate(super::InnerData),
}

impl Operation for Op {}

/// Student operation enumeration
///
/// This is the list of all possible operations related to the
/// student list we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentOp {
    /// Add a new student
    Add(students::Student),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, students::Student),
}

/// Period operation enumeration
///
/// This is the list of all possible operations related to the
/// periods we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeriodOp {
    /// Set the start of periods on a specific week
    ChangeStartDate(Option<collomatique_time::WeekStart>),
    /// Add a new (empty) period at the beginning
    ///
    /// Periods are always created week-less; weeks are then spliced in with the
    /// [WeekOp] family. This is what makes `apply_week` the single writer of
    /// week data.
    AddFront,
    /// Add a new (empty) period after an existing period
    AddAfter(PeriodId),
    /// Remove an existing period
    ///
    /// The period must be week-empty (empty it first with [WeekOp::Remove]).
    Remove(PeriodId),
}

/// Week operation enumeration
///
/// This is the list of all possible operations related to individual weeks
/// (as opposed to whole periods) we can do on a [Data]. Weeks live inside
/// periods; these ops splice a single week in or out, edit it, or move it
/// (possibly to another period), carrying its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeekOp {
    /// Add a week at the front of a period
    AddFront(PeriodId, periods::WeekDesc),
    /// Add a week right after an existing week
    AddAfter(WeekId, periods::WeekDesc),
    /// Remove an existing week
    Remove(WeekId),
    /// Update the status/annotation of an existing week
    Update(WeekId, periods::WeekDesc),
    /// Move a week to a position (same or different period), preserving its id
    /// and its content. The position is interpreted after the week is
    /// detached from its current spot.
    Move(WeekId, PeriodId, usize),
}

/// Subject operation enumeration
///
/// This is the list of all possible operations related to the
/// subjects we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectOp {
    /// Add a subject after an existing subject
    /// If `None`, it is placed first
    AddAfter(Option<SubjectId>, subjects::Subject),
    /// Remove an existing subject
    Remove(SubjectId),
    /// Move a subject to another position in the list
    ChangePosition(SubjectId, usize),
    /// Update the parameters of an existing subject
    Update(SubjectId, subjects::Subject),
}

/// Teacher operation enumeration
///
/// This is the list of all possible operations related to the
/// teachers we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeacherOp {
    /// Add a teacher
    Add(teachers::Teacher),
    /// Remove an existing teacher
    Remove(TeacherId),
    /// Update the parameters of an existing teacher
    Update(TeacherId, teachers::Teacher),
}

/// Assignment operation enumeration
///
/// This is the list of all possible operations related to the
/// assignments of students we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentOp {
    /// Assign (or deassign) a student to a subject on a given period
    Assign(PeriodId, StudentId, SubjectId, bool),
}

/// Week pattern operation enumeration
///
/// This is the list of all possible operations related to
/// week patterns we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeekPatternOp {
    /// Add a week pattern
    Add(week_patterns::WeekPattern),
    /// Remove an existing week pattern
    Remove(WeekPatternId),
    /// Update the parameters of an existing week pattern
    Update(WeekPatternId, week_patterns::WeekPattern),
}

/// Slot operation enumeration
///
/// This is the list of all possible operations related to the
/// slots we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotOp {
    /// Add a slot after an existing slot
    /// If `None`, it is placed first
    ///
    /// The subject the slot belongs to is carried by the slot itself
    /// (`slot.subject_id`).
    AddAfter(Option<SlotId>, slots::Slot),
    /// Remove an existing slot
    Remove(SlotId),
    /// Move a subject to another position in the list
    ChangePosition(SlotId, usize),
    /// Update the parameters of an existing subject
    Update(SlotId, slots::Slot),
}

/// Incompat operation enumeration
///
/// This is the list of all possible operations related to the
/// incompatibilities we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompatOp {
    /// Add an incompatibility
    Add(incompats::Incompatibility),
    /// Remove an existing incompatibility
    Remove(IncompatId),
    /// Update an incompatibility
    Update(IncompatId, incompats::Incompatibility),
}

/// Group list operation enumeration
///
/// This is the list of all possible operations related to the
/// group lists we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupListOp {
    /// Add a group list
    Add(group_lists::GroupListParameters),
    /// Remove an existing group list
    Remove(GroupListId),
    /// Update a group list
    Update(GroupListId, group_lists::GroupListParameters),
    /// Set filling strategy for a group list
    SetFilling(GroupListId, group_lists::GroupListFilling),
    /// Assign a group list to a subject
    AssignToSubject(PeriodId, SubjectId, Option<GroupListId>),
}

/// Settings operation enumeration
///
/// This is the list of all possible operations related to the
/// settings we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOp {
    /// Update the settings
    Update(settings::Settings),
}

/// Pairing rule operation enumeration
///
/// This is the list of all possible operations related to the
/// pairing rules we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingOp {
    /// Add a pairing rule
    Add(pairings::PairingRule),
    /// Remove an existing pairing rule
    Remove(PairingRuleId),
    /// Update an existing pairing rule
    Update(PairingRuleId, pairings::PairingRule),
}

/// Slot pairing rule operation enumeration
///
/// This is the list of all possible operations related to the
/// slot pairing rules we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotPairingOp {
    /// Add a new slot pairing rule
    Add(slot_pairings::SlotPairingRule),
    /// Remove a slot pairing rule
    Remove(SlotPairingRuleId),
    /// Update an existing slot pairing rule
    Update(SlotPairingRuleId, slot_pairings::SlotPairingRule),
}

/// Balancing operation enumeration
///
/// This is the list of all possible operations related to the
/// balancing configuration we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BalancingOp {
    /// Update the balancing configuration
    Update(balancing::Balancing),
}

/// Colloscope operation enumeration
///
/// This is the list of all possible operations related to the
/// colloscopes we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColloscopeOp {
    /// Update a group list
    UpdateGroupList(GroupListId, colloscopes::ColloscopeGroupList),
    /// Update an interrogation
    UpdateInterrogation(
        PeriodId,
        SlotId,
        usize,
        colloscopes::ColloscopeInterrogation,
    ),
}

/// Export configuration operation enumeration
///
/// This is the list of all possible operations related to the
/// export configuration we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportConfigOp {
    UpdateGlobalConfig(export_config::GlobalConfig),
    UpdateColloscopeEnabled(bool),
    UpdateAllGroupsEnabled(bool),
    UpdatePrefilledGroupsEnabled(bool),
    UpdateAutomaticGroupsEnabled(bool),
    UpdatePerGroupListEnabled(bool),
    UpdateColloscopeConfig(export_config::ColloscopeConfig),
    UpdateAllGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdatePrefilledGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdateAutomaticGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdatePerGroupListConfig(export_config::PerGroupListConfig),
}

/// Annotated operation
///
/// Compared to [Op], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedOp {
    /// Operation on the student list
    Student(AnnotatedStudentOp),
    /// Operation on the periods
    Period(AnnotatedPeriodOp),
    /// Operation on the weeks
    Week(AnnotatedWeekOp),
    /// Operation on the subjects
    Subject(AnnotatedSubjectOp),
    /// Operation on the teachers
    Teacher(AnnotatedTeacherOp),
    /// Operation on assignments
    Assignment(AnnotatedAssignmentOp),
    /// Operation on week patterns
    WeekPattern(AnnotatedWeekPatternOp),
    /// Operation on slots
    Slot(AnnotatedSlotOp),
    /// Operation on slots
    Incompat(AnnotatedIncompatOp),
    /// Operation on group lists
    GroupList(AnnotatedGroupListOp),
    /// Operation on settings
    Settings(AnnotatedSettingsOp),
    /// Operation on pairings
    Pairing(AnnotatedPairingOp),
    /// Operation on slot pairings
    SlotPairing(AnnotatedSlotPairingOp),
    /// Operation on balancing
    Balancing(AnnotatedBalancingOp),
    /// Operation on colloscopes
    Colloscope(AnnotatedColloscopeOp),
    /// Operation on export configuration
    ExportConfig(AnnotatedExportConfigOp),
    /// Global update of all data at once
    GlobalUpdate(super::InnerData),
}

impl From<AnnotatedStudentOp> for AnnotatedOp {
    fn from(value: AnnotatedStudentOp) -> Self {
        AnnotatedOp::Student(value)
    }
}

impl From<AnnotatedPeriodOp> for AnnotatedOp {
    fn from(value: AnnotatedPeriodOp) -> Self {
        AnnotatedOp::Period(value)
    }
}

impl From<AnnotatedWeekOp> for AnnotatedOp {
    fn from(value: AnnotatedWeekOp) -> Self {
        AnnotatedOp::Week(value)
    }
}

impl From<AnnotatedSubjectOp> for AnnotatedOp {
    fn from(value: AnnotatedSubjectOp) -> Self {
        AnnotatedOp::Subject(value)
    }
}

impl From<AnnotatedTeacherOp> for AnnotatedOp {
    fn from(value: AnnotatedTeacherOp) -> Self {
        AnnotatedOp::Teacher(value)
    }
}

impl From<AnnotatedAssignmentOp> for AnnotatedOp {
    fn from(value: AnnotatedAssignmentOp) -> Self {
        AnnotatedOp::Assignment(value)
    }
}

impl From<AnnotatedWeekPatternOp> for AnnotatedOp {
    fn from(value: AnnotatedWeekPatternOp) -> Self {
        AnnotatedOp::WeekPattern(value)
    }
}

impl From<AnnotatedSlotOp> for AnnotatedOp {
    fn from(value: AnnotatedSlotOp) -> Self {
        AnnotatedOp::Slot(value)
    }
}

impl From<AnnotatedIncompatOp> for AnnotatedOp {
    fn from(value: AnnotatedIncompatOp) -> Self {
        AnnotatedOp::Incompat(value)
    }
}

impl From<AnnotatedGroupListOp> for AnnotatedOp {
    fn from(value: AnnotatedGroupListOp) -> Self {
        AnnotatedOp::GroupList(value)
    }
}

impl From<AnnotatedPairingOp> for AnnotatedOp {
    fn from(value: AnnotatedPairingOp) -> Self {
        AnnotatedOp::Pairing(value)
    }
}

impl From<AnnotatedSlotPairingOp> for AnnotatedOp {
    fn from(value: AnnotatedSlotPairingOp) -> Self {
        AnnotatedOp::SlotPairing(value)
    }
}

impl From<AnnotatedSettingsOp> for AnnotatedOp {
    fn from(value: AnnotatedSettingsOp) -> Self {
        AnnotatedOp::Settings(value)
    }
}

impl From<AnnotatedBalancingOp> for AnnotatedOp {
    fn from(value: AnnotatedBalancingOp) -> Self {
        AnnotatedOp::Balancing(value)
    }
}

impl From<AnnotatedColloscopeOp> for AnnotatedOp {
    fn from(value: AnnotatedColloscopeOp) -> Self {
        AnnotatedOp::Colloscope(value)
    }
}

impl From<AnnotatedExportConfigOp> for AnnotatedOp {
    fn from(value: AnnotatedExportConfigOp) -> Self {
        AnnotatedOp::ExportConfig(value)
    }
}

/// Student annotated operation enumeration
///
/// Compared to [StudentOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedStudentOp {
    /// Add a new student (with fixed id)
    Add(StudentId, students::Student),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, students::Student),
}

/// Period annotated operation enumeration
///
/// Compared to [PeriodOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedPeriodOp {
    /// Set the start of periods on a specific week
    ChangeStartDate(Option<collomatique_time::WeekStart>),
    /// Add a new (empty) period at the beginning
    ///
    /// The parameter is the period id for the new period.
    AddFront(PeriodId),
    /// Add a new (empty) period after an existing period
    ///
    /// The first parameter is the period id for the new period.
    AddAfter(PeriodId, PeriodId),
    /// Remove an existing period
    Remove(PeriodId),
}

/// Week annotated operation enumeration
///
/// Compared to [WeekOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedWeekOp {
    /// Add a week at the front of a period
    /// First parameter is the week id for the new week
    AddFront(WeekId, PeriodId, periods::WeekDesc),
    /// Add a week right after an existing week
    /// First parameter is the week id for the new week
    AddAfter(WeekId, WeekId, periods::WeekDesc),
    /// Remove an existing week
    Remove(WeekId),
    /// Update the status/annotation of an existing week
    Update(WeekId, periods::WeekDesc),
    /// Move a week to a position (same or different period), preserving its id
    /// and its content
    Move(WeekId, PeriodId, usize),
}

/// Subject annotated operation enumeration
///
/// Compared to [SubjectOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedSubjectOp {
    /// Add a period after an existing period
    /// First parameter is the period id for the new period
    /// If the second parameter is `None`, the subject is added at the first place
    AddAfter(SubjectId, Option<SubjectId>, subjects::Subject),
    /// Remove an existing subject
    Remove(SubjectId),
    /// Move a subject to another position in the list
    ChangePosition(SubjectId, usize),
    /// Update the parameters of an existing subject
    Update(SubjectId, subjects::Subject),
}

/// Teacher annotated operation enumeration
///
/// Compared to [TeacherOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedTeacherOp {
    /// Add a teacher
    /// First parameter is the teacher id for the new teacher
    Add(TeacherId, teachers::Teacher),
    /// Remove an existing teacher
    Remove(TeacherId),
    /// Update the parameters of an existing teacher
    Update(TeacherId, teachers::Teacher),
}

/// Assignment annotated operation enumeration
///
/// Compared to [AssignmentOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedAssignmentOp {
    /// Assign (or deassign) a student to a subject on a given period
    Assign(PeriodId, StudentId, SubjectId, bool),
}

/// Week pattern operation enumeration
///
/// Compared to [WeekPatternOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedWeekPatternOp {
    /// Add a week pattern
    /// First parameter is the week pattern id for the new week pattern
    Add(WeekPatternId, week_patterns::WeekPattern),
    /// Remove an existing week pattern
    Remove(WeekPatternId),
    /// Update the parameters of an existing week pattern
    Update(WeekPatternId, week_patterns::WeekPattern),
}

/// Slot operation enumeration
///
/// Compared to [SlotOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedSlotOp {
    /// Add a slot after an existing slot
    /// If `None`, it is placed first
    /// First parameter is the slot id for the new slot
    ///
    /// The subject the slot belongs to is carried by the slot itself
    /// (`slot.subject_id`).
    AddAfter(SlotId, Option<SlotId>, slots::Slot),
    /// Remove an existing slot
    Remove(SlotId),
    /// Move a subject to another position in the list
    ChangePosition(SlotId, usize),
    /// Update the parameters of an existing subject
    Update(SlotId, slots::Slot),
}

/// Incompat operation enumeration
///
/// Compared to [IncompatOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedIncompatOp {
    /// Add an incompatibility
    /// First parameter is the incompat id for the new incompatibility
    Add(IncompatId, incompats::Incompatibility),
    /// Remove an existing incompat
    Remove(IncompatId),
    /// Update an existing incompat
    Update(IncompatId, incompats::Incompatibility),
}

/// Group list operation enumeration
///
/// Compared to [GroupListOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedGroupListOp {
    /// Add a group list
    /// First parameter is the group list id for the new group list
    ///
    /// The filling is always default (automatic, no exclusion) when
    /// annotating a [GroupListOp::Add]; it only carries information when
    /// the op is the reverse of a [AnnotatedGroupListOp::Remove], so that
    /// undoing a removal restores the original filling
    Add(
        GroupListId,
        group_lists::GroupListParameters,
        group_lists::GroupListFilling,
    ),
    /// Remove an existing group list
    Remove(GroupListId),
    /// Update a group list
    Update(GroupListId, group_lists::GroupListParameters),
    /// Set filling strategy for a group list
    SetFilling(GroupListId, group_lists::GroupListFilling),
    /// Assign a group list to a subject
    AssignToSubject(PeriodId, SubjectId, Option<GroupListId>),
}

/// Pairing rule annotated operation enumeration
///
/// Compared to [PairingOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedPairingOp {
    /// Add a pairing rule
    /// First parameter is the pairing rule id for the new rule
    Add(PairingRuleId, pairings::PairingRule),
    /// Remove an existing pairing rule
    Remove(PairingRuleId),
    /// Update an existing pairing rule
    Update(PairingRuleId, pairings::PairingRule),
}

/// Slot pairing rule annotated operation enumeration
///
/// Compared to [SlotPairingOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedSlotPairingOp {
    /// Add a slot pairing rule
    /// First parameter is the slot pairing rule id for the new rule
    Add(SlotPairingRuleId, slot_pairings::SlotPairingRule),
    /// Remove an existing slot pairing rule
    Remove(SlotPairingRuleId),
    /// Update an existing slot pairing rule
    Update(SlotPairingRuleId, slot_pairings::SlotPairingRule),
}

/// Settings operation enumeration
///
/// Compared to [SettingsOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedSettingsOp {
    /// Update the settings
    Update(settings::Settings),
}

/// Balancing annotated operation enumeration
///
/// Compared to [BalancingOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedBalancingOp {
    /// Update the balancing configuration
    Update(balancing::Balancing),
}

/// Colloscope operation enumeration
///
/// Compared to [ColloscopeOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedColloscopeOp {
    /// Update a group list
    UpdateGroupList(GroupListId, colloscopes::ColloscopeGroupList),
    /// Update an interrogation
    UpdateInterrogation(
        PeriodId,
        SlotId,
        usize,
        colloscopes::ColloscopeInterrogation,
    ),
}

/// Export configuration annotated operation enumeration
///
/// Compared to [ExportConfigOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedExportConfigOp {
    UpdateGlobalConfig(export_config::GlobalConfig),
    UpdateColloscopeEnabled(bool),
    UpdateAllGroupsEnabled(bool),
    UpdatePrefilledGroupsEnabled(bool),
    UpdateAutomaticGroupsEnabled(bool),
    UpdatePerGroupListEnabled(bool),
    UpdateColloscopeConfig(export_config::ColloscopeConfig),
    UpdateAllGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdatePrefilledGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdateAutomaticGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdatePerGroupListConfig(export_config::PerGroupListConfig),
}

impl Operation for AnnotatedOp {}

impl AnnotatedOp {
    /// Used internally
    ///
    /// Annotate an operation
    ///
    /// Takes a partial description of an operation of type [Op]
    /// and annotates it to make it reproducible.
    ///
    /// This might lead to the creation of new unique ids
    /// through an [IdIssuer].
    pub(crate) fn annotate(op: Op, id_issuer: &mut IdIssuer) -> (AnnotatedOp, Option<NewId>) {
        match op {
            Op::Student(student_op) => {
                let (op, id) = AnnotatedStudentOp::annotate(student_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Period(period_op) => {
                let (op, id) = AnnotatedPeriodOp::annotate(period_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Week(week_op) => {
                let (op, id) = AnnotatedWeekOp::annotate(week_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Subject(subject_op) => {
                let (op, id) = AnnotatedSubjectOp::annotate(subject_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Teacher(teacher_op) => {
                let (op, id) = AnnotatedTeacherOp::annotate(teacher_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Assignment(assignment_op) => {
                let op = AnnotatedAssignmentOp::annotate(assignment_op);
                (op.into(), None)
            }
            Op::WeekPattern(week_pattern_op) => {
                let (op, id) = AnnotatedWeekPatternOp::annotate(week_pattern_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Slot(slot_op) => {
                let (op, id) = AnnotatedSlotOp::annotate(slot_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Incompat(incompat_op) => {
                let (op, id) = AnnotatedIncompatOp::annotate(incompat_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::GroupList(group_list_op) => {
                let (op, id) = AnnotatedGroupListOp::annotate(group_list_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Pairing(pairing_op) => {
                let (op, id) = AnnotatedPairingOp::annotate(pairing_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::SlotPairing(slot_pairing_op) => {
                let (op, id) = AnnotatedSlotPairingOp::annotate(slot_pairing_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Settings(settings_op) => {
                let op = AnnotatedSettingsOp::annotate(settings_op);
                (op.into(), None)
            }
            Op::Balancing(balancing_op) => {
                let op = AnnotatedBalancingOp::annotate(balancing_op);
                (op.into(), None)
            }
            Op::Colloscope(colloscope_op) => {
                let op = AnnotatedColloscopeOp::annotate(colloscope_op);
                (op.into(), None)
            }
            Op::ExportConfig(export_config_op) => {
                let op = AnnotatedExportConfigOp::annotate(export_config_op);
                (op.into(), None)
            }
            Op::GlobalUpdate(inner_data) => {
                if let Some(max_id) = inner_data.ids().max() {
                    id_issuer.skip_to_id(max_id + 1).expect(
                        "GlobalUpdate: ID space exhausted. \
                         This is either a critical bug or a malicious data payload.",
                    );
                }
                (AnnotatedOp::GlobalUpdate(inner_data), None)
            }
        }
    }
}

impl AnnotatedStudentOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [StudentOp].
    fn annotate(
        student_op: StudentOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedStudentOp, Option<StudentId>) {
        match student_op {
            StudentOp::Add(student) => {
                let new_id = id_issuer.get_student_id();
                (AnnotatedStudentOp::Add(new_id, student), Some(new_id))
            }
            StudentOp::Remove(student_id) => (AnnotatedStudentOp::Remove(student_id), None),
            StudentOp::Update(student_id, student) => {
                (AnnotatedStudentOp::Update(student_id, student), None)
            }
        }
    }
}

impl AnnotatedPeriodOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [PeriodOp].
    fn annotate(
        period_op: PeriodOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedPeriodOp, Option<PeriodId>) {
        match period_op {
            PeriodOp::ChangeStartDate(date) => (AnnotatedPeriodOp::ChangeStartDate(date), None),
            PeriodOp::AddFront => {
                let new_id = id_issuer.get_period_id();
                (AnnotatedPeriodOp::AddFront(new_id), Some(new_id))
            }
            PeriodOp::AddAfter(after_id) => {
                let new_id = id_issuer.get_period_id();
                (AnnotatedPeriodOp::AddAfter(new_id, after_id), Some(new_id))
            }
            PeriodOp::Remove(period_id) => (AnnotatedPeriodOp::Remove(period_id), None),
        }
    }
}

impl AnnotatedWeekOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [WeekOp].
    fn annotate(week_op: WeekOp, id_issuer: &mut IdIssuer) -> (AnnotatedWeekOp, Option<WeekId>) {
        match week_op {
            WeekOp::AddFront(period_id, desc) => {
                let new_id = id_issuer.get_week_id();
                (
                    AnnotatedWeekOp::AddFront(new_id, period_id, desc),
                    Some(new_id),
                )
            }
            WeekOp::AddAfter(after_id, desc) => {
                let new_id = id_issuer.get_week_id();
                (
                    AnnotatedWeekOp::AddAfter(new_id, after_id, desc),
                    Some(new_id),
                )
            }
            WeekOp::Remove(week_id) => (AnnotatedWeekOp::Remove(week_id), None),
            WeekOp::Update(week_id, desc) => (AnnotatedWeekOp::Update(week_id, desc), None),
            WeekOp::Move(week_id, period_id, pos) => {
                (AnnotatedWeekOp::Move(week_id, period_id, pos), None)
            }
        }
    }
}

impl AnnotatedSubjectOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [SubjectOp].
    fn annotate(
        subject_op: SubjectOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedSubjectOp, Option<SubjectId>) {
        match subject_op {
            SubjectOp::AddAfter(after_id, parameters) => {
                let new_id = id_issuer.get_subject_id();
                (
                    AnnotatedSubjectOp::AddAfter(new_id, after_id, parameters),
                    Some(new_id),
                )
            }
            SubjectOp::ChangePosition(id, pos) => {
                (AnnotatedSubjectOp::ChangePosition(id, pos), None)
            }
            SubjectOp::Remove(id) => (AnnotatedSubjectOp::Remove(id), None),
            SubjectOp::Update(id, new_params) => (AnnotatedSubjectOp::Update(id, new_params), None),
        }
    }
}

impl AnnotatedTeacherOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [TeacherOp].
    fn annotate(
        teacher_op: TeacherOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedTeacherOp, Option<TeacherId>) {
        match teacher_op {
            TeacherOp::Add(teacher) => {
                let new_id = id_issuer.get_teacher_id();
                (AnnotatedTeacherOp::Add(new_id, teacher), Some(new_id))
            }
            TeacherOp::Remove(id) => (AnnotatedTeacherOp::Remove(id), None),
            TeacherOp::Update(id, new_teacher) => {
                (AnnotatedTeacherOp::Update(id, new_teacher), None)
            }
        }
    }
}

impl AnnotatedAssignmentOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [AssignmentOp].
    fn annotate(assignment_op: AssignmentOp) -> AnnotatedAssignmentOp {
        match assignment_op {
            AssignmentOp::Assign(period_id, student_id, subject_id, status) => {
                AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, status)
            }
        }
    }
}

impl AnnotatedWeekPatternOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [WeekPatternOp].
    fn annotate(
        week_pattern_op: WeekPatternOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedWeekPatternOp, Option<WeekPatternId>) {
        match week_pattern_op {
            WeekPatternOp::Add(week_pattern) => {
                let new_id = id_issuer.get_week_pattern_id();
                (
                    AnnotatedWeekPatternOp::Add(new_id, week_pattern),
                    Some(new_id),
                )
            }
            WeekPatternOp::Remove(id) => (AnnotatedWeekPatternOp::Remove(id), None),
            WeekPatternOp::Update(id, new_week_pattern) => {
                (AnnotatedWeekPatternOp::Update(id, new_week_pattern), None)
            }
        }
    }
}

impl AnnotatedSlotOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [SlotOp].
    fn annotate(slot_op: SlotOp, id_issuer: &mut IdIssuer) -> (AnnotatedSlotOp, Option<SlotId>) {
        match slot_op {
            SlotOp::AddAfter(after_id, slot) => {
                let new_id = id_issuer.get_slot_id();
                (
                    AnnotatedSlotOp::AddAfter(new_id, after_id, slot),
                    Some(new_id),
                )
            }
            SlotOp::ChangePosition(slot_id, new_pos) => {
                (AnnotatedSlotOp::ChangePosition(slot_id, new_pos), None)
            }
            SlotOp::Remove(slot_id) => (AnnotatedSlotOp::Remove(slot_id), None),
            SlotOp::Update(slot_id, slot) => (AnnotatedSlotOp::Update(slot_id, slot), None),
        }
    }
}

impl AnnotatedIncompatOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [IncompatOp].
    fn annotate(
        incompat_op: IncompatOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedIncompatOp, Option<IncompatId>) {
        match incompat_op {
            IncompatOp::Add(incompat) => {
                let new_id = id_issuer.get_incompat_id();
                (AnnotatedIncompatOp::Add(new_id, incompat), Some(new_id))
            }
            IncompatOp::Remove(incompat_id) => (AnnotatedIncompatOp::Remove(incompat_id), None),
            IncompatOp::Update(incompat_id, incompat) => {
                (AnnotatedIncompatOp::Update(incompat_id, incompat), None)
            }
        }
    }
}

impl AnnotatedGroupListOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [GroupListOp].
    fn annotate(
        group_list_op: GroupListOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedGroupListOp, Option<GroupListId>) {
        match group_list_op {
            GroupListOp::Add(params) => {
                let new_id = id_issuer.get_group_list_id();
                (
                    AnnotatedGroupListOp::Add(
                        new_id,
                        params,
                        group_lists::GroupListFilling::default(),
                    ),
                    Some(new_id),
                )
            }
            GroupListOp::Remove(group_list_id) => {
                (AnnotatedGroupListOp::Remove(group_list_id), None)
            }
            GroupListOp::Update(group_list_id, params) => {
                (AnnotatedGroupListOp::Update(group_list_id, params), None)
            }
            GroupListOp::SetFilling(group_list_id, filling) => (
                AnnotatedGroupListOp::SetFilling(group_list_id, filling),
                None,
            ),
            GroupListOp::AssignToSubject(period_id, subject_id, group_list_id) => (
                AnnotatedGroupListOp::AssignToSubject(period_id, subject_id, group_list_id),
                None,
            ),
        }
    }
}

impl AnnotatedPairingOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [PairingOp].
    fn annotate(
        pairing_op: PairingOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedPairingOp, Option<PairingRuleId>) {
        match pairing_op {
            PairingOp::Add(rule) => {
                let new_id = id_issuer.get_pairing_rule_id();
                (AnnotatedPairingOp::Add(new_id, rule), Some(new_id))
            }
            PairingOp::Remove(id) => (AnnotatedPairingOp::Remove(id), None),
            PairingOp::Update(id, rule) => (AnnotatedPairingOp::Update(id, rule), None),
        }
    }
}

impl AnnotatedSlotPairingOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [SlotPairingOp].
    fn annotate(
        slot_pairing_op: SlotPairingOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedSlotPairingOp, Option<SlotPairingRuleId>) {
        match slot_pairing_op {
            SlotPairingOp::Add(rule) => {
                let new_id = id_issuer.get_slot_pairing_rule_id();
                (AnnotatedSlotPairingOp::Add(new_id, rule), Some(new_id))
            }
            SlotPairingOp::Remove(id) => (AnnotatedSlotPairingOp::Remove(id), None),
            SlotPairingOp::Update(id, rule) => (AnnotatedSlotPairingOp::Update(id, rule), None),
        }
    }
}

impl AnnotatedSettingsOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [SettingsOp].
    fn annotate(settings_op: SettingsOp) -> AnnotatedSettingsOp {
        match settings_op {
            SettingsOp::Update(general_settings) => AnnotatedSettingsOp::Update(general_settings),
        }
    }
}

impl AnnotatedBalancingOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [BalancingOp].
    fn annotate(balancing_op: BalancingOp) -> AnnotatedBalancingOp {
        match balancing_op {
            BalancingOp::Update(balancing) => AnnotatedBalancingOp::Update(balancing),
        }
    }
}

impl AnnotatedColloscopeOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [ColloscopeOp].
    fn annotate(colloscope_op: ColloscopeOp) -> AnnotatedColloscopeOp {
        match colloscope_op {
            ColloscopeOp::UpdateGroupList(group_list_id, group_list) => {
                AnnotatedColloscopeOp::UpdateGroupList(group_list_id, group_list)
            }
            ColloscopeOp::UpdateInterrogation(
                period_id,
                slot_id,
                week_in_period,
                interrogation,
            ) => AnnotatedColloscopeOp::UpdateInterrogation(
                period_id,
                slot_id,
                week_in_period,
                interrogation,
            ),
        }
    }
}

impl AnnotatedExportConfigOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [ExportConfigOp].
    fn annotate(export_config_op: ExportConfigOp) -> AnnotatedExportConfigOp {
        match export_config_op {
            ExportConfigOp::UpdateGlobalConfig(v) => AnnotatedExportConfigOp::UpdateGlobalConfig(v),
            ExportConfigOp::UpdateColloscopeEnabled(v) => {
                AnnotatedExportConfigOp::UpdateColloscopeEnabled(v)
            }
            ExportConfigOp::UpdateAllGroupsEnabled(v) => {
                AnnotatedExportConfigOp::UpdateAllGroupsEnabled(v)
            }
            ExportConfigOp::UpdatePrefilledGroupsEnabled(v) => {
                AnnotatedExportConfigOp::UpdatePrefilledGroupsEnabled(v)
            }
            ExportConfigOp::UpdateAutomaticGroupsEnabled(v) => {
                AnnotatedExportConfigOp::UpdateAutomaticGroupsEnabled(v)
            }
            ExportConfigOp::UpdatePerGroupListEnabled(v) => {
                AnnotatedExportConfigOp::UpdatePerGroupListEnabled(v)
            }
            ExportConfigOp::UpdateColloscopeConfig(v) => {
                AnnotatedExportConfigOp::UpdateColloscopeConfig(v)
            }
            ExportConfigOp::UpdateAllGroupsConfig(v) => {
                AnnotatedExportConfigOp::UpdateAllGroupsConfig(v)
            }
            ExportConfigOp::UpdatePrefilledGroupsConfig(v) => {
                AnnotatedExportConfigOp::UpdatePrefilledGroupsConfig(v)
            }
            ExportConfigOp::UpdateAutomaticGroupsConfig(v) => {
                AnnotatedExportConfigOp::UpdateAutomaticGroupsConfig(v)
            }
            ExportConfigOp::UpdatePerGroupListConfig(v) => {
                AnnotatedExportConfigOp::UpdatePerGroupListConfig(v)
            }
        }
    }
}
