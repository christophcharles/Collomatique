#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

use super::time;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Subject {0} has empty students_per_slot: {1:?}")]
    SubjectWithInvalidStudentsPerSlotRange(usize, RangeInclusive<NonZeroUsize>),
    #[error("Subject {0} has the slot {1} placed after the week count ({2}) of the schedule")]
    SubjectWithSlotAfterLastWeek(usize, usize, u32),
    #[error("Subject {0} has the slot {1} overlapping next day")]
    SubjectWithSlotOverlappingNextDay(usize, usize),
    #[error("Subject {0} has invalid subject number ({2}) in slot {1}")]
    SubjectWithInvalidTeacher(usize, usize, usize),
    #[error(
        "Subject {0} has a duplicated student ({1}) found first in group {2} and in group {3}"
    )]
    SubjectWithDuplicatedStudentInGroups(usize, usize, usize, usize),
    #[error("Subject {0} has a duplicated student ({1}) found first in group {2} and unassigned")]
    SubjectWithDuplicatedStudentInGroupsAndUnassigned(usize, usize, usize),
    #[error("Subject {0} has and invalid student ({1}) in the not-assigned list")]
    SubjectWithInvalidNotAssignedStudent(usize, usize),
    #[error("Subject {0} has and invalid student ({2}) in the group {1}")]
    SubjectWithInvalidAssignedStudent(usize, usize, usize),
    #[error(
        "Subject {0} has an invalid group ({1}) which is too large given the constraint ({2:?})"
    )]
    SubjectWithTooLargeAssignedGroup(usize, usize, RangeInclusive<NonZeroUsize>),
    #[error(
        "Subject {0} has an invalid non-extensible group ({1}) which is too small given the constraint ({2:?})"
    )]
    SubjectWithTooSmallNonExtensibleGroup(usize, usize, RangeInclusive<NonZeroUsize>),
    #[error("Subject {0} has not enough groups to fit all non-assigned students within the high bound of the range {1:?}")]
    SubjectWithTooFewGroups(usize, RangeInclusive<NonZeroUsize>),
    #[error("Subject {0} has too many groups to satisfy the low bound on the range {1:?}")]
    SubjectWithTooManyGroups(usize, RangeInclusive<NonZeroUsize>),
    #[error("Subject {0} has a larger periodicity than the number of weeks {1}. A full period is needed for the algorithm to work")]
    SubjectWithPeriodicityTooBig(u32, u32),
    #[error("Student {0} references an invalid incompatibility number ({1})")]
    StudentWithInvalidIncompatibility(usize, usize),
    #[error("Incompatibility {0} has slot {1} after the week count ({2}) of the schedule")]
    IncompatibilityWithSlotAfterLastWeek(usize, usize, u32),
    #[error("Incompatibility {0} has interrogation slot {1} overlapping next day")]
    IncompatibilityWithSlotOverlappingNextDay(usize, usize),
    #[error("The slot grouping {0} has an invalid slot ref {1:?} with invalid subject reference")]
    SlotGroupingWithInvalidSubject(usize, SlotRef),
    #[error("The slot grouping {0} has an invalid slot ref {1:?} with invalid slot reference")]
    SlotGroupingWithInvalidSlot(usize, SlotRef),
    #[error("The slot groupings {0} and {1} refer to the same slot {2:?}")]
    SlotGroupingOverlap(usize, usize, SlotRef),
    #[error("The grouping incompatibility {0} does not have enough groupings (only {1})")]
    SlotGroupingIncompatDoesNotHaveEnoughGroupings(usize, usize),
    #[error("The grouping incompatibility {0} has an invalid slot grouping reference {1}")]
    SlotGroupingIncompatWithInvalidSlotGrouping(usize, usize),
    #[error(
        "The grouping incompatibility {0} limit ({1}) is larger than the number of groupings ({2})"
    )]
    SlotGroupingIncompatWithLimitTooBig(usize, usize, usize),
    #[error("The range {0:?} for the number of interrogations per week is empty")]
    SlotGeneralDataWithInvalidInterrogationsPerWeek(std::ops::Range<u32>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotStart {
    pub week: u32,
    pub weekday: time::Weekday,
    pub start_time: time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotWithDuration {
    pub start: SlotStart,
    pub duration: NonZeroU32,
}

impl SlotWithDuration {
    pub fn end_time(&self) -> time::Time {
        self.start.start_time.add(self.duration.get() - 1).unwrap()
    }

    pub fn overlap_with(&self, other: &SlotWithDuration) -> bool {
        if self.start.week != other.start.week {
            return false;
        }
        if self.start.weekday != other.start.weekday {
            return false;
        }

        if self.start.start_time < other.start.start_time {
            self.end_time() >= other.start.start_time
        } else {
            self.start.start_time < other.end_time()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotWithTeacher {
    pub teacher: usize,
    pub start: SlotStart,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupDesc {
    pub students: BTreeSet<usize>,
    pub can_be_extended: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GroupsDesc {
    pub prefilled_groups: Vec<GroupDesc>,
    pub not_assigned: BTreeSet<usize>,
}

impl GroupsDesc {
    fn students_iterator(&self) -> impl Iterator<Item = &usize> {
        self.prefilled_groups
            .iter()
            .flat_map(|group| group.students.iter())
            .chain(self.not_assigned.iter())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingRequirements {
    pub teachers: bool,
    pub timeslots: bool,
}

impl Default for BalancingRequirements {
    fn default() -> Self {
        BalancingRequirements {
            teachers: false,
            timeslots: false,
        }
    }
}

impl BalancingRequirements {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    pub max_groups_per_slot: NonZeroUsize,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub is_tutorial: bool,
    pub balancing_requirements: BalancingRequirements,
    pub duration: NonZeroU32,
    pub slots: Vec<SlotWithTeacher>,
    pub groups: GroupsDesc,
}

impl Default for Subject {
    fn default() -> Self {
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            balancing_requirements: BalancingRequirements::default(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![],
            groups: GroupsDesc::default(),
        }
    }
}

pub type SubjectList = Vec<Subject>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotRef {
    pub subject: usize,
    pub slot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotGrouping {
    pub slots: BTreeSet<SlotRef>,
}

pub type SlotGroupingList = Vec<SlotGrouping>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotGroupingIncompat {
    pub groupings: BTreeSet<usize>,
    pub max_count: NonZeroUsize,
}

pub type SlotGroupingIncompatSet = BTreeSet<SlotGroupingIncompat>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility {
    pub slots: Vec<SlotWithDuration>,
}

pub type IncompatibilityList = Vec<Incompatibility>;

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Student {
    pub incompatibilities: BTreeSet<usize>,
}

pub type StudentList = Vec<Student>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralData {
    pub teacher_count: usize,
    pub week_count: NonZeroU32,
    pub interrogations_per_week: Option<std::ops::Range<u32>>,
    pub max_interrogations_per_day: Option<NonZeroU32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedData {
    general: GeneralData,
    subjects: SubjectList,
    incompatibilities: IncompatibilityList,
    students: StudentList,
    slot_groupings: SlotGroupingList,
    slot_grouping_incompats: SlotGroupingIncompatSet,
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
        grouping_incompats: SlotGroupingIncompatSet,
    ) -> Result<ValidatedData> {
        for (i, subject) in subjects.iter().enumerate() {
            if subject.period.get() > general.week_count.get() {
                return Err(Error::SubjectWithPeriodicityTooBig(
                    subject.period.get(),
                    general.week_count.get(),
                ));
            }

            if subject.students_per_group.is_empty() {
                return Err(Error::SubjectWithInvalidStudentsPerSlotRange(
                    i,
                    subject.students_per_group.clone(),
                ));
            }

            for (j, slot) in subject.slots.iter().enumerate() {
                if slot.teacher >= general.teacher_count {
                    return Err(Error::SubjectWithInvalidTeacher(i, j, slot.teacher));
                }
                if !Self::validate_slot_start(&general, &slot.start) {
                    return Err(Error::SubjectWithSlotAfterLastWeek(
                        i,
                        j,
                        general.week_count.get(),
                    ));
                }
                if !Self::validate_slot_overlap(&slot.start, subject.duration) {
                    return Err(Error::SubjectWithSlotOverlappingNextDay(i, j));
                }
            }

            for (j, group) in subject.groups.prefilled_groups.iter().enumerate() {
                for k in &group.students {
                    if *k >= students.len() {
                        return Err(Error::SubjectWithInvalidAssignedStudent(i, j, *k));
                    }
                }
            }

            for j in &subject.groups.not_assigned {
                if *j >= students.len() {
                    return Err(Error::SubjectWithInvalidNotAssignedStudent(i, *j));
                }
            }

            let mut remaining_seats = 0usize;
            for group in &subject.groups.prefilled_groups {
                if !group.can_be_extended {
                    continue;
                }
                if group.students.len() >= subject.students_per_group.end().get() {
                    continue;
                }
                remaining_seats += subject.students_per_group.end().get() - group.students.len();
            }
            if subject.groups.not_assigned.len() > remaining_seats {
                return Err(Error::SubjectWithTooFewGroups(
                    i,
                    subject.students_per_group.clone(),
                ));
            }

            let mut min_seats = 0usize;
            for group in &subject.groups.prefilled_groups {
                if !group.can_be_extended {
                    continue;
                }
                if group.students.len() >= subject.students_per_group.start().get() {
                    continue;
                }
                min_seats += subject.students_per_group.start().get() - group.students.len();
            }
            if subject.groups.not_assigned.len() < min_seats {
                return Err(Error::SubjectWithTooManyGroups(
                    i,
                    subject.students_per_group.clone(),
                ));
            }

            let mut students_no_duplicate = BTreeMap::new();

            for (j, group) in subject.groups.prefilled_groups.iter().enumerate() {
                for k in &group.students {
                    if let Some(first_j) = students_no_duplicate.get(k) {
                        return Err(Error::SubjectWithDuplicatedStudentInGroups(
                            i, *k, *first_j, j,
                        ));
                    } else {
                        students_no_duplicate.insert(*k, j);
                    }
                }
                if group.students.len() > subject.students_per_group.end().get() {
                    return Err(Error::SubjectWithTooLargeAssignedGroup(
                        i,
                        j,
                        subject.students_per_group.clone(),
                    ));
                }
                if group.students.len() < subject.students_per_group.start().get()
                    && !group.can_be_extended
                {
                    return Err(Error::SubjectWithTooSmallNonExtensibleGroup(
                        i,
                        j,
                        subject.students_per_group.clone(),
                    ));
                }
            }

            for k in &subject.groups.not_assigned {
                if let Some(j) = students_no_duplicate.get(&k) {
                    return Err(Error::SubjectWithDuplicatedStudentInGroupsAndUnassigned(
                        i, *k, *j,
                    ));
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
            for &incompatibility in &student.incompatibilities {
                if incompatibility >= incompatibilities.len() {
                    return Err(Error::StudentWithInvalidIncompatibility(i, incompatibility));
                }
            }
        }

        let mut slot_grouping_previous_refs = BTreeMap::new();
        for (i, slot_grouping) in slot_groupings.iter().enumerate() {
            for slot_ref in &slot_grouping.slots {
                if slot_ref.subject >= subjects.len() {
                    return Err(Error::SlotGroupingWithInvalidSubject(i, slot_ref.clone()));
                }
                if slot_ref.slot >= subjects[slot_ref.subject].slots.len() {
                    return Err(Error::SlotGroupingWithInvalidSlot(i, slot_ref.clone()));
                }
                match slot_grouping_previous_refs.get(slot_ref) {
                    Some(j) => return Err(Error::SlotGroupingOverlap(*j, i, slot_ref.clone())),
                    None => {
                        slot_grouping_previous_refs.insert(slot_ref, i);
                    }
                }
            }
        }

        for (i, grouping_incompat) in grouping_incompats.iter().enumerate() {
            for grouping in grouping_incompat.groupings.iter().copied() {
                if grouping >= slot_groupings.len() {
                    return Err(Error::SlotGroupingIncompatWithInvalidSlotGrouping(
                        i, grouping,
                    ));
                }
            }
            if grouping_incompat.groupings.len() < 2 {
                return Err(Error::SlotGroupingIncompatDoesNotHaveEnoughGroupings(
                    i,
                    grouping_incompat.groupings.len(),
                ));
            }
            if grouping_incompat.max_count.get() >= grouping_incompat.groupings.len() {
                return Err(Error::SlotGroupingIncompatWithLimitTooBig(
                    i,
                    grouping_incompat.max_count.get(),
                    grouping_incompat.groupings.len(),
                ));
            }
        }

        if let Some(interrogations_range) = general.interrogations_per_week.clone() {
            if interrogations_range.is_empty() {
                return Err(Error::SlotGeneralDataWithInvalidInterrogationsPerWeek(
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
            slot_grouping_incompats: grouping_incompats,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variable {
    GroupInSlot {
        subject: usize,
        slot: usize,
        group: usize,
    },
    StudentNotInLastPeriod {
        subject: usize,
        student: usize,
    },
    DynamicGroupAssignment {
        subject: usize,
        slot: usize,
        group: usize,
        student: usize,
    },
    StudentInGroup {
        subject: usize,
        student: usize,
        group: usize,
    },
    Periodicity {
        subject: usize,
        student: usize,
        week_modulo: u32,
    },
    UseGrouping(usize),
}

impl<'a> From<&'a Variable> for Variable {
    fn from(value: &'a Variable) -> Self {
        value.clone()
    }
}

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variable::GroupInSlot {
                subject,
                slot,
                group,
            } => write!(f, "GiS_{}_{}_{}", *subject, *slot, *group),
            Variable::StudentNotInLastPeriod { subject, student } => {
                write!(f, "SniLP_{}_{}", *subject, *student)
            }
            Variable::DynamicGroupAssignment {
                subject,
                slot,
                group,
                student,
            } => write!(f, "DGA_{}_{}_{}_{}", *subject, *slot, *group, *student),
            Variable::StudentInGroup {
                subject,
                student,
                group,
            } => write!(f, "SiG_{}_{}_{}", *subject, *student, *group),
            Variable::Periodicity {
                subject,
                student,
                week_modulo,
            } => write!(f, "P_{}_{}_{}", *subject, *student, *week_modulo),
            Variable::UseGrouping(num) => write!(f, "UG_{}", num),
        }
    }
}

impl ValidatedData {
    pub fn ilp_translator<'a>(&'a self) -> IlpTranslator<'a> {
        IlpTranslator {
            data: self,
            problem_builder_cache: RefCell::new(None),
        }
    }
}

#[derive(Clone, Debug)]
pub struct IlpTranslator<'a> {
    data: &'a ValidatedData,
    problem_builder_cache: RefCell<Option<ProblemBuilder<Variable>>>,
}

use crate::ilp::initializers::ConfigInitializer;
use crate::ilp::linexpr::{Constraint, Expr};
use crate::ilp::solvers::FeasabilitySolver;
use crate::ilp::{Config, DefaultRepr, FeasableConfig, Problem, ProblemBuilder};

pub trait GenericInitializer: ConfigInitializer<Variable, DefaultRepr<Variable>> {}

impl<T: ConfigInitializer<Variable, DefaultRepr<Variable>>> GenericInitializer for T {}

pub trait GenericSolver: FeasabilitySolver<Variable, DefaultRepr<Variable>> {}

impl<S: FeasabilitySolver<Variable, DefaultRepr<Variable>>> GenericSolver for S {}

enum StudentStatus {
    Assigned(usize),
    ToBeAssigned(BTreeSet<usize>),
    NotConcerned,
}

trait BalancingData: Clone + PartialEq + Eq + PartialOrd + Ord {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self;

    fn is_slot_relevant(&self, slot: &SlotWithTeacher) -> bool {
        let other = Self::from_subject_slot(slot);
        *self == other
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TeacherBalancing {
    teacher: usize,
}

impl BalancingData for TeacherBalancing {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self {
        TeacherBalancing {
            teacher: slot.teacher,
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TimeslotBalancing {
    weekday: time::Weekday,
    time: time::Time,
}

impl BalancingData for TimeslotBalancing {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self {
        TimeslotBalancing {
            weekday: slot.start.weekday,
            time: slot.start.start_time.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TeacherAndTimeslotBalancing {
    teacher: usize,
    weekday: time::Weekday,
    time: time::Time,
}

impl BalancingData for TeacherAndTimeslotBalancing {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self {
        TeacherAndTimeslotBalancing {
            teacher: slot.teacher,
            weekday: slot.start.weekday,
            time: slot.start.start_time.clone(),
        }
    }
}

impl<'a> IlpTranslator<'a> {
    fn is_group_fixed(group: &GroupDesc, subject: &Subject) -> bool {
        !group.can_be_extended || (group.students.len() == subject.students_per_group.end().get())
    }

    fn compute_needed_time_resolution(&self) -> u32 {
        let mut result = 24 * 60;

        use crate::math::gcd;

        for subject in &self.data.subjects {
            result = gcd(result, subject.duration.get());
            for slot in &subject.slots {
                result = gcd(result, slot.start.start_time.get())
            }
        }

        for incompatibility in &self.data.incompatibilities {
            for slot in &incompatibility.slots {
                result = gcd(result, slot.duration.get());
                result = gcd(result, slot.start.start_time.get());
            }
        }

        result
    }

    fn is_last_period_incomplete(&self, subject: &Subject) -> bool {
        self.data.general.week_count.get() % subject.period.get() != 0
    }

    fn subject_needs_periodicity_variables(&self, subject: &Subject) -> bool {
        subject.period_is_strict || self.is_last_period_incomplete(subject)
    }

    fn build_group_in_slot_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject
                    .slots
                    .iter()
                    .enumerate()
                    .flat_map(move |(j, _slot)| {
                        subject.groups.prefilled_groups.iter().enumerate().map(
                            move |(k, _group)| Variable::GroupInSlot {
                                subject: i,
                                slot: j,
                                group: k,
                            },
                        )
                    })
            })
            .collect()
    }

    fn build_dynamic_group_assignment_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject.groups.not_assigned.iter().flat_map(move |l| {
                    subject
                        .slots
                        .iter()
                        .enumerate()
                        .flat_map(move |(j, _slot)| {
                            subject
                                .groups
                                .prefilled_groups
                                .iter()
                                .enumerate()
                                .filter_map(move |(k, group)| {
                                    if Self::is_group_fixed(group, subject) {
                                        return None;
                                    }

                                    Some(Variable::DynamicGroupAssignment {
                                        subject: i,
                                        slot: j,
                                        group: k,
                                        student: *l,
                                    })
                                })
                        })
                })
            })
            .collect()
    }

    fn build_student_in_group_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject.groups.not_assigned.iter().flat_map(move |j| {
                    subject
                        .groups
                        .prefilled_groups
                        .iter()
                        .enumerate()
                        .filter_map(move |(k, group)| {
                            if Self::is_group_fixed(group, subject) {
                                return None;
                            }

                            Some(Variable::StudentInGroup {
                                subject: i,
                                student: *j,
                                group: k,
                            })
                        })
                })
            })
            .collect()
    }

    fn build_periodicity_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .filter_map(|(i, subject)| {
                if !self.subject_needs_periodicity_variables(subject) {
                    return None;
                }

                let student_iterator = subject.groups.students_iterator();

                Some(student_iterator.flat_map(move |j| {
                    (0..subject.period.get())
                        .into_iter()
                        .map(move |k| Variable::Periodicity {
                            subject: i,
                            student: *j,
                            week_modulo: k,
                        })
                }))
            })
            .flatten()
            .collect()
    }

    fn build_student_not_in_last_period_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .filter_map(|(i, subject)| {
                if !self.is_last_period_incomplete(subject) {
                    return None;
                }

                let student_iterator = subject.groups.students_iterator();
                Some(
                    student_iterator.map(move |k| Variable::StudentNotInLastPeriod {
                        subject: i,
                        student: *k,
                    }),
                )
            })
            .flatten()
            .collect()
    }

    fn build_use_grouping_variables(&self) -> BTreeSet<Variable> {
        self.data
            .slot_groupings
            .iter()
            .enumerate()
            .map(|(i, _grouping)| Variable::UseGrouping(i))
            .collect()
    }

    fn build_at_most_max_groups_per_slot_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject.slots.iter().enumerate().map(move |(j, _slot)| {
                    let mut expr = Expr::constant(0);

                    for (k, _group) in subject.groups.prefilled_groups.iter().enumerate() {
                        expr = expr
                            + Expr::var(Variable::GroupInSlot {
                                subject: i,
                                slot: j,
                                group: k,
                            });
                    }

                    let max_groups_per_slot = subject
                        .max_groups_per_slot
                        .get()
                        .try_into()
                        .expect("Should be less than 2^31 maximum");
                    expr.leq(&Expr::constant(max_groups_per_slot))
                })
            })
            .collect()
    }

    fn is_time_unit_in_slot(
        week: u32,
        weekday: super::time::Weekday,
        time: super::time::Time,
        _time_resolution: u32,
        subject: &Subject,
        slot: &SlotWithTeacher,
    ) -> bool {
        if week != slot.start.week {
            return false;
        }
        if weekday != slot.start.weekday {
            return false;
        }
        time.fit_in(&slot.start.start_time, subject.duration.get())
    }

    fn get_student_status(student: usize, subject: &Subject) -> StudentStatus {
        if subject.groups.not_assigned.contains(&student) {
            let list = subject
                .groups
                .prefilled_groups
                .iter()
                .enumerate()
                .filter_map(|(k, g)| {
                    if Self::is_group_fixed(g, subject) {
                        return None;
                    }
                    Some(k)
                })
                .collect();
            return StudentStatus::ToBeAssigned(list);
        }
        for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
            if group.students.contains(&student) {
                return StudentStatus::Assigned(k);
            }
        }
        StudentStatus::NotConcerned
    }

    fn build_at_most_one_interrogation_constraint_for_one_time_unit_and_one_student(
        &self,
        week: u32,
        weekday: super::time::Weekday,
        time: super::time::Time,
        time_resolution: u32,
        student: usize,
    ) -> Option<Constraint<Variable>> {
        let mut expr = Expr::<Variable>::constant(0);

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, slot) in subject.slots.iter().enumerate() {
                if Self::is_time_unit_in_slot(
                    week,
                    weekday,
                    time.clone(),
                    time_resolution,
                    subject,
                    slot,
                ) {
                    match Self::get_student_status(student, subject) {
                        StudentStatus::Assigned(k) => {
                            expr = expr
                                + Expr::var(Variable::GroupInSlot {
                                    subject: i,
                                    slot: j,
                                    group: k,
                                });
                        }
                        StudentStatus::ToBeAssigned(k_list) => {
                            for k in k_list {
                                expr = expr
                                    + Expr::var(Variable::DynamicGroupAssignment {
                                        subject: i,
                                        slot: j,
                                        group: k,
                                        student,
                                    });
                            }
                        }
                        StudentStatus::NotConcerned => {}
                    }
                }
            }
        }

        if expr == Expr::constant(0) {
            return None;
        }

        Some(expr.leq(&Expr::constant(1)))
    }

    fn build_at_most_one_interrogation_for_this_time_unit_constraints(
        &self,
        week: u32,
        weekday: super::time::Weekday,
        time: super::time::Time,
        time_resolution: u32,
    ) -> BTreeSet<Constraint<Variable>> {
        self.data
            .students
            .iter()
            .enumerate()
            .filter_map(|(student, _)| {
                self.build_at_most_one_interrogation_constraint_for_one_time_unit_and_one_student(
                    week,
                    weekday,
                    time.clone(),
                    time_resolution,
                    student,
                )
            })
            .collect()
    }

    fn build_at_most_one_interrogation_per_time_unit_constraints(
        &self,
    ) -> BTreeSet<Constraint<Variable>> {
        let time_resolution = self.compute_needed_time_resolution();

        let mut output = BTreeSet::new();

        for week in 0u32..self.data.general.week_count.get() {
            for weekday in super::time::Weekday::iter() {
                let init_time = super::time::Time::from_hm(0, 0).unwrap();
                for time in init_time.iterate_until_end_of_day(time_resolution) {
                    output.extend(
                        self.build_at_most_one_interrogation_for_this_time_unit_constraints(
                            week,
                            weekday,
                            time,
                            time_resolution,
                        ),
                    );
                }
            }
        }

        output
    }

    fn build_one_interrogation_per_period_contraint_for_not_assigned_student(
        &self,
        i: usize,
        subject: &Subject,
        period: std::ops::Range<u32>,
        student: usize,
    ) -> Constraint<Variable> {
        let mut expr = Expr::constant(0);

        for (j, slot) in subject.slots.iter().enumerate() {
            if period.contains(&slot.start.week) {
                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    if !Self::is_group_fixed(group, subject) {
                        expr = expr
                            + Expr::var(Variable::DynamicGroupAssignment {
                                subject: i,
                                slot: j,
                                group: k,
                                student,
                            });
                    }
                }
            }
        }

        let current_period_length = period.end - period.start;

        assert!(current_period_length <= subject.period.get());
        if current_period_length < subject.period.get() {
            expr = expr
                + Expr::var(Variable::StudentNotInLastPeriod {
                    subject: i,
                    student,
                });
        }

        expr.eq(&Expr::constant(1))
    }

    fn build_one_interrogation_per_period_contraint_for_assigned_student(
        &self,
        i: usize,
        subject: &Subject,
        period: std::ops::Range<u32>,
        k: usize,
        _group: &GroupDesc,
        student: usize,
    ) -> Constraint<Variable> {
        let mut expr = Expr::constant(0);

        for (j, slot) in subject.slots.iter().enumerate() {
            if period.contains(&slot.start.week) {
                expr = expr
                    + Expr::var(Variable::GroupInSlot {
                        subject: i,
                        slot: j,
                        group: k,
                    });
            }
        }

        let current_period_length = period.end - period.start;

        assert!(current_period_length <= subject.period.get());
        if current_period_length < subject.period.get() {
            expr = expr
                + Expr::var(Variable::StudentNotInLastPeriod {
                    subject: i,
                    student,
                });
        }

        expr.eq(&Expr::constant(1))
    }

    fn build_one_interrogation_per_period_contraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            let whole_period_count = self.data.general.week_count.get() / subject.period.get();
            for p in 0..whole_period_count {
                let start = p * subject.period.get();
                let period = start..(start + subject.period.get());

                for student in subject.groups.not_assigned.iter().copied() {
                    constraints.insert(
                        self.build_one_interrogation_per_period_contraint_for_not_assigned_student(
                            i,
                            subject,
                            period.clone(),
                            student,
                        ),
                    );
                }

                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    for student in group.students.iter().copied() {
                        constraints.insert(
                            self.build_one_interrogation_per_period_contraint_for_assigned_student(
                                i,
                                subject,
                                period.clone(),
                                k,
                                group,
                                student,
                            ),
                        );
                    }
                }
            }

            let there_is_an_incomplete_period =
                self.data.general.week_count.get() % subject.period.get() != 0;
            if there_is_an_incomplete_period {
                let p = self.data.general.week_count.get() / subject.period.get();
                let start = p * subject.period.get();
                let period = start..self.data.general.week_count.get();

                for student in subject.groups.not_assigned.iter().copied() {
                    constraints.insert(
                        self.build_one_interrogation_per_period_contraint_for_not_assigned_student(
                            i,
                            subject,
                            period.clone(),
                            student,
                        ),
                    );
                }

                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    for student in group.students.iter().copied() {
                        constraints.insert(
                            self.build_one_interrogation_per_period_contraint_for_assigned_student(
                                i,
                                subject,
                                period.clone(),
                                k,
                                group,
                                student,
                            ),
                        );
                    }
                }
            }
        }

        constraints
    }

    fn build_at_most_one_interrogation_per_period_for_empty_groups_contraint_for_group(
        &self,
        i: usize,
        subject: &Subject,
        period: std::ops::Range<u32>,
        k: usize,
        _group: &GroupDesc,
    ) -> Constraint<Variable> {
        let mut expr = Expr::constant(0);

        for (j, slot) in subject.slots.iter().enumerate() {
            if period.contains(&slot.start.week) {
                expr = expr
                    + Expr::var(Variable::GroupInSlot {
                        subject: i,
                        slot: j,
                        group: k,
                    });
            }
        }

        expr.leq(&Expr::constant(1))
    }

    fn build_at_most_one_interrogation_per_period_for_empty_groups_contraints(
        &self,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            let period_count = (self.data.general.week_count.get() + subject.period.get() - 1)
                / subject.period.get();
            for p in 0..period_count {
                let start = p * subject.period.get();
                let end = (start + subject.period.get()).min(self.data.general.week_count.get());
                let period = start..end;

                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    if !group.students.is_empty() {
                        continue;
                    }
                    constraints.insert(
                        self.build_at_most_one_interrogation_per_period_for_empty_groups_contraint_for_group(
                            i,
                            subject,
                            period.clone(),
                            k,
                            group,
                        ),
                    );
                }
            }
        }

        constraints
    }

    fn build_students_per_group_lhs_for_group(
        &self,
        i: usize,
        subject: &Subject,
        k: usize,
    ) -> Expr<Variable> {
        let mut expr = Expr::constant(0);
        for student in subject.groups.not_assigned.iter().copied() {
            expr = expr
                + Expr::var(Variable::StudentInGroup {
                    subject: i,
                    student,
                    group: k,
                });
        }
        expr
    }

    fn build_students_per_group_lower_bound_constraint_for_group(
        &self,
        i: usize,
        subject: &Subject,
        k: usize,
        group: &GroupDesc,
    ) -> Option<Constraint<Variable>> {
        let min = subject.students_per_group.start().get();
        if min <= group.students.len() {
            return None;
        }

        let min_i32: i32 = (min - group.students.len())
            .try_into()
            .expect("Should be less than 2^31 minimum");
        let lhs = self.build_students_per_group_lhs_for_group(i, subject, k);
        Some(lhs.geq(&Expr::constant(min_i32)))
    }

    fn build_students_per_group_upper_bound_constraint_for_group(
        &self,
        i: usize,
        subject: &Subject,
        k: usize,
        group: &GroupDesc,
    ) -> Constraint<Variable> {
        let max = subject.students_per_group.end().get();
        assert!(group.students.len() <= max);

        let max_i32: i32 = (max - group.students.len())
            .try_into()
            .expect("Should be less than 2^31 maximum");
        let lhs = self.build_students_per_group_lhs_for_group(i, subject, k);
        lhs.leq(&Expr::constant(max_i32))
    }

    fn build_students_per_group_count_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                if !Self::is_group_fixed(group, subject) {
                    constraints.extend(
                        self.build_students_per_group_lower_bound_constraint_for_group(
                            i, subject, k, group,
                        ),
                    );
                    constraints.insert(
                        self.build_students_per_group_upper_bound_constraint_for_group(
                            i, subject, k, group,
                        ),
                    );
                }
            }
        }

        constraints
    }

    fn build_student_in_single_group_constraint_for_student(
        &self,
        i: usize,
        subject: &Subject,
        student: usize,
    ) -> Constraint<Variable> {
        let mut expr = Expr::constant(0);

        for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
            if !Self::is_group_fixed(group, subject) {
                expr = expr
                    + Expr::var(Variable::StudentInGroup {
                        subject: i,
                        student,
                        group: k,
                    });
            }
        }

        expr.eq(&Expr::constant(1))
    }

    fn build_student_in_single_group_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for student in subject.groups.not_assigned.iter().copied() {
                constraints.insert(
                    self.build_student_in_single_group_constraint_for_student(i, subject, student),
                );
            }
        }

        constraints
    }

    fn build_dynamic_group_student_in_group_constraint_for_case(
        &self,
        i: usize,
        j: usize,
        k: usize,
        student: usize,
    ) -> Constraint<Variable> {
        let lhs = Expr::var(Variable::DynamicGroupAssignment {
            subject: i,
            slot: j,
            group: k,
            student,
        });
        let rhs = Expr::var(Variable::StudentInGroup {
            subject: i,
            group: k,
            student,
        });

        lhs.leq(&rhs)
    }

    fn build_dynamic_groups_student_in_group_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, _slot) in subject.slots.iter().enumerate() {
                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    if !Self::is_group_fixed(group, subject) {
                        for student in subject.groups.not_assigned.iter().copied() {
                            constraints.insert(
                                self.build_dynamic_group_student_in_group_constraint_for_case(
                                    i, j, k, student,
                                ),
                            );
                        }
                    }
                }
            }
        }

        constraints
    }

    fn build_dynamic_groups_group_in_slot_constraint_for_case(
        &self,
        i: usize,
        j: usize,
        k: usize,
        student: usize,
    ) -> Constraint<Variable> {
        let lhs = Expr::var(Variable::DynamicGroupAssignment {
            subject: i,
            slot: j,
            group: k,
            student,
        });
        let rhs = Expr::var(Variable::GroupInSlot {
            subject: i,
            slot: j,
            group: k,
        });

        lhs.leq(&rhs)
    }

    fn build_dynamic_groups_group_in_slot_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, _slot) in subject.slots.iter().enumerate() {
                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    if !Self::is_group_fixed(group, subject) {
                        for student in subject.groups.not_assigned.iter().copied() {
                            constraints.insert(
                                self.build_dynamic_groups_group_in_slot_constraint_for_case(
                                    i, j, k, student,
                                ),
                            );
                        }
                    }
                }
            }
        }

        constraints
    }

    fn build_one_periodicity_choice_per_student_constraint_for_student(
        &self,
        i: usize,
        subject: &Subject,
        student: usize,
    ) -> Constraint<Variable> {
        let mut expr = Expr::constant(0);

        for week_modulo in 0..subject.period.get() {
            expr = expr
                + Expr::var(Variable::Periodicity {
                    subject: i,
                    student,
                    week_modulo,
                });
        }

        expr.eq(&Expr::constant(1))
    }

    fn build_one_periodicity_choice_per_student_constraints(
        &self,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            if !self.subject_needs_periodicity_variables(subject) {
                continue;
            }
            for student in subject.groups.students_iterator().copied() {
                constraints.insert(
                    self.build_one_periodicity_choice_per_student_constraint_for_student(
                        i, subject, student,
                    ),
                );
            }
        }

        constraints
    }

    fn is_periodicity_inequality_needed(&self, subject: &Subject, slot: &SlotWithTeacher) -> bool {
        if subject.period_is_strict {
            return true;
        }
        if !self.is_last_period_incomplete(subject) {
            return false;
        }

        let full_period_count = self.data.general.week_count.get() / subject.period.get();
        let period_number = slot.start.week / subject.period.get();

        period_number + 1 >= full_period_count
    }

    fn build_periodicity_constraint_for_assigned_student(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        slot: &SlotWithTeacher,
        k: usize,
        student: usize,
    ) -> Constraint<Variable> {
        let week_modulo = slot.start.week % subject.period.get();

        let lhs = Expr::var(Variable::GroupInSlot {
            subject: i,
            slot: j,
            group: k,
        });
        let rhs = Expr::var(Variable::Periodicity {
            subject: i,
            student,
            week_modulo,
        });

        lhs.leq(&rhs)
    }

    fn build_periodicity_constraint_for_not_assigned_student(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        slot: &SlotWithTeacher,
        k: usize,
        student: usize,
    ) -> Constraint<Variable> {
        let week_modulo = slot.start.week % subject.period.get();

        let lhs = Expr::var(Variable::DynamicGroupAssignment {
            subject: i,
            slot: j,
            group: k,
            student,
        });
        let rhs = Expr::var(Variable::Periodicity {
            subject: i,
            student,
            week_modulo,
        });

        lhs.leq(&rhs)
    }

    fn build_periodicity_constraint_for_incomplete_period(
        &self,
        i: usize,
        subject: &Subject,
        student: usize,
    ) -> Constraint<Variable> {
        let lhs = Expr::var(Variable::StudentNotInLastPeriod {
            subject: i,
            student,
        });

        let mut rhs = Expr::constant(0);
        let start = self.data.general.week_count.get() % subject.period.get();
        let end = subject.period.get();
        for week_modulo in start..end {
            rhs = rhs
                + Expr::var(Variable::Periodicity {
                    subject: i,
                    student,
                    week_modulo,
                });
        }

        lhs.leq(&rhs)
    }

    fn build_periodicity_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, slot) in subject.slots.iter().enumerate() {
                if !self.is_periodicity_inequality_needed(subject, slot) {
                    continue;
                }

                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    for student in group.students.iter().copied() {
                        constraints.insert(self.build_periodicity_constraint_for_assigned_student(
                            i, subject, j, slot, k, student,
                        ));
                    }
                }

                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    if Self::is_group_fixed(group, subject) {
                        continue;
                    }
                    for student in subject.groups.not_assigned.iter().copied() {
                        constraints.insert(
                            self.build_periodicity_constraint_for_not_assigned_student(
                                i, subject, j, slot, k, student,
                            ),
                        );
                    }
                }
            }

            if self.is_last_period_incomplete(subject) {
                for student in subject.groups.students_iterator().copied() {
                    constraints.insert(
                        self.build_periodicity_constraint_for_incomplete_period(
                            i, subject, student,
                        ),
                    );
                }
            }
        }

        constraints
    }

    fn build_interrogations_per_week_constraints_for_student(
        &self,
        range: &std::ops::Range<u32>,
        student: usize,
        week: u32,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut expr = Expr::constant(0);

        for (i, subject) in self.data.subjects.iter().enumerate() {
            if subject.is_tutorial {
                // ignore tutorial sessions for interrogation count
                continue;
            }
            for (j, slot) in subject.slots.iter().enumerate() {
                if slot.start.week != week {
                    continue;
                }

                if subject.groups.not_assigned.contains(&student) {
                    for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                        if Self::is_group_fixed(group, subject) {
                            continue;
                        }
                        expr = expr
                            + Expr::var(Variable::DynamicGroupAssignment {
                                subject: i,
                                slot: j,
                                group: k,
                                student,
                            });
                    }
                } else {
                    for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                        if group.students.contains(&student) {
                            expr = expr
                                + Expr::var(Variable::GroupInSlot {
                                    subject: i,
                                    slot: j,
                                    group: k,
                                });
                        }
                    }
                }
            }
        }

        let min = i32::try_from(range.start).unwrap();
        let max = i32::try_from(range.end).unwrap() - 1;

        if min != max {
            BTreeSet::from([
                expr.leq(&Expr::constant(max)),
                expr.geq(&Expr::constant(min)),
            ])
        } else {
            BTreeSet::from([expr.eq(&Expr::constant(max))])
        }
    }

    fn build_interrogations_per_week_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        let range = match &self.data.general.interrogations_per_week {
            Some(r) => r,
            None => return constraints,
        };

        for week in 0..self.data.general.week_count.get() {
            for (student, _) in self.data.students.iter().enumerate() {
                constraints.extend(
                    self.build_interrogations_per_week_constraints_for_student(
                        range, student, week,
                    ),
                );
            }
        }

        constraints
    }

    fn build_max_interrogations_per_day_constraints_for_student(
        &self,
        max_count: NonZeroU32,
        student: usize,
        week: u32,
        day: time::Weekday,
    ) -> Option<Constraint<Variable>> {
        let mut expr = Expr::constant(0);

        for (i, subject) in self.data.subjects.iter().enumerate() {
            if subject.is_tutorial {
                // ignore tutorial sessions for interrogation count
                continue;
            }
            for (j, slot) in subject.slots.iter().enumerate() {
                if slot.start.week != week {
                    continue;
                }
                if slot.start.weekday != day {
                    continue;
                }

                if subject.groups.not_assigned.contains(&student) {
                    for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                        if Self::is_group_fixed(group, subject) {
                            continue;
                        }
                        expr = expr
                            + Expr::var(Variable::DynamicGroupAssignment {
                                subject: i,
                                slot: j,
                                group: k,
                                student,
                            });
                    }
                } else {
                    for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                        if group.students.contains(&student) {
                            expr = expr
                                + Expr::var(Variable::GroupInSlot {
                                    subject: i,
                                    slot: j,
                                    group: k,
                                });
                        }
                    }
                }
            }
        }

        if expr == Expr::constant(0) {
            return None;
        }

        let max_i32 = i32::try_from(max_count.get()).unwrap();
        Some(expr.leq(&Expr::constant(max_i32)))
    }

    fn build_max_interrogations_per_day_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        let max_count = match self.data.general.max_interrogations_per_day {
            Some(r) => r,
            None => return constraints,
        };

        for week in 0..self.data.general.week_count.get() {
            for day in time::Weekday::iter() {
                for (student, _) in self.data.students.iter().enumerate() {
                    constraints.extend(
                        self.build_max_interrogations_per_day_constraints_for_student(
                            max_count, student, week, day,
                        ),
                    );
                }
            }
        }

        constraints
    }

    fn build_grouping_constraint_for_group_in_slot(
        &self,
        i: usize,
        slot: &SlotRef,
        k: usize,
    ) -> Constraint<Variable> {
        let lhs = Expr::var(Variable::GroupInSlot {
            subject: slot.subject,
            slot: slot.slot,
            group: k,
        });
        let rhs = Expr::var(Variable::UseGrouping(i));

        lhs.leq(&rhs)
    }

    fn build_grouping_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, slot_grouping) in self.data.slot_groupings.iter().enumerate() {
            for slot in &slot_grouping.slots {
                let subject = &self.data.subjects[slot.subject];
                for (k, _group) in subject.groups.prefilled_groups.iter().enumerate() {
                    constraints
                        .insert(self.build_grouping_constraint_for_group_in_slot(i, slot, k));
                }
            }
        }

        constraints
    }

    fn build_grouping_incompats_constraint_for_incompat(
        &self,
        grouping_incompat: &SlotGroupingIncompat,
    ) -> Constraint<Variable> {
        let mut lhs = Expr::constant(0);

        for grouping in grouping_incompat.groupings.iter().copied() {
            lhs = lhs + Expr::var(Variable::UseGrouping(grouping));
        }

        let max_count_i32 = i32::try_from(grouping_incompat.max_count.get()).unwrap();

        lhs.leq(&Expr::constant(max_count_i32))
    }

    fn build_grouping_incompats_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for grouping_incompat in &self.data.slot_grouping_incompats {
            constraints
                .insert(self.build_grouping_incompats_constraint_for_incompat(grouping_incompat));
        }

        constraints
    }

    fn is_slot_compatible_with_student(
        &self,
        slot_start: &SlotStart,
        duration: NonZeroU32,
        student: usize,
    ) -> bool {
        for incompat in self.data.students[student]
            .incompatibilities
            .iter()
            .copied()
        {
            for slot in self.data.incompatibilities[incompat].slots.iter() {
                if slot.overlap_with(&SlotWithDuration {
                    start: slot_start.clone(),
                    duration,
                }) {
                    return false;
                }
            }
        }
        true
    }

    fn build_students_incompats_constraint_for_group(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        slot: &SlotWithTeacher,
        k: usize,
        group: &GroupDesc,
    ) -> Option<Constraint<Variable>> {
        let mut ok = true;
        for student in group.students.iter().copied() {
            if !self.is_slot_compatible_with_student(&slot.start, subject.duration, student) {
                ok = false;
            }
        }
        if ok {
            return None;
        }

        let lhs = Expr::var(Variable::GroupInSlot {
            subject: i,
            slot: j,
            group: k,
        });
        Some(lhs.eq(&Expr::constant(0)))
    }

    fn build_students_incompats_constraint_for_dynamic_student(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        slot: &SlotWithTeacher,
        k: usize,
        student: usize,
    ) -> Option<Constraint<Variable>> {
        if self.is_slot_compatible_with_student(&slot.start, subject.duration, student) {
            return None;
        }

        let lhs = Expr::var(Variable::DynamicGroupAssignment {
            subject: i,
            slot: j,
            group: k,
            student,
        });
        Some(lhs.eq(&Expr::constant(0)))
    }

    fn build_students_incompats_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, slot) in subject.slots.iter().enumerate() {
                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    constraints.extend(self.build_students_incompats_constraint_for_group(
                        i, subject, j, slot, k, group,
                    ));
                }
            }
        }

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, slot) in subject.slots.iter().enumerate() {
                for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                    if Self::is_group_fixed(group, subject) {
                        continue;
                    }
                    for student in subject.groups.not_assigned.iter().copied() {
                        constraints.extend(
                            self.build_students_incompats_constraint_for_dynamic_student(
                                i, subject, j, slot, k, student,
                            ),
                        );
                    }
                }
            }
        }

        constraints
    }

    fn build_max_min_balancing_from_expr(
        &self,
        subject: &Subject,
        count: usize,
        expr: Expr<Variable>,
    ) -> BTreeSet<Constraint<Variable>> {
        let group_count = subject.groups.prefilled_groups.len();
        let week_count = usize::try_from(self.data.general.week_count.get()).unwrap();
        let period = usize::try_from(subject.period.get()).unwrap();
        let needed_slots = (group_count * week_count) as f64 / (period as f64);

        let updated_count = (count as f64) * needed_slots / (subject.slots.len() as f64);

        let expected_use_per_group = updated_count / (group_count as f64);

        let max_use = expected_use_per_group.ceil() as i32;
        let min_use = expected_use_per_group.floor() as i32;

        if max_use == min_use {
            BTreeSet::from([expr.eq(&Expr::constant(max_use))])
        } else {
            BTreeSet::from([
                expr.leq(&Expr::constant(max_use)),
                expr.geq(&Expr::constant(min_use)),
            ])
        }
    }

    fn build_balancing_constraints_for_subject_and_not_assigned_student<T: BalancingData>(
        &self,
        i: usize,
        subject: &Subject,
        slot_type: &T,
        count: usize,
        student: usize,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut expr = Expr::constant(0);

        for (j, slot) in subject.slots.iter().enumerate() {
            if !slot_type.is_slot_relevant(slot) {
                continue;
            }

            for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                if Self::is_group_fixed(group, subject) {
                    continue;
                }
                expr = expr
                    + Expr::var(Variable::DynamicGroupAssignment {
                        subject: i,
                        slot: j,
                        group: k,
                        student,
                    });
            }
        }

        self.build_max_min_balancing_from_expr(subject, count, expr)
    }

    fn build_balancing_constraints_for_subject_and_group<T: BalancingData>(
        &self,
        i: usize,
        subject: &Subject,
        slot_type: &T,
        count: usize,
        k: usize,
        _group: &GroupDesc,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut expr = Expr::constant(0);

        for (j, slot) in subject.slots.iter().enumerate() {
            if !slot_type.is_slot_relevant(slot) {
                continue;
            }

            expr = expr
                + Expr::var(Variable::GroupInSlot {
                    subject: i,
                    slot: j,
                    group: k,
                });
        }

        self.build_max_min_balancing_from_expr(subject, count, expr)
    }

    fn build_balancing_constraints_for_subject<T: BalancingData>(
        &self,
        i: usize,
        subject: &Subject,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut counts = BTreeMap::<T, usize>::new();

        for slot in &subject.slots {
            let data = T::from_subject_slot(slot);
            match counts.get_mut(&data) {
                Some(c) => {
                    *c += 1;
                }
                None => {
                    counts.insert(data, 1);
                }
            }
        }

        let mut constraints = BTreeSet::new();

        for (slot_type, count) in counts {
            for student in subject.groups.not_assigned.iter().copied() {
                constraints.extend(
                    self.build_balancing_constraints_for_subject_and_not_assigned_student(
                        i, subject, &slot_type, count, student,
                    ),
                );
            }

            for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                if group.students.is_empty() {
                    // Ignore empty groups
                    // But groups with students assigned (the group might be fixed or dynamic, it does not matter)
                    // should have a constraint
                    continue;
                }
                constraints.extend(self.build_balancing_constraints_for_subject_and_group(
                    i, subject, &slot_type, count, k, group,
                ));
            }
        }

        constraints
    }

    fn build_balancing_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            match (
                subject.balancing_requirements.teachers,
                subject.balancing_requirements.timeslots,
            ) {
                (true, true) => {
                    constraints.extend(
                        self.build_balancing_constraints_for_subject::<TeacherAndTimeslotBalancing>(
                            i,
                            subject,
                        )
                    );
                }
                (true, false) => {
                    constraints.extend(
                        self.build_balancing_constraints_for_subject::<TeacherBalancing>(
                            i, subject,
                        ),
                    );
                }
                (false, true) => {
                    constraints.extend(
                        self.build_balancing_constraints_for_subject::<TimeslotBalancing>(
                            i, subject,
                        ),
                    );
                }
                (false, false) => {
                    // ignore, no balancing needed
                }
            }
        }

        constraints
    }

    fn problem_builder_internal(&self) -> ProblemBuilder<Variable> {
        ProblemBuilder::new()
            .add_variables(self.build_group_in_slot_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_dynamic_group_assignment_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_student_in_group_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_periodicity_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_student_not_in_last_period_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_use_grouping_variables())
            .expect("Should not have duplicates")
            .add_constraints(self.build_at_most_max_groups_per_slot_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_at_most_one_interrogation_per_time_unit_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_one_interrogation_per_period_contraints())
            .expect("Variables should be declared")
            .add_constraints(
                self.build_at_most_one_interrogation_per_period_for_empty_groups_contraints(),
            )
            .expect("Variables should be declared")
            .add_constraints(self.build_students_per_group_count_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_student_in_single_group_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_dynamic_groups_student_in_group_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_dynamic_groups_group_in_slot_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_one_periodicity_choice_per_student_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_periodicity_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_interrogations_per_week_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_max_interrogations_per_day_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_grouping_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_grouping_incompats_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_students_incompats_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_balancing_constraints())
            .expect("Variables should be declared")
    }

    pub fn problem_builder(&self) -> ProblemBuilder<Variable> {
        let mut r = self.problem_builder_cache.borrow_mut();
        match r.as_ref() {
            Some(pb_builder) => pb_builder.clone(),
            None => {
                let pb_builder = self.problem_builder_internal();

                *r = Some(pb_builder.clone());

                pb_builder
            }
        }
    }

    pub fn problem(&self) -> Problem<Variable> {
        self.problem_builder().build()
    }

    fn compute_period_length(&self) -> NonZeroU32 {
        use crate::math::lcm;

        let mut result = 1;
        for subject in &self.data.subjects {
            result = lcm(result, subject.period.get());
        }

        NonZeroU32::new(result).unwrap()
    }

    pub fn incremental_initializer<T: GenericInitializer, S: GenericSolver>(
        &self,
        initializer: T,
        solver: S,
        max_steps: Option<usize>,
        retries: usize,
    ) -> IncrementalInitializer<T, S> {
        let period_length = self.compute_period_length();

        let last_period_full = (self.data.general.week_count.get() % period_length.get()) != 0;
        let period_count = NonZeroU32::new(
            (self.data.general.week_count.get() + period_length.get() - 1) / period_length.get(),
        )
        .unwrap();

        let subject_week_map = self
            .data
            .subjects
            .iter()
            .map(|subject| subject.slots.iter().map(|slot| slot.start.week).collect())
            .collect();
        let subject_strictness = self
            .data
            .subjects
            .iter()
            .map(|subject| subject.period_is_strict)
            .collect();
        let subject_period = self
            .data
            .subjects
            .iter()
            .map(|subject| subject.period)
            .collect();
        let grouping_week_map = self
            .data
            .slot_groupings
            .iter()
            .map(|grouping| {
                grouping
                    .slots
                    .iter()
                    .map(|slot_ref| {
                        self.data.subjects[slot_ref.subject].slots[slot_ref.slot]
                            .start
                            .week
                    })
                    .collect()
            })
            .collect();

        IncrementalInitializer {
            period_length,
            last_period_full,
            period_count,
            subject_strictness,
            subject_week_map,
            subject_period,
            grouping_week_map,
            max_steps,
            retries,
            initializer,
            solver,
        }
    }

    fn read_subject(
        &self,
        config: &FeasableConfig<'_, Variable>,
        i: usize,
        subject: &Subject,
    ) -> Option<ColloscopeSubject> {
        let mut groups = Vec::with_capacity(subject.groups.prefilled_groups.len());

        for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
            let mut output = group.students.clone();

            if Self::is_group_fixed(group, subject) {
                groups.push(output);
                continue;
            }

            for student in subject.groups.not_assigned.iter().copied() {
                if config
                    .get(&Variable::StudentInGroup {
                        subject: i,
                        student,
                        group: k,
                    })
                    .ok()?
                {
                    output.insert(student);
                }
            }

            groups.push(output);
        }

        let mut slots = Vec::with_capacity(subject.slots.len());

        for (j, _slot) in subject.slots.iter().enumerate() {
            let mut assigned_groups = BTreeSet::new();

            for k in 0..subject.groups.prefilled_groups.len() {
                if config
                    .get(&Variable::GroupInSlot {
                        subject: i,
                        slot: j,
                        group: k,
                    })
                    .ok()?
                {
                    assigned_groups.insert(k);
                }
            }

            slots.push(assigned_groups);
        }

        Some(ColloscopeSubject { groups, slots })
    }

    pub fn read_solution(&self, config: &FeasableConfig<'_, Variable>) -> Option<Colloscope> {
        let mut subjects = Vec::with_capacity(self.data.subjects.len());

        for (i, subject) in self.data.subjects.iter().enumerate() {
            subjects.push(self.read_subject(config, i, subject)?);
        }

        Some(Colloscope { subjects })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope {
    pub subjects: Vec<ColloscopeSubject>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSubject {
    pub groups: Vec<BTreeSet<usize>>,
    pub slots: Vec<BTreeSet<usize>>,
}

#[derive(Clone, Debug)]
pub struct IncrementalInitializer<T: GenericInitializer, S: GenericSolver> {
    period_length: NonZeroU32,
    last_period_full: bool,
    period_count: NonZeroU32,
    subject_strictness: Vec<bool>,
    subject_week_map: Vec<Vec<u32>>,
    subject_period: Vec<NonZeroU32>,
    grouping_week_map: Vec<BTreeSet<u32>>,
    max_steps: Option<usize>,
    retries: usize,
    initializer: T,
    solver: S,
}

impl<T: GenericInitializer, S: GenericSolver> ConfigInitializer<Variable, DefaultRepr<Variable>>
    for IncrementalInitializer<T, S>
{
    fn build_init_config<'a, 'b>(
        &'a self,
        problem: &'b Problem<Variable, DefaultRepr<Variable>>,
    ) -> Option<Config<'b, Variable, DefaultRepr<Variable>>> {
        let problem_builder = problem.clone().into_builder();

        let mut set_variables = BTreeMap::new();

        let first_period_problem_no_periodicity = self.construct_period_problem(
            problem_builder.clone().filter_variables(|v| {
                if let Variable::Periodicity {
                    subject: _,
                    student: _,
                    week_modulo: _,
                } = v
                {
                    false
                } else {
                    true
                }
            }),
            &set_variables,
            0,
        );
        let first_period_config_no_periodicity =
            self.construct_init_config(&first_period_problem_no_periodicity)?;
        Self::update_set_variables(&mut set_variables, &first_period_config_no_periodicity);

        for period in 0..self.period_count.get() {
            let period_problem =
                self.construct_period_problem(problem_builder.clone(), &set_variables, period);

            let partial_config = self.construct_init_config(&period_problem)?;
            Self::update_set_variables(&mut set_variables, &partial_config);
        }

        let vars: BTreeSet<_> = set_variables
            .iter()
            .filter_map(|(v, val)| if *val { Some(v.clone()) } else { None })
            .collect();

        problem.config_from(&vars).ok()
    }
}

impl<T: GenericInitializer, S: GenericSolver> IncrementalInitializer<T, S> {
    fn construct_init_config<'a, 'b>(
        &'a self,
        problem: &'b Problem<Variable>,
    ) -> Option<FeasableConfig<'b, Variable, DefaultRepr<Variable>>> {
        for _i in 0..self.retries {
            if let Some(config) = self.construct_init_config_one_try(problem) {
                return Some(config);
            }
        }
        None
    }

    fn construct_init_config_one_try<'a, 'b>(
        &'a self,
        problem: &'b Problem<Variable>,
    ) -> Option<FeasableConfig<'b, Variable, DefaultRepr<Variable>>> {
        let init_config = self.initializer.build_init_config(&problem)?;
        self.solver
            .restore_feasability_with_max_steps(&init_config, self.max_steps.clone())
    }

    fn update_set_variables(
        set_variables: &mut BTreeMap<Variable, bool>,
        config: &FeasableConfig<'_, Variable, DefaultRepr<Variable>>,
    ) {
        for var in config
            .get_problem()
            .get_variables()
            .into_iter()
            .chain(config.get_problem().get_constants())
        {
            if !set_variables.contains_key(var) {
                set_variables.insert(
                    var.clone(),
                    config.get(var).expect("var should be a valid variable"),
                );
            }
        }
    }

    fn is_week_in_period(&self, week: u32, period: u32) -> bool {
        let period_for_week = week / self.period_length.get();

        period_for_week == period
    }

    fn is_subject_concerned_by_period(&self, subject: usize, period: u32) -> bool {
        for week in self.subject_week_map[subject].iter().copied() {
            if self.is_week_in_period(week, period) {
                return true;
            }
        }
        false
    }

    fn is_grouping_concerned_by_period(&self, grouping: usize, period: u32) -> bool {
        for week in self.grouping_week_map[grouping].iter().copied() {
            if self.is_week_in_period(week, period) {
                return true;
            }
        }
        false
    }

    fn are_subject_and_period_concerned_with_periodicity(
        &self,
        subject: usize,
        period: u32,
    ) -> bool {
        if self.subject_strictness[subject] {
            return true;
        }

        // Normally this should be true or we should not have any Periodicity variables
        assert!(!self.last_period_full);

        if period == self.period_count.get() - 1 {
            return true;
        }

        assert!(self.period_count.get() >= 2);
        assert!(period != self.period_count.get() - 2);

        // If we are on the second to last period, we might need to fix periodicity
        // if the penultimate subject period (which might be shorter) leaks on that period
        (2 * self.subject_period[subject].get()) > self.period_length.get()
    }

    fn filter_relevant_variables(
        &self,
        problem_builder: ProblemBuilder<Variable>,
        period: u32,
    ) -> ProblemBuilder<Variable> {
        problem_builder.filter_variables(|v| match v {
            Variable::GroupInSlot {
                subject,
                slot,
                group: _,
            } => {
                let week = self.subject_week_map[*subject][*slot];
                self.is_week_in_period(week, period)
            }
            Variable::StudentNotInLastPeriod {
                subject: _,
                student: _,
            } => period == self.period_count.get() - 1,
            Variable::DynamicGroupAssignment {
                subject,
                slot,
                group: _,
                student: _,
            } => {
                let week = self.subject_week_map[*subject][*slot];
                self.is_week_in_period(week, period)
            }
            Variable::StudentInGroup {
                subject,
                student: _,
                group: _,
            } => self.is_subject_concerned_by_period(*subject, period),
            Variable::Periodicity {
                subject,
                student: _,
                week_modulo: _,
            } => self.are_subject_and_period_concerned_with_periodicity(*subject, period),
            Variable::UseGrouping(grouping) => {
                self.is_grouping_concerned_by_period(*grouping, period)
            }
        })
    }

    fn set_relevant_variables(
        &self,
        mut problem_builder: ProblemBuilder<Variable>,
        set_variables: &BTreeMap<Variable, bool>,
    ) -> ProblemBuilder<Variable> {
        let variables = problem_builder.get_variables().clone();
        for var in &variables {
            if let Some(val) = set_variables.get(var) {
                problem_builder = problem_builder
                    .add_constraint(crate::ilp::linexpr::Expr::var(var.clone()).eq(
                        &crate::ilp::linexpr::Expr::constant(if *val { 1 } else { 0 }),
                    ))
                    .expect("Should be a valid constraint");
            }
        }
        problem_builder
    }

    fn construct_period_problem(
        &self,
        problem_builder: ProblemBuilder<Variable>,
        set_variables: &BTreeMap<Variable, bool>,
        period: u32,
    ) -> Problem<Variable> {
        let filtered_problem = self.filter_relevant_variables(problem_builder, period);
        self.set_relevant_variables(filtered_problem, set_variables)
            .simplify_trivial_constraints()
            .build()
    }
}
