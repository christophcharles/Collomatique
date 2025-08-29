//! Variables submodule of [crate::base].
//!
//! This submodule defines the various ILP variables internally

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainVariable<SubjectId: Identifier, StudentId: Identifier> {
    GroupForStudent {
        subject: SubjectId,
        student: StudentId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructureVariable<StudentId: Identifier, SubjectId: Identifier> {
    StudentInGroup {
        subject: SubjectId,
        student: StudentId,
        group: u32,
    },
    NonEmptyGroup {
        subject: SubjectId,
        group: u32,
    },
}
