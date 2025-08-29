//! Variables submodule of [crate::base].
//!
//! This submodule defines the various ILP variables internally

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainVariable<PeriodId: Identifier, SubjectId: Identifier, StudentId: Identifier> {
    GroupForStudent {
        period: PeriodId,
        subject: SubjectId,
        student: StudentId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructureVariable<PeriodId: Identifier, SubjectId: Identifier, StudentId: Identifier> {
    StudentInGroup {
        period: PeriodId,
        subject: SubjectId,
        student: StudentId,
        group: u32,
    },
    NonEmptyGroup {
        period: PeriodId,
        subject: SubjectId,
        group: u32,
    },
}
