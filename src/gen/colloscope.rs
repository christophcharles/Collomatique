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
    #[error("Subject {0} has overlapping slot selections in its balacing requirements for slot selection {1}")]
    SubjectWithOverlappingSlotsInBalancingSlotSelection(usize, usize),
    #[error("Subject {0} has empty slot selection ({1}) in its balacing requirements")]
    SubjectWithEmptySlotSelectionInBalancing(usize, usize),
    #[error("Subject {0} has empty slot group ({2}) in its balacing requirements for slot selection {1}")]
    SubjectWithEmptySlotGroupInBalancing(usize, usize, usize),
    #[error("Subject {0} has an invalid slot number ({1}) in its balacing requirements")]
    SubjectWithInvalidSlotInBalancing(usize, usize),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalancingConstraints {
    OptimizeOnly,
    OverallOnly,
    StrictWithCuts,
    StrictWithCutsAndOverall,
    Strict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingSlotGroup {
    pub slots: BTreeSet<usize>,
    pub count: usize,
}

impl BalancingSlotGroup {
    pub fn extract_weeks(&self, slots: &Vec<SlotWithTeacher>) -> Option<BTreeSet<u32>> {
        self.slots
            .iter()
            .map(|&slot_num| slots.get(slot_num).map(|slot| slot.start.week))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingSlotSelection {
    pub slot_groups: Vec<BalancingSlotGroup>,
}

impl BalancingSlotSelection {
    pub fn contains_slot(&self, slot: usize) -> bool {
        for slot_group in &self.slot_groups {
            if slot_group.slots.contains(&slot) {
                return true;
            }
        }
        false
    }

    pub fn extract_slots(&self) -> BTreeSet<usize> {
        self.slot_groups
            .iter()
            .flat_map(|slot_group| slot_group.slots.iter().copied())
            .collect()
    }

    pub fn extract_weeks(&self, slots: &Vec<SlotWithTeacher>) -> Option<BTreeSet<u32>> {
        let mut output = BTreeSet::new();

        for slot_group in &self.slot_groups {
            let weeks = slot_group.extract_weeks(slots)?;
            output.extend(weeks);
        }

        Some(output)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingRequirements {
    pub constraints: BalancingConstraints,
    pub slot_selections: Vec<BalancingSlotSelection>,
}

impl BalancingRequirements {
    pub fn extract_slots_by_selection(&self) -> Vec<BTreeSet<usize>> {
        self.slot_selections
            .iter()
            .map(|slot_selection| slot_selection.extract_slots())
            .collect()
    }

    fn build_data<T: BalancingData>(slots: &Vec<SlotWithTeacher>) -> Vec<BalancingSlotSelection> {
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

        let mut slot_type_week_map: BTreeMap<BTreeMap<T, usize>, BTreeSet<u32>> = BTreeMap::new();

        for (week, data) in week_map {
            match slot_type_week_map.get_mut(&data) {
                Some(value) => {
                    value.insert(week);
                }
                None => {
                    slot_type_week_map.insert(data, BTreeSet::from([week]));
                }
            }
        }

        slot_type_week_map
            .into_iter()
            .map(|(descs, weeks)| BalancingSlotSelection {
                slot_groups: descs
                    .into_iter()
                    .map(|(slot_type, count)| BalancingSlotGroup {
                        slots: slots
                            .iter()
                            .enumerate()
                            .filter(|(_i, x)| {
                                slot_type.is_slot_relevant(x) && weeks.contains(&x.start.week)
                            })
                            .map(|(i, _x)| i)
                            .collect(),
                        count,
                    })
                    .collect(),
            })
            .collect()
    }

    pub fn balance_teachers_from_slots(
        slots: &Vec<SlotWithTeacher>,
    ) -> Vec<BalancingSlotSelection> {
        Self::build_data::<TeacherBalancing>(slots)
    }

    pub fn balance_timeslots_from_slots(
        slots: &Vec<SlotWithTeacher>,
    ) -> Vec<BalancingSlotSelection> {
        Self::build_data::<TimeslotBalancing>(slots)
    }

    pub fn balance_teachers_and_timeslots_from_slots(
        slots: &Vec<SlotWithTeacher>,
    ) -> Vec<BalancingSlotSelection> {
        Self::build_data::<TeacherAndTimeslotBalancing>(slots)
    }

    pub fn default_from_slots(slots: &Vec<SlotWithTeacher>) -> Self {
        BalancingRequirements {
            constraints: BalancingConstraints::OptimizeOnly,
            slot_selections: Self::balance_teachers_and_timeslots_from_slots(slots),
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotsInformation {
    pub slots: Vec<SlotWithTeacher>,
    pub balancing_requirements: BalancingRequirements,
}

impl SlotsInformation {
    pub fn from_slots(slots: Vec<SlotWithTeacher>) -> Self {
        SlotsInformation {
            balancing_requirements: BalancingRequirements::default_from_slots(&slots),
            slots,
        }
    }

    pub fn balance_teachers_and_timeslots_from_slots(
        slots: Vec<SlotWithTeacher>,
        constraints: BalancingConstraints,
    ) -> Self {
        SlotsInformation {
            balancing_requirements: BalancingRequirements {
                constraints,
                slot_selections: BalancingRequirements::balance_teachers_and_timeslots_from_slots(
                    &slots,
                ),
            },
            slots,
        }
    }

    pub fn balance_teachers_from_slots(
        slots: Vec<SlotWithTeacher>,
        constraints: BalancingConstraints,
    ) -> Self {
        SlotsInformation {
            balancing_requirements: BalancingRequirements {
                constraints,
                slot_selections: BalancingRequirements::balance_teachers_from_slots(&slots),
            },
            slots,
        }
    }

    pub fn balance_timeslots_from_slots(
        slots: Vec<SlotWithTeacher>,
        constraints: BalancingConstraints,
    ) -> Self {
        SlotsInformation {
            balancing_requirements: BalancingRequirements {
                constraints,
                slot_selections: BalancingRequirements::balance_timeslots_from_slots(&slots),
            },
            slots,
        }
    }
}

impl Default for SlotsInformation {
    fn default() -> Self {
        Self::from_slots(vec![])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    pub max_groups_per_slot: NonZeroUsize,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub is_tutorial: bool,
    pub duration: NonZeroU32,
    pub slots_information: SlotsInformation,
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
            duration: NonZeroU32::new(60).unwrap(),
            slots_information: SlotsInformation::default(),
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
            for (j, slot_selection) in subject
                .slots_information
                .balancing_requirements
                .slot_selections
                .iter()
                .enumerate()
            {
                if slot_selection.slot_groups.is_empty() {
                    return Err(Error::SubjectWithEmptySlotSelectionInBalancing(i, j));
                }

                let mut used_slots = BTreeSet::new();

                for (k, slot_group) in slot_selection.slot_groups.iter().enumerate() {
                    if slot_group.slots.is_empty() {
                        return Err(Error::SubjectWithEmptySlotGroupInBalancing(i, j, k));
                    }

                    for &slot in &slot_group.slots {
                        if used_slots.contains(&slot) {
                            return Err(
                                Error::SubjectWithOverlappingSlotsInBalancingSlotSelection(i, j),
                            );
                        }
                        if slot >= subject.slots_information.slots.len() {
                            return Err(Error::SubjectWithInvalidSlotInBalancing(i, slot));
                        }
                        used_slots.insert(slot);
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

            for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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
                if slot_ref.slot >= subjects[slot_ref.subject].slots_information.slots.len() {
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
    GroupOnSlotSelection {
        subject: usize,
        slot_selection: usize,
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
            Variable::GroupOnSlotSelection {
                subject,
                slot_selection,
                group,
            } => write!(f, "GoSS_{}_{}_{}", *subject, *slot_selection, *group),
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

use crate::ilp::linexpr::{Constraint, Expr};
use crate::ilp::{FeasableConfig, Problem, ProblemBuilder};

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
            for slot in &subject.slots_information.slots {
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
                    .slots_information
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

    fn build_group_on_slot_selection_variables(&self) -> BTreeSet<Variable> {
        self.data
            .subjects
            .iter()
            .enumerate()
            .flat_map(|(i, subject)| {
                subject
                    .slots_information
                    .balancing_requirements
                    .slot_selections
                    .iter()
                    .enumerate()
                    .flat_map(move |(j, _ws)| {
                        subject.groups.prefilled_groups.iter().enumerate().map(
                            move |(k, _group)| Variable::GroupOnSlotSelection {
                                subject: i,
                                slot_selection: j,
                                group: k,
                            },
                        )
                    })
                    .collect::<Vec<_>>()
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
                    subject.slots_information.slots.iter().enumerate().flat_map(
                        move |(j, _slot)| {
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
                        },
                    )
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
                subject
                    .slots_information
                    .slots
                    .iter()
                    .enumerate()
                    .map(move |(j, _slot)| {
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
            for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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

        for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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

        for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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

        for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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
            for (j, _slot) in subject.slots_information.slots.iter().enumerate() {
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
            for (j, _slot) in subject.slots_information.slots.iter().enumerate() {
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
            for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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
            for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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
            for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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
            for (j, slot) in subject.slots_information.slots.iter().enumerate() {
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

    fn build_balancing_constraints_for_subject_overall_internal_for_slot_group(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        week_span: u32,
        relevant_slots: &BTreeSet<usize>,
        count: usize,
        total_count: usize,
        k: usize,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut expr = Expr::constant(0);

        for (j, _slot) in subject.slots_information.slots.iter().enumerate() {
            if !relevant_slots.contains(&j) {
                continue;
            }

            expr = expr
                + Expr::var(Variable::GroupInSlot {
                    subject: i,
                    slot: j,
                    group: k,
                });
        }

        let period_count = (week_span as f64) / (subject.period.get() as f64);
        let period_count_min = period_count.floor();
        let period_count_max = period_count.ceil();

        let expected_use_per_group_min = (count as f64) / (total_count as f64) * period_count_min;
        let expected_use_per_group_max = (count as f64) / (total_count as f64) * period_count_max;

        let max_use = expected_use_per_group_max.ceil() as i32;
        let min_use = expected_use_per_group_min.floor() as i32;

        let rhs_cond = Expr::var(Variable::GroupOnSlotSelection {
            subject: i,
            slot_selection: j,
            group: k,
        });

        if max_use == min_use {
            BTreeSet::from([expr.eq(&(max_use * &rhs_cond))])
        } else {
            BTreeSet::from([
                expr.leq(&(max_use * &rhs_cond)),
                expr.geq(&(min_use * &rhs_cond)),
            ])
        }
    }

    fn build_balancing_constraints_for_subject_overall_internal_for_slot_selection(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        slot_selection: &BalancingSlotSelection,
    ) -> BTreeSet<Constraint<Variable>> {
        let weeks = slot_selection
            .extract_weeks(&subject.slots_information.slots)
            .expect("Slot number in slot selection should be valid");
        let first_week_in_selection = weeks
            .first()
            .cloned()
            .expect("There should be weeks in slot selection");
        let last_week_in_selection = weeks
            .last()
            .cloned()
            .expect("There should be weeks in slot selection");

        let week_span = last_week_in_selection - first_week_in_selection + 1;

        let total_count = slot_selection
            .slot_groups
            .iter()
            .map(|slot_group| slot_group.count)
            .sum();

        let mut constraints = BTreeSet::new();

        for slot_group in &slot_selection.slot_groups {
            let relevant_slots = &slot_group.slots;
            let count = slot_group.count;
            for (k, _group) in subject.groups.prefilled_groups.iter().enumerate() {
                constraints.extend(
                    self.build_balancing_constraints_for_subject_overall_internal_for_slot_group(
                        i,
                        subject,
                        j,
                        week_span,
                        relevant_slots,
                        count,
                        total_count,
                        k,
                    ),
                );
            }
        }

        constraints
    }

    fn build_balancing_constraints_for_subject_overall(
        &self,
        i: usize,
        subject: &Subject,
        slot_selections: &Vec<BalancingSlotSelection>,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (j, slot_selection) in slot_selections.iter().enumerate() {
            constraints.extend(
                self.build_balancing_constraints_for_subject_overall_internal_for_slot_selection(
                    i,
                    subject,
                    j,
                    slot_selection,
                ),
            );
        }

        constraints
    }

    fn build_balancing_constraints_for_subject_strict_internal_for_range_group_and_slot_group(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        relevant_slots: &BTreeSet<usize>,
        count: usize,
        window_size: u32,
        range: &std::ops::Range<u32>,
        k: usize,
    ) -> Constraint<Variable> {
        let mut expr = Expr::constant(0);

        for (slot_num, slot) in subject.slots_information.slots.iter().enumerate() {
            if !range.contains(&slot.start.week) {
                continue;
            }
            if !relevant_slots.contains(&slot_num) {
                continue;
            }

            expr = expr
                + Expr::var(Variable::GroupInSlot {
                    subject: i,
                    slot: slot_num,
                    group: k,
                });
        }

        let current_range_length = range.end - range.start;

        assert!(current_range_length <= window_size);
        let count_i32 = i32::try_from(count).expect("Slot count for balancing should fit in i32");
        let goss = Expr::var(Variable::GroupOnSlotSelection {
            subject: i,
            slot_selection: j,
            group: k,
        });
        if current_range_length < window_size {
            expr.leq(&(count_i32 * goss))
        } else {
            expr.eq(&(count_i32 * goss))
        }
    }

    fn generate_cuts_ranges(&self) -> Vec<std::ops::Range<u32>> {
        let mut output = Vec::new();

        let mut prev = 0;
        for cut in &self.data.general.periodicity_cuts {
            output.push(prev..cut.get());
            prev = cut.get();
        }

        output.push(prev..self.data.general.week_count.get());

        output
    }

    fn generate_rolling_ranges(
        &self,
        initial_ranges: Vec<std::ops::Range<u32>>,
        window_size: u32,
    ) -> Vec<std::ops::Range<u32>> {
        let mut output = Vec::new();

        for range in initial_ranges {
            let range_size = range.end - range.start;
            if window_size >= range_size {
                output.push(range);
                continue;
            }

            let start = range.start;
            let end = range.end - window_size + 1;
            for i in start..end {
                output.push(i..(i + window_size));
            }
        }

        output
    }

    fn generate_ranges_for_balancing(
        &self,
        subject: &Subject,
        slot_selection: &BalancingSlotSelection,
        allow_cuts: bool,
    ) -> (Vec<std::ops::Range<u32>>, u32) {
        let total_count_usize: usize = slot_selection
            .slot_groups
            .iter()
            .map(|slot_group| slot_group.count)
            .sum();
        let total_count = u32::try_from(total_count_usize)
            .expect("Number of slots for balancing data should fit in u32");

        let window_size = total_count * subject.period.get();

        let initial_ranges = if allow_cuts {
            self.generate_cuts_ranges()
        } else {
            vec![0..self.data.general.week_count.get()]
        };

        let rolling_ranges = self.generate_rolling_ranges(initial_ranges, window_size);

        let weeks = slot_selection
            .extract_weeks(&subject.slots_information.slots)
            .expect("Slot number in slot selection should be valid");
        let first_week_in_selection = weeks
            .first()
            .cloned()
            .expect("There should be weeks in slot selection");
        let last_week_in_selection = weeks
            .last()
            .cloned()
            .expect("There should be weeks in slot selection");

        (
            rolling_ranges
                .into_iter()
                .filter(|range| {
                    (range.start >= first_week_in_selection)
                        && (range.end <= last_week_in_selection + 1)
                })
                .collect(),
            window_size,
        )
    }

    fn build_balancing_constraints_for_subject_strict_internal_for_slot_selection(
        &self,
        i: usize,
        subject: &Subject,
        j: usize,
        slot_selection: &BalancingSlotSelection,
        allow_cuts: bool,
    ) -> BTreeSet<Constraint<Variable>> {
        let (ranges, window_size) =
            self.generate_ranges_for_balancing(subject, slot_selection, allow_cuts);

        let mut output = BTreeSet::new();

        for range in ranges {
            for slot_group in &slot_selection.slot_groups {
                let count = slot_group.count;
                let relevant_slots = &slot_group.slots;
                for (k, _group) in subject.groups.prefilled_groups.iter().enumerate() {
                    output.insert(
                        self.build_balancing_constraints_for_subject_strict_internal_for_range_group_and_slot_group(
                            i,
                            subject,
                            j,
                            relevant_slots,
                            count,
                            window_size,
                            &range,
                            k,
                        )
                    );
                }
            }
        }

        output
    }

    fn build_balancing_constraints_for_subject_strict(
        &self,
        i: usize,
        subject: &Subject,
        slot_selections: &Vec<BalancingSlotSelection>,
        allow_cuts: bool,
    ) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (j, slot_selection) in slot_selections.iter().enumerate() {
            constraints.extend(
                self.build_balancing_constraints_for_subject_strict_internal_for_slot_selection(
                    i,
                    subject,
                    j,
                    slot_selection,
                    allow_cuts,
                ),
            );
        }

        constraints
    }

    fn build_balancing_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            let slot_selections = &subject
                .slots_information
                .balancing_requirements
                .slot_selections;
            match &subject.slots_information.balancing_requirements.constraints {
                BalancingConstraints::OptimizeOnly => {} // Ignore, no strict constraint in this case
                BalancingConstraints::OverallOnly => {
                    constraints.extend(self.build_balancing_constraints_for_subject_overall(
                        i,
                        subject,
                        slot_selections,
                    ))
                }
                BalancingConstraints::Strict => {
                    constraints.extend(self.build_balancing_constraints_for_subject_strict(
                        i,
                        subject,
                        slot_selections,
                        false,
                    ))
                }
                BalancingConstraints::StrictWithCuts => {
                    constraints.extend(self.build_balancing_constraints_for_subject_strict(
                        i,
                        subject,
                        slot_selections,
                        true,
                    ))
                }
                BalancingConstraints::StrictWithCutsAndOverall => {
                    constraints.extend(self.build_balancing_constraints_for_subject_strict(
                        i,
                        subject,
                        slot_selections,
                        true,
                    ));
                    constraints.extend(self.build_balancing_constraints_for_subject_overall(
                        i,
                        subject,
                        slot_selections,
                    ));
                }
            }
        }

        constraints
    }

    fn build_balancing_optimizer(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            constraints.extend(
                self.build_balancing_constraints_for_subject_strict(
                    i,
                    subject,
                    &subject
                        .slots_information
                        .balancing_requirements
                        .slot_selections,
                    false,
                ),
            );
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

        for (l, slot) in subject.slots_information.slots.iter().enumerate() {
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

    fn build_group_on_slot_selection_constraints_at_least_one_slot_in_selection(
        &self,
        i: usize,
        j: usize,
        slot_selection: &BalancingSlotSelection,
        k: usize,
    ) -> Constraint<Variable> {
        let mut expr_sum = Expr::constant(0);
        let expr_goss = Expr::var(Variable::GroupOnSlotSelection {
            subject: i,
            slot_selection: j,
            group: k,
        });

        for slot in slot_selection.extract_slots() {
            let expr_gis = Expr::var(Variable::GroupInSlot {
                subject: i,
                slot,
                group: k,
            });

            expr_sum = expr_sum + &expr_gis;
        }

        expr_goss.leq(&expr_sum)
    }

    fn build_group_on_slot_selection_constraints_slot_allowed_if_in_selection(
        &self,
        i: usize,
        k: usize,
        slot: usize,
        slot_selections: &Vec<BalancingSlotSelection>,
    ) -> Constraint<Variable> {
        let mut expr_sum = Expr::constant(0);
        let expr_gis = Expr::var(Variable::GroupInSlot {
            subject: i,
            slot,
            group: k,
        });

        for (j, slot_selection) in slot_selections.iter().enumerate() {
            if slot_selection.contains_slot(slot) {
                let expr_goss = Expr::var(Variable::GroupOnSlotSelection {
                    subject: i,
                    slot_selection: j,
                    group: k,
                });

                expr_sum = expr_sum + &expr_goss;
            }
        }

        expr_gis.leq(&expr_sum)
    }

    fn build_group_on_slot_selection_constraints_choice_for_subject_and_group(
        &self,
        i: usize,
        k: usize,
        slot_selections: &Vec<BalancingSlotSelection>,
    ) -> Constraint<Variable> {
        let mut choice_expr = Expr::constant(0);

        for (j, _slot_selection) in slot_selections.iter().enumerate() {
            choice_expr = choice_expr
                + Expr::var(Variable::GroupOnSlotSelection {
                    subject: i,
                    slot_selection: j,
                    group: k,
                });
        }

        choice_expr.eq(&Expr::constant(1))
    }

    fn build_group_on_slot_selection_constraints(&self) -> BTreeSet<Constraint<Variable>> {
        let mut constraints = BTreeSet::new();

        for (i, subject) in self.data.subjects.iter().enumerate() {
            let slot_selections = &subject
                .slots_information
                .balancing_requirements
                .slot_selections;

            for (k, _group) in subject.groups.prefilled_groups.iter().enumerate() {
                let choice_constraint = self
                    .build_group_on_slot_selection_constraints_choice_for_subject_and_group(
                        i,
                        k,
                        &slot_selections,
                    );
                constraints.insert(choice_constraint);

                for (j, slot_selection) in slot_selections.iter().enumerate() {
                    constraints.insert(
                        self.build_group_on_slot_selection_constraints_at_least_one_slot_in_selection(
                            i,
                            j,
                            slot_selection,
                            k
                        )
                    );
                }

                for (slot, _) in subject.slots_information.slots.iter().enumerate() {
                    constraints.insert(
                        self.build_group_on_slot_selection_constraints_slot_allowed_if_in_selection(
                            i,
                            k,
                            slot,
                            &slot_selections,
                        )
                    );
                }
            }
        }

        constraints
    }

    fn problem_builder_soft(&self) -> ProblemBuilder<Variable> {
        ProblemBuilder::new()
            .add_bool_variables(self.build_group_in_slot_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_dynamic_group_assignment_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_student_in_group_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_use_grouping_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_incompat_group_for_student_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_group_on_slot_selection_variables())
            .expect("Should not have duplicates")
            .add_constraints(self.build_interrogations_per_week_optimizer())
            .expect("Variables should be declared")
            .add_constraints(self.build_max_interrogations_per_day_optimizer())
            .expect("Variables should be declared")
            .add_constraints(self.build_one_interrogation_per_period_optimizer())
            .expect("Variables should be declared")
            .add_constraints(self.build_balancing_optimizer())
            .expect("Variables should be declared")
    }

    fn problem_builder_hard(&self) -> ProblemBuilder<Variable> {
        ProblemBuilder::new()
            .add_bool_variables(self.build_group_in_slot_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_dynamic_group_assignment_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_student_in_group_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_use_grouping_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_incompat_group_for_student_variables())
            .expect("Should not have duplicates")
            .add_bool_variables(self.build_group_on_slot_selection_variables())
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
            .add_constraints(self.build_group_on_slot_selection_constraints())
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
                    let bool_vars = x.get_bool_vars();

                    let mut manual_costs = 0.;
                    for (var, &value) in &bool_vars {
                        if let Variable::GroupInSlot {
                            subject,
                            slot,
                            group: _,
                        } = var
                        {
                            if value {
                                manual_costs += f64::from(
                                    subjects[*subject].slots_information.slots[*slot].cost,
                                );
                            }
                        }
                    }

                    let soft_config = soft_problem
                        .config_from(bool_vars)
                        .expect("Variables should match");
                    // If some constraints are inequalities, this will still measure the difference to equality
                    let sq2_cost = soft_config.compute_lhs_sq_norm2();

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
                    .get_bool(&Variable::StudentInGroup {
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

        let mut slots = Vec::with_capacity(subject.slots_information.slots.len());

        for (j, _slot) in subject.slots_information.slots.iter().enumerate() {
            let mut assigned_groups = BTreeSet::new();

            for k in 0..subject.groups.prefilled_groups.len() {
                if config
                    .get_bool(&Variable::GroupInSlot {
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
