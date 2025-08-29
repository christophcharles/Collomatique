//! Ids submodule
//!
//! This module defines various types for Identifier
//! and provides the [IdIssuer] struct to simplify getting
//! new ids.
//!

use super::*;

/// This type represents an ID for a student
///
/// Every student gets a unique ID. IDs then identify students
/// internally.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StudentId(u64);

/// Id issuer
///
/// This is a helper struct. It helps generate
/// new, unique ids every time we need one.
#[derive(Debug)]
pub struct IdIssuer {
    next_available_id: u64,
}

impl IdIssuer {
    /// Create a new IdIssuer
    ///
    /// It takes a list of all used ids so far
    pub fn new<'a>(student_ids: impl Iterator<Item = &'a StudentId>) -> Result<IdIssuer> {
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

        let next_available_id = match max_so_far {
            None => 0,
            Some(val) => {
                if val > (u64::MAX >> 1) {
                    return Err(Error::EndOfTheUniverse);
                } else {
                    val + 1
                }
            }
        };

        Ok(IdIssuer { next_available_id })
    }

    /// Used internally
    ///
    /// This function generates a new ID.
    /// This code is factored out as we need
    /// it for the generation of every ID type.
    fn get_new_id(&mut self) -> u64 {
        let new_id = self.next_available_id;
        self.next_available_id += 1;
        new_id
    }

    /// Get a new unused ID for a student
    pub fn get_student_id(&mut self) -> StudentId {
        StudentId(self.get_new_id())
    }
}
