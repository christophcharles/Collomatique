//! IDs submodule
//!
//! This submodule contains the code for
//! handling unique IDs for colloscopes
//!

use collomatique_state::EntityId;
use collomatique_state::tools;
use serde::{Deserialize, Serialize};

use crate::group_lists::GroupList;
use crate::incompats::Incompatibility;
use crate::pairings::PairingRule;
use crate::slot_pairings::SlotPairingRule;
use crate::slots::Slot;
use crate::students::Student;
use crate::subjects::Subject;
use crate::teachers::Teacher;
use crate::week_patterns::WeekPattern;
use crate::weeks::Week;

pub use collomatique_state::ids::Id;

/// This type represents an ID for a student
///
/// Every student gets a unique ID. IDs then identify students
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(Student)]
pub struct StudentId(u64);

/// This type represents an ID for a period
///
/// Every period gets a unique ID. IDs then identify periods
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(())]
pub struct PeriodId(u64);

/// This type represents an ID for a week
///
/// Every week gets a unique ID. IDs then identify weeks internally, resolving
/// to the standalone [Week] entity stored in the periods submodule.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(Week)]
pub struct WeekId(u64);

/// This type represents an ID for a subject
///
/// Every subject gets a unique ID. IDs then identify periods
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(Subject)]
pub struct SubjectId(u64);

/// This type represents an ID for a teacher
///
/// Every teacher gets a unique ID. IDs then identify teachers
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(Teacher)]
pub struct TeacherId(u64);

/// This type represents an ID for a week pattern
///
/// Every week pattern gets a unique ID. IDs then identify week patterns
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(WeekPattern)]
pub struct WeekPatternId(u64);

/// This type represents an ID for an interrogation slot
///
/// Every interrogation slot gets a unique ID. IDs then identify slots
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(Slot)]
pub struct SlotId(u64);

/// This type represents an ID for an schedule incompatibility
///
/// Every incompatibility gets a unique ID. IDs then identify incompatibilities
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(Incompatibility)]
pub struct IncompatId(u64);

/// This type represents an ID for a group list
///
/// Every group list gets a unique ID. IDs then identify group lists
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(GroupList)]
pub struct GroupListId(u64);

/// This type represents an ID for a pairing rule
///
/// Every pairing rule gets a unique ID. IDs then identify pairing rules
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(PairingRule)]
pub struct PairingRuleId(u64);

/// This type represents an ID for a slot pairing rule
///
/// Every slot pairing rule gets a unique ID. IDs then identify slot pairing rules
/// internally.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EntityId,
)]
#[entity(SlotPairingRule)]
pub struct SlotPairingRuleId(u64);

// The document order on the ids. An id is a scalar
// reference token with no internal content, so wherever it appears as a field
// *value* it is an atom: two ids are content-equivalent when they are the same
// id, and otherwise incomparable. That also makes `==` content identity, so
// the macro emits `ContentIdentity` too and ids may be used as container keys
// and set elements.
//
// This is deliberately *not* the numeric `Ord` those types also carry: the
// numeric order is what makes them `BTreeMap` keys, and it says nothing about
// document content.
collomatique_state::impl_content_ord_atom!(
    PeriodId,
    WeekId,
    SubjectId,
    TeacherId,
    StudentId,
    WeekPatternId,
    SlotId,
    IncompatId,
    GroupListId,
    PairingRuleId,
    SlotPairingRuleId,
);

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

    /// Advance the counter to at least `next_id`
    pub fn skip_to_id(&mut self, next_id: u64) -> Result<(), tools::IdError> {
        self.helper.skip_to_id(next_id)
    }

    /// Get a new unused ID for a student
    pub fn get_student_id(&mut self) -> StudentId {
        StudentId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a period
    pub fn get_period_id(&mut self) -> PeriodId {
        PeriodId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a week
    pub fn get_week_id(&mut self) -> WeekId {
        WeekId(self.helper.get_new_id().inner())
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

    /// Get a new unused ID for a pairing rule
    pub fn get_pairing_rule_id(&mut self) -> PairingRuleId {
        PairingRuleId(self.helper.get_new_id().inner())
    }

    /// Get a new unused ID for a slot pairing rule
    pub fn get_slot_pairing_rule_id(&mut self) -> SlotPairingRuleId {
        SlotPairingRuleId(self.helper.get_new_id().inner())
    }
}

/// Potential new id returned by annotation
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewId {
    StudentId(StudentId),
    PeriodId(PeriodId),
    WeekId(WeekId),
    SubjectId(SubjectId),
    TeacherId(TeacherId),
    WeekPatternId(WeekPatternId),
    SlotId(SlotId),
    IncompatId(IncompatId),
    GroupListId(GroupListId),
    PairingRuleId(PairingRuleId),
    SlotPairingRuleId(SlotPairingRuleId),
}

impl NewId {
    /// The raw `u64` behind whichever id kind this is.
    ///
    /// The typed wrapper is erased — two different id kinds sharing the same
    /// underlying number compare equal through this. Used where a single
    /// numeric id space is needed across all tables (e.g. duplicate scanning
    /// and the `IdIssuer` high-water mark).
    pub fn inner(&self) -> u64 {
        match *self {
            NewId::StudentId(id) => id.inner(),
            NewId::PeriodId(id) => id.inner(),
            NewId::WeekId(id) => id.inner(),
            NewId::SubjectId(id) => id.inner(),
            NewId::TeacherId(id) => id.inner(),
            NewId::WeekPatternId(id) => id.inner(),
            NewId::SlotId(id) => id.inner(),
            NewId::IncompatId(id) => id.inner(),
            NewId::GroupListId(id) => id.inner(),
            NewId::PairingRuleId(id) => id.inner(),
            NewId::SlotPairingRuleId(id) => id.inner(),
        }
    }
}

impl From<StudentId> for NewId {
    fn from(value: StudentId) -> Self {
        NewId::StudentId(value)
    }
}

impl From<PeriodId> for NewId {
    fn from(value: PeriodId) -> Self {
        NewId::PeriodId(value)
    }
}

impl From<WeekId> for NewId {
    fn from(value: WeekId) -> Self {
        NewId::WeekId(value)
    }
}

impl From<SubjectId> for NewId {
    fn from(value: SubjectId) -> Self {
        NewId::SubjectId(value)
    }
}

impl From<TeacherId> for NewId {
    fn from(value: TeacherId) -> Self {
        NewId::TeacherId(value)
    }
}

impl From<WeekPatternId> for NewId {
    fn from(value: WeekPatternId) -> Self {
        NewId::WeekPatternId(value)
    }
}

impl From<SlotId> for NewId {
    fn from(value: SlotId) -> Self {
        NewId::SlotId(value)
    }
}

impl From<IncompatId> for NewId {
    fn from(value: IncompatId) -> Self {
        NewId::IncompatId(value)
    }
}

impl From<GroupListId> for NewId {
    fn from(value: GroupListId) -> Self {
        NewId::GroupListId(value)
    }
}

impl From<PairingRuleId> for NewId {
    fn from(value: PairingRuleId) -> Self {
        NewId::PairingRuleId(value)
    }
}

impl From<SlotPairingRuleId> for NewId {
    fn from(value: SlotPairingRuleId) -> Self {
        NewId::SlotPairingRuleId(value)
    }
}
