#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

use crate::time;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Invalid periodicity cut {0}. There are only {1} weeks.")]
    InvalidPeriodicityCut(u32, u32),
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
    #[error("Subject {0} has a larger periodicity {1} than the number of weeks {2}. A full period is needed for the algorithm to work")]
    SubjectWithPeriodicityTooBig(usize, u32, u32),
    #[error("Subject {0} has overlapping week selections in its balacing requirements")]
    SubjectWithOverlappingWeekSelections(usize),
    #[error("Student {0} references an invalid incompatibility number ({1})")]
    StudentWithInvalidIncompatibility(usize, usize),
    #[error("Incompatibility {0} references an invalid incompatibility group ({1})")]
    IncompatibilityWithInvalidIncompatibilityGroup(usize, usize),
    #[error("Incompatibility {0} has max_count larger ({1}) than the number of groups ({2})")]
    IncompatibilityWithMaxCountTooBig(usize, usize, usize),
    #[error(
        "Incompatibility group {0} has slot ({1:?}) after the week count ({2}) of the schedule"
    )]
    IncompatibilityGroupWithSlotAfterLastWeek(usize, SlotWithDuration, u32),
    #[error("Incompatibility group {0} has interrogation slot ({1:?}) overlapping next day")]
    IncompatibilityGroupWithSlotOverlappingNextDay(usize, SlotWithDuration),
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotStart {
    pub week: u32,
    pub weekday: time::Weekday,
    pub start_time: time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub cost: u32,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BalancingStrictness {
    #[default]
    OverallOnly,
    StrictWithCuts,
    Strict,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeekSelection {
    pub selection: BTreeSet<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalancingObject {
    Teachers(BTreeMap<WeekSelection, BTreeMap<TeacherBalancing, usize>>),
    Timeslots(BTreeMap<WeekSelection, BTreeMap<TimeslotBalancing, usize>>),
    TeachersAndTimeslots(BTreeMap<WeekSelection, BTreeMap<TeacherAndTimeslotBalancing, usize>>),
}

impl BalancingObject {
    fn build_data<T: BalancingData>(
        slots: &Vec<SlotWithTeacher>,
    ) -> BTreeMap<WeekSelection, BTreeMap<T, usize>> {
        let mut week_map: BTreeMap<u32, BTreeMap<T, usize>> = BTreeMap::new();

        for slot in slots {
            let data = T::from_subject_slot(slot);

            match week_map.get_mut(&slot.start.week) {
                Some(val) => match val.get_mut(&data) {
                    Some(value) => {
                        *value += 1;
                    }
                    None => {
                        val.insert(data, 1);
                    }
                },
                None => {
                    week_map.insert(slot.start.week, BTreeMap::from([(data, 1)]));
                }
            }
        }

        let mut reverse_data: BTreeMap<BTreeMap<T, usize>, WeekSelection> = BTreeMap::new();

        for (week, data) in week_map {
            match reverse_data.get_mut(&data) {
                Some(value) => {
                    value.selection.insert(week);
                }
                None => {
                    reverse_data.insert(
                        data,
                        WeekSelection {
                            selection: BTreeSet::from([week]),
                        },
                    );
                }
            }
        }

        reverse_data
            .into_iter()
            .map(|(data, week_select)| (week_select, data))
            .collect()
    }

    pub fn teachers_from_slots(slots: &Vec<SlotWithTeacher>) -> Self {
        BalancingObject::Teachers(Self::build_data(slots))
    }

    pub fn timeslots_from_slots(slots: &Vec<SlotWithTeacher>) -> Self {
        BalancingObject::Timeslots(Self::build_data(slots))
    }

    pub fn teachers_and_timeslots_from_slots(slots: &Vec<SlotWithTeacher>) -> Self {
        BalancingObject::TeachersAndTimeslots(Self::build_data(slots))
    }

    fn extract_week_selections(&self) -> Vec<WeekSelection> {
        match self {
            BalancingObject::Teachers(map) => map.iter().map(|(ws, _)| ws.clone()).collect(),
            BalancingObject::Timeslots(map) => map.iter().map(|(ws, _)| ws.clone()).collect(),
            BalancingObject::TeachersAndTimeslots(map) => {
                map.iter().map(|(ws, _)| ws.clone()).collect()
            }
        }
    }
}

trait BalancingData: Clone + PartialEq + Eq + PartialOrd + Ord {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self;

    fn is_slot_relevant(&self, slot: &SlotWithTeacher) -> bool {
        let other = Self::from_subject_slot(slot);
        *self == other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeacherBalancing {
    pub teacher: usize,
}

impl BalancingData for TeacherBalancing {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self {
        TeacherBalancing {
            teacher: slot.teacher,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeslotBalancing {
    pub weekday: time::Weekday,
    pub time: time::Time,
}

impl BalancingData for TimeslotBalancing {
    fn from_subject_slot(slot: &SlotWithTeacher) -> Self {
        TimeslotBalancing {
            weekday: slot.start.weekday,
            time: slot.start.start_time.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeacherAndTimeslotBalancing {
    pub teacher: usize,
    pub weekday: time::Weekday,
    pub time: time::Time,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingRequirements {
    pub strictness: BalancingStrictness,
    pub object: BalancingObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    pub max_groups_per_slot: NonZeroUsize,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub is_tutorial: bool,
    pub balancing_requirements: Option<BalancingRequirements>,
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
            balancing_requirements: None,
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
pub struct IncompatibilityGroup {
    pub slots: BTreeSet<SlotWithDuration>,
}

pub type IncompatibilityGroupList = Vec<IncompatibilityGroup>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility {
    pub groups: BTreeSet<usize>,
    pub max_count: usize,
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
    pub periodicity_cuts: BTreeSet<NonZeroU32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedData {
    general: GeneralData,
    subjects: SubjectList,
    incompatibility_groups: IncompatibilityGroupList,
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
        incompatibility_groups: IncompatibilityGroupList,
        incompatibilities: IncompatibilityList,
        students: StudentList,
        slot_groupings: SlotGroupingList,
        grouping_incompats: SlotGroupingIncompatSet,
    ) -> Result<ValidatedData> {
        for cut in &general.periodicity_cuts {
            if cut.get() >= general.week_count.get() {
                return Err(Error::InvalidPeriodicityCut(
                    cut.get(),
                    general.week_count.get(),
                ));
            }
        }

        for (i, subject) in subjects.iter().enumerate() {
            if let Some(balancing_requirements) = &subject.balancing_requirements {
                let week_selection_list = balancing_requirements.object.extract_week_selections();

                let mut weeks = BTreeSet::new();

                for week_selection in week_selection_list {
                    for week in week_selection.selection {
                        if weeks.contains(&week) {
                            return Err(Error::SubjectWithOverlappingWeekSelections(i));
                        }
                        weeks.insert(week);
                    }
                }
            }

            if subject.period.get() > general.week_count.get() {
                return Err(Error::SubjectWithPeriodicityTooBig(
                    i,
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
            for incompat_group in &incompatibility.groups {
                if *incompat_group >= incompatibility_groups.len() {
                    return Err(Error::IncompatibilityWithInvalidIncompatibilityGroup(
                        i,
                        *incompat_group,
                    ));
                }
            }
            if incompatibility.max_count >= incompatibility.groups.len() {
                return Err(Error::IncompatibilityWithMaxCountTooBig(
                    i,
                    incompatibility.max_count,
                    incompatibility.groups.len(),
                ));
            }
        }

        for (i, incompat_group) in incompatibility_groups.iter().enumerate() {
            for slot in &incompat_group.slots {
                if !Self::validate_slot_start(&general, &slot.start) {
                    return Err(Error::IncompatibilityGroupWithSlotAfterLastWeek(
                        i,
                        slot.clone(),
                        general.week_count.get(),
                    ));
                }
                if !Self::validate_slot_overlap(&slot.start, slot.duration) {
                    return Err(Error::IncompatibilityGroupWithSlotOverlappingNextDay(
                        i,
                        slot.clone(),
                    ));
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
            incompatibility_groups,
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
    GroupOnWeekSelection {
        subject: usize,
        week_selection: usize,
        group: usize,
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
    UseGrouping(usize),
    IncompatGroupForStudent {
        incompat_group: usize,
        student: usize,
    },
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
            Variable::GroupOnWeekSelection {
                subject,
                week_selection,
                group,
            } => write!(f, "GoWS_{}_{}_{}", *subject, *week_selection, *group),
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
            Variable::UseGrouping(num) => write!(f, "UG_{}", num),
            Variable::IncompatGroupForStudent {
                incompat_group,
                student,
            } => write!(f, "IGfS_{}_{}", *incompat_group, *student),
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

        for incompat_group in &self.data.incompatibility_groups {
            for slot in &incompat_group.slots {
                result = gcd(result, slot.duration.get());
                result = gcd(result, slot.start.start_time.get());
            }
        }

        result
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

    fn build_group_on_week_selection_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject.balancing_requirements.iter().flat_map(move |br| {
                    let week_selections = br.object.extract_week_selections();

                    week_selections
                        .into_iter()
                        .enumerate()
                        .flat_map(move |(j, _ws)| {
                            subject.groups.prefilled_groups.iter().enumerate().map(
                                move |(k, _group)| Variable::GroupOnWeekSelection {
                                    subject: i,
                                    week_selection: j,
                                    group: k,
                                },
                            )
                        })
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

    fn build_use_grouping_variables(&self) -> BTreeSet<Variable> {
        self.data
            .slot_groupings
            .iter()
            .enumerate()
            .map(|(i, _grouping)| Variable::UseGrouping(i))
            .collect()
    }

    fn build_incompat_group_for_student_variables(&self) -> BTreeSet<Variable> {
        self.data
            .students
            .iter()
            .enumerate()
            .flat_map(|(i, student)| {
                student.incompatibilities.iter().flat_map(move |j| {
                    let incompat = &self.data.incompatibilities[*j];
                    incompat
                        .groups
                        .iter()
                        .map(move |k| Variable::IncompatGroupForStudent {
                            incompat_group: *k,
                            student: i,
                        })
                })
            })
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
        weekday: time::Weekday,
        time: time::Time,
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
        weekday: time::Weekday,
        time: time::Time,
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
        weekday: time::Weekday,
        time: time::Time,
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
            for weekday in time::Weekday::iter() {
                let init_time = time::Time::from_hm(0, 0).unwrap();
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
            expr.leq(&Expr::constant(1))
        } else {
            expr.eq(&Expr::constant(1))
        }
    }

    fn build_one_interrogation_per_period_contraint_for_assigned_student(
        &self,
        i: usize,
        subject: &Subject,
        period: std::ops::Range<u32>,
        k: usize,
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
            expr.leq(&Expr::constant(1))
        } else {
            expr.eq(&Expr::constant(1))
        }
    }

    fn build_one_interrogation_per_period_contraints_for_one_subject_period(
        &self,
        i: usize,
        subject: &Subject,
        period: std::ops::Range<u32>,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

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
            if !group.students.is_empty() {
                constraints.insert(
                    self.build_one_interrogation_per_period_contraint_for_assigned_student(
                        i,
                        subject,
                        period.clone(),
                        k,
                    ),
                );
            }
        }

        constraints
    }

    fn generate_period_list_for_range(
        &self,
        subject: &Subject,
        strict: bool,
        range: std::ops::Range<u32>,
    ) -> Vec<std::ops::Range<u32>> {
        let week_count = range.end - range.start;

        if week_count < subject.period.get() {
            return vec![range];
        }

        if strict {
            (range.start..(range.end - subject.period.get() + 1))
                .into_iter()
                .map(|start| start..(start + subject.period.get()))
                .collect()
        } else {
            let whole_period_count = week_count / subject.period.get();
            let mut period_list: Vec<_> = (0..whole_period_count)
                .into_iter()
                .map(|p| {
                    let start = range.start + p * subject.period.get();
                    let period = start..(start + subject.period.get());
                    period
                })
                .collect();

            let week_remainder = week_count % subject.period.get();
            let there_is_an_incomplete_period = week_remainder != 0;
            if there_is_an_incomplete_period {
                let first_week_to_add = range.end - week_remainder - subject.period.get() + 1;
                for i in 0..week_remainder {
                    let start = first_week_to_add + i;
                    period_list.push(start..(start + subject.period.get()));
                }
            }

            period_list
        }
    }

    fn generate_period_list(&self, subject: &Subject, strict: bool) -> Vec<std::ops::Range<u32>> {
        let mut output = Vec::new();

        let mut start = 0;
        for cut in &self.data.general.periodicity_cuts {
            let range = start..cut.get();

            output.extend(self.generate_period_list_for_range(subject, strict, range));

            start = cut.get();
        }

        let week_count = self.data.general.week_count.get();
        let range = start..week_count;
        output.extend(self.generate_period_list_for_range(subject, strict, range));

        output
    }

    fn build_one_interrogation_per_period_constraints_for_subject(
        &self,
        i: usize,
        subject: &Subject,
        strict: bool,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        let period_list = self.generate_period_list(subject, strict);

        for period in period_list {
            constraints.extend(
                self.build_one_interrogation_per_period_contraints_for_one_subject_period(
                    i, subject, period,
                ),
            );
        }

        constraints
    }

    fn build_one_interrogation_per_period_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            constraints.extend(
                self.build_one_interrogation_per_period_constraints_for_subject(
                    i,
                    subject,
                    subject.period_is_strict,
                ),
            );
        }

        constraints
    }

    fn build_one_interrogation_per_period_optimizer(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            if !subject.period_is_strict {
                constraints.extend(
                    self.build_one_interrogation_per_period_constraints_for_subject(
                        i, subject, true,
                    ),
                );
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

    fn build_interrogations_per_week_optimizer_for_student_expr(
        &self,
        student: usize,
        week: u32,
    ) -> Expr<Variable> {
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

        expr
    }

    fn build_interrogations_per_week_optimizer_for_student(
        &self,
        student: usize,
        week: u32,
    ) -> Constraint<Variable> {
        let expr1 = self.build_interrogations_per_week_optimizer_for_student_expr(student, week);
        let expr2 =
            self.build_interrogations_per_week_optimizer_for_student_expr(student, week + 1);

        expr1.eq(&expr2)
    }

    fn build_interrogations_per_week_optimizer(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints: BTreeSet<Constraint<Variable>> = BTreeSet::new();

        for (student, _) in self.data.students.iter().enumerate() {
            for week in 0..(self.data.general.week_count.get() - 1) {
                constraints.insert(
                    self.build_interrogations_per_week_optimizer_for_student(student, week),
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

    fn build_max_interrogations_per_day_optimizer_for_student(
        &self,
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

        Some(expr.eq(&Expr::constant(0)))
    }

    fn build_max_interrogations_per_day_optimizer(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for week in 0..self.data.general.week_count.get() {
            for day in time::Weekday::iter() {
                for (student, _) in self.data.students.iter().enumerate() {
                    constraints.extend(
                        self.build_max_interrogations_per_day_optimizer_for_student(
                            student, week, day,
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
            for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
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
            match &subject.balancing_requirements {
                None => {} // ignore, no balancing needed
                Some(value) => match &value.object {
                    BalancingObject::Teachers(_) => constraints.extend(
                        self.build_balancing_constraints_for_subject::<TeacherBalancing>(i, subject),
                    ),
                    BalancingObject::Timeslots(_) => constraints.extend(
                        self.build_balancing_constraints_for_subject::<TimeslotBalancing>(i, subject),
                    ),
                    BalancingObject::TeachersAndTimeslots(_) => constraints.extend(
                        self.build_balancing_constraints_for_subject::<TeacherAndTimeslotBalancing>(i, subject),
                    ),
                }
            }
        }

        constraints
    }

    fn build_incompat_group_for_student_constraint_for_student_and_incompat_group_and_slot_assigned_version(
        &self,
        student: usize,
        subject: usize,
        incompat_group: usize,
        slot: usize,
        group: usize,
    ) -> Constraint<Variable> {
        let expr = Expr::var(Variable::GroupInSlot {
            subject,
            slot,
            group,
        });

        expr.leq(&Expr::var(Variable::IncompatGroupForStudent {
            incompat_group,
            student,
        }))
    }

    fn build_incompat_group_for_student_constraint_for_student_and_incompat_group_and_slot_dynamic_version(
        &self,
        student: usize,
        subject: usize,
        incompat_group: usize,
        slot: usize,
        group: usize,
    ) -> Constraint<Variable> {
        let expr = Expr::var(Variable::DynamicGroupAssignment {
            subject,
            slot,
            group,
            student,
        });

        expr.leq(&Expr::var(Variable::IncompatGroupForStudent {
            incompat_group,
            student,
        }))
    }

    fn need_building_for_slot_and_incompat_group(
        &self,
        slot_start: &SlotStart,
        duration: NonZeroU32,
        incompat_group: &IncompatibilityGroup,
    ) -> bool {
        for slot in &incompat_group.slots {
            if slot.overlap_with(&SlotWithDuration {
                start: slot_start.clone(),
                duration,
            }) {
                return true;
            }
        }

        false
    }

    fn build_incompat_group_for_student_constraints_for_subject_slot_student_and_incompat_group(
        &self,
        i: usize,
        j: usize,
        subject: &Subject,
        k: usize,
        l: usize,
        slot: &SlotWithTeacher,
        q: usize,
        incompat_group: &IncompatibilityGroup,
        assigned: bool,
    ) -> Option<Constraint<Variable>> {
        if !self.need_building_for_slot_and_incompat_group(
            &slot.start,
            subject.duration,
            incompat_group,
        ) {
            return None;
        }

        if assigned {
            Some(self.build_incompat_group_for_student_constraint_for_student_and_incompat_group_and_slot_assigned_version(
                i,
                j,
                q,
                l,
                k,
            ))
        } else {
            Some(self.build_incompat_group_for_student_constraint_for_student_and_incompat_group_and_slot_dynamic_version(
                i,
                j,
                q,
                l,
                k,
            ))
        }
    }

    fn build_incompat_group_for_student_constraints_for_subject_and_student(
        &self,
        i: usize,
        student: &Student,
        j: usize,
        subject: &Subject,
        k: usize,
        assigned: bool,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (l, slot) in subject.slots.iter().enumerate() {
            for p in student.incompatibilities.iter().copied() {
                let incompat = &self.data.incompatibilities[p];
                for q in incompat.groups.iter().copied() {
                    let incompat_group = &self.data.incompatibility_groups[q];
                    constraints.extend(
                        self.build_incompat_group_for_student_constraints_for_subject_slot_student_and_incompat_group(
                            i,
                            j,
                            subject,
                            k,
                            l,
                            slot,
                            q,
                            incompat_group,
                            assigned
                        )
                    );
                }
            }
        }

        constraints
    }

    fn build_incompat_group_for_student_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (j, subject) in self.data.subjects.iter().enumerate() {
            for (k, group) in subject.groups.prefilled_groups.iter().enumerate() {
                for i in group.students.iter().copied() {
                    let student = &self.data.students[i];
                    constraints.extend(
                        self.build_incompat_group_for_student_constraints_for_subject_and_student(
                            i, student, j, subject, k, true,
                        ),
                    );
                }

                if !Self::is_group_fixed(group, subject) {
                    for i in subject.groups.not_assigned.iter().copied() {
                        let student = &self.data.students[i];
                        constraints.extend(
                            self.build_incompat_group_for_student_constraints_for_subject_and_student(
                                i,
                                student,
                                j,
                                subject,
                                k,
                                false,
                            )
                        );
                    }
                }
            }
        }

        constraints
    }

    fn build_student_incompat_max_count_constraint_for_student_and_incompat(
        &self,
        student: usize,
        incompat: &Incompatibility,
    ) -> Constraint<Variable> {
        let mut expr = Expr::<Variable>::constant(0);

        for incompat_group in incompat.groups.iter().copied() {
            expr = expr
                + Expr::var(Variable::IncompatGroupForStudent {
                    incompat_group,
                    student,
                })
        }

        let max_count_i32 = incompat.max_count.try_into().expect("Less than 2^31");
        expr.leq(&Expr::constant(max_count_i32))
    }

    fn build_student_incompat_max_count_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, student) in self.data.students.iter().enumerate() {
            for j in student.incompatibilities.iter().copied() {
                let incompat = &self.data.incompatibilities[j];
                constraints.insert(
                    self.build_student_incompat_max_count_constraint_for_student_and_incompat(
                        i, incompat,
                    ),
                );
            }
        }

        constraints
    }

    fn build_group_on_week_selection_constraints_for_subject_week_selection_and_group(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        week_selection: &WeekSelection,
        k: usize,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        let mut expr_sum = Expr::constant(0);
        let expr_gows = Expr::var(Variable::GroupOnWeekSelection {
            subject: i,
            week_selection: j,
            group: k,
        });

        for (slot_num, slot) in subject.slots.iter().enumerate() {
            if !week_selection.selection.contains(&slot.start.week) {
                continue;
            }

            let expr_gis = Expr::var(Variable::GroupInSlot {
                subject: i,
                slot: slot_num,
                group: k,
            });

            expr_sum = expr_sum + &expr_gis;

            let constraint = expr_gis.leq(&expr_gows);
            constraints.insert(constraint);
        }

        let constraint = expr_gows.leq(&expr_sum);
        constraints.insert(constraint);

        constraints
    }

    fn build_group_on_week_selection_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            if let Some(br) = &subject.balancing_requirements {
                let week_selection_list = br.object.extract_week_selections();

                for (j, week_selection) in week_selection_list.iter().enumerate() {
                    for (k, _group) in subject.groups.prefilled_groups.iter().enumerate() {
                        constraints.extend(
                            self.build_group_on_week_selection_constraints_for_subject_week_selection_and_group(
                                i,
                                subject,
                                j,
                                week_selection,
                                k
                            )
                        );
                    }
                }
            }
        }

        constraints
    }

    fn problem_builder_soft(&self) -> ProblemBuilder<Variable> {
        ProblemBuilder::new()
            .add_variables(self.build_group_in_slot_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_dynamic_group_assignment_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_student_in_group_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_use_grouping_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_incompat_group_for_student_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_group_on_week_selection_variables())
            .expect("Should not have duplicates")
            .add_constraints(self.build_interrogations_per_week_optimizer())
            .expect("Variables should be declared")
            .add_constraints(self.build_max_interrogations_per_day_optimizer())
            .expect("Variables should be declared")
            .add_constraints(self.build_one_interrogation_per_period_optimizer())
            .expect("Variables should be declared")
    }

    fn problem_builder_hard(&self) -> ProblemBuilder<Variable> {
        ProblemBuilder::new()
            .add_variables(self.build_group_in_slot_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_dynamic_group_assignment_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_student_in_group_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_use_grouping_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_incompat_group_for_student_variables())
            .expect("Should not have duplicates")
            .add_variables(self.build_group_on_week_selection_variables())
            .expect("Should not have duplicates")
            .add_constraints(self.build_at_most_max_groups_per_slot_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_at_most_one_interrogation_per_time_unit_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_one_interrogation_per_period_constraints())
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
            .add_constraints(self.build_interrogations_per_week_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_max_interrogations_per_day_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_grouping_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_grouping_incompats_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_incompat_group_for_student_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_student_incompat_max_count_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_group_on_week_selection_constraints())
            .expect("Variables should be declared")
            .add_constraints(self.build_balancing_constraints())
            .expect("Variables should be declared")
    }

    fn problem_builder_internal(&self) -> ProblemBuilder<Variable> {
        let soft_problem = self.problem_builder_soft().build();

        let subjects = self.data.subjects.clone();

        let hard_problem_builder =
            self.problem_builder_hard()
                .eval_fn(crate::debuggable!(move |x| {
                    let vars = x.get_vars();
                    let soft_config = soft_problem
                        .config_from(&vars)
                        .expect("Variables should match");
                    // If some constraints are inequalities, this will still measure the difference to equality
                    let sq2_cost = soft_config.compute_lhs_sq_norm2();

                    let mut manual_costs = 0.;
                    for var in &vars {
                        if let Variable::GroupInSlot {
                            subject,
                            slot,
                            group: _,
                        } = var
                        {
                            manual_costs += f64::from(subjects[*subject].slots[*slot].cost);
                        }
                    }

                    sq2_cost + manual_costs
                }));
        hard_problem_builder
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
        quick_init: bool,
    ) -> IncrementalInitializer<T, S> {
        let period_length = self.compute_period_length();

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
        let subject_week_selection_map = self
            .data
            .subjects
            .iter()
            .map(|subject| match &subject.balancing_requirements {
                None => vec![],
                Some(br) => br
                    .object
                    .extract_week_selections()
                    .into_iter()
                    .map(|ws| ws.selection)
                    .collect(),
            })
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
        let incompat_groups_week_map = self
            .data
            .incompatibility_groups
            .iter()
            .map(|incompat_group| {
                incompat_group
                    .slots
                    .iter()
                    .map(|slot| slot.start.week)
                    .collect()
            })
            .collect();

        IncrementalInitializer {
            period_length,
            period_count,
            subject_week_map,
            subject_week_selection_map,
            grouping_week_map,
            incompat_groups_week_map,
            max_steps,
            retries,
            initializer,
            solver,
            quick_init,
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
    period_count: NonZeroU32,
    subject_week_map: Vec<Vec<u32>>,
    subject_week_selection_map: Vec<Vec<BTreeSet<u32>>>,
    grouping_week_map: Vec<BTreeSet<u32>>,
    incompat_groups_week_map: Vec<BTreeSet<u32>>,
    max_steps: Option<usize>,
    retries: usize,
    initializer: T,
    solver: S,
    quick_init: bool,
}

impl<T: GenericInitializer, S: GenericSolver> ConfigInitializer<Variable, DefaultRepr<Variable>>
    for IncrementalInitializer<T, S>
{
    fn build_init_config<'a, 'b>(
        &'a self,
        problem: &'b Problem<Variable, DefaultRepr<Variable>>,
    ) -> Config<'b, Variable, DefaultRepr<Variable>> {
        let mut set_variables = BTreeMap::new();

        let _ = self.build_init_config_internal(problem, &mut set_variables);

        let vars: BTreeSet<_> = set_variables
            .iter()
            .filter_map(|(v, val)| if *val { Some(v.clone()) } else { None })
            .collect();

        problem
            .config_from(&vars)
            .expect("Should be valid variables")
    }
}

impl<T: GenericInitializer, S: GenericSolver> IncrementalInitializer<T, S> {
    fn build_init_config_internal<'a, 'b>(
        &'a self,
        problem: &'b Problem<Variable, DefaultRepr<Variable>>,
        set_variables: &mut BTreeMap<Variable, bool>,
    ) -> Option<()> {
        let problem_builder = problem.clone().into_builder();

        let first_period_problem_no_periodicity =
            self.construct_period_problem(problem_builder.clone(), &set_variables, 0);
        let first_period_config_no_periodicity =
            self.construct_init_config(&first_period_problem_no_periodicity)?;
        Self::update_set_variables(set_variables, &first_period_config_no_periodicity);

        for period in 0..self.period_count.get() {
            let period_problem =
                self.construct_period_problem(problem_builder.clone(), &set_variables, period);

            let partial_config = self.construct_init_config(&period_problem)?;
            Self::update_set_variables(set_variables, &partial_config);
        }

        Some(())
    }

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
        let init_config = self.initializer.build_init_config(&problem);
        let hint_only = self.quick_init;
        self.solver
            .restore_feasability_with_origin_and_max_steps_and_hint_only(
                &init_config,
                None,
                self.max_steps.clone(),
                hint_only,
            )
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

    fn is_incompat_group_concerned_by_period(&self, incompat_group: usize, period: u32) -> bool {
        for week in self.incompat_groups_week_map[incompat_group]
            .iter()
            .copied()
        {
            if self.is_week_in_period(week, period) {
                return true;
            }
        }
        false
    }

    fn does_week_selection_cover_period(
        &self,
        week_selection: &BTreeSet<u32>,
        period: u32,
    ) -> bool {
        for week in week_selection {
            if self.is_week_in_period(*week, period) {
                return true;
            }
        }
        false
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
            Variable::GroupOnWeekSelection {
                subject,
                week_selection,
                group: _,
            } => {
                let week_selection = &self.subject_week_selection_map[*subject][*week_selection];
                self.does_week_selection_cover_period(week_selection, period)
            }
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
            Variable::UseGrouping(grouping) => {
                self.is_grouping_concerned_by_period(*grouping, period)
            }
            Variable::IncompatGroupForStudent {
                incompat_group,
                student: _,
            } => self.is_incompat_group_concerned_by_period(*incompat_group, period),
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
