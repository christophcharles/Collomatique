#[cfg(test)]
mod tests;

use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use super::time;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Subject {0} has empty students_per_interrogation: {1:?}")]
    SubjectWithInvalidStudentsPerInterrogationRange(usize, RangeInclusive<NonZeroU32>),
    #[error("Interrogation slot is too long to fit in a day")]
    SlotOverlapsNextDay,
    #[error("Subject {0} has invalid subject number ({2}) in interrogation {1}")]
    SubjectWithInvalidTeacher(usize, usize, usize),
    #[error("Student {0} references an invalid subject number ({1})")]
    StudentWithInvalidSubject(usize, usize),
    #[error("Student {0} references an invalid incompatibility number ({1})")]
    StudentWithInvalidIncompatibility(usize, usize),
    #[error("Slot groupings {0} and {1} are duplicates of each other")]
    SlotGroupingsDuplicated(usize, usize),
    #[error("The slot grouping {0} has an invalid slot ref {1:?} with invalid subject reference")]
    SlotGroupingWithInvalidSubject(usize, SlotRef),
    #[error(
        "The slot grouping {0} has an invalid slot ref {1:?} with invalid interrogation reference"
    )]
    SlotGroupingWithInvalidInterrogation(usize, SlotRef),
    #[error("The slot grouping {0} has an invalid slot ref {1:?} with invalid slot reference")]
    SlotGroupingWithInvalidSlot(usize, SlotRef),
    #[error("The grouping incompatibility {0:?} has an invalid slot grouping reference {1}")]
    GroupingIncompatWithInvalidSlotGrouping(GroupingIncompat, usize),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotStart {
    week: u32,
    weekday: time::Weekday,
    start_time: time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub start: SlotStart,
    pub duration: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interrogation {
    pub teacher: usize,
    pub slots: Vec<SlotStart>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub students_per_interrogation: RangeInclusive<NonZeroU32>,
    pub period: NonZeroU32,
    pub duration: NonZeroU32,
    pub interrogations: Vec<Interrogation>,
}

pub type SubjectList = Vec<Subject>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotRef {
    pub subject: usize,
    pub interrogation: usize,
    pub slot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotGrouping {
    pub slots: BTreeSet<SlotRef>,
}

pub type SlotGroupingList = Vec<SlotGrouping>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupingIncompat {
    pub groupings: BTreeSet<usize>,
}

pub type GroupingIncompatSet = BTreeSet<GroupingIncompat>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility {
    pub slots: Vec<Slot>,
}

pub type IncompatibilityList = Vec<Incompatibility>;

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Student {
    pub subjects: BTreeSet<usize>,
    pub incompatibilities: BTreeSet<usize>,
}

pub type StudentList = Vec<Student>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralData {
    pub teacher_count: usize,
    pub week_count: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedData {
    general: GeneralData,
    subjects: SubjectList,
    incompatibilities: IncompatibilityList,
    students: StudentList,
    slot_groupings: SlotGroupingList,
    grouping_incompats: GroupingIncompatSet,
}

impl ValidatedData {
    fn validate_slot_start(general: &GeneralData, slot_start: &SlotStart) -> bool {
        slot_start.week < general.week_count.get()
    }

    fn validate_slot(general: &GeneralData, slot_start: &SlotStart, duration: NonZeroU32) -> bool {
        Self::validate_slot_start(general, &slot_start)
            && slot_start.start_time.fit_in_day(duration.get())
    }

    pub fn new(
        general: GeneralData,
        subjects: SubjectList,
        incompatibilities: IncompatibilityList,
        students: StudentList,
        slot_groupings: SlotGroupingList,
        grouping_incompats: GroupingIncompatSet,
    ) -> Result<ValidatedData> {
        for (i, subject) in subjects.iter().enumerate() {
            if subject.students_per_interrogation.is_empty() {
                return Err(Error::SubjectWithInvalidStudentsPerInterrogationRange(
                    i,
                    subject.students_per_interrogation.clone(),
                ));
            }

            for (j, interrogation) in subject.interrogations.iter().enumerate() {
                if interrogation.teacher >= general.teacher_count {
                    return Err(Error::SubjectWithInvalidTeacher(
                        i,
                        j,
                        interrogation.teacher,
                    ));
                }
                for slot_start in &interrogation.slots {
                    if !Self::validate_slot(&general, slot_start, subject.duration) {
                        return Err(Error::SlotOverlapsNextDay);
                    }
                }
            }
        }

        for incompatibility in &incompatibilities {
            for slot in &incompatibility.slots {
                if !Self::validate_slot(&general, &slot.start, slot.duration) {
                    return Err(Error::SlotOverlapsNextDay);
                }
            }
        }

        for (i, student) in students.iter().enumerate() {
            for &subject in &student.subjects {
                if subject >= subjects.len() {
                    return Err(Error::StudentWithInvalidSubject(i, subject));
                }
            }

            for &incompatibility in &student.incompatibilities {
                if incompatibility >= incompatibilities.len() {
                    return Err(Error::StudentWithInvalidIncompatibility(i, incompatibility));
                }
            }
        }

        Ok(ValidatedData {
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats,
        })
    }
}
