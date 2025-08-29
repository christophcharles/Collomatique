//! periods submodule
//!
//! This module defines the periods entry for the JSON description
//!
use super::*;

/// JSON desc of periods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// Optional date for the start of periods
    ///
    /// The date *should* refer to a monday
    /// but this will be checked afterwards
    pub first_week: Option<chrono::NaiveDate>,

    /// ordered period list
    ///
    /// each period is described by an id (which should not
    /// be duplicate) and list of bools describing for
    /// each each whether we should put interrogations in them.
    pub ordered_period_list: Vec<(u64, Vec<bool>)>,
}
