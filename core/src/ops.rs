//! Ops module
//!
//! This modules defines all the operations (that means atomic modification)
//! we can do on the data

use super::*;

/// Operation enumeration
///
/// This is the list of all possible operations on [super::Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Operation on the student list
    Student(StudentOp),
}

/// Student operation enumeration
///
/// This is the list of all possible operations related to the
/// student list we can do on a [super::Data]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentOp {
    /// Add a new student
    Add(PersonWithContacts),
    /// Remove an existing student identified through its id
    Remove(StudentId),
    /// Update the data on an existing student
    Update(StudentId, PersonWithContacts),
}
