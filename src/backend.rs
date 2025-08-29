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

pub trait OrdId: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord {}
impl<T: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord> OrdId for T {}

use std::collections::BTreeMap;

#[trait_variant::make(Send)]
pub trait Storage {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;
    type SubjectGroupId: OrdId;

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
        pattern: WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError>;
    async fn week_patterns_remove(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>;
    async fn week_patterns_update(
        &self,
        index: Self::WeekPatternId,
        pattern: WeekPattern,
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
        teacher: Teacher,
    ) -> std::result::Result<Self::TeacherId, Self::InternalError>;
    async fn teachers_remove(
        &self,
        index: Self::TeacherId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>;
    async fn teachers_update(
        &self,
        index: Self::TeacherId,
        teacher: Teacher,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>;

    async fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::SubjectGroupId, SubjectGroup>, Self::InternalError>;
    async fn subject_groups_get(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_add(
        &self,
        subject_group: SubjectGroup,
    ) -> std::result::Result<Self::SubjectGroupId, Self::InternalError>;
    async fn subject_groups_remove(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_update(
        &self,
        index: Self::SubjectGroupId,
        subject_group: SubjectGroup,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;
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
pub struct SubjectGroup {
    pub name: String,
    pub optional: bool,
}
