use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupListId {
    id: collomatique_state_colloscopes::GroupListId,
}

#[pymethods]
impl GroupListId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::GroupListId> for GroupListId {
    fn from(value: &collomatique_state_colloscopes::GroupListId) -> Self {
        GroupListId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::GroupListId> for GroupListId {
    fn from(value: collomatique_state_colloscopes::GroupListId) -> Self {
        GroupListId::from(&value)
    }
}

impl From<&GroupListId> for collomatique_state_colloscopes::GroupListId {
    fn from(value: &GroupListId) -> Self {
        value.id.clone()
    }
}

impl From<GroupListId> for collomatique_state_colloscopes::GroupListId {
    fn from(value: GroupListId) -> Self {
        collomatique_state_colloscopes::GroupListId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList {
    #[pyo3(set, get)]
    pub parameters: GroupListParameters,
    #[pyo3(set, get)]
    pub filling: GroupListFilling,
}

#[pymethods]
impl GroupList {
    #[new]
    fn new(parameters: GroupListParameters) -> Self {
        GroupList {
            parameters,
            filling: GroupListFilling::Automatic {
                excluded_students: BTreeSet::new(),
            },
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::group_lists::GroupList> for GroupList {
    fn from(value: collomatique_state_colloscopes::group_lists::GroupList) -> Self {
        GroupList {
            parameters: value.params.into(),
            filling: value.filling.into(),
        }
    }
}

use std::collections::BTreeSet;
use std::num::NonZeroU32;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListParameters {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub students_per_group_min: NonZeroU32,
    #[pyo3(set, get)]
    pub students_per_group_max: NonZeroU32,
    #[pyo3(set, get)]
    pub group_names: Vec<Option<String>>,
}

#[pymethods]
impl GroupListParameters {
    #[new]
    fn new(name: String) -> Self {
        GroupListParameters {
            name,
            students_per_group_min: NonZeroU32::new(2).unwrap(),
            students_per_group_max: NonZeroU32::new(3).unwrap(),
            group_names: vec![None; 16],
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::group_lists::GroupListParameters>
    for GroupListParameters
{
    fn from(value: collomatique_state_colloscopes::group_lists::GroupListParameters) -> Self {
        GroupListParameters {
            name: value.name,
            students_per_group_min: *value.students_per_group.start(),
            students_per_group_max: *value.students_per_group.end(),
            group_names: value
                .group_names
                .into_iter()
                .map(|opt| opt.map(|s| s.into_inner()))
                .collect(),
        }
    }
}

impl From<GroupListParameters>
    for collomatique_state_colloscopes::group_lists::GroupListParameters
{
    fn from(value: GroupListParameters) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListParameters {
            name: value.name,
            students_per_group: value.students_per_group_min..=value.students_per_group_max,
            group_names: value
                .group_names
                .into_iter()
                .map(|opt| opt.and_then(|s| non_empty_string::NonEmptyString::new(s).ok()))
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupListFilling {
    Prefilled {
        groups: Vec<PrefilledGroup>,
    },
    Automatic {
        excluded_students: BTreeSet<StudentId>,
    },
}

#[pymethods]
impl GroupListFilling {
    #[staticmethod]
    fn prefilled(groups: Vec<PrefilledGroup>) -> Self {
        GroupListFilling::Prefilled { groups }
    }

    #[staticmethod]
    fn automatic(excluded_students: BTreeSet<StudentId>) -> Self {
        GroupListFilling::Automatic { excluded_students }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::group_lists::GroupListFilling> for GroupListFilling {
    fn from(value: collomatique_state_colloscopes::group_lists::GroupListFilling) -> Self {
        match value {
            collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
                GroupListFilling::Prefilled {
                    groups: groups.into_iter().map(|x| x.into()).collect(),
                }
            }
            collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                excluded_students,
            } => GroupListFilling::Automatic {
                excluded_students: excluded_students.into_iter().map(|x| x.into()).collect(),
            },
        }
    }
}

impl From<GroupListFilling> for collomatique_state_colloscopes::group_lists::GroupListFilling {
    fn from(value: GroupListFilling) -> Self {
        match value {
            GroupListFilling::Prefilled { groups } => {
                collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                    groups: groups.into_iter().map(|x| x.into()).collect(),
                }
            }
            GroupListFilling::Automatic { excluded_students } => {
                collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                    excluded_students: excluded_students.into_iter().map(|x| x.into()).collect(),
                }
            }
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefilledGroup {
    #[pyo3(set, get)]
    pub students: BTreeSet<StudentId>,
}

#[pymethods]
impl PrefilledGroup {
    #[new]
    fn new() -> Self {
        PrefilledGroup {
            students: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::group_lists::PrefilledGroup> for PrefilledGroup {
    fn from(value: collomatique_state_colloscopes::group_lists::PrefilledGroup) -> Self {
        PrefilledGroup {
            students: value.students.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<PrefilledGroup> for collomatique_state_colloscopes::group_lists::PrefilledGroup {
    fn from(value: PrefilledGroup) -> Self {
        collomatique_state_colloscopes::group_lists::PrefilledGroup {
            students: value.students.into_iter().map(|x| x.into()).collect(),
        }
    }
}
