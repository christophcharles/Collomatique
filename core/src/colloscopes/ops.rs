//! Ops module
//!
//! This modules defines all the operations (that means atomic modification)
//! we can do on colloscopes data
//!
//! The main type is [Op] which defines all possible modification operations
//! that can be done on the data.
//!
//! [AnnotatedOp] is the corresponding annotated type. See [crate::history]
//! for a full discussion of annotation.

use super::*;

/// Operation enumeration
///
/// This is the list of all possible operations on [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Operation on the student list
    Student(StudentOp),
}

impl Operation for Op {}

/// Student operation enumeration
///
/// This is the list of all possible operations related to the
/// student list we can do on a [Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentOp {
    /// Add a new student
    Add(PersonWithContacts),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, PersonWithContacts),
}

/// Annotated operation
///
/// Compared to [Op], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [super::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedOp {
    /// Operation on the student list
    Student(AnnotatedStudentOp),
}

/// Student annotated operation enumeration
///
/// Compared to [StudentOp], this is a annotated operation,
/// meaning the operation has been annotated to contain
/// all the necessary data to make it *reproducible*.
///
/// See [super::history] for a complete discussion of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedStudentOp {
    /// Add a new student (with fixed id)
    Add(StudentId, PersonWithContacts),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, PersonWithContacts),
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
    pub(crate) fn annotate(op: Op, id_issuer: &IdIssuer) -> AnnotatedOp {
        match op {
            Op::Student(student_op) => {
                AnnotatedOp::Student(AnnotatedStudentOp::annotate(student_op, id_issuer))
            }
        }
    }
}

impl AnnotatedStudentOp {
    /// Used internally
    ///
    /// Annotates the subcategory of operations [StudentOp].
    fn annotate(student_op: StudentOp, id_issuer: &IdIssuer) -> AnnotatedStudentOp {
        match student_op {
            StudentOp::Add(student) => AnnotatedStudentOp::Add(id_issuer.get_student_id(), student),
            StudentOp::Remove(student_id) => AnnotatedStudentOp::Remove(student_id),
            StudentOp::Update(student_id, student) => {
                AnnotatedStudentOp::Update(student_id, student)
            }
        }
    }
}
