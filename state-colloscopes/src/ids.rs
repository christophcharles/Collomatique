//! IDs submodule
//!
//! This submodule contains the code for
//! handling unique IDs for colloscopes
//!

use collomatique_state::tools;

/// This type represents an ID for a student
///
/// Every student gets a unique ID. IDs then identify students
/// internally.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StudentId(u64);

impl StudentId {
    /// Returns the value for the ID
    pub fn inner(&self) -> u64 {
        self.0
    }

    pub(crate) unsafe fn new(value: u64) -> StudentId {
        StudentId(value)
    }
}

#[derive(Debug)]
pub(crate) struct IdIssuer {
    helper: tools::IdIssuerHelper,
}

impl IdIssuer {
    /// Create a new IdIssuer
    ///
    /// It takes a list of all used ids so far
    pub fn new(
        student_ids: impl Iterator<Item = u64>,
    ) -> std::result::Result<IdIssuer, tools::IdError> {
        let existing_ids = student_ids;
        Ok(IdIssuer {
            helper: tools::IdIssuerHelper::new(existing_ids)?,
        })
    }

    /// Get a new unused ID for a student
    pub fn get_student_id(&self) -> StudentId {
        StudentId(self.helper.get_new_id().inner())
    }
}
