pub mod sqlite;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdError<T, Id>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CheckedIdError<T, Id, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CheckData: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Check failed. Data provided is: {0:?}")]
    CheckFailed(CheckData),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

impl<T, Id, CheckData> CheckedIdError<T, Id, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CheckData: std::fmt::Debug,
{
    fn from_id_error(id_error: IdError<T, Id>) -> Self {
        match id_error {
            IdError::InvalidId(id) => CheckedIdError::InvalidId(id),
            IdError::InternalError(int_err) => CheckedIdError::InternalError(int_err),
        }
    }
}

#[derive(Error, Debug)]
pub enum Id2Error<T, Id1, Id2>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossId3Error<T, Id1, Id2, Id3, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
    Id3: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
    #[error("Id {0:?} is invalid")]
    InvalidId3(Id3),
    #[error("Id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossError<T, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossIdError<T, Id, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3Error<T, CrossId1, CrossId2, CrossId3>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3IdError<T, Id, CrossId1, CrossId2, CrossId3>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum InvalidCrossError<T, Data, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Data: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Data to be stored is invalid: {0:?}")]
    InvalidData(Data),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum InvalidCrossIdError<T, Data, Id, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Data: std::fmt::Debug,
    Id: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Data to be stored is invalid: {0:?}")]
    InvalidData(Data),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

pub trait OrdId:
    std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy
{
}
impl<T: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy> OrdId
    for T
{
}

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

#[trait_variant::make(Send)]
pub trait Storage: Send + Sync + std::fmt::Debug {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;
    type StudentId: OrdId;
    type SubjectGroupId: OrdId;
    type IncompatId: OrdId;
    type GroupListId: OrdId;
    type SubjectId: OrdId;
    type TimeSlotId: OrdId;
    type GroupingId: OrdId;
    type GroupingIncompatId: OrdId;

    type InternalError: std::fmt::Debug + std::error::Error;

