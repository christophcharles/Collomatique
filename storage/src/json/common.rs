//! Common submodule
//!
//! This module contains common types that can be reused in
//! different entries.
//!
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonWithContact {
    pub firstname: String,
    pub surname: String,
    pub telephone: Option<String>,
    pub email: Option<String>,
}
