use super::*;
use pyo3::types::PyString;

use super::general_planning::PeriodId;
use super::group_lists::GroupListId;
use super::slots::SlotId;
use super::students::StudentId;
use std::collections::BTreeSet;

#[pyclass(frozen)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope {
    #[pyo3(get)]
    pub period_map: BTreeMap<PeriodId, ColloscopePeriod>,
    #[pyo3(get)]
    pub group_lists: BTreeMap<GroupListId, ColloscopeGroupList>,
}

#[pymethods]
impl Colloscope {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::Colloscope> for Colloscope {
    fn from(value: collomatique_state_colloscopes::colloscopes::Colloscope) -> Self {
        Colloscope {
            period_map: value
                .period_map
                .into_iter()
                .map(|(id, period)| (id.into(), period.into()))
                .collect(),
            group_lists: value
                .group_lists
                .into_iter()
                .map(|(id, gl)| (id.into(), gl.into()))
                .collect(),
        }
    }
}

#[pyclass(frozen)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopePeriod {
    #[pyo3(get)]
    pub slot_map: BTreeMap<SlotId, ColloscopeSlot>,
}

#[pymethods]
impl ColloscopePeriod {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopePeriod> for ColloscopePeriod {
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopePeriod) -> Self {
        ColloscopePeriod {
            slot_map: value
                .slot_map
                .into_iter()
                .map(|(id, slot)| (id.into(), slot.into()))
                .collect(),
        }
    }
}

#[pyclass(frozen)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSlot {
    #[pyo3(get)]
    pub interrogations: Vec<Option<ColloscopeInterrogation>>,
}

#[pymethods]
impl ColloscopeSlot {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopeSlot> for ColloscopeSlot {
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopeSlot) -> Self {
        ColloscopeSlot {
            interrogations: value
                .interrogations
                .into_iter()
                .map(|opt| opt.map(|i| i.into()))
                .collect(),
        }
    }
}

#[pyclass(frozen)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeInterrogation {
    #[pyo3(get)]
    pub assigned_groups: BTreeSet<u32>,
}

#[pymethods]
impl ColloscopeInterrogation {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation>
    for ColloscopeInterrogation
{
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation) -> Self {
        ColloscopeInterrogation {
            assigned_groups: value.assigned_groups,
        }
    }
}

#[pyclass(frozen)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupList {
    #[pyo3(get)]
    pub groups_for_students: BTreeMap<StudentId, u32>,
}

#[pymethods]
impl ColloscopeGroupList {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopeGroupList>
    for ColloscopeGroupList
{
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopeGroupList) -> Self {
        ColloscopeGroupList {
            groups_for_students: value
                .groups_for_students
                .into_iter()
                .map(|(id, group)| (id.into(), group))
                .collect(),
        }
    }
}
