use super::*;
use pyo3::types::PyString;

use super::general_planning::PeriodId;
use super::group_lists::GroupListId;
use super::slots::SlotId;
use super::students::StudentId;
use std::collections::BTreeSet;

#[pyclass(frozen, from_py_object)]
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

impl Colloscope {
    /// Builds the dense Python view from the (possibly sparse) mem colloscope
    /// and its parameters.
    ///
    /// The dense skeleton — which period runs which slots, and which weeks each
    /// slot can hold an interrogation — is re-derived from the parameters
    /// (`is_interrogation_possible` mirrors the old Some-cell rule); the cell
    /// contents come from the sparse surface. On validated data this yields the
    /// exact byte-for-byte pyclass the previous `From` produced, but reads the
    /// colloscope only through its surface, so 1d's repr swap leaves this glue
    /// untouched.
    pub fn from_mem(
        colloscope: &collomatique_state_colloscopes::colloscopes::Colloscope,
        params: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> Self {
        let period_map = params
            .periods
            .period_ids()
            .map(|period_id| {
                let week_ids = params
                    .weeks
                    .weeks_for_period(period_id)
                    .into_iter()
                    .flatten()
                    .map(|(week_id, _week)| *week_id)
                    .collect::<Vec<_>>();
                let mut slot_map = BTreeMap::new();
                for (subject_id, subject) in params.subjects.ordered_subject_list.iter() {
                    if subject.excluded_periods.contains(&period_id) {
                        continue;
                    }
                    if subject.parameters.interrogation_parameters.is_none() {
                        continue;
                    }
                    let Some(subject_slots) = params.slots.slots_for_subject(subject_id) else {
                        continue;
                    };
                    for (slot_id, _slot) in subject_slots {
                        let interrogations = week_ids
                            .iter()
                            .map(|&week_id| {
                                if params.is_interrogation_possible(*slot_id, week_id) {
                                    let assigned_groups = colloscope
                                        .interrogation(*slot_id, week_id)
                                        .cloned()
                                        .unwrap_or_default();
                                    Some(ColloscopeInterrogation { assigned_groups })
                                } else {
                                    None
                                }
                            })
                            .collect();
                        slot_map.insert((*slot_id).into(), ColloscopeSlot { interrogations });
                    }
                }
                (period_id.into(), ColloscopePeriod { slot_map })
            })
            .collect();

        let group_lists = params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, group_list)| !group_list.is_prefilled())
            .map(|(id, _group_list)| {
                let groups_for_students = colloscope
                    .group_list(id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(student_id, group)| (student_id.into(), group))
                    .collect();
                (
                    id.into(),
                    ColloscopeGroupList {
                        groups_for_students,
                    },
                )
            })
            .collect();

        Colloscope {
            period_map,
            group_lists,
        }
    }
}

#[pyclass(frozen, from_py_object)]
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

#[pyclass(frozen, from_py_object)]
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

#[pyclass(frozen, from_py_object)]
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

#[pyclass(frozen, from_py_object)]
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
