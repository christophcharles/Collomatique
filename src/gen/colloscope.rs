#[cfg(test)]
mod tests;

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
    #[error("The grouping incompatibility {0} has an invalid slot grouping reference {1}")]
    SlotGroupingIncompatWithInvalidSlotGrouping(usize, usize),
    #[error("The range {0:?} for the number of interrogations per week is empty")]
    SlotGeneralDataWithInvalidInterrogationsPerWeek(std::ops::Range<u32>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotStart {
    week: u32,
    weekday: time::Weekday,
    start_time: time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotWithDuration {
    pub start: SlotStart,
    pub duration: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotWithTeacher {
    pub teacher: usize,
    pub start: SlotStart,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupDesc {
    students: BTreeSet<usize>,
    can_be_extended: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupsDesc {
    assigned_to_group: Vec<GroupDesc>,
    not_assigned: BTreeSet<usize>,
}

impl GroupsDesc {
    fn students_iterator(&self) -> impl Iterator<Item = &usize> {
        self.assigned_to_group
            .iter()
            .flat_map(|group| group.students.iter())
            .chain(self.not_assigned.iter())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub students_per_slot: RangeInclusive<NonZeroUsize>,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub duration: NonZeroU32,
    pub slots: Vec<SlotWithTeacher>,
    pub groups: GroupsDesc,
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
}

pub type SlotGroupingIncompatList = Vec<SlotGroupingIncompat>;

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedData {
    general: GeneralData,
    subjects: SubjectList,
    incompatibilities: IncompatibilityList,
    students: StudentList,
    slot_groupings: SlotGroupingList,
    slot_grouping_incompats: SlotGroupingIncompatList,
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
        grouping_incompats: SlotGroupingIncompatList,
    ) -> Result<ValidatedData> {
        for (i, subject) in subjects.iter().enumerate() {
            if subject.period.get() > general.week_count.get() {
                return Err(Error::SubjectWithPeriodicityTooBig(
                    subject.period.get(),
                    general.week_count.get(),
                ));
            }

            if subject.students_per_slot.is_empty() {
                return Err(Error::SubjectWithInvalidStudentsPerSlotRange(
                    i,
                    subject.students_per_slot.clone(),
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

            for (j, group) in subject.groups.assigned_to_group.iter().enumerate() {
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
            for group in &subject.groups.assigned_to_group {
                if !group.can_be_extended {
                    continue;
                }
                if group.students.len() >= subject.students_per_slot.end().get() {
                    continue;
                }
                remaining_seats += subject.students_per_slot.end().get() - group.students.len();
            }
            if subject.groups.not_assigned.len() > remaining_seats {
                return Err(Error::SubjectWithTooFewGroups(
                    i,
                    subject.students_per_slot.clone(),
                ));
            }

            let mut min_seats = 0usize;
            for group in &subject.groups.assigned_to_group {
                if !group.can_be_extended {
                    continue;
                }
                if group.students.len() >= subject.students_per_slot.start().get() {
                    continue;
                }
                min_seats += subject.students_per_slot.start().get() - group.students.len();
            }
            if subject.groups.not_assigned.len() < min_seats {
                return Err(Error::SubjectWithTooManyGroups(
                    i,
                    subject.students_per_slot.clone(),
                ));
            }

            let mut students_no_duplicate = BTreeMap::new();

            for (j, group) in subject.groups.assigned_to_group.iter().enumerate() {
                for k in &group.students {
                    if let Some(first_j) = students_no_duplicate.get(k) {
                        return Err(Error::SubjectWithDuplicatedStudentInGroups(
                            i, *k, *first_j, j,
                        ));
                    } else {
                        students_no_duplicate.insert(*k, j);
                    }
                }
                if group.students.len() > subject.students_per_slot.end().get() {
                    return Err(Error::SubjectWithTooLargeAssignedGroup(
                        i,
                        j,
                        subject.students_per_slot.clone(),
                    ));
                }
                if group.students.len() < subject.students_per_slot.start().get()
                    && !group.can_be_extended
                {
                    return Err(Error::SubjectWithTooSmallNonExtensibleGroup(
                        i,
                        j,
                        subject.students_per_slot.clone(),
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

        for (i, slot_grouping) in slot_groupings.iter().enumerate() {
            for slot_ref in &slot_grouping.slots {
                if slot_ref.subject >= subjects.len() {
                    return Err(Error::SlotGroupingWithInvalidSubject(i, slot_ref.clone()));
                }
                if slot_ref.slot >= subjects[slot_ref.subject].slots.len() {
                    return Err(Error::SlotGroupingWithInvalidSlot(i, slot_ref.clone()));
                }
            }
        }

        for (i, grouping_incompat) in grouping_incompats.iter().enumerate() {
            for &grouping in &grouping_incompat.groupings {
                if grouping >= slot_groupings.len() {
                    return Err(Error::SlotGroupingIncompatWithInvalidSlotGrouping(
                        i, grouping,
                    ));
                }
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
        }
    }
}

impl ValidatedData {
    pub fn ilp_translator<'a>(&'a self) -> IlpTranslator<'a> {
        IlpTranslator { data: self }
    }
}

#[derive(Clone, Debug)]
pub struct IlpTranslator<'a> {
    data: &'a ValidatedData,
}

use crate::ilp::linexpr::{Constraint, Expr};
use crate::ilp::{Problem, ProblemBuilder};

enum StudentStatus {
    Assigned(usize),
    ToBeAssigned(BTreeSet<usize>),
    NotConcerned,
}

impl<'a> IlpTranslator<'a> {
    fn is_group_fixed(group: &GroupDesc, subject: &Subject) -> bool {
        !group.can_be_extended || (group.students.len() == subject.students_per_slot.end().get())
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
                        subject.groups.assigned_to_group.iter().enumerate().map(
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
                                .assigned_to_group
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
                        .assigned_to_group
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

    fn build_student_not_in_last_period(&self) -> BTreeSet<Variable> {
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

    fn build_at_most_one_group_per_slot_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject.slots.iter().enumerate().map(move |(j, _slot)| {
                    let mut expr = Expr::constant(0);

                    for (k, _group) in subject.groups.assigned_to_group.iter().enumerate() {
                        expr = expr
                            + Expr::var(Variable::GroupInSlot {
                                subject: i,
                                slot: j,
                                group: k,
                            });
                    }

                    expr.leq(&Expr::constant(1))
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
                .assigned_to_group
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
        for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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
                for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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

                for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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

                for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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
        let min = subject.students_per_slot.start().get();
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
        let max = subject.students_per_slot.end().get();
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
            for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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

        for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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

    fn build_dynamic_group_constraint_for_case(
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

    fn build_dynamic_groups_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            for (j, _slot) in subject.slots.iter().enumerate() {
                for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
                    if !Self::is_group_fixed(group, subject) {
                        for student in subject.groups.not_assigned.iter().copied() {
                            constraints.insert(
                                self.build_dynamic_group_constraint_for_case(i, j, k, student),
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

                for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
                    for student in group.students.iter().copied() {
                        constraints.insert(self.build_periodicity_constraint_for_assigned_student(
                            i, subject, j, slot, k, student,
                        ));
                    }
                }

                for (k, group) in subject.groups.assigned_to_group.iter().enumerate() {
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

    pub fn problem_builder(&self) -> ProblemBuilder<Variable> {
        ProblemBuilder::new()
            .add_variables(self.build_group_in_slot_variables())
            .add_variables(self.build_dynamic_group_assignment_variables())
            .add_variables(self.build_student_in_group_variables())
            .add_variables(self.build_periodicity_variables())
            .add_variables(self.build_student_not_in_last_period())
            .add_constraints(self.build_at_most_one_group_per_slot_constraints())
            .add_constraints(self.build_at_most_one_interrogation_per_time_unit_constraints())
            .add_constraints(self.build_one_interrogation_per_period_contraints())
            .add_constraints(self.build_students_per_group_count_constraints())
            .add_constraints(self.build_student_in_single_group_constraints())
            .add_constraints(self.build_dynamic_groups_constraints())
            .add_constraints(self.build_one_periodicity_choice_per_student_constraints())
            .add_constraints(self.build_periodicity_constraints())
    }

    pub fn problem(&self) -> Problem<Variable> {
        self.problem_builder()
            .build()
            .expect("Automatically built problem should be valid")
    }
}
