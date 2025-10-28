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
    /// Operation on rules
    Rule(RuleOp),
    /// Operation on settings
    Settings(SettingsOp),
    /// Operation on colloscopes
    Colloscopes(ColloscopeOp),
}

impl Operation for Op {}

/// Student operation enumeration
///
/// This is the list of all possible operations related to the
/// student list we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentOp {
    /// Add a new student
    Add(students::Student<PeriodId>),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, students::Student<PeriodId>),
}

/// Period operation enumeration
///
/// This is the list of all possible operations related to the
/// periods we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeriodOp {
    /// Set the start of periods on a specific week
    ChangeStartDate(Option<collomatique_time::NaiveMondayDate>),
    /// Add a new period at the beginning
    AddFront(Vec<periods::WeekDesc>),
    /// Add a period after an existing period
    AddAfter(PeriodId, Vec<periods::WeekDesc>),
    /// Remove an existing period
    Remove(PeriodId),
    /// Update an existing period
    Update(PeriodId, Vec<periods::WeekDesc>),
}

/// Subject operation enumeration
///
/// This is the list of all possible operations related to the
/// subjects we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectOp {
    /// Add a subject after an existing subject
    /// If `None`, it is placed first
    AddAfter(Option<SubjectId>, subjects::Subject<PeriodId>),
    /// Remove an existing subject
    Remove(SubjectId),
    /// Move a subject to another position in the list
    ChangePosition(SubjectId, usize),
    /// Update the parameters of an existing subject
    Update(SubjectId, subjects::Subject<PeriodId>),
}

/// Teacher operation enumeration
///
/// This is the list of all possible operations related to the
/// teachers we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeacherOp {
    /// Add a teacher
    Add(teachers::Teacher<SubjectId>),
    /// Remove an existing teacher
    Remove(TeacherId),
    /// Update the parameters of an existing teacher
    Update(TeacherId, teachers::Teacher<SubjectId>),
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
    AddAfter(
        SubjectId,
        Option<SlotId>,
        slots::Slot<TeacherId, WeekPatternId>,
    ),
    /// Remove an existing slot
    Remove(SlotId),
    /// Move a subject to another position in the list
    ChangePosition(SlotId, usize),
    /// Update the parameters of an existing subject
    Update(SlotId, slots::Slot<TeacherId, WeekPatternId>),
}

/// Incompat operation enumeration
///
/// This is the list of all possible operations related to the
/// incompatibilities we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompatOp {
    /// Add an incompatibility
    Add(incompats::Incompatibility<SubjectId, WeekPatternId>),
    /// Remove an existing incompatibility
    Remove(IncompatId),
    /// Update an incompatibility
    Update(
        IncompatId,
        incompats::Incompatibility<SubjectId, WeekPatternId>,
    ),
}

/// Group list operation enumeration
///
/// This is the list of all possible operations related to the
/// group lists we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupListOp {
    /// Add a group list
    Add(group_lists::GroupListParameters<StudentId>),
    /// Remove an existing group list
    Remove(GroupListId),
    /// Update a group list
    Update(GroupListId, group_lists::GroupListParameters<StudentId>),
    /// Change pre-fill for a group list
    PreFill(
        GroupListId,
        group_lists::GroupListPrefilledGroups<StudentId>,
    ),
    /// Assign a group list to a subject
    AssignToSubject(PeriodId, SubjectId, Option<GroupListId>),
}

/// Rule operation enumeration
///
/// This is the list of all possible operations related to the
/// rules we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleOp {
    /// Add a rule
    Add(rules::Rule<PeriodId, SlotId>),
    /// Remove an existing rule
    Remove(RuleId),
    /// Update a rule
    Update(RuleId, rules::Rule<PeriodId, SlotId>),
}

/// Settings operation enumeration
///
/// This is the list of all possible operations related to the
/// settings we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOp {
    /// Update the settings
    Update(settings::GeneralSettings),
}

