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

pub trait OrdId: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord {}
impl<T: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord> OrdId for T {}

use std::collections::BTreeMap;
use std::num::NonZeroU32;

#[trait_variant::make(Send)]
pub trait Storage {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;
    type StudentId: OrdId;
    type SubjectGroupId: OrdId;
    type IncompatId: OrdId;

    type InternalError: std::fmt::Debug + std::error::Error;

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
