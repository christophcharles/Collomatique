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
}

impl Operation for Op {}

/// Student operation enumeration
///
/// This is the list of all possible operations related to the
/// student list we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentOp {
    /// Add a new student
    Add(PersonWithContact),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, PersonWithContact),
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
    Add(StudentId, PersonWithContact),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, PersonWithContact),
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
