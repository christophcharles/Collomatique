//! Core functionnality of collomatique
//!
//! This crate defines the core functionnality of collomatique in a
//! UI-agnostic way. This should allow for implementation of different
//! UIs all using the same core code.

pub mod ids;
pub mod ops;

use ids::{IdIssuer, StudentId};
use ops::{Op, StudentOp};
use std::collections::BTreeMap;

/// Errors for the core crate
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Generating new IDs is not secure: half the usable IDs have been used already.
    ///
    /// This *should* not happen. If this happen, most probably a malicious
    /// file was opened.
    EndOfTheUniverse,

    /// A student id is invalid
    InvalidStudentId(StudentId),
}

/// Result type with error type set to [Error]
type Result<T> = std::result::Result<T, Error>;

/// Description of a student
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Student {
    pub surname: String,
    pub firstname: String,
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
    id_issuer: IdIssuer,
    student_list: BTreeMap<StudentId, Student>,
}

impl Data {
    /// Create a new [Data]
    ///
    /// This [Data] is basically empty and corresponds to the
    /// state of a new file
    pub fn new() -> Data {
        let student_list = BTreeMap::new();
        Self::from_lists(student_list).expect("Lists are empty and should be valid")
    }

    /// Create a new [Data] from existing lists
    ///
    /// This will check the consistency of the lists
    /// and will also do some internal checks, so this might fail.
    pub fn from_lists(student_list: BTreeMap<StudentId, Student>) -> Result<Data> {
        let student_ids = student_list.keys();
        Ok(Data {
            id_issuer: IdIssuer::new(student_ids)?,
            student_list,
        })
    }

    /// Get the student list
    pub fn get_student_list(&self) -> &BTreeMap<StudentId, Student> {
        &self.student_list
    }

    /// Apply an operation on the data
    pub fn apply(&mut self, op: Op) -> Result<()> {
        match op {
            Op::Student(student_op) => self.apply_student(student_op),
        }
    }

    fn apply_student(&mut self, student_op: StudentOp) -> Result<()> {
        match student_op {
            StudentOp::Add(student) => {
                let student_id = self.id_issuer.get_student_id();
                self.student_list.insert(student_id, student);
                Ok(())
            }
            StudentOp::Remove(student_id) => {
                if self.student_list.remove(&student_id).is_none() {
                    Err(Error::InvalidStudentId(student_id))
                } else {
                    Ok(())
                }
            }
            StudentOp::Update(student_id, student) => {
                if let Some(old_student) = self.student_list.get_mut(&student_id) {
                    *old_student = student;
                    Ok(())
                } else {
                    Err(Error::InvalidStudentId(student_id))
                }
            }
        }
    }
}
