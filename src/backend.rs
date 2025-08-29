pub mod sqlite;

#[trait_variant::make(Send)]
pub trait Storage {
    type WeekPatternError;

    async fn week_pattern_get_all(&self) -> Result<Vec<WeekPattern>, Self::WeekPatternError>;
    async fn week_pattern_get(
        &self,
        index: WeekPatternId,
    ) -> Result<WeekPattern, Self::WeekPatternError>;
    async fn week_pattern_add(
        &self,
        pattern: WeekPattern,
    ) -> Result<WeekPatternId, Self::WeekPatternError>;
    async fn week_pattern_remove(&self, index: WeekPatternId)
        -> Result<(), Self::WeekPatternError>;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeekPatternId(usize);

impl std::fmt::Display for WeekPatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}
