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

pub trait OrdId: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord {}
impl<T: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord> OrdId for T {}

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

#[trait_variant::make(Send)]
pub trait Storage {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;
    type StudentId: OrdId;
    type SubjectGroupId: OrdId;
    type IncompatId: OrdId;
    type GroupListId: OrdId;
    type SubjectId: OrdId;

    type InternalError: std::fmt::Debug + std::error::Error;

    async fn general_data_set(
        &self,
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
        &self,
        pattern: &WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError>;
    async fn week_patterns_remove(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>;
    async fn week_patterns_update(
        &self,
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
        &self,
        teacher: &Teacher,
    ) -> std::result::Result<Self::TeacherId, Self::InternalError>;
    async fn teachers_remove(
        &self,
        index: Self::TeacherId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>;
    async fn teachers_update(
        &self,
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
        &self,
        student: &Student,
    ) -> std::result::Result<Self::StudentId, Self::InternalError>;
    async fn students_remove(
        &self,
        index: Self::StudentId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::StudentId>>;
    async fn students_update(
        &self,
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
        &self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<Self::SubjectGroupId, Self::InternalError>;
    async fn subject_groups_remove(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_update(
        &self,
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
    async fn incompats_add(
        &self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<Self::IncompatId, CrossError<Self::InternalError, Self::WeekPatternId>>;
    async fn incompats_remove(
        &self,
        index: Self::IncompatId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::IncompatId>>;
    async fn incompats_update(
        &self,
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
    async fn group_lists_add(
        &self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<
        Self::GroupListId,
        InvalidCrossError<Self::InternalError, GroupList<Self::StudentId>, Self::StudentId>,
    >;
    async fn group_lists_remove(
        &self,
        index: Self::GroupListId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::GroupListId>>;
    async fn group_lists_update(
        &self,
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
    async fn subjects_add(
        &self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<
        Self::SubjectId,
        Cross3Error<Self::InternalError, Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    >;
    async fn subjects_remove(
        &self,
        index: Self::SubjectId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectId>>;
    async fn subjects_update(
        &self,
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
pub struct TimeSlot {
    day: crate::time::Weekday,
    time: crate::time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatSlot<WeekPatternId: OrdId> {
    week_pattern_id: WeekPatternId,
    start: TimeSlot,
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
