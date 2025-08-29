use std::num::NonZeroU32;
use std::ops::RangeInclusive;

#[derive(Clone, Debug)]
pub struct Slot {}

#[derive(Clone, Debug)]
pub struct Subject {
    pub name: String,
    pub students_per_interrogation: RangeInclusive<NonZeroU32>,
    pub period: NonZeroU32,
    pub duration: NonZeroU32,
}

pub type SubjectList = Vec<Subject>;

#[derive(Clone, Debug)]
pub struct Incompatibility {
    pub name: String,
    pub slots: Vec<Slot>,
}

pub type IncompatibilityList = Vec<Incompatibility>;

use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Student {
    pub firstname: String,
    pub surname: String,
    pub subjects: BTreeSet<usize>,
    pub incompatibilities: BTreeSet<usize>,
}

pub type StudentList = Vec<Student>;

#[derive(Clone, Debug)]
pub struct Teacher {
    pub firstname: String,
    pub surname: String,
    pub contact: String,
}

pub type TeacherList = Vec<Teacher>;

#[derive(Clone, Debug)]
pub struct Interrogation {
    pub subject: usize,
    pub teacher: usize,
    pub slots: Vec<Slot>,
}

pub type InterrogationList = Vec<Interrogation>;

#[derive(Clone, Debug)]
pub struct ValidatedData {
    subjects: SubjectList,
    teachers: TeacherList,
    incompatibilities: IncompatibilityList,
    students: StudentList,
    interrogations: InterrogationList,
}
