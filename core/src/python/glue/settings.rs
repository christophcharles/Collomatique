use std::num::NonZeroU32;

use super::*;
use pyo3::types::PyString;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralSettings {
    #[pyo3(set, get)]
    pub strict_limits: StrictLimits,
}

impl From<collomatique_state_colloscopes::settings::GeneralSettings> for GeneralSettings {
    fn from(value: collomatique_state_colloscopes::settings::GeneralSettings) -> Self {
        GeneralSettings {
            strict_limits: value.strict_limits.into(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrictLimits {
    #[pyo3(set, get)]
    pub interrogations_per_week: Option<common::RangeInclusiveU32>,
    #[pyo3(set, get)]
    pub max_interrogations_per_day: Option<NonZeroU32>,
}

#[pymethods]
impl StrictLimits {
    #[new]
    fn new() -> Self {
        StrictLimits {
            interrogations_per_week: None,
            max_interrogations_per_day: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::settings::StrictLimits> for StrictLimits {
    fn from(value: collomatique_state_colloscopes::settings::StrictLimits) -> Self {
        StrictLimits {
            interrogations_per_week: value.interrogations_per_week.map(|x| x.into()),
            max_interrogations_per_day: value.max_interrogations_per_day,
        }
    }
}

impl From<StrictLimits> for collomatique_state_colloscopes::settings::StrictLimits {
    fn from(value: StrictLimits) -> Self {
        collomatique_state_colloscopes::settings::StrictLimits {
            interrogations_per_week: value.interrogations_per_week.map(|x| x.into()),
            max_interrogations_per_day: value.max_interrogations_per_day,
        }
    }
}
