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

#[trait_variant::make(Send)]
pub trait Storage {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;

    type InternalError: std::fmt::Debug + std::error::Error;

    async fn week_pattern_get_all(
        &self,
    ) -> std::result::Result<Vec<WeekPattern>, Self::InternalError>;
    async fn week_pattern_get(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<Self::InternalError, Self::WeekPatternId>>;
    async fn week_pattern_add(
        &self,
        pattern: WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError>;
    async fn week_pattern_remove(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>;

    async fn teachers_get_all(&self) -> std::result::Result<Vec<Teacher>, Self::InternalError>;
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
