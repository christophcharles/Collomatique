use std::num::NonZeroU32;

use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncompatId {
    id: collomatique_state_colloscopes::IncompatId,
}

#[pymethods]
impl IncompatId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::IncompatId> for IncompatId {
    fn from(value: &collomatique_state_colloscopes::IncompatId) -> Self {
        IncompatId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::IncompatId> for IncompatId {
    fn from(value: collomatique_state_colloscopes::IncompatId) -> Self {
        IncompatId::from(&value)
    }
}

impl From<&IncompatId> for collomatique_state_colloscopes::IncompatId {
    fn from(value: &IncompatId) -> Self {
        value.id.clone()
    }
}

impl From<IncompatId> for collomatique_state_colloscopes::IncompatId {
    fn from(value: IncompatId) -> Self {
        collomatique_state_colloscopes::IncompatId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompat {
    #[pyo3(set, get)]
    pub subject_id: SubjectId,
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub slots: Vec<time::SlotWithDuration>,
    #[pyo3(set, get)]
    pub minimum_free_slots: NonZeroU32,
    #[pyo3(set, get)]
    pub week_pattern_id: Option<WeekPatternId>,
}

#[pymethods]
impl Incompat {
    #[new]
    fn new(
        subject_id: SubjectId,
        name: String,
        slots: Vec<time::SlotWithDuration>,
        minimum_free_slots: NonZeroU32,
    ) -> Self {
        Incompat {
            subject_id,
            name,
            slots,
            minimum_free_slots,
            week_pattern_id: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    > for Incompat
{
    fn from(
        value: collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) -> Self {
        Incompat {
            subject_id: value.subject_id.into(),
            name: value.name,
            slots: value.slots.into_iter().map(|x| x.into()).collect(),
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.into()),
        }
    }
}

impl TryFrom<Incompat>
    for collomatique_state_colloscopes::incompats::Incompatibility<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::WeekPatternId,
    >
{
    type Error = super::time::SlotWithDurationError;
    fn try_from(value: Incompat) -> Result<Self, Self::Error> {
        let mut slots = vec![];
        for slot in value.slots {
            slots.push(slot.try_into()?);
        }

        Ok(collomatique_state_colloscopes::incompats::Incompatibility {
            subject_id: value.subject_id.into(),
            name: value.name,
            slots,
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.into()),
        })
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeIncompatId {
    id: collomatique_state_colloscopes::ColloscopeIncompatId,
}

#[pymethods]
impl ColloscopeIncompatId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeIncompatId> for ColloscopeIncompatId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeIncompatId) -> Self {
        ColloscopeIncompatId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeIncompatId> for ColloscopeIncompatId {
    fn from(value: collomatique_state_colloscopes::ColloscopeIncompatId) -> Self {
        ColloscopeIncompatId::from(&value)
    }
}

impl From<&ColloscopeIncompatId> for collomatique_state_colloscopes::ColloscopeIncompatId {
    fn from(value: &ColloscopeIncompatId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopeIncompatId> for collomatique_state_colloscopes::ColloscopeIncompatId {
    fn from(value: ColloscopeIncompatId) -> Self {
        collomatique_state_colloscopes::ColloscopeIncompatId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeIncompat {
    #[pyo3(set, get)]
    pub subject_id: ColloscopeSubjectId,
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub slots: Vec<time::SlotWithDuration>,
    #[pyo3(set, get)]
    pub minimum_free_slots: NonZeroU32,
    #[pyo3(set, get)]
    pub week_pattern_id: Option<ColloscopeWeekPatternId>,
}

#[pymethods]
impl ColloscopeIncompat {
    #[new]
    fn new(
        subject_id: ColloscopeSubjectId,
        name: String,
        slots: Vec<time::SlotWithDuration>,
        minimum_free_slots: NonZeroU32,
    ) -> Self {
        ColloscopeIncompat {
            subject_id,
            name,
            slots,
            minimum_free_slots,
            week_pattern_id: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::ColloscopeSubjectId,
            collomatique_state_colloscopes::ColloscopeWeekPatternId,
        >,
    > for ColloscopeIncompat
{
    fn from(
        value: collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::ColloscopeSubjectId,
            collomatique_state_colloscopes::ColloscopeWeekPatternId,
        >,
    ) -> Self {
        ColloscopeIncompat {
            subject_id: value.subject_id.into(),
            name: value.name,
            slots: value.slots.into_iter().map(|x| x.into()).collect(),
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.into()),
        }
    }
}

impl TryFrom<ColloscopeIncompat>
    for collomatique_state_colloscopes::incompats::Incompatibility<
        collomatique_state_colloscopes::ColloscopeSubjectId,
        collomatique_state_colloscopes::ColloscopeWeekPatternId,
    >
{
    type Error = super::time::SlotWithDurationError;
    fn try_from(value: ColloscopeIncompat) -> Result<Self, Self::Error> {
        let mut slots = vec![];
        for slot in value.slots {
            slots.push(slot.try_into()?);
        }

        Ok(collomatique_state_colloscopes::incompats::Incompatibility {
            subject_id: value.subject_id.into(),
            name: value.name,
            slots,
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.into()),
        })
    }
}
