//! IDs submodule
//!
//! This submodule contains the code for
//! handling unique IDs for colloscopes
//!

use collomatique_state::tools;
use serde::{Deserialize, Serialize};

pub trait Id:
    Clone
    + Copy
    + std::fmt::Debug
    + Ord
    + PartialOrd
    + Eq
    + PartialEq
    + std::hash::Hash
    + Send
    + Sync
    + 'static
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleId(u64);

impl Id for RuleId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> RuleId {
        RuleId(value)
    }
}

/// This type represents an ID for a colloscope
///
/// Every colloscope gets a unique ID. IDs then identify colloscopes
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeId(u64);

impl Id for ColloscopeId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeId {
        ColloscopeId(value)
    }
}

/// This type represents an ID for a student inside a colloscope
///
/// Every student gets a unique ID. IDs then identify students
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeStudentId(u64);

impl Id for ColloscopeStudentId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeStudentId {
        ColloscopeStudentId(value)
    }
}

/// This type represents an ID for a period inside a colloscope
///
/// Every period gets a unique ID. IDs then identify periods
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopePeriodId(u64);

impl Id for ColloscopePeriodId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopePeriodId {
        ColloscopePeriodId(value)
    }
}

/// This type represents an ID for a subject inside a colloscope
///
/// Every subject gets a unique ID. IDs then identify periods
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeSubjectId(u64);

impl Id for ColloscopeSubjectId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeSubjectId {
        ColloscopeSubjectId(value)
    }
}

/// This type represents an ID for a teacher inside a colloscope
///
/// Every teacher gets a unique ID. IDs then identify teachers
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeTeacherId(u64);

impl Id for ColloscopeTeacherId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeTeacherId {
        ColloscopeTeacherId(value)
    }
}

/// This type represents an ID for a week pattern inside a colloscope
///
/// Every week pattern gets a unique ID. IDs then identify week patterns
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeWeekPatternId(u64);

impl Id for ColloscopeWeekPatternId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeWeekPatternId {
        ColloscopeWeekPatternId(value)
    }
}

/// This type represents an ID for an interrogation slot inside a colloscope
///
/// Every interrogation slot gets a unique ID. IDs then identify slots
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeSlotId(u64);

impl Id for ColloscopeSlotId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeSlotId {
        ColloscopeSlotId(value)
    }
}

/// This type represents an ID for an schedule incompatibility inside a colloscope
///
/// Every incompatibility gets a unique ID. IDs then identify incompatibilities
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeIncompatId(u64);

impl Id for ColloscopeIncompatId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeIncompatId {
        ColloscopeIncompatId(value)
    }
}

/// This type represents an ID for a group list inside a colloscope
///
/// Every group list gets a unique ID. IDs then identify group lists
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeGroupListId(u64);

impl Id for ColloscopeGroupListId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeGroupListId {
        ColloscopeGroupListId(value)
    }
}

/// This type represents an ID for a rule inside a colloscope
///
/// Every rule gets a unique ID. IDs then identify rules
/// internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColloscopeRuleId(u64);

impl Id for ColloscopeRuleId {
    fn inner(&self) -> u64 {
        self.0
    }

    unsafe fn new(value: u64) -> ColloscopeRuleId {
        ColloscopeRuleId(value)
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
        existing_ids: impl Iterator<Item = u64>,
    ) -> std::result::Result<IdIssuer, tools::IdError> {
        Ok(IdIssuer {
            helper: tools::IdIssuerHelper::new(existing_ids)?,
        })
    }

    /// Returns internal counter
    pub fn get_internal_counter(&self) -> u64 {
        self.helper.get_internal_counter()
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

    /// Get a new unused ID for a colloscope
    pub fn get_colloscope_id(&mut self) -> ColloscopeId {
        ColloscopeId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a student in a colloscope
    pub fn get_colloscope_student_id(&mut self) -> ColloscopeStudentId {
        ColloscopeStudentId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a period in a colloscope
    pub fn get_colloscope_period_id(&mut self) -> ColloscopePeriodId {
        ColloscopePeriodId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a subject in a colloscope
    pub fn get_colloscope_subject_id(&mut self) -> ColloscopeSubjectId {
        ColloscopeSubjectId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a teacher in a colloscope
    pub fn get_colloscope_teacher_id(&mut self) -> ColloscopeTeacherId {
        ColloscopeTeacherId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a week pattern in a colloscope
    pub fn get_colloscope_week_pattern_id(&mut self) -> ColloscopeWeekPatternId {
        ColloscopeWeekPatternId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a slot in a colloscope
    pub fn get_colloscope_slot_id(&mut self) -> ColloscopeSlotId {
        ColloscopeSlotId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a schedule incompatibility in a colloscope
    pub fn get_colloscope_incompat_id(&mut self) -> ColloscopeIncompatId {
        ColloscopeIncompatId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a group list in a colloscope
    pub fn get_colloscope_group_list_id(&mut self) -> ColloscopeGroupListId {
        ColloscopeGroupListId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a rule in a colloscope
    pub fn get_colloscope_rule_id(&mut self) -> ColloscopeRuleId {
        ColloscopeRuleId(self.helper.get_new_id().inner())
    }
}
