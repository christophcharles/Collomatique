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

impl From<collomatique_state_colloscopes::slots::Slot> for SlotParameters {
    fn from(value: collomatique_state_colloscopes::slots::Slot) -> Self {
        SlotParameters {
            teacher_id: value.teacher_id.into(),
            start_time: value.start_time.into(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.into()),
            cost: value.cost,
        }
    }
}

impl From<SlotParameters> for collomatique_state_colloscopes::slots::Slot {
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