    async fn general_data_set(
        &mut self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn general_data_get(&self) -> std::result::Result<GeneralData, Self::InternalError>;

    async fn week_patterns_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::WeekPatternId, WeekPattern>, Self::InternalError>;
    async fn week_patterns_get(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<Self::InternalError, Self::WeekPatternId>>;
    async fn week_patterns_add(
        &mut self,
        pattern: &WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError>;
    async unsafe fn week_patterns_remove_unchecked(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn week_patterns_update(
        &mut self,
        index: Self::WeekPatternId,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>;

    async fn teachers_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::TeacherId, Teacher>, Self::InternalError>;
    async fn teachers_get(
        &self,
        index: Self::TeacherId,
    ) -> std::result::Result<Teacher, IdError<Self::InternalError, Self::TeacherId>>;
    async fn teachers_add(
        &mut self,
        teacher: &Teacher,
    ) -> std::result::Result<Self::TeacherId, Self::InternalError>;
    async unsafe fn teachers_remove_unchecked(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn teachers_update(
        &mut self,
        index: Self::TeacherId,
        teacher: &Teacher,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>;

    async fn students_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::StudentId, Student>, Self::InternalError>;
    async fn students_get(
        &self,
        index: Self::StudentId,
    ) -> std::result::Result<Student, IdError<Self::InternalError, Self::StudentId>>;
    async fn students_add(
        &mut self,
        student: &Student,
    ) -> std::result::Result<Self::StudentId, Self::InternalError>;
    async unsafe fn students_remove_unchecked(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn students_update(
        &mut self,
        index: Self::StudentId,
        student: &Student,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::StudentId>>;

    async fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::SubjectGroupId, SubjectGroup>, Self::InternalError>;
    async fn subject_groups_get(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_add(
        &mut self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<Self::SubjectGroupId, Self::InternalError>;
    async unsafe fn subject_groups_remove_unchecked(
        &mut self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn subject_groups_update(
        &mut self,
        index: Self::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;

    async fn incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::IncompatId, Incompat<Self::WeekPatternId>>,
        Self::InternalError,
    >;
    async fn incompats_get(
        &self,
        index: Self::IncompatId,
    ) -> std::result::Result<
        Incompat<Self::WeekPatternId>,
        IdError<Self::InternalError, Self::IncompatId>,
    >;
    async unsafe fn incompats_add_unchecked(
        &mut self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<Self::IncompatId, Self::InternalError>;
    async unsafe fn incompats_remove_unchecked(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn incompats_update_unchecked(
        &mut self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn group_lists_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupListId, GroupList<Self::StudentId>>,
        Self::InternalError,
    >;
    async fn group_lists_get(
        &self,
        index: Self::GroupListId,
    ) -> std::result::Result<
        GroupList<Self::StudentId>,
        IdError<Self::InternalError, Self::GroupListId>,
    >;
    async unsafe fn group_lists_add_unchecked(
        &mut self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<Self::GroupListId, Self::InternalError>;
    async unsafe fn group_lists_remove_unchecked(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn group_lists_update_unchecked(
        &mut self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn subjects_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<
            Self::SubjectId,
            Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        >,
        Self::InternalError,
    >;
    async fn subjects_get(
        &self,
        index: Self::SubjectId,
    ) -> std::result::Result<
        Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        IdError<Self::InternalError, Self::SubjectId>,
    >;
    async unsafe fn subjects_add_unchecked(
        &mut self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<Self::SubjectId, Self::InternalError>;
    async unsafe fn subjects_remove_unchecked(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn subjects_update_unchecked(
        &mut self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn time_slots_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::TimeSlotId, TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>>,
        Self::InternalError,
    >;
    async fn time_slots_get(
        &self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<
        TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
        IdError<Self::InternalError, Self::TimeSlotId>,
    >;
    async unsafe fn time_slots_add_unchecked(
        &mut self,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<Self::TimeSlotId, Self::InternalError>;
    async unsafe fn time_slots_remove_unchecked(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn time_slots_update_unchecked(
        &mut self,
        index: Self::TimeSlotId,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn groupings_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingId, Grouping<Self::TimeSlotId>>,
        Self::InternalError,
    >;
    async fn groupings_get(
        &self,
        index: Self::GroupingId,
    ) -> std::result::Result<
        Grouping<Self::TimeSlotId>,
        IdError<Self::InternalError, Self::GroupingId>,
    >;
    async unsafe fn groupings_add_unchecked(
        &mut self,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<Self::GroupingId, Self::InternalError>;
    async unsafe fn groupings_remove_unchecked(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn groupings_update_unchecked(
        &mut self,
        index: Self::GroupingId,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn grouping_incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingIncompatId, GroupingIncompat<Self::GroupingId>>,
        Self::InternalError,
    >;
    async fn grouping_incompats_get(
        &self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<
        GroupingIncompat<Self::GroupingId>,
        IdError<Self::InternalError, Self::GroupingIncompatId>,
    >;
    async unsafe fn grouping_incompats_add_unchecked(
        &mut self,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<Self::GroupingIncompatId, Self::InternalError>;
    async unsafe fn grouping_incompats_remove_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn grouping_incompats_update_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async unsafe fn subject_group_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
        subject_id: Option<Self::SubjectId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn subject_group_for_student_get(
        &self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
    ) -> std::result::Result<
        Option<Self::SubjectId>,
        Id2Error<Self::InternalError, Self::StudentId, Self::SubjectGroupId>,
    >;

    async unsafe fn incompat_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
        enabled: bool,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn incompat_for_student_get(
        &self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
    ) -> std::result::Result<bool, Id2Error<Self::InternalError, Self::StudentId, Self::IncompatId>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralData {
    interrogations_per_week: Option<std::ops::Range<u32>>,
    max_interrogations_per_day: Option<NonZeroU32>,
}

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Week(u32);

impl Week {
    pub fn new(number: u32) -> Week {
        Week(number)
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekPattern {
    pub name: String,
    pub weeks: BTreeSet<Week>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teacher {
    pub surname: String,
    pub firstname: String,
    pub contact: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Student {
    pub surname: String,
    pub firstname: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectGroup {
    pub name: String,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotStart {
    day: crate::time::Weekday,
    time: crate::time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatSlot<WeekPatternId: OrdId> {
    week_pattern_id: WeekPatternId,
    start: SlotStart,
    duration: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatGroup<WeekPatternId: OrdId> {
    slots: BTreeSet<IncompatSlot<WeekPatternId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompat<WeekPatternId: OrdId> {
    pub name: String,
    pub max_count: usize,
    pub groups: BTreeSet<IncompatGroup<WeekPatternId>>,
}

impl<WeekPatternId: OrdId> Incompat<WeekPatternId> {
    pub fn references_week_pattern(&self, week_pattern_id: WeekPatternId) -> bool {
        for group in &self.groups {
            for slot in &group.slots {
                if slot.week_pattern_id == week_pattern_id {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub extendable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: OrdId> {
    pub name: String,
    pub groups: Vec<Group>,
    pub students_mapping: BTreeMap<StudentId, usize>,
}

impl<StudentId: OrdId> GroupList<StudentId> {
    pub fn references_student(&self, student_id: StudentId) -> bool {
        self.students_mapping.contains_key(&student_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingRequirements {
    pub teachers: bool,
    pub timeslots: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject<SubjectGroupId: OrdId, IncompatId: OrdId, GroupListId: OrdId> {
    pub name: String,
    pub subject_group_id: SubjectGroupId,
    pub incompat_id: Option<IncompatId>,
    pub group_list_id: Option<GroupListId>,
    pub duration: NonZeroU32,
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub is_tutorial: bool,
    pub max_groups_per_slot: NonZeroUsize,
    pub balancing_requirements: BalancingRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeSlot<SubjectId: OrdId, TeacherId: OrdId, WeekPatternId: OrdId> {
    pub subject_id: SubjectId,
    pub teacher_id: TeacherId,
    pub start: SlotStart,
    pub week_pattern_id: WeekPatternId,
    pub room: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grouping<TimeSlotId: OrdId> {
    pub name: String,
    pub slots: BTreeSet<TimeSlotId>,
}

impl<TimeSlotId: OrdId> Grouping<TimeSlotId> {
    pub fn references_time_slot(&self, time_slot_id: TimeSlotId) -> bool {
        self.slots.contains(&time_slot_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupingIncompat<GroupingId: OrdId> {
    pub max_count: NonZeroUsize,
    pub groupings: BTreeSet<GroupingId>,
}

impl<GroupingId: OrdId> GroupingIncompat<GroupingId> {
    pub fn references_grouping(&self, grouping_id: GroupingId) -> bool {
        self.groupings.contains(&grouping_id)
    }
}

#[derive(Clone, Debug)]
pub enum WeekPatternDependancy<IncompatId: OrdId, TimeSlotId: OrdId> {
    Incompat(IncompatId),
    TimeSlot(TimeSlotId),
}

#[derive(Clone, Debug)]
pub enum SubjectGroupDependancy<SubjectId: OrdId, StudentId: OrdId> {
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Clone, Debug)]
pub enum IncompatDependancy<SubjectId: OrdId, StudentId: OrdId> {
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Clone, Debug)]
pub enum SubjectDependancy<TimeSlotId: OrdId, StudentId: OrdId> {
    TimeSlot(TimeSlotId),
    Student(StudentId),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithId<Id: OrdId> {
    Ok,
    BadCrossId(Id),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithId3<Id1: OrdId, Id2: OrdId, Id3: OrdId> {
    Ok,
    BadCrossId1(Id1),
    BadCrossId2(Id2),
    BadCrossId3(Id3),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithIdAndInvalidState<Id: OrdId> {
    Ok,
    InvalidData,
    BadCrossId(Id),
}

#[derive(Debug)]
pub struct Logic<T: Storage> {
    storage: T,
}

impl<T: Storage> Logic<T> {
    pub fn new(storage: T) -> Self {
        Logic { storage }
    }
}

impl<T: Storage> Logic<T> {
    pub async fn general_data_set(
        &mut self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), T::InternalError> {
        self.storage.general_data_set(general_data).await
    }
    pub async fn general_data_get(&self) -> std::result::Result<GeneralData, T::InternalError> {
        self.storage.general_data_get().await
    }

    pub async fn week_patterns_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::WeekPatternId, WeekPattern>, T::InternalError> {
        self.storage.week_patterns_get_all().await
    }
    pub async fn week_patterns_get(
        &self,
        index: T::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<T::InternalError, T::WeekPatternId>> {
        self.storage.week_patterns_get(index).await
    }
    pub async fn week_patterns_add(
        &mut self,
        pattern: &WeekPattern,
    ) -> std::result::Result<T::WeekPatternId, T::InternalError> {
        self.storage.week_patterns_add(pattern).await
    }
    pub async fn week_patterns_update(
        &mut self,
        index: T::WeekPatternId,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), IdError<T::InternalError, T::WeekPatternId>> {
        self.storage.week_patterns_update(index, pattern).await
    }
    pub async fn week_patterns_check_can_remove(
        &mut self,
        index: T::WeekPatternId,
    ) -> std::result::Result<
        Vec<WeekPatternDependancy<T::IncompatId, T::TimeSlotId>>,
        IdError<T::InternalError, T::WeekPatternId>,
    > {
        let week_patterns = self.week_patterns_get_all().await?;

        if !week_patterns.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let incompats = self.incompats_get_all().await?;
        for (incompat_id, incompat) in incompats {
            if incompat.references_week_pattern(index) {
                dependancies.push(WeekPatternDependancy::Incompat(incompat_id));
            }
        }

        let time_slots = self.time_slots_get_all().await?;
        for (time_slot_id, time_slot) in time_slots {
            if time_slot.week_pattern_id == index {
                dependancies.push(WeekPatternDependancy::TimeSlot(time_slot_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn week_patterns_remove(
        &mut self,
        index: T::WeekPatternId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::WeekPatternId,
            Vec<WeekPatternDependancy<T::IncompatId, T::TimeSlotId>>,
        >,
    > {
        let dependancies = self
            .week_patterns_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.week_patterns_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn teachers_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::TeacherId, Teacher>, T::InternalError> {
        self.storage.teachers_get_all().await
    }
    pub async fn teachers_get(
        &self,
        index: T::TeacherId,
    ) -> std::result::Result<Teacher, IdError<T::InternalError, T::TeacherId>> {
        self.storage.teachers_get(index).await
    }
    pub async fn teachers_add(
        &mut self,
        teacher: &Teacher,
    ) -> std::result::Result<T::TeacherId, T::InternalError> {
        self.storage.teachers_add(teacher).await
    }
    pub async fn teachers_update(
        &mut self,
        index: T::TeacherId,
        teacher: &Teacher,
    ) -> std::result::Result<(), IdError<T::InternalError, T::TeacherId>> {
        self.storage.teachers_update(index, teacher).await
    }
    pub async fn teachers_check_can_remove(
        &mut self,
        index: T::TeacherId,
    ) -> std::result::Result<Vec<T::TimeSlotId>, IdError<T::InternalError, T::TeacherId>> {
        let teachers = self.teachers_get_all().await?;

        if !teachers.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let time_slots = self.time_slots_get_all().await?;
        for (time_slot_id, time_slot) in time_slots {
            if time_slot.teacher_id == index {
                dependancies.push(time_slot_id);
            }
        }

        Ok(dependancies)
    }
    pub async fn teachers_patterns_remove(
        &mut self,
        index: T::TeacherId,
    ) -> std::result::Result<(), CheckedIdError<T::InternalError, T::TeacherId, Vec<T::TimeSlotId>>>
    {
        let dependancies = self
            .teachers_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.teachers_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn students_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::StudentId, Student>, T::InternalError> {
        self.storage.students_get_all().await
    }
    pub async fn students_get(
        &self,
        index: T::StudentId,
    ) -> std::result::Result<Student, IdError<T::InternalError, T::StudentId>> {
        self.storage.students_get(index).await
    }
    pub async fn students_add(
        &mut self,
        student: &Student,
    ) -> std::result::Result<T::StudentId, T::InternalError> {
        self.storage.students_add(student).await
    }
    pub async fn students_update(
        &mut self,
        index: T::StudentId,
        student: &Student,
    ) -> std::result::Result<(), IdError<T::InternalError, T::StudentId>> {
        self.storage.students_update(index, student).await
    }
    pub async fn students_check_can_remove(
        &mut self,
        index: T::StudentId,
    ) -> std::result::Result<Vec<T::GroupListId>, IdError<T::InternalError, T::StudentId>> {
        let students = self.students_get_all().await?;

        if !students.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let group_lists = self.group_lists_get_all().await?;
        for (group_list_id, group_list) in group_lists {
            if group_list.references_student(index) {
                dependancies.push(group_list_id)
            }
        }

        Ok(dependancies)
    }
    pub async fn students_remove(
        &mut self,
        index: T::StudentId,
    ) -> std::result::Result<(), CheckedIdError<T::InternalError, T::StudentId, Vec<T::GroupListId>>>
    {
        let dependancies = self
            .students_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.students_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::SubjectGroupId, SubjectGroup>, T::InternalError> {
        self.storage.subject_groups_get_all().await
    }
    pub async fn subject_groups_get(
        &self,
        index: T::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<T::InternalError, T::SubjectGroupId>> {
        self.storage.subject_groups_get(index).await
    }
    pub async fn subject_groups_add(
        &mut self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<T::SubjectGroupId, T::InternalError> {
        self.storage.subject_groups_add(subject_group).await
    }
    pub async fn subject_groups_update(
        &mut self,
        index: T::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<(), IdError<T::InternalError, T::SubjectGroupId>> {
        self.storage
            .subject_groups_update(index, subject_group)
            .await
    }
    pub async fn subject_groups_can_remove(
        &mut self,
        index: T::SubjectGroupId,
    ) -> std::result::Result<
        Vec<SubjectGroupDependancy<T::SubjectId, T::StudentId>>,
        IdError<T::InternalError, T::SubjectGroupId>,
    > {
        let subject_groups = self.subject_groups_get_all().await?;

        if !subject_groups.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let subjects = self.subjects_get_all().await?;
        for (subject_id, subject) in subjects {
            if subject.subject_group_id == index {
                dependancies.push(SubjectGroupDependancy::Subject(subject_id));
            }
        }

        let students = self.students_get_all().await?;
        for (student_id, _student) in students {
            let subject_for_student = self.subject_group_for_student_get(student_id, index)
                .await
                .map_err(
                    |e| match e {
                        Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                        Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was tested valid a few instructions ago", id2),
                    }
                )?;
            if subject_for_student.is_some() {
                dependancies.push(SubjectGroupDependancy::Student(student_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn subject_groups_remove(
        &mut self,
        index: T::SubjectGroupId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::SubjectGroupId,
            Vec<SubjectGroupDependancy<T::SubjectId, T::StudentId>>,
        >,
    > {
        let dependancies = self
            .subject_groups_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.subject_groups_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn incompats_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::IncompatId, Incompat<T::WeekPatternId>>, T::InternalError>
    {
        self.storage.incompats_get_all().await
    }
    pub async fn incompats_get(
        &self,
        index: T::IncompatId,
    ) -> std::result::Result<Incompat<T::WeekPatternId>, IdError<T::InternalError, T::IncompatId>>
    {
        self.storage.incompats_get(index).await
    }
    pub async fn incompats_check_id(
        &self,
        index: T::IncompatId,
    ) -> std::result::Result<bool, T::InternalError> {
        let incompats = self.incompats_get_all().await?;

        Ok(incompats.contains_key(&index))
    }
    pub async fn incompats_check_data(
        &self,
        incompat: &Incompat<T::WeekPatternId>,
    ) -> std::result::Result<DataStatusWithId<T::WeekPatternId>, T::InternalError> {
        let week_patterns = self.week_patterns_get_all().await?;

        for incompat_group in &incompat.groups {
            for incompat_slot in &incompat_group.slots {
                if !week_patterns.contains_key(&incompat_slot.week_pattern_id) {
                    return Ok(DataStatusWithId::BadCrossId(incompat_slot.week_pattern_id));
                }
            }
        }

        Ok(DataStatusWithId::Ok)
    }
    pub async fn incompats_add(
        &mut self,
        incompat: &Incompat<T::WeekPatternId>,
    ) -> std::result::Result<T::IncompatId, CrossError<T::InternalError, T::WeekPatternId>> {
        let data_status = self.incompats_check_data(incompat).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                let id = unsafe { self.storage.incompats_add_unchecked(incompat) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn incompats_update(
        &mut self,
        index: T::IncompatId,
        incompat: &Incompat<T::WeekPatternId>,
    ) -> std::result::Result<(), CrossIdError<T::InternalError, T::IncompatId, T::WeekPatternId>>
    {
        if !self.incompats_check_id(index).await? {
            return Err(CrossIdError::InvalidId(index));
        }

        let data_status = self.incompats_check_data(incompat).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                unsafe { self.storage.incompats_update_unchecked(index, incompat) }.await?;
                Ok(())
            }
        }
    }
    pub async fn incompats_can_remove(
        &mut self,
        index: T::IncompatId,
    ) -> std::result::Result<
        Vec<IncompatDependancy<T::SubjectId, T::StudentId>>,
        IdError<T::InternalError, T::IncompatId>,
    > {
        if !self.incompats_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let subjects = self.subjects_get_all().await?;
        for (subject_id, subject) in subjects {
            if subject.incompat_id == Some(index) {
                dependancies.push(IncompatDependancy::Subject(subject_id));
            }
        }

        let students = self.students_get_all().await?;
        for (student_id, _student) in students {
            let incompat_for_student = self.incompat_for_student_get(student_id, index)
                .await
                .map_err(
                    |e| match e {
                        Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                        Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was tested valid a few instructions ago", id2),
                    }
                )?;
            if incompat_for_student {
                dependancies.push(IncompatDependancy::Student(student_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn incompats_remove(
        &mut self,
        index: T::IncompatId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::IncompatId,
            Vec<IncompatDependancy<T::SubjectId, T::StudentId>>,
        >,
    > {
        let dependancies = self
            .incompats_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.incompats_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn group_lists_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::GroupListId, GroupList<T::StudentId>>, T::InternalError>
    {
        self.storage.group_lists_get_all().await
    }
    pub async fn group_lists_get(
        &self,
        index: T::GroupListId,
    ) -> std::result::Result<GroupList<T::StudentId>, IdError<T::InternalError, T::GroupListId>>
    {
        self.storage.group_lists_get(index).await
    }
    pub async fn group_lists_check_id(
        &self,
        index: T::GroupListId,
    ) -> std::result::Result<bool, T::InternalError> {
        let group_lists = self.group_lists_get_all().await?;

        Ok(group_lists.contains_key(&index))
    }
    pub async fn group_lists_check_data(
        &self,
        group_list: &GroupList<T::StudentId>,
    ) -> std::result::Result<DataStatusWithIdAndInvalidState<T::StudentId>, T::InternalError> {
        let students = self.students_get_all().await?;

        for (&student_id, &group) in &group_list.students_mapping {
            if !students.contains_key(&student_id) {
                return Ok(DataStatusWithIdAndInvalidState::BadCrossId(student_id));
            }

            if group >= group_list.groups.len() {
                return Ok(DataStatusWithIdAndInvalidState::InvalidData);
            }
        }

        Ok(DataStatusWithIdAndInvalidState::Ok)
    }
    pub async fn group_lists_add(
        &mut self,
        group_list: &GroupList<T::StudentId>,
    ) -> std::result::Result<
        T::GroupListId,
        InvalidCrossError<T::InternalError, GroupList<T::StudentId>, T::StudentId>,
    > {
        let data_status = self.group_lists_check_data(group_list).await?;
        match data_status {
            DataStatusWithIdAndInvalidState::BadCrossId(id) => {
                Err(InvalidCrossError::InvalidCrossId(id))
            }
            DataStatusWithIdAndInvalidState::InvalidData => {
                Err(InvalidCrossError::InvalidData(group_list.clone()))
            }
            DataStatusWithIdAndInvalidState::Ok => {
                let id = unsafe { self.storage.group_lists_add_unchecked(group_list) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn group_lists_update(
        &mut self,
        index: T::GroupListId,
        group_list: &GroupList<T::StudentId>,
    ) -> std::result::Result<
        (),
        InvalidCrossIdError<
            T::InternalError,
            GroupList<T::StudentId>,
            T::GroupListId,
            T::StudentId,
        >,
    > {
        if !self.group_lists_check_id(index).await? {
            return Err(InvalidCrossIdError::InvalidId(index));
        }

        let data_status = self.group_lists_check_data(group_list).await?;
        match data_status {
            DataStatusWithIdAndInvalidState::BadCrossId(id) => {
                Err(InvalidCrossIdError::InvalidCrossId(id))
            }
            DataStatusWithIdAndInvalidState::InvalidData => {
                Err(InvalidCrossIdError::InvalidData(group_list.clone()))
            }
            DataStatusWithIdAndInvalidState::Ok => {
                unsafe { self.storage.group_lists_update_unchecked(index, group_list) }.await?;
                Ok(())
            }
        }
    }
    pub async fn group_lists_can_remove(
        &mut self,
        index: T::GroupListId,
    ) -> std::result::Result<Vec<T::SubjectId>, IdError<T::InternalError, T::GroupListId>> {
        if !self.group_lists_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let subjects = self.subjects_get_all().await?;
        for (subject_id, subject) in subjects {
            if subject.group_list_id == Some(index) {
                dependancies.push(subject_id);
            }
        }

        Ok(dependancies)
    }
    pub async fn group_lists_remove(
        &mut self,
        index: T::GroupListId,
    ) -> std::result::Result<(), CheckedIdError<T::InternalError, T::GroupListId, Vec<T::SubjectId>>>
    {
        let dependancies = self
            .group_lists_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.group_lists_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn subjects_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::SubjectId, Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>>,
        T::InternalError,
    > {
        self.storage.subjects_get_all().await
    }
    pub async fn subjects_get(
        &self,
        index: T::SubjectId,
    ) -> std::result::Result<
        Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
        IdError<T::InternalError, T::SubjectId>,
    > {
        self.storage.subjects_get(index).await
    }
    pub async fn subjects_check_id(
        &self,
        index: T::SubjectId,
    ) -> std::result::Result<bool, T::InternalError> {
        let subjects = self.subjects_get_all().await?;

        Ok(subjects.contains_key(&index))
    }
    pub async fn subjects_check_data(
        &self,
        subject: &Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    ) -> std::result::Result<
        DataStatusWithId3<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
        T::InternalError,
    > {
        let subject_groups = self.subject_groups_get_all().await?;
        if !subject_groups.contains_key(&subject.subject_group_id) {
            return Ok(DataStatusWithId3::BadCrossId1(subject.subject_group_id));
        }

        if let Some(incompat_id) = subject.incompat_id {
            let incompats = self.incompats_get_all().await?;
            if !incompats.contains_key(&incompat_id) {
                return Ok(DataStatusWithId3::BadCrossId2(incompat_id));
            }
        }

        if let Some(group_list_id) = subject.group_list_id {
            let group_lists = self.group_lists_get_all().await?;
            if !group_lists.contains_key(&group_list_id) {
                return Ok(DataStatusWithId3::BadCrossId3(group_list_id));
            }
        }

        Ok(DataStatusWithId3::Ok)
    }
    pub async fn subjects_add(
        &mut self,
        subject: &Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    ) -> std::result::Result<
        T::SubjectId,
        Cross3Error<T::InternalError, T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    > {
        let data_status = self.subjects_check_data(subject).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3Error::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3Error::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3Error::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                let id = unsafe { self.storage.subjects_add_unchecked(subject) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn subjects_update(
        &mut self,
        index: T::SubjectId,
        subject: &Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<
            T::InternalError,
            T::SubjectId,
            T::SubjectGroupId,
            T::IncompatId,
            T::GroupListId,
        >,
    > {
        if !self.subjects_check_id(index).await? {
            return Err(Cross3IdError::InvalidId(index));
        }

        let data_status = self.subjects_check_data(subject).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3IdError::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3IdError::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3IdError::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                unsafe { self.storage.subjects_update_unchecked(index, subject) }.await?;
                Ok(())
            }
        }
    }
    pub async fn subjects_can_remove(
        &mut self,
        index: T::SubjectId,
    ) -> std::result::Result<
        Vec<SubjectDependancy<T::TimeSlotId, T::StudentId>>,
        IdError<T::InternalError, T::SubjectId>,
    > {
        let subject = self.subjects_get(index).await?;

        let mut dependancies = Vec::new();

        let time_slots = self.time_slots_get_all().await?;
        for (time_slot_id, time_slot) in time_slots {
            if time_slot.subject_id == index {
                dependancies.push(SubjectDependancy::TimeSlot(time_slot_id));
            }
        }

        let students = self.students_get_all().await?;
        for (student_id, _student) in students {
            let subject_group_id = subject.subject_group_id;
            let subject_group_for_student = self.subject_group_for_student_get(student_id, subject_group_id)
                .await
                .map_err(
                    |e| match e {
                        Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                        Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was retrieved from the database", id2),
                    }
                )?;
            if subject_group_for_student == Some(index) {
                dependancies.push(SubjectDependancy::Student(student_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn subjects_remove(
        &mut self,
        index: T::SubjectId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::SubjectId,
            Vec<SubjectDependancy<T::TimeSlotId, T::StudentId>>,
        >,
    > {
        let dependancies = self
            .subjects_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.subjects_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn time_slots_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::TimeSlotId, TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>>,
        T::InternalError,
    > {
        self.storage.time_slots_get_all().await
    }
    pub async fn time_slots_get(
        &self,
        index: T::TimeSlotId,
    ) -> std::result::Result<
        TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
        IdError<T::InternalError, T::TimeSlotId>,
    > {
        self.storage.time_slots_get(index).await
    }
    pub async fn time_slots_check_id(
        &self,
        index: T::TimeSlotId,
    ) -> std::result::Result<bool, T::InternalError> {
        let time_slots = self.time_slots_get_all().await?;

        Ok(time_slots.contains_key(&index))
    }
    pub async fn time_slots_check_data(
        &self,
        time_slot: &TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
    ) -> std::result::Result<
        DataStatusWithId3<T::SubjectId, T::TeacherId, T::WeekPatternId>,
        T::InternalError,
    > {
        let subjects = self.subjects_get_all().await?;
        if !subjects.contains_key(&time_slot.subject_id) {
            return Ok(DataStatusWithId3::BadCrossId1(time_slot.subject_id));
        }

        let teachers = self.teachers_get_all().await?;
        if !teachers.contains_key(&time_slot.teacher_id) {
            return Ok(DataStatusWithId3::BadCrossId2(time_slot.teacher_id));
        }

        let week_patterns = self.week_patterns_get_all().await?;
        if !week_patterns.contains_key(&time_slot.week_pattern_id) {
            return Ok(DataStatusWithId3::BadCrossId3(time_slot.week_pattern_id));
        }

        Ok(DataStatusWithId3::Ok)
    }
    pub async fn time_slots_add(
        &mut self,
        time_slot: &TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
    ) -> std::result::Result<
        T::TimeSlotId,
        Cross3Error<T::InternalError, T::SubjectId, T::TeacherId, T::WeekPatternId>,
    > {
        let data_status = self.time_slots_check_data(time_slot).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3Error::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3Error::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3Error::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                let id = unsafe { self.storage.time_slots_add_unchecked(time_slot) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn time_slots_update(
        &mut self,
        index: T::TimeSlotId,
        time_slot: &TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<
            T::InternalError,
            T::TimeSlotId,
            T::SubjectId,
            T::TeacherId,
            T::WeekPatternId,
        >,
    > {
        if !self.time_slots_check_id(index).await? {
            return Err(Cross3IdError::InvalidId(index));
        }

        let data_status = self.time_slots_check_data(time_slot).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3IdError::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3IdError::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3IdError::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                unsafe { self.storage.time_slots_update_unchecked(index, time_slot) }.await?;
                Ok(())
            }
        }
    }
    pub async fn time_slots_can_remove(
        &mut self,
        index: T::TimeSlotId,
    ) -> std::result::Result<Vec<T::GroupingId>, IdError<T::InternalError, T::TimeSlotId>> {
        if !self.time_slots_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let groupings = self.groupings_get_all().await?;
        for (grouping_id, grouping) in groupings {
            if grouping.references_time_slot(index) {
                dependancies.push(grouping_id);
            }
        }

        Ok(dependancies)
    }
    pub async fn time_slots_remove(
        &mut self,
        index: T::TimeSlotId,
    ) -> std::result::Result<(), CheckedIdError<T::InternalError, T::TimeSlotId, Vec<T::GroupingId>>>
    {
        let dependancies = self
            .time_slots_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.time_slots_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn groupings_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::GroupingId, Grouping<T::TimeSlotId>>, T::InternalError>
    {
        self.storage.groupings_get_all().await
    }
    pub async fn groupings_get(
        &self,
        index: T::GroupingId,
    ) -> std::result::Result<Grouping<T::TimeSlotId>, IdError<T::InternalError, T::GroupingId>>
    {
        self.storage.groupings_get(index).await
    }
    pub async fn groupings_check_id(
        &self,
        index: T::GroupingId,
    ) -> std::result::Result<bool, T::InternalError> {
        let groupings = self.groupings_get_all().await?;

        Ok(groupings.contains_key(&index))
    }
    pub async fn groupings_check_data(
        &self,
        grouping: &Grouping<T::TimeSlotId>,
    ) -> std::result::Result<DataStatusWithId<T::TimeSlotId>, T::InternalError> {
        let time_slots = self.time_slots_get_all().await?;
        for &slot_id in &grouping.slots {
            if !time_slots.contains_key(&slot_id) {
                return Ok(DataStatusWithId::BadCrossId(slot_id));
            }
        }

        Ok(DataStatusWithId::Ok)
    }
    pub async fn groupings_add(
        &mut self,
        grouping: &Grouping<T::TimeSlotId>,
    ) -> std::result::Result<T::GroupingId, CrossError<T::InternalError, T::TimeSlotId>> {
        let data_status = self.groupings_check_data(grouping).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                let id = unsafe { self.storage.groupings_add_unchecked(grouping) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn groupings_update(
        &mut self,
        index: T::GroupingId,
        grouping: &Grouping<T::TimeSlotId>,
    ) -> std::result::Result<(), CrossIdError<T::InternalError, T::GroupingId, T::TimeSlotId>> {
        if !self.groupings_check_id(index).await? {
            return Err(CrossIdError::InvalidId(index));
        }

        let data_status = self.groupings_check_data(grouping).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                unsafe { self.storage.groupings_update_unchecked(index, grouping) }.await?;
                Ok(())
            }
        }
    }
    pub async fn groupings_can_remove(
        &mut self,
        index: T::GroupingId,
    ) -> std::result::Result<Vec<T::GroupingIncompatId>, IdError<T::InternalError, T::GroupingId>>
    {
        if !self.groupings_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let grouping_incompats = self.grouping_incompats_get_all().await?;
        for (grouping_incompat_id, grouping_incompat) in grouping_incompats {
            if grouping_incompat.references_grouping(index) {
                dependancies.push(grouping_incompat_id);
            }
        }

        Ok(dependancies)
    }
    pub async fn groupings_remove(
        &mut self,
        index: T::GroupingId,
    ) -> std::result::Result<
        (),
        CheckedIdError<T::InternalError, T::GroupingId, Vec<T::GroupingIncompatId>>,
    > {
        let dependancies = self
            .groupings_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.groupings_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn grouping_incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::GroupingIncompatId, GroupingIncompat<T::GroupingId>>,
        T::InternalError,
    > {
        self.storage.grouping_incompats_get_all().await
    }
    pub async fn grouping_incompats_get(
        &self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<
        GroupingIncompat<T::GroupingId>,
        IdError<T::InternalError, T::GroupingIncompatId>,
    > {
        self.storage.grouping_incompats_get(index).await
    }
    pub async fn grouping_incompats_check_id(
        &self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<bool, T::InternalError> {
        let grouping_incompats = self.grouping_incompats_get_all().await?;

        Ok(grouping_incompats.contains_key(&index))
    }
    pub async fn grouping_incompats_check_data(
        &self,
        grouping_incompat: &GroupingIncompat<T::GroupingId>,
    ) -> std::result::Result<DataStatusWithId<T::GroupingId>, T::InternalError> {
        let groupings = self.groupings_get_all().await?;

        for &grouping_id in &grouping_incompat.groupings {
            if !groupings.contains_key(&grouping_id) {
                return Ok(DataStatusWithId::BadCrossId(grouping_id));
            }
        }

        Ok(DataStatusWithId::Ok)
    }
    pub async fn grouping_incompats_add(
        &mut self,
        grouping_incompat: &GroupingIncompat<T::GroupingId>,
    ) -> std::result::Result<T::GroupingIncompatId, CrossError<T::InternalError, T::GroupingId>>
    {
        let data_status = self
            .grouping_incompats_check_data(grouping_incompat)
            .await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                let id = unsafe {
                    self.storage
                        .grouping_incompats_add_unchecked(grouping_incompat)
                }
                .await?;
                Ok(id)
            }
        }
    }
    pub async fn grouping_incompats_update(
        &mut self,
        index: T::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<T::GroupingId>,
    ) -> std::result::Result<(), CrossIdError<T::InternalError, T::GroupingIncompatId, T::GroupingId>>
    {
        if !self.grouping_incompats_check_id(index).await? {
            return Err(CrossIdError::InvalidId(index));
        }

        let data_status = self
            .grouping_incompats_check_data(grouping_incompat)
            .await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                unsafe {
                    self.storage
                        .grouping_incompats_update_unchecked(index, grouping_incompat)
                }
                .await?;
                Ok(())
            }
        }
    }
    pub async fn grouping_incompats_can_remove(
        &mut self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::GroupingIncompatId>> {
        if !self.grouping_incompats_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        Ok(())
    }
    pub async fn grouping_incompats_remove(
        &mut self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::GroupingIncompatId>> {
        self.grouping_incompats_can_remove(index).await?;

        unsafe { self.storage.grouping_incompats_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn subject_group_for_student_get(
        &self,
        student_id: T::StudentId,
        subject_group_id: T::SubjectGroupId,
    ) -> std::result::Result<
        Option<T::SubjectId>,
        Id2Error<T::InternalError, T::StudentId, T::SubjectGroupId>,
    > {
        self.storage
            .subject_group_for_student_get(student_id, subject_group_id)
            .await
    }
    pub async fn subject_group_for_student_set(
        &mut self,
        student_id: T::StudentId,
        subject_group_id: T::SubjectGroupId,
        subject_id: Option<T::SubjectId>,
    ) -> std::result::Result<
        (),
        CrossId3Error<
            T::InternalError,
            T::StudentId,
            T::SubjectGroupId,
            T::SubjectId,
            T::SubjectId,
        >,
    > {
        let students = self.students_get_all().await?;
        if !students.contains_key(&student_id) {
            return Err(CrossId3Error::InvalidId1(student_id));
        }
        let subject_groups = self.subject_groups_get_all().await?;
        if !subject_groups.contains_key(&subject_group_id) {
            return Err(CrossId3Error::InvalidId2(subject_group_id));
        }
        if let Some(id) = subject_id {
            let subject = self.subjects_get(id).await.map_err(|e| match e {
                IdError::InternalError(int_err) => CrossId3Error::InternalError(int_err),
                IdError::InvalidId(_) => CrossId3Error::InvalidId3(id),
            })?;
            if subject.subject_group_id != subject_group_id {
                return Err(CrossId3Error::InvalidCrossId(id));
            }
        }
        unsafe {
            self.storage.subject_group_for_student_set_unchecked(
                student_id,
                subject_group_id,
                subject_id,
            )
        }
        .await?;
        Ok(())
    }

    pub async fn incompat_for_student_get(
        &self,
        student_id: T::StudentId,
        incompat_id: T::IncompatId,
    ) -> std::result::Result<bool, Id2Error<T::InternalError, T::StudentId, T::IncompatId>> {
        self.storage
            .incompat_for_student_get(student_id, incompat_id)
            .await
    }
    pub async fn incompat_for_student_set(
        &mut self,
        student_id: T::StudentId,
        incompat_id: T::IncompatId,
        enabled: bool,
    ) -> std::result::Result<(), Id2Error<T::InternalError, T::StudentId, T::IncompatId>> {
        let students = self.students_get_all().await?;
        if !students.contains_key(&student_id) {
            return Err(Id2Error::InvalidId1(student_id));
        }
        if !self.incompats_check_id(incompat_id).await? {
            return Err(Id2Error::InvalidId2(incompat_id));
        }
        unsafe {
            self.storage
                .incompat_for_student_set_unchecked(student_id, incompat_id, enabled)
        }
        .await?;
        Ok(())
    }
}

impl<T: Storage> Logic<T> {
    pub fn gen_colloscope_translator<'a>(&'a self) -> GenColloscopeTranslator<'a, T> {
        GenColloscopeTranslator { data_storage: self }
    }
}

#[derive(Clone, Debug)]
pub struct GenColloscopeTranslator<'a, T: Storage> {
    data_storage: &'a Logic<T>,
}

#[derive(Debug, Error)]
pub enum GenColloscopeError<StorageError: std::fmt::Debug + std::error::Error> {
    #[error("Error in the storage backend: {0:?}")]
    StorageError(#[from] StorageError),
    #[error("Error while validating data: {0:?}")]
    ValidationError(crate::gen::colloscope::Error),
    #[error("No weeks in storage")]
    NoWeeks,
}

impl<StorageError: std::fmt::Debug + std::error::Error> GenColloscopeError<StorageError> {
    fn from_validation(validation_error: crate::gen::colloscope::Error) -> Self {
        GenColloscopeError::ValidationError(validation_error)
    }
}

type GenColloscopeResult<R, T> = Result<R, GenColloscopeError<<T as Storage>::InternalError>>;

struct GenColloscopeData<T: Storage> {
    general_data: GeneralData,
    week_patterns: BTreeMap<T::WeekPatternId, WeekPattern>,
    teachers: BTreeMap<T::TeacherId, Teacher>,
    incompats: BTreeMap<T::IncompatId, Incompat<T::WeekPatternId>>,
    students: BTreeMap<T::StudentId, Student>,
    incompat_for_student_data: BTreeSet<(T::StudentId, T::IncompatId)>,
    subjects: BTreeMap<T::SubjectId, Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>>,
    subject_for_student_data: BTreeSet<(T::StudentId, T::SubjectId)>,
}

impl<'a, T: Storage> GenColloscopeTranslator<'a, T> {
    async fn extract_data(&self) -> GenColloscopeResult<GenColloscopeData<T>, T> {
        let incompats = self.data_storage.incompats_get_all().await?;
        let students = self.data_storage.students_get_all().await?;

        let mut incompat_for_student_data = BTreeSet::new();
        for (&student_id, _student) in &students {
            for (&incompat_id, _incompat) in &incompats {
                if self
                    .data_storage
                    .incompat_for_student_get(student_id, incompat_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid", id1),
                        Id2Error::InvalidId2(id2) => {
                            panic!("Incompat id {:?} should be valid", id2)
                        }
                        Id2Error::InternalError(int_err) => int_err,
                    })?
                {
                    incompat_for_student_data.insert((student_id, incompat_id));
                }
            }
        }

        let subjects = self.data_storage.subjects_get_all().await?;
        let subject_groups = self.data_storage.subject_groups_get_all().await?;

        let mut subject_for_student_data = BTreeSet::new();
        for (&student_id, _student) in &students {
            for (&subject_group_id, _subject_group) in &subject_groups {
                let subject_opt = self
                    .data_storage
                    .subject_group_for_student_get(student_id, subject_group_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid", id1),
                        Id2Error::InvalidId2(id2) => {
                            panic!("Subject group id {:?} should be valid", id2)
                        }
                        Id2Error::InternalError(int_err) => int_err,
                    })?;

                if let Some(subject_id) = subject_opt {
                    subject_for_student_data.insert((student_id, subject_id));
                }
            }
        }

        Ok(GenColloscopeData {
            general_data: self.data_storage.general_data_get().await?,
            week_patterns: self.data_storage.week_patterns_get_all().await?,
            teachers: self.data_storage.teachers_get_all().await?,
            incompats,
            students,
            incompat_for_student_data,
            subjects,
            subject_for_student_data,
        })
    }

    fn is_week_in_week_pattern(
        data: &GenColloscopeData<T>,
        week_pattern_id: T::WeekPatternId,
        week: u32,
    ) -> bool {
        match data.week_patterns.get(&week_pattern_id) {
            None => false,
            Some(week_pattern) => week_pattern.weeks.contains(&Week(week)),
        }
    }

    fn build_general_data(
        &self,
        data: &GenColloscopeData<T>,
    ) -> GenColloscopeResult<crate::gen::colloscope::GeneralData, T> {
        let week_count_u32 = data
            .week_patterns
            .iter()
            .filter_map(|(_pattern_id, pattern)| pattern.weeks.last().map(|w| w.0 + 1))
            .max()
            .unwrap_or(0);

        let week_count = NonZeroU32::new(week_count_u32).ok_or(GenColloscopeError::NoWeeks)?;

        Ok(crate::gen::colloscope::GeneralData {
            teacher_count: data.teachers.len(),
            week_count,
            interrogations_per_week: data.general_data.interrogations_per_week.clone(),
            max_interrogations_per_day: data.general_data.max_interrogations_per_day,
        })
    }
}

#[derive(Clone, Debug)]
struct IncompatibilitiesData<T: Storage> {
    incompat_list: crate::gen::colloscope::IncompatibilityList,
    incompat_group_list: crate::gen::colloscope::IncompatibilityGroupList,
    id_map: BTreeMap<T::IncompatId, BTreeSet<usize>>,
}

impl<'a, T: Storage> GenColloscopeTranslator<'a, T> {
    fn is_week_in_incompat_group(
        data: &GenColloscopeData<T>,
        group: &IncompatGroup<T::WeekPatternId>,
        week: u32,
    ) -> bool {
        for slot in &group.slots {
            if Self::is_week_in_week_pattern(data, slot.week_pattern_id, week) {
                return true;
            }
        }
        false
    }

    fn build_incompatibility_data(
        &self,
        data: &GenColloscopeData<T>,
        week_count: NonZeroU32,
    ) -> GenColloscopeResult<IncompatibilitiesData<T>, T> {
        use crate::gen::colloscope::{Incompatibility, IncompatibilityGroup, SlotWithDuration};

        let mut output = IncompatibilitiesData {
            incompat_list: vec![],
            incompat_group_list: vec![],
            id_map: BTreeMap::new(),
        };

        for (&incompat_id, incompat) in &data.incompats {
            let mut ids = BTreeSet::new();

            for week in 0..week_count.get() {
                let mut new_incompat = Incompatibility {
                    max_count: incompat.max_count,
                    groups: BTreeSet::new(),
                };

                for group in &incompat.groups {
                    if !Self::is_week_in_incompat_group(data, group, week) {
                        continue;
                    }

                    let slots = group
                        .slots
                        .iter()
                        .map(|s| SlotWithDuration {
                            start: crate::gen::colloscope::SlotStart {
                                week,
                                weekday: s.start.day,
                                start_time: s.start.time.clone(),
                            },
                            duration: s.duration,
                        })
                        .collect();
                    let new_group = IncompatibilityGroup { slots };

                    new_incompat.groups.insert(output.incompat_group_list.len());
                    output.incompat_group_list.push(new_group);
                }

                if !new_incompat.groups.is_empty() {
                    ids.insert(output.incompat_list.len());
                    output.incompat_list.push(new_incompat);
                }
            }

            output.id_map.insert(incompat_id, ids);
        }

        Ok(output)
    }
}

#[derive(Clone, Debug)]
struct StudentData<T: Storage> {
    student_list: crate::gen::colloscope::StudentList,
    id_map: BTreeMap<T::StudentId, usize>,
}

impl<'a, T: Storage> GenColloscopeTranslator<'a, T> {
    fn build_student_data(
        &self,
        data: &GenColloscopeData<T>,
        incompat_id_map: &BTreeMap<T::IncompatId, BTreeSet<usize>>,
    ) -> GenColloscopeResult<StudentData<T>, T> {
        use crate::gen::colloscope::Student;

        let mut output = StudentData {
            student_list: vec![],
            id_map: BTreeMap::new(),
        };

        for (&student_id, _student) in &data.students {
            let mut new_student = Student {
                incompatibilities: BTreeSet::new(),
            };

            for (&incompat_id, _incompat) in &data.incompats {
                if data
                    .incompat_for_student_data
                    .contains(&(student_id, incompat_id))
                {
                    new_student.incompatibilities.extend(
                        incompat_id_map
                            .get(&incompat_id)
                            .expect("Incompat id should be valid in map")
                            .iter()
                            .cloned(),
                    )
                }
            }

            for (&subject_id, subject) in &data.subjects {
                if let Some(incompat_id) = subject.incompat_id {
                    if data
                        .subject_for_student_data
                        .contains(&(student_id, subject_id))
                    {
                        // Because we are using BTreeSet, we don't care if the incompat was already added
                        new_student.incompatibilities.extend(
                            incompat_id_map
                                .get(&incompat_id)
                                .expect("Incompat id should be valid in map")
                                .iter()
                                .cloned(),
                        )
                    }
                }
            }

            output.id_map.insert(student_id, output.student_list.len());
            output.student_list.push(new_student);
        }

        Ok(output)
    }

    fn build_subjects(
        &self,
        _data: &GenColloscopeData<T>,
    ) -> GenColloscopeResult<crate::gen::colloscope::SubjectList, T> {
        todo!()
    }

    fn build_slot_groupings(
        &self,
        _data: &GenColloscopeData<T>,
    ) -> GenColloscopeResult<crate::gen::colloscope::SlotGroupingList, T> {
        todo!()
    }

    fn build_grouping_incompats(
        &self,
        _data: &GenColloscopeData<T>,
    ) -> GenColloscopeResult<crate::gen::colloscope::SlotGroupingIncompatSet, T> {
        todo!()
    }

    pub async fn build_validated_data(
        &self,
    ) -> GenColloscopeResult<crate::gen::colloscope::ValidatedData, T> {
        let data = self.extract_data().await?;

        let general = self.build_general_data(&data)?;
        let incompatibility_data = self.build_incompatibility_data(&data, general.week_count)?;
        let student_data = self.build_student_data(&data, &incompatibility_data.id_map)?;
        let subjects = self.build_subjects(&data)?;
        let slot_groupings = self.build_slot_groupings(&data)?;
        let grouping_incompats = self.build_grouping_incompats(&data)?;

        crate::gen::colloscope::ValidatedData::new(
            general,
            subjects,
            incompatibility_data.incompat_group_list,
            incompatibility_data.incompat_list,
            student_data.student_list,
            slot_groupings,
            grouping_incompats,
        )
        .map_err(GenColloscopeError::from_validation)
    }
}
