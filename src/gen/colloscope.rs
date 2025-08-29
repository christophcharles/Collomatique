use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use super::time;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Subject has an empty range for students_per_interrogation")]
    InvalidStudentsPerInterrogationRange,
    #[error("Interrogation slot is too long to fit in a day")]
    SlotOverlapsNextDay,
    #[error("Teacher number is invalid")]
    InvalidTeacherNumber,
    #[error("Subject number is invalid")]
    InvalidSubjectNumber,
    #[error("Incompatibility number is invalid")]
    InvalidIncompatibilityNumber,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct SlotStart {
    week: u32,
    weekday: time::Weekday,
    start_time: time::Time,
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub start: SlotStart,
    pub duration: NonZeroU32,
}

#[derive(Clone, Debug)]
pub struct Interrogation {
    pub teacher: usize,
    pub slots: Vec<SlotStart>,
}

#[derive(Clone, Debug)]
pub struct Subject {
    pub students_per_interrogation: RangeInclusive<NonZeroU32>,
    pub period: NonZeroU32,
    pub duration: NonZeroU32,
    pub interrogations: Vec<Interrogation>,
}

pub type SubjectList = Vec<Subject>;

#[derive(Clone, Debug)]
pub struct Incompatibility {
    pub slots: Vec<Slot>,
}

pub type IncompatibilityList = Vec<Incompatibility>;

use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Student {
    pub subjects: BTreeSet<usize>,
    pub incompatibilities: BTreeSet<usize>,
}

pub type StudentList = Vec<Student>;

#[derive(Clone, Debug)]
pub struct GeneralData {
    pub teacher_count: usize,
    pub week_count: NonZeroU32,
}

#[derive(Clone, Debug)]
pub struct ValidatedData {
    general: GeneralData,
    subjects: SubjectList,
    incompatibilities: IncompatibilityList,
    students: StudentList,
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
    ) -> Result<ValidatedData> {
        for subject in &subjects {
            if subject.students_per_interrogation.is_empty() {
                return Err(Error::InvalidStudentsPerInterrogationRange);
            }

            for interrogation in &subject.interrogations {
                if interrogation.teacher >= general.teacher_count {
                    return Err(Error::InvalidTeacherNumber);
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

        for student in &students {
            for &subject in &student.subjects {
                if subject >= subjects.len() {
                    return Err(Error::InvalidSubjectNumber);
                }
            }

            for &incompatibility in &student.incompatibilities {
                if incompatibility >= incompatibilities.len() {
                    return Err(Error::InvalidIncompatibilityNumber);
                }
            }
        }

        Ok(ValidatedData {
            general,
            subjects,
            incompatibilities,
            students,
        })
    }
}
