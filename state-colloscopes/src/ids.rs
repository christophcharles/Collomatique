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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

/// This type represents an ID for a period
///
/// Every period gets a unique ID. IDs then identify periods
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeriodId(u64);

impl PeriodId {
    /// Returns the value for the ID
    pub fn inner(&self) -> u64 {
        self.0
    }

    pub(crate) unsafe fn new(value: u64) -> PeriodId {
        PeriodId(value)
    }
}

/// This type represents an ID for a subject
///
/// Every subject gets a unique ID. IDs then identify periods
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubjectId(u64);

impl SubjectId {
    /// Returns the value for the ID
    pub fn inner(&self) -> u64 {
        self.0
    }

    pub(crate) unsafe fn new(value: u64) -> SubjectId {
        SubjectId(value)
    }
}

/// This type represents an ID for a teacher
///
/// Every teacher gets a unique ID. IDs then identify teachers
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeacherId(u64);

impl TeacherId {
    /// Returns the value for the ID
    pub fn inner(&self) -> u64 {
        self.0
    }

    pub(crate) unsafe fn new(value: u64) -> TeacherId {
        TeacherId(value)
    }
}

/// This type represents an ID for a week pattern
///
/// Every week pattern gets a unique ID. IDs then identify week patterns
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeekPatternId(u64);

impl WeekPatternId {
    /// Returns the value for the ID
    pub fn inner(&self) -> u64 {
        self.0
    }

    pub(crate) unsafe fn new(value: u64) -> WeekPatternId {
        WeekPatternId(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct IdIssuer {
    helper: tools::IdIssuerHelper,
}

impl IdIssuer {
    /// Create a new IdIssuer
    ///
    /// It takes a list of all used ids so far
    pub fn new(
        student_ids: impl Iterator<Item = u64>,
        period_ids: impl Iterator<Item = u64>,
        subject_ids: impl Iterator<Item = u64>,
        teacher_ids: impl Iterator<Item = u64>,
        week_patterns_ids: impl Iterator<Item = u64>,
    ) -> std::result::Result<IdIssuer, tools::IdError> {
        let existing_ids = student_ids
            .chain(period_ids)
            .chain(subject_ids)
            .chain(teacher_ids)
            .chain(week_patterns_ids);
        Ok(IdIssuer {
            helper: tools::IdIssuerHelper::new(existing_ids)?,
        })
    }

    /// Get a new unused ID for a student
    pub fn get_student_id(&mut self) -> StudentId {
        StudentId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a period
    pub fn get_period_id(&mut self) -> PeriodId {
        PeriodId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a subject
    pub fn get_subject_id(&mut self) -> SubjectId {
        SubjectId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a teacher
    pub fn get_teacher_id(&mut self) -> TeacherId {
        TeacherId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a week pattern
    pub fn get_week_pattern_id(&mut self) -> WeekPatternId {
        WeekPatternId(self.helper.get_new_id().inner())
    }
}
