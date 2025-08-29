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
pub enum CrossId2Error<T, Id1, Id2, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
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

#[derive(Error, Debug)]
pub enum WeekPatternDependancy<IncompatId: OrdId, TimeSlotId: OrdId> {
    Incompat(IncompatId),
    TimeSlot(TimeSlotId),
}

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

#[trait_variant::make(Send)]
pub trait Storage: Send + Sync {
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
    async fn week_patterns_check_can_remove(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<
        Vec<WeekPatternDependancy<Self::IncompatId, Self::TimeSlotId>>,
        IdError<Self::InternalError, Self::WeekPatternId>,
    > {
        async move {
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
    }
    async fn week_patterns_remove(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            Self::InternalError,
            Self::WeekPatternId,
            Vec<WeekPatternDependancy<Self::IncompatId, Self::TimeSlotId>>,
        >,
    > {
        async move {
            let dependancies = self
                .week_patterns_check_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.week_patterns_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

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
    async fn teachers_check_can_remove(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<Vec<Self::TimeSlotId>, IdError<Self::InternalError, Self::TeacherId>>
    {
        async move {
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
    }
    async fn teachers_patterns_remove(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::TeacherId, Vec<Self::TimeSlotId>>,
    > {
        async move {
            let dependancies = self
                .teachers_check_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.teachers_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

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
    async fn students_check_can_remove(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<Vec<Self::GroupListId>, IdError<Self::InternalError, Self::StudentId>>
    {
        async move {
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
    }
    async fn students_remove(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::StudentId, Vec<Self::GroupListId>>,
    > {
        async move {
            let dependancies = self
                .students_check_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.students_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

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
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;
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
    ) -> std::result::Result<Self::IncompatId, CrossError<Self::InternalError, Self::WeekPatternId>>;
    async unsafe fn incompats_remove_unchecked(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::IncompatId>>;
    async unsafe fn incompats_update_unchecked(
        &mut self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<
        (),
        CrossIdError<Self::InternalError, Self::IncompatId, Self::WeekPatternId>,
    >;

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
    ) -> std::result::Result<
        Self::GroupListId,
        InvalidCrossError<Self::InternalError, GroupList<Self::StudentId>, Self::StudentId>,
    >;
    async unsafe fn group_lists_remove_unchecked(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::GroupListId>>;
    async unsafe fn group_lists_update_unchecked(
        &mut self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<
        (),
        InvalidCrossIdError<
            Self::InternalError,
            GroupList<Self::StudentId>,
            Self::GroupListId,
            Self::StudentId,
        >,
    >;

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
    ) -> std::result::Result<
        Self::SubjectId,
        Cross3Error<Self::InternalError, Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    >;
    async unsafe fn subjects_remove_unchecked(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectId>>;
    async unsafe fn subjects_update_unchecked(
        &mut self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<
            Self::InternalError,
            Self::SubjectId,
            Self::SubjectGroupId,
            Self::IncompatId,
            Self::GroupListId,
        >,
    >;

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
    ) -> std::result::Result<
        Self::TimeSlotId,
        Cross3Error<Self::InternalError, Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    >;
    async unsafe fn time_slots_remove_unchecked(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TimeSlotId>>;
    async unsafe fn time_slots_update_unchecked(
        &mut self,
        index: Self::TimeSlotId,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<
            Self::InternalError,
            Self::TimeSlotId,
            Self::SubjectId,
            Self::TeacherId,
            Self::WeekPatternId,
        >,
    >;

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
    ) -> std::result::Result<Self::GroupingId, CrossError<Self::InternalError, Self::TimeSlotId>>;
    async unsafe fn groupings_remove_unchecked(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::GroupingId>>;
    async unsafe fn groupings_update_unchecked(
        &mut self,
        index: Self::GroupingId,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<
        (),
        CrossIdError<Self::InternalError, Self::GroupingId, Self::TimeSlotId>,
    >;

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
    ) -> std::result::Result<
        Self::GroupingIncompatId,
        CrossError<Self::InternalError, Self::GroupingId>,
    >;
    async fn grouping_incompats_remove(
        &mut self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::GroupingIncompatId>>;
    async unsafe fn grouping_incompats_update_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<
        (),
        CrossIdError<Self::InternalError, Self::GroupingIncompatId, Self::GroupingId>,
    >;

    async unsafe fn subject_group_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
        subject_id: Option<Self::SubjectId>,
    ) -> std::result::Result<
        (),
        CrossId2Error<Self::InternalError, Self::StudentId, Self::SubjectGroupId, Self::SubjectId>,
    >;
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
    ) -> std::result::Result<(), Id2Error<Self::InternalError, Self::StudentId, Self::IncompatId>>;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupingIncompat<GroupingId: OrdId> {
    pub max_count: NonZeroUsize,
    pub groupings: BTreeSet<GroupingId>,
}
