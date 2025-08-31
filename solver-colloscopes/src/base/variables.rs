//! Variables submodule of [crate::base].
//!
//! This submodule defines the various ILP variables internally

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainVariable<
    GroupListId: Identifier,
    StudentId: Identifier,
    SubjectId: Identifier,
    SlotId: Identifier,
> {
    GroupForStudent {
        group_list: GroupListId,
        student: StudentId,
    },
    GroupInSlot {
        subject: SubjectId,
        slot: SlotId,
        week: usize,
        group: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructureVariable<
    GroupListId: Identifier,
    StudentId: Identifier,
    SubjectId: Identifier,
    SlotId: Identifier,
> {
    StudentInGroup {
        group_list: GroupListId,
        student: StudentId,
        group: usize,
    },
    NonEmptyGroup {
        group_list: GroupListId,
        group: usize,
    },
    StudentInGroupForSubjectAndAssignmentZone {
        subject: SubjectId,
        student: StudentId,
        group: usize,
        assignment_zone: usize,
    },
    NonEmptyGroupForSubjectAndAssignmentZone {
        subject: SubjectId,
        group: usize,
        assignment_zone: usize,
    },
    StudentInGroupAndSlot {
        subject: SubjectId,
        student: StudentId,
        group: usize,
        slot: SlotId,
        week: usize,
    },
    NonEmptySlot {
        subject: SubjectId,
        slot: SlotId,
        week: usize,
    },
}
