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
        group: u32,
    },
}

use std::collections::BTreeSet;

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
        group: u32,
    },
    NonEmptyGroup {
        group_list: GroupListId,
        group: u32,
    },
    NonEmptyGroupForSubClass {
        subclass: BTreeSet<StudentId>,
        group_list: GroupListId,
        group: u32,
    },
    StudentInGroupAndSlot {
        subject: SubjectId,
        student: StudentId,
        group: u32,
        slot: SlotId,
        week: usize,
    },
    StudentInSlot {
        subject: SubjectId,
        student: StudentId,
        slot: SlotId,
        week: usize,
    },
    NonEmptySlot {
        subject: SubjectId,
        slot: SlotId,
        week: usize,
    },
}
