#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

use super::time;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Subject {0} has empty students_per_interrogation: {1:?}")]
    SubjectWithInvalidStudentsPerInterrogationRange(usize, RangeInclusive<NonZeroUsize>),
    #[error("Subject {0} has in interrogation {1} the slot {2} after the week count ({3}) of the schedule")]
    SubjectWithSlotAfterLastWeek(usize, usize, usize, u32),
    #[error("Subject {0} has in interrogation {1} the slot {2} overlapping next day")]
    SubjectWithSlotOverlappingNextDay(usize, usize, usize),
    #[error("Subject {0} has invalid subject number ({2}) in interrogation {1}")]
    SubjectWithInvalidTeacher(usize, usize, usize),
    #[error("Student {0} references an invalid subject number ({1})")]
    StudentWithInvalidSubject(usize, usize),
    #[error("Student {0} references an invalid incompatibility number ({1})")]
    StudentWithInvalidIncompatibility(usize, usize),
    #[error(
        "Incompatibility {0} has interrogation slot {1} after the week count ({2}) of the schedule"
    )]
    IncompatibilityWithSlotAfterLastWeek(usize, usize, u32),
    #[error("Incompatibility {0} has interrogation slot {1} overlapping next day")]
    IncompatibilityWithSlotOverlappingNextDay(usize, usize),
    #[error("The slot grouping {0} has an invalid slot ref {1:?} with invalid subject reference")]
    SlotGroupingWithInvalidSubject(usize, SlotRef),
    #[error(
        "The slot grouping {0} has an invalid slot ref {1:?} with invalid interrogation reference"
    )]
    SlotGroupingWithInvalidInterrogation(usize, SlotRef),
    #[error("The slot grouping {0} has an invalid slot ref {1:?} with invalid slot reference")]
    SlotGroupingWithInvalidSlot(usize, SlotRef),
    #[error("The grouping incompatibility {0} has an invalid slot grouping reference {1}")]
    GroupingIncompatWithInvalidSlotGrouping(usize, usize),
    #[error("The range {0:?} for the number of interrogations per week is empty")]
    GeneralDataWithInvalidInterrogationsPerWeek(std::ops::Range<u32>),
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
    pub students_per_interrogation: RangeInclusive<NonZeroUsize>,
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

pub type GroupingIncompatList = Vec<GroupingIncompat>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility {
    pub slots: Vec<Slot>,
}

pub type IncompatibilityList = Vec<Incompatibility>;

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Student {
    pub subjects: BTreeSet<usize>,
    pub incompatibilities: BTreeSet<usize>,
}

pub type StudentList = Vec<Student>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralData {
    pub teacher_count: usize,
    pub week_count: NonZeroU32,
    pub interrogations_per_week: Option<std::ops::Range<u32>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedData {
    general: GeneralData,
    subjects: SubjectList,
    incompatibilities: IncompatibilityList,
    students: StudentList,
    slot_groupings: SlotGroupingList,
    grouping_incompats: GroupingIncompatList,
}

impl ValidatedData {
    fn validate_slot_start(general: &GeneralData, slot_start: &SlotStart) -> bool {
        slot_start.week < general.week_count.get()
    }

    fn validate_slot_overlap(slot_start: &SlotStart, duration: NonZeroU32) -> bool {
        slot_start.start_time.fit_in_day(duration.get())
    }

    pub fn new(
        general: GeneralData,
        subjects: SubjectList,
        incompatibilities: IncompatibilityList,
        students: StudentList,
        slot_groupings: SlotGroupingList,
        grouping_incompats: GroupingIncompatList,
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
                for (k, slot_start) in interrogation.slots.iter().enumerate() {
                    if !Self::validate_slot_start(&general, slot_start) {
                        return Err(Error::SubjectWithSlotAfterLastWeek(
                            i,
                            j,
                            k,
                            general.week_count.get(),
                        ));
                    }
                    if !Self::validate_slot_overlap(slot_start, subject.duration) {
                        return Err(Error::SubjectWithSlotOverlappingNextDay(i, j, k));
                    }
                }
            }
        }

        for (i, incompatibility) in incompatibilities.iter().enumerate() {
            for (j, slot) in incompatibility.slots.iter().enumerate() {
                if !Self::validate_slot_start(&general, &slot.start) {
                    return Err(Error::IncompatibilityWithSlotAfterLastWeek(
                        i,
                        j,
                        general.week_count.get(),
                    ));
                }
                if !Self::validate_slot_overlap(&slot.start, slot.duration) {
                    return Err(Error::IncompatibilityWithSlotOverlappingNextDay(i, j));
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

        for (i, slot_grouping) in slot_groupings.iter().enumerate() {
            for slot_ref in &slot_grouping.slots {
                if slot_ref.subject >= subjects.len() {
                    return Err(Error::SlotGroupingWithInvalidSubject(i, slot_ref.clone()));
                }
                if slot_ref.interrogation >= subjects[slot_ref.subject].interrogations.len() {
                    return Err(Error::SlotGroupingWithInvalidInterrogation(
                        i,
                        slot_ref.clone(),
                    ));
                }
                if slot_ref.slot
                    >= subjects[slot_ref.subject].interrogations[slot_ref.interrogation]
                        .slots
                        .len()
                {
                    return Err(Error::SlotGroupingWithInvalidSlot(i, slot_ref.clone()));
                }
            }
        }

        for (i, grouping_incompat) in grouping_incompats.iter().enumerate() {
            for &grouping in &grouping_incompat.groupings {
                if grouping >= slot_groupings.len() {
                    return Err(Error::GroupingIncompatWithInvalidSlotGrouping(i, grouping));
                }
            }
        }

        if let Some(interrogations_range) = general.interrogations_per_week.clone() {
            if interrogations_range.is_empty() {
                return Err(Error::GeneralDataWithInvalidInterrogationsPerWeek(
                    interrogations_range,
                ));
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

impl ValidatedData {
    fn count_student_specializations(&self) -> BTreeMap<Student, NonZeroUsize> {
        let mut output: BTreeMap<Student, NonZeroUsize> = BTreeMap::new();

        for student in &self.students {
            match output.get_mut(student) {
                Some(counter) => {
                    *counter = counter
                        .checked_add(1)
                        .expect("There should be less than 2^32 student");
                }
                None => {
                    output.insert(student.clone(), NonZeroUsize::new(1).unwrap());
                }
            }
        }

        output
    }
}
