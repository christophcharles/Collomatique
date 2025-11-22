use std::num::NonZeroU32;

use super::*;
use pyo3::types::PyString;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    #[pyo3(set, get)]
    pub global: Limits,
    #[pyo3(set, get)]
    pub students: BTreeMap<StudentId, Limits>,
}

impl From<collomatique_state_colloscopes::settings::Settings> for Settings {
    fn from(value: collomatique_state_colloscopes::settings::Settings) -> Self {
        Settings {
            global: value.global.into(),
            students: value
                .students
                .into_iter()
                .map(|(id, limits)| (id.into(), limits.into()))
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoftU32 {
    #[pyo3(set, get)]
    pub soft: bool,
    #[pyo3(set, get)]
    pub value: u32,
}

#[pymethods]
impl SoftU32 {
    #[new]
    fn new(value: u32) -> Self {
        SoftU32 { soft: false, value }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::settings::SoftParam<u32>> for SoftU32 {
    fn from(value: collomatique_state_colloscopes::settings::SoftParam<u32>) -> Self {
        SoftU32 {
            soft: value.soft,
            value: value.value,
        }
    }
}

impl From<SoftU32> for collomatique_state_colloscopes::settings::SoftParam<u32> {
    fn from(value: SoftU32) -> Self {
        collomatique_state_colloscopes::settings::SoftParam {
            soft: value.soft,
            value: value.value,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoftNonZeroU32 {
    #[pyo3(set, get)]
    pub soft: bool,
    #[pyo3(set, get)]
    pub value: NonZeroU32,
}

#[pymethods]
impl SoftNonZeroU32 {
    #[new]
    fn new(value: NonZeroU32) -> Self {
        SoftNonZeroU32 { soft: false, value }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::settings::SoftParam<NonZeroU32>> for SoftNonZeroU32 {
    fn from(value: collomatique_state_colloscopes::settings::SoftParam<NonZeroU32>) -> Self {
        SoftNonZeroU32 {
            soft: value.soft,
            value: value.value,
        }
    }
}

impl From<SoftNonZeroU32> for collomatique_state_colloscopes::settings::SoftParam<NonZeroU32> {
    fn from(value: SoftNonZeroU32) -> Self {
        collomatique_state_colloscopes::settings::SoftParam {
            soft: value.soft,
            value: value.value,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    #[pyo3(set, get)]
    pub interrogations_per_week_min: Option<SoftU32>,
    #[pyo3(set, get)]
    pub interrogations_per_week_max: Option<SoftU32>,
    #[pyo3(set, get)]
    pub max_interrogations_per_day: Option<SoftNonZeroU32>,
}

#[pymethods]
impl Limits {
    #[new]
    fn new() -> Self {
        Limits {
            interrogations_per_week_min: None,
            interrogations_per_week_max: None,
            max_interrogations_per_day: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::settings::Limits> for Limits {
    fn from(value: collomatique_state_colloscopes::settings::Limits) -> Self {
        Limits {
            interrogations_per_week_min: value.interrogations_per_week_min.map(|x| x.into()),
            interrogations_per_week_max: value.interrogations_per_week_max.map(|x| x.into()),
            max_interrogations_per_day: value.max_interrogations_per_day.map(|x| x.into()),
        }
    }
}

impl From<Limits> for collomatique_state_colloscopes::settings::Limits {
    fn from(value: Limits) -> Self {
        collomatique_state_colloscopes::settings::Limits {
            interrogations_per_week_min: value.interrogations_per_week_min.map(|x| x.into()),
            interrogations_per_week_max: value.interrogations_per_week_max.map(|x| x.into()),
            max_interrogations_per_day: value.max_interrogations_per_day.map(|x| x.into()),
        }
    }
}