/// Colloscope operation enumeration
///
/// This is the list of all possible operations related to the
/// colloscopes we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColloscopeOp {
    /// Add a colloscope
    Add(colloscopes::Colloscope),
    /// Update a colloscope
    Update(ColloscopeId, colloscopes::Colloscope),
    /// Remove an existing colloscope
    Remove(ColloscopeId),
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
    /// Operation on rules
    Rule(AnnotatedRuleOp),
    /// Operation on settings
    Settings(AnnotatedSettingsOp),
    /// Operation on colloscopes
    Colloscopes(AnnotatedColloscopeOp),
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

impl From<AnnotatedRuleOp> for AnnotatedOp {
    fn from(value: AnnotatedRuleOp) -> Self {
        AnnotatedOp::Rule(value)
    }
}

impl From<AnnotatedSettingsOp> for AnnotatedOp {
    fn from(value: AnnotatedSettingsOp) -> Self {
        AnnotatedOp::Settings(value)
    }
}

impl From<AnnotatedColloscopeOp> for AnnotatedOp {
    fn from(value: AnnotatedColloscopeOp) -> Self {
        AnnotatedOp::Colloscopes(value)
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
    Add(StudentId, students::Student<PeriodId>),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, students::Student<PeriodId>),
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
    ChangeStartDate(Option<collomatique_time::NaiveMondayDate>),
    /// Add a new period at the beginning
    AddFront(PeriodId, Vec<periods::WeekDesc>),
    /// Add a period after an existing period
    /// First parameter is the period id for the new period
    AddAfter(PeriodId, PeriodId, Vec<periods::WeekDesc>),
    /// Remove an existing period
    Remove(PeriodId),
    /// Update an existing period
    Update(PeriodId, Vec<periods::WeekDesc>),
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
    AddAfter(SubjectId, Option<SubjectId>, subjects::Subject<PeriodId>),
    /// Remove an existing subject
    Remove(SubjectId),
    /// Move a subject to another position in the list
    ChangePosition(SubjectId, usize),
    /// Update the parameters of an existing subject
    Update(SubjectId, subjects::Subject<PeriodId>),
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
    Add(TeacherId, teachers::Teacher<SubjectId>),
    /// Remove an existing teacher
    Remove(TeacherId),
    /// Update the parameters of an existing teacher
    Update(TeacherId, teachers::Teacher<SubjectId>),
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
    AddAfter(
        SlotId,
        SubjectId,
        Option<SlotId>,
        slots::Slot<TeacherId, WeekPatternId>,
    ),
    /// Remove an existing slot
    Remove(SlotId),
    /// Move a subject to another position in the list
    ChangePosition(SlotId, usize),
    /// Update the parameters of an existing subject
    Update(SlotId, slots::Slot<TeacherId, WeekPatternId>),
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
    Add(
        IncompatId,
        incompats::Incompatibility<SubjectId, WeekPatternId>,
    ),
    /// Remove an existing incompat
    Remove(IncompatId),
    /// Update an existing incompat
    Update(
        IncompatId,
        incompats::Incompatibility<SubjectId, WeekPatternId>,
    ),
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
    Add(GroupListId, group_lists::GroupListParameters<StudentId>),
    /// Remove an existing group list
    Remove(GroupListId),
    /// Update a group list
    Update(GroupListId, group_lists::GroupListParameters<StudentId>),
    /// Change pre-fill for a group list
    PreFill(
        GroupListId,
        group_lists::GroupListPrefilledGroups<StudentId>,
    ),
    /// Assign a group list to a subject
    AssignToSubject(PeriodId, SubjectId, Option<GroupListId>),
}

