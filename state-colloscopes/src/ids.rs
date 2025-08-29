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

#[derive(Debug)]
pub(crate) struct IdIssuer {
    helper: tools::IdIssuerHelper,
}

impl IdIssuer {
    /// Create a new IdIssuer
    ///
    /// It takes a list of all used ids so far
    pub fn new<'a>(
        student_ids: impl Iterator<Item = &'a StudentId>,
    ) -> std::result::Result<IdIssuer, tools::IdError> {
        let mut max_so_far = None;
        for student_id in student_ids {
            match max_so_far {
                Some(v) => {
                    if student_id.0 > v {
                        max_so_far = Some(student_id.0);
                    }
                }
                None => {
                    max_so_far = Some(student_id.0);
                }
            }
        }

        Ok(IdIssuer {
            helper: tools::IdIssuerHelper::new(max_so_far)?,
        })
    }

    /// Get a new unused ID for a student
    pub fn get_student_id(&self) -> StudentId {
        StudentId(self.helper.get_new_id().inner())
    }
}
