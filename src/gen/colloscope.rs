use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use super::time;

#[derive(Clone, Debug)]
pub struct SlotStart {
    week: u32,
    weekday: time::Weekday,
    start: time::Time,
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
