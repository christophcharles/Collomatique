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

use crate::ids::PeriodId;

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
    ChangeStartDate(Option<collomatique_time::NaiveMondayDate>),
    /// Add a new period at the beginning
    AddFront(Vec<bool>),
    /// Add a period after an existing period
    AddAfter(PeriodId, Vec<bool>),
    /// Remove an existing period
    Remove(PeriodId),
    /// Update an existing period
    Update(PeriodId, Vec<bool>),
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
    AddAfter(SubjectId, Option<SlotId>, slots::Slot),
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
    ChangeStartDate(Option<collomatique_time::NaiveMondayDate>),
    /// Add a new period at the beginning
    AddFront(PeriodId, Vec<bool>),
    /// Add a period after an existing period
    /// First parameter is the period id for the new period
    AddAfter(PeriodId, PeriodId, Vec<bool>),
    /// Remove an existing period
    Remove(PeriodId),
    /// Update an existing period
    Update(PeriodId, Vec<bool>),
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
    AddAfter(SlotId, SubjectId, Option<SlotId>, slots::Slot),
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
