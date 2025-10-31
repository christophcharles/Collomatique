use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekPatternId {
    id: collomatique_state_colloscopes::WeekPatternId,
}

#[pymethods]
impl WeekPatternId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::WeekPatternId> for WeekPatternId {
    fn from(value: &collomatique_state_colloscopes::WeekPatternId) -> Self {
        WeekPatternId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::WeekPatternId> for WeekPatternId {
    fn from(value: collomatique_state_colloscopes::WeekPatternId) -> Self {
        WeekPatternId::from(&value)
    }
}

impl From<&WeekPatternId> for collomatique_state_colloscopes::WeekPatternId {
    fn from(value: &WeekPatternId) -> Self {
        value.id.clone().into()
    }
}

impl From<WeekPatternId> for collomatique_state_colloscopes::WeekPatternId {
    fn from(value: WeekPatternId) -> Self {
        collomatique_state_colloscopes::WeekPatternId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekPattern {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub weeks: Vec<bool>,
}

#[pymethods]
impl WeekPattern {
    #[new]
    fn new(name: String) -> Self {
        WeekPattern {
            name,
            weeks: vec![],
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::week_patterns::WeekPattern> for WeekPattern {
    fn from(value: collomatique_state_colloscopes::week_patterns::WeekPattern) -> Self {
        WeekPattern {
            name: value.name,
            weeks: value.weeks,
        }
    }
}

impl From<WeekPattern> for collomatique_state_colloscopes::week_patterns::WeekPattern {
    fn from(value: WeekPattern) -> Self {
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: value.name,
            weeks: value.weeks,
        }
    }
}
