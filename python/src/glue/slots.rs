use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId {
    id: collomatique_state_colloscopes::SlotId,
}

#[pymethods]
impl SlotId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::SlotId> for SlotId {
    fn from(value: &collomatique_state_colloscopes::SlotId) -> Self {
        SlotId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::SlotId> for SlotId {
    fn from(value: collomatique_state_colloscopes::SlotId) -> Self {
        SlotId::from(&value)
    }
}

impl From<&SlotId> for collomatique_state_colloscopes::SlotId {
    fn from(value: &SlotId) -> Self {
        value.id.clone()
    }
}

impl From<SlotId> for collomatique_state_colloscopes::SlotId {
    fn from(value: SlotId) -> Self {
        collomatique_state_colloscopes::SlotId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    #[pyo3(set, get)]
    pub id: SlotId,
    #[pyo3(set, get)]
    pub parameters: SlotParameters,
}

#[pymethods]
impl Slot {
    #[new]
    fn new(id: SlotId, parameters: SlotParameters) -> Self {
        Slot { id, parameters }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotParameters {
    #[pyo3(set, get)]
    pub teacher_id: TeacherId,
    #[pyo3(set, get)]
    pub start_time: time::SlotStart,
    #[pyo3(set, get)]
    pub extra_info: String,
    #[pyo3(set, get)]
    pub week_pattern: Option<WeekPatternId>,
    #[pyo3(set, get)]
    pub cost: i32,
}

#[pymethods]
impl SlotParameters {
    #[new]
    fn new(teacher_id: TeacherId, start_time: time::SlotStart) -> Self {
        SlotParameters {
            teacher_id,
            start_time,
            extra_info: String::new(),
            week_pattern: None,
            cost: 0,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    > for SlotParameters
{
    fn from(
        value: collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) -> Self {
        SlotParameters {
            teacher_id: value.teacher_id.into(),
            start_time: value.start_time.into(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.into()),
            cost: value.cost,
        }
    }
}

impl From<SlotParameters>
    for collomatique_state_colloscopes::slots::Slot<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::WeekPatternId,
    >
{
    fn from(value: SlotParameters) -> Self {
        collomatique_state_colloscopes::slots::Slot {
            teacher_id: value.teacher_id.into(),
            start_time: value.start_time.into(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.into()),
            cost: value.cost,
        }
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeSlotId {
    id: collomatique_state_colloscopes::ColloscopeSlotId,
}

#[pymethods]
impl ColloscopeSlotId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeSlotId> for ColloscopeSlotId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeSlotId) -> Self {
        ColloscopeSlotId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeSlotId> for ColloscopeSlotId {
    fn from(value: collomatique_state_colloscopes::ColloscopeSlotId) -> Self {
        ColloscopeSlotId::from(&value)
    }
}

impl From<&ColloscopeSlotId> for collomatique_state_colloscopes::ColloscopeSlotId {
    fn from(value: &ColloscopeSlotId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopeSlotId> for collomatique_state_colloscopes::ColloscopeSlotId {
    fn from(value: ColloscopeSlotId) -> Self {
        collomatique_state_colloscopes::ColloscopeSlotId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSlot {
    #[pyo3(set, get)]
    pub id: ColloscopeSlotId,
    #[pyo3(set, get)]
    pub parameters: ColloscopeSlotParameters,
}

#[pymethods]
impl ColloscopeSlot {
    #[new]
    fn new(id: ColloscopeSlotId, parameters: ColloscopeSlotParameters) -> Self {
        ColloscopeSlot { id, parameters }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSlotParameters {
    #[pyo3(set, get)]
    pub teacher_id: ColloscopeTeacherId,
    #[pyo3(set, get)]
    pub start_time: time::SlotStart,
    #[pyo3(set, get)]
    pub extra_info: String,
    #[pyo3(set, get)]
    pub week_pattern: Option<ColloscopeWeekPatternId>,
    #[pyo3(set, get)]
    pub cost: i32,
}

#[pymethods]
impl ColloscopeSlotParameters {
    #[new]
    fn new(teacher_id: ColloscopeTeacherId, start_time: time::SlotStart) -> Self {
        ColloscopeSlotParameters {
            teacher_id,
            start_time,
            extra_info: String::new(),
            week_pattern: None,
            cost: 0,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::ColloscopeTeacherId,
            collomatique_state_colloscopes::ColloscopeWeekPatternId,
        >,
    > for ColloscopeSlotParameters
{
    fn from(
        value: collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::ColloscopeTeacherId,
            collomatique_state_colloscopes::ColloscopeWeekPatternId,
        >,
    ) -> Self {
        ColloscopeSlotParameters {
            teacher_id: value.teacher_id.into(),
            start_time: value.start_time.into(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.into()),
            cost: value.cost,
        }
    }
}

impl From<ColloscopeSlotParameters>
    for collomatique_state_colloscopes::slots::Slot<
        collomatique_state_colloscopes::ColloscopeTeacherId,
        collomatique_state_colloscopes::ColloscopeWeekPatternId,
    >
{
    fn from(value: ColloscopeSlotParameters) -> Self {
        collomatique_state_colloscopes::slots::Slot {
            teacher_id: value.teacher_id.into(),
            start_time: value.start_time.into(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.into()),
            cost: value.cost,
        }
    }
}