/// Rule operation enumeration
///
/// Compared to [RuleOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [collomatique_state::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedRuleOp {
    /// Add a rule
    /// First parameter is the rule id for the new rule
    Add(RuleId, rules::Rule<PeriodId, SlotId>),
    /// Remove an existing rule
    Remove(RuleId),
    /// Update a rule
    Update(RuleId, rules::Rule<PeriodId, SlotId>),
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
    Update(settings::GeneralSettings),
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
    /// Add an empty colloscope
    /// First parameter is the colloscope id for the new colloscope
    Add(ColloscopeId, colloscopes::Colloscope),
    /// Update a colloscope
    Update(ColloscopeId, colloscopes::Colloscope),
    /// Remove an existing colloscope
    Remove(ColloscopeId),
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
            Op::Rule(rule_op) => {
                let (op, id) = AnnotatedRuleOp::annotate(rule_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
            }
            Op::Settings(settings_op) => {
                let op = AnnotatedSettingsOp::annotate(settings_op);
                (op.into(), None)
            }
            Op::Colloscopes(colloscopes_op) => {
                let (op, id) = AnnotatedColloscopeOp::annotate(colloscopes_op, id_issuer);
                (op.into(), id.map(|x| x.into()))
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
            PeriodOp::AddFront(desc) => {
                let new_id = id_issuer.get_period_id();
                (AnnotatedPeriodOp::AddFront(new_id, desc), Some(new_id))
            }
            PeriodOp::AddAfter(after_id, desc) => {
                let new_id = id_issuer.get_period_id();
                (
                    AnnotatedPeriodOp::AddAfter(new_id, after_id, desc),
                    Some(new_id),
                )
            }
            PeriodOp::Remove(period_id) => (AnnotatedPeriodOp::Remove(period_id), None),
            PeriodOp::Update(period_id, desc) => (AnnotatedPeriodOp::Update(period_id, desc), None),
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
            SlotOp::AddAfter(subject_id, after_id, slot) => {
                let new_id = id_issuer.get_slot_id();
                (
                    AnnotatedSlotOp::AddAfter(new_id, subject_id, after_id, slot),
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
                (AnnotatedGroupListOp::Add(new_id, params), Some(new_id))
            }
            GroupListOp::Remove(group_list_id) => {
                (AnnotatedGroupListOp::Remove(group_list_id), None)
            }
            GroupListOp::Update(group_list_id, params) => {
                (AnnotatedGroupListOp::Update(group_list_id, params), None)
            }
            GroupListOp::PreFill(group_list_id, map) => {
                (AnnotatedGroupListOp::PreFill(group_list_id, map), None)
            }
            GroupListOp::AssignToSubject(period_id, subject_id, group_list_id) => (
                AnnotatedGroupListOp::AssignToSubject(period_id, subject_id, group_list_id),
                None,
            ),
        }
    }
}

impl AnnotatedRuleOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [RuleOp].
    fn annotate(rule_op: RuleOp, id_issuer: &mut IdIssuer) -> (AnnotatedRuleOp, Option<RuleId>) {
        match rule_op {
            RuleOp::Add(rule) => {
                let new_id = id_issuer.get_rule_id();
                (AnnotatedRuleOp::Add(new_id, rule), Some(new_id))
            }
            RuleOp::Remove(rule_id) => (AnnotatedRuleOp::Remove(rule_id), None),
            RuleOp::Update(rule_id, rule) => (AnnotatedRuleOp::Update(rule_id, rule), None),
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

impl AnnotatedColloscopeOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [ColloscopeOp].
    fn annotate(
        colloscope_op: ColloscopeOp,
        id_issuer: &mut IdIssuer,
    ) -> (AnnotatedColloscopeOp, Option<ColloscopeId>) {
        match colloscope_op {
            ColloscopeOp::Add(colloscope) => {
                let new_id = id_issuer.get_colloscope_id();
                (AnnotatedColloscopeOp::Add(new_id, colloscope), Some(new_id))
            }
            ColloscopeOp::Update(colloscope_id, colloscope) => (
                AnnotatedColloscopeOp::Update(colloscope_id, colloscope),
                None,
            ),
            ColloscopeOp::Remove(colloscope_id) => {
                (AnnotatedColloscopeOp::Remove(colloscope_id), None)
            }
        }
    }
}
