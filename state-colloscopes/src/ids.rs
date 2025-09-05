//! IDs submodule
//!
//! This submodule contains the code for
//! handling unique IDs for colloscopes
//!

use collomatique_state::tools;

pub trait Id:
    Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync + 'static
{
    /// Returns the value for the ID
    fn inner(&self) -> u64;
    /// Builds a new ID from u64
    ///
    /// This is unsafe as invariants should be checked first (to avoid duplicated ids)
    unsafe fn new(value: u64) -> Self;
}

/// This type represents an ID for a student
///
/// Every student gets a unique ID. IDs then identify students
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StudentId(u64);

impl Id for StudentId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> StudentId {
        StudentId(value)
    }
}

/// This type represents an ID for a period
///
/// Every period gets a unique ID. IDs then identify periods
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeriodId(u64);

impl Id for PeriodId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> PeriodId {
        PeriodId(value)
    }
}

/// This type represents an ID for a subject
///
/// Every subject gets a unique ID. IDs then identify periods
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubjectId(u64);

impl Id for SubjectId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> SubjectId {
        SubjectId(value)
    }
}

/// This type represents an ID for a teacher
///
/// Every teacher gets a unique ID. IDs then identify teachers
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeacherId(u64);

impl Id for TeacherId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> TeacherId {
        TeacherId(value)
    }
}

/// This type represents an ID for a week pattern
///
/// Every week pattern gets a unique ID. IDs then identify week patterns
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeekPatternId(u64);

impl Id for WeekPatternId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> WeekPatternId {
        WeekPatternId(value)
    }
}

/// This type represents an ID for an interrogation slot
///
/// Every interrogation slot gets a unique ID. IDs then identify slots
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotId(u64);

impl Id for SlotId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> SlotId {
        SlotId(value)
    }
}

/// This type represents an ID for an schedule incompatibility
///
/// Every incompatibility gets a unique ID. IDs then identify incompatibilities
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatId(u64);

impl Id for IncompatId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> IncompatId {
        IncompatId(value)
    }
}

/// This type represents an ID for a group list
///
/// Every group list gets a unique ID. IDs then identify group lists
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupListId(u64);

impl Id for GroupListId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> GroupListId {
        GroupListId(value)
    }
}

/// This type represents an ID for a rule
///
/// Every rule gets a unique ID. IDs then identify rules
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleId(u64);

impl Id for RuleId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> RuleId {
        RuleId(value)
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
        slot_ids: impl Iterator<Item = u64>,
        incompat_ids: impl Iterator<Item = u64>,
        group_list_ids: impl Iterator<Item = u64>,
        rule_ids: impl Iterator<Item = u64>,
    ) -> std::result::Result<IdIssuer, tools::IdError> {
        let existing_ids = student_ids
            .chain(period_ids)
            .chain(subject_ids)
            .chain(teacher_ids)
            .chain(week_patterns_ids)
            .chain(slot_ids)
            .chain(incompat_ids)
            .chain(group_list_ids)
            .chain(rule_ids);
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

    /// Get a new unused ID for a slot
    pub fn get_slot_id(&mut self) -> SlotId {
        SlotId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a schedule incompatibility
    pub fn get_incompat_id(&mut self) -> IncompatId {
        IncompatId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a group list
    pub fn get_group_list_id(&mut self) -> GroupListId {
        GroupListId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a rule
    pub fn get_rule_id(&mut self) -> RuleId {
        RuleId(self.helper.get_new_id().inner())
    }
}
