use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen, from_py_object)]
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
        WeekPatternId { id: *value }
    }
}

impl From<collomatique_state_colloscopes::WeekPatternId> for WeekPatternId {
    fn from(value: collomatique_state_colloscopes::WeekPatternId) -> Self {
        WeekPatternId::from(&value)
    }
}

impl From<&WeekPatternId> for collomatique_state_colloscopes::WeekPatternId {
    fn from(value: &WeekPatternId) -> Self {
        value.id
    }
}

impl From<WeekPatternId> for collomatique_state_colloscopes::WeekPatternId {
    fn from(value: WeekPatternId) -> Self {
        collomatique_state_colloscopes::WeekPatternId::from(&value)
    }
}

#[pyclass(from_py_object)]
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
    fn new(name: String, week_count: usize) -> Self {
        WeekPattern {
            name,
            weeks: vec![true; week_count],
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl WeekPattern {
    /// Projects the sparse core pattern to the dense positional pyclass view,
    /// given the schedule's week ids in global walk order.
    pub fn from_mem(
        value: collomatique_state_colloscopes::week_patterns::WeekPattern,
        week_ids: &[collomatique_state_colloscopes::WeekId],
    ) -> Self {
        WeekPattern {
            name: value.name,
            weeks: week_ids
                .iter()
                .map(|week_id| !value.excluded_weeks.contains(week_id))
                .collect(),
        }
    }

    /// Folds the dense positional pyclass view back into the sparse core
    /// exclusion set, given the schedule's week ids in global walk order.
    pub fn into_mem(
        self,
        week_ids: &[collomatique_state_colloscopes::WeekId],
    ) -> collomatique_state_colloscopes::week_patterns::WeekPattern {
        let excluded_weeks = week_ids
            .iter()
            .zip(self.weeks)
            .filter_map(|(&week_id, active)| (!active).then_some(week_id))
            .collect();
        collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: self.name,
            excluded_weeks,
        }
    }
}
