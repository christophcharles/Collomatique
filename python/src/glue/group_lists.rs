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

impl
    From<
        collomatique_state_colloscopes::group_lists::GroupList<
            collomatique_state_colloscopes::StudentId,
        >,
    > for GroupList
{
    fn from(
        value: collomatique_state_colloscopes::group_lists::GroupList<
            collomatique_state_colloscopes::StudentId,
        >,
    ) -> Self {
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

impl
    From<
        collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::StudentId,
        >,
    > for GroupListParameters
{
    fn from(
        value: collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::StudentId,
        >,
    ) -> Self {
        GroupListParameters {
            name: value.name,
            students_per_group_min: *value.students_per_group.start(),
            students_per_group_max: *value.students_per_group.end(),
            group_count_min: *value.group_count.start(),
            group_count_max: *value.group_count.end(),
            excluded_students: value
                .excluded_students
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

impl From<GroupListParameters>
    for collomatique_state_colloscopes::group_lists::GroupListParameters<
        collomatique_state_colloscopes::StudentId,
    >
{
    fn from(value: GroupListParameters) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListParameters {
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
    pub name: String,
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
            name: String::new(),
            students: BTreeSet::new(),
            sealed: false,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::group_lists::PrefilledGroup<
            collomatique_state_colloscopes::StudentId,
        >,
    > for PrefilledGroup
{
    fn from(
        value: collomatique_state_colloscopes::group_lists::PrefilledGroup<
            collomatique_state_colloscopes::StudentId,
        >,
    ) -> Self {
        PrefilledGroup {
            name: match value.name {
                None => String::new(),
                Some(n) => n.into_inner(),
            },
            students: value.students.into_iter().map(|x| x.into()).collect(),
            sealed: value.sealed,
        }
    }
}

impl From<PrefilledGroup>
    for collomatique_state_colloscopes::group_lists::PrefilledGroup<
        collomatique_state_colloscopes::StudentId,
    >
{
    fn from(value: PrefilledGroup) -> Self {
        collomatique_state_colloscopes::group_lists::PrefilledGroup {
            name: non_empty_string::NonEmptyString::new(value.name).ok(),
            students: value.students.into_iter().map(|x| x.into()).collect(),
            sealed: value.sealed,
        }
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeGroupListId {
    id: collomatique_state_colloscopes::ColloscopeGroupListId,
}

#[pymethods]
impl ColloscopeGroupListId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeGroupListId> for ColloscopeGroupListId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeGroupListId) -> Self {
        ColloscopeGroupListId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeGroupListId> for ColloscopeGroupListId {
    fn from(value: collomatique_state_colloscopes::ColloscopeGroupListId) -> Self {
        ColloscopeGroupListId::from(&value)
    }
}

impl From<&ColloscopeGroupListId> for collomatique_state_colloscopes::ColloscopeGroupListId {
    fn from(value: &ColloscopeGroupListId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopeGroupListId> for collomatique_state_colloscopes::ColloscopeGroupListId {
    fn from(value: ColloscopeGroupListId) -> Self {
        collomatique_state_colloscopes::ColloscopeGroupListId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupList {
    #[pyo3(set, get)]
    pub parameters: ColloscopeGroupListParameters,
    #[pyo3(set, get)]
    pub prefilled_groups: Vec<ColloscopePrefilledGroup>,
}

#[pymethods]
impl ColloscopeGroupList {
    #[new]
    fn new(parameters: ColloscopeGroupListParameters) -> Self {
        ColloscopeGroupList {
            parameters,
            prefilled_groups: vec![],
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::group_lists::GroupList<
            collomatique_state_colloscopes::ColloscopeStudentId,
        >,
    > for ColloscopeGroupList
{
    fn from(
        value: collomatique_state_colloscopes::group_lists::GroupList<
            collomatique_state_colloscopes::ColloscopeStudentId,
        >,
    ) -> Self {
        ColloscopeGroupList {
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

impl From<ColloscopeGroupList>
    for collomatique_state_colloscopes::group_lists::GroupList<
        collomatique_state_colloscopes::ColloscopeStudentId,
    >
{
    fn from(
        value: ColloscopeGroupList,
    ) -> collomatique_state_colloscopes::group_lists::GroupList<
        collomatique_state_colloscopes::ColloscopeStudentId,
    > {
        collomatique_state_colloscopes::group_lists::GroupList {
            params: value.parameters.into(),
            prefilled_groups:
                collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups {
                    groups: value
                        .prefilled_groups
                        .into_iter()
                        .map(|x| x.into())
                        .collect(),
                },
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupListParameters {
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
    pub excluded_students: BTreeSet<ColloscopeStudentId>,
}

#[pymethods]
impl ColloscopeGroupListParameters {
    #[new]
    fn new(name: String) -> Self {
        ColloscopeGroupListParameters {
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

impl
    From<
        collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::ColloscopeStudentId,
        >,
    > for ColloscopeGroupListParameters
{
    fn from(
        value: collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::ColloscopeStudentId,
        >,
    ) -> Self {
        ColloscopeGroupListParameters {
            name: value.name,
            students_per_group_min: *value.students_per_group.start(),
            students_per_group_max: *value.students_per_group.end(),
            group_count_min: *value.group_count.start(),
            group_count_max: *value.group_count.end(),
            excluded_students: value
                .excluded_students
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

impl From<ColloscopeGroupListParameters>
    for collomatique_state_colloscopes::group_lists::GroupListParameters<
        collomatique_state_colloscopes::ColloscopeStudentId,
    >
{
    fn from(value: ColloscopeGroupListParameters) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListParameters {
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
pub struct ColloscopePrefilledGroup {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub students: BTreeSet<ColloscopeStudentId>,
    #[pyo3(set, get)]
    pub sealed: bool,
}

#[pymethods]
impl ColloscopePrefilledGroup {
    #[new]
    fn new() -> Self {
        ColloscopePrefilledGroup {
            name: String::new(),
            students: BTreeSet::new(),
            sealed: false,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::group_lists::PrefilledGroup<
            collomatique_state_colloscopes::ColloscopeStudentId,
        >,
    > for ColloscopePrefilledGroup
{
    fn from(
        value: collomatique_state_colloscopes::group_lists::PrefilledGroup<
            collomatique_state_colloscopes::ColloscopeStudentId,
        >,
    ) -> Self {
        ColloscopePrefilledGroup {
            name: match value.name {
                None => String::new(),
                Some(n) => n.into_inner(),
            },
            students: value.students.into_iter().map(|x| x.into()).collect(),
            sealed: value.sealed,
        }
    }
}

impl From<ColloscopePrefilledGroup>
    for collomatique_state_colloscopes::group_lists::PrefilledGroup<
        collomatique_state_colloscopes::ColloscopeStudentId,
    >
{
    fn from(value: ColloscopePrefilledGroup) -> Self {
        collomatique_state_colloscopes::group_lists::PrefilledGroup {
            name: non_empty_string::NonEmptyString::new(value.name).ok(),
            students: value.students.into_iter().map(|x| x.into()).collect(),
            sealed: value.sealed,
        }
    }
}
