use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupListId {
    id: crate::rpc::cmd_msg::MsgGroupListId,
}

#[pymethods]
impl GroupListId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgGroupListId> for GroupListId {
    fn from(value: &crate::rpc::cmd_msg::MsgGroupListId) -> Self {
        GroupListId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgGroupListId> for GroupListId {
    fn from(value: crate::rpc::cmd_msg::MsgGroupListId) -> Self {
        GroupListId::from(&value)
    }
}

impl From<&GroupListId> for crate::rpc::cmd_msg::MsgGroupListId {
    fn from(value: &GroupListId) -> Self {
        value.id.clone()
    }
}

impl From<GroupListId> for crate::rpc::cmd_msg::MsgGroupListId {
    fn from(value: GroupListId) -> Self {
        crate::rpc::cmd_msg::MsgGroupListId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList {
    #[pyo3(set, get)]
    pub parameters: GroupListParameters,
    #[pyo3(set, get)]
    pub prefilled_groups: Vec<PrefilledGroup>,
}

#[pymethods]
impl GroupList {
    #[new]
    fn new(parameters: GroupListParameters) -> Self {
        GroupList {
            parameters,
            prefilled_groups: vec![],
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
            prefilled_groups: value
                .prefilled_groups
                .groups
                .into_iter()
                .map(|x| x.into())
                .collect(),
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
    pub group_count_min: u32,
    #[pyo3(set, get)]
    pub group_count_max: u32,
    #[pyo3(set, get)]
    pub excluded_students: BTreeSet<StudentId>,
}

#[pymethods]
impl GroupListParameters {
    #[new]
    fn new(name: String) -> Self {
        GroupListParameters {
            name,
            students_per_group_min: NonZeroU32::new(2).unwrap(),
            students_per_group_max: NonZeroU32::new(3).unwrap(),
            group_count_min: 0,
            group_count_max: 1,
            excluded_students: BTreeSet::new(),
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
            group_count_min: *value.group_count.start(),
            group_count_max: *value.group_count.end(),
            excluded_students: value
                .excluded_students
                .into_iter()
                .map(|x| MsgStudentId::from(x).into())
                .collect(),
        }
    }
}

impl From<GroupListParameters> for crate::rpc::cmd_msg::group_lists::GroupListParametersMsg {
    fn from(value: GroupListParameters) -> Self {
        use crate::rpc::cmd_msg::group_lists::GroupListParametersMsg;
        GroupListParametersMsg {
            name: value.name,
            students_per_group: value.students_per_group_min..=value.students_per_group_max,
            group_count: value.group_count_min..=value.group_count_max,
            excluded_students: value
                .excluded_students
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefilledGroup {
    #[pyo3(set, get)]
    pub students: BTreeSet<StudentId>,
    #[pyo3(set, get)]
    pub sealed: bool,
}

#[pymethods]
impl PrefilledGroup {
    #[new]
    fn new() -> Self {
        PrefilledGroup {
            students: BTreeSet::new(),
            sealed: false,
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
            students: value
                .students
                .into_iter()
                .map(|x| MsgStudentId::from(x).into())
                .collect(),
            sealed: value.sealed,
        }
    }
}

impl From<PrefilledGroup> for crate::rpc::cmd_msg::group_lists::PrefilledGroupMsg {
    fn from(value: PrefilledGroup) -> Self {
        use crate::rpc::cmd_msg::group_lists::PrefilledGroupMsg;
        PrefilledGroupMsg {
            students: value.students.into_iter().map(|x| x.into()).collect(),
            sealed: value.sealed,
        }
    }
}
