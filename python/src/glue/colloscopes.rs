use crate::glue::params::ColloscopeParameters;

use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeId {
    id: collomatique_state_colloscopes::ColloscopeId,
}

#[pymethods]
impl ColloscopeId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeId> for ColloscopeId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeId) -> Self {
        ColloscopeId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeId> for ColloscopeId {
    fn from(value: collomatique_state_colloscopes::ColloscopeId) -> Self {
        ColloscopeId::from(&value)
    }
}

impl From<&ColloscopeId> for collomatique_state_colloscopes::ColloscopeId {
    fn from(value: &ColloscopeId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopeId> for collomatique_state_colloscopes::ColloscopeId {
    fn from(value: ColloscopeId) -> Self {
        collomatique_state_colloscopes::ColloscopeId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(get)]
    pub params: ColloscopeParameters,
    #[pyo3(get)]
    pub id_maps: ColloscopeIdMaps,
    #[pyo3(set, get)]
    pub data: ColloscopeData,
}

#[pymethods]
impl Colloscope {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl TryFrom<collomatique_state_colloscopes::colloscopes::Colloscope> for Colloscope {
    type Error = PyErr;
    fn try_from(value: collomatique_state_colloscopes::colloscopes::Colloscope) -> PyResult<Self> {
        Ok(Colloscope {
            name: value.name,
            params: value.params.try_into()?,
            id_maps: value.id_maps.into(),
            data: value.data.into(),
        })
    }
}

impl TryFrom<Colloscope> for collomatique_state_colloscopes::colloscopes::Colloscope {
    type Error = PyErr;
    fn try_from(value: Colloscope) -> PyResult<Self> {
        Ok(collomatique_state_colloscopes::colloscopes::Colloscope {
            name: value.name,
            params: value.params.try_into()?,
            id_maps: value.id_maps.into(),
            data: value.data.into(),
        })
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeIdMaps {
    #[pyo3(get)]
    pub periods: BTreeMap<PeriodId, ColloscopePeriodId>,
    #[pyo3(get)]
    pub subjects: BTreeMap<SubjectId, ColloscopeSubjectId>,
    #[pyo3(get)]
    pub teachers: BTreeMap<TeacherId, ColloscopeTeacherId>,
    #[pyo3(get)]
    pub students: BTreeMap<StudentId, ColloscopeStudentId>,
    #[pyo3(get)]
    pub week_patterns: BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    #[pyo3(get)]
    pub slots: BTreeMap<slots::SlotId, slots::ColloscopeSlotId>,
    #[pyo3(get)]
    pub incompats: BTreeMap<incompatibilities::IncompatId, incompatibilities::ColloscopeIncompatId>,
    #[pyo3(get)]
    pub group_lists: BTreeMap<group_lists::GroupListId, group_lists::ColloscopeGroupListId>,
    #[pyo3(get)]
    pub rules: BTreeMap<RuleId, ColloscopeRuleId>,
}

#[pymethods]
impl ColloscopeIdMaps {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::colloscope_params::ColloscopeIdMaps<
            collomatique_state_colloscopes::PeriodId,
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::WeekPatternId,
            collomatique_state_colloscopes::SlotId,
            collomatique_state_colloscopes::IncompatId,
            collomatique_state_colloscopes::GroupListId,
            collomatique_state_colloscopes::RuleId,
        >,
    > for ColloscopeIdMaps
{
    fn from(
        value: collomatique_state_colloscopes::colloscope_params::ColloscopeIdMaps<
            collomatique_state_colloscopes::PeriodId,
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::WeekPatternId,
            collomatique_state_colloscopes::SlotId,
            collomatique_state_colloscopes::IncompatId,
            collomatique_state_colloscopes::GroupListId,
            collomatique_state_colloscopes::RuleId,
        >,
    ) -> Self {
        ColloscopeIdMaps {
            periods: value
                .periods
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            subjects: value
                .subjects
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            teachers: value
                .teachers
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            students: value
                .students
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            week_patterns: value
                .week_patterns
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            slots: value
                .slots
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            incompats: value
                .incompats
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            group_lists: value
                .group_lists
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            rules: value
                .rules
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
        }
    }
}

impl From<ColloscopeIdMaps>
    for collomatique_state_colloscopes::colloscope_params::ColloscopeIdMaps<
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::RuleId,
    >
{
    fn from(value: ColloscopeIdMaps) -> Self {
        collomatique_state_colloscopes::colloscope_params::ColloscopeIdMaps {
            periods: value
                .periods
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            subjects: value
                .subjects
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            teachers: value
                .teachers
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            students: value
                .students
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            week_patterns: value
                .week_patterns
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            slots: value
                .slots
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            incompats: value
                .incompats
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            group_lists: value
                .group_lists
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
            rules: value
                .rules
                .into_iter()
                .map(|(id, collo_id)| (id.into(), collo_id.into()))
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeData {
    #[pyo3(set, get)]
    pub period_map: BTreeMap<ColloscopePeriodId, ColloscopePeriodDesc>,
    #[pyo3(set, get)]
    pub group_lists: BTreeMap<group_lists::ColloscopeGroupListId, ColloscopeGroupListDesc>,
}

#[pymethods]
impl ColloscopeData {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopeData> for ColloscopeData {
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopeData) -> Self {
        ColloscopeData {
            period_map: value
                .period_map
                .into_iter()
                .map(|(id, data)| (id.into(), data.into()))
                .collect(),
            group_lists: value
                .group_lists
                .into_iter()
                .map(|(id, data)| (id.into(), data.into()))
                .collect(),
        }
    }
}

impl From<ColloscopeData> for collomatique_state_colloscopes::colloscopes::ColloscopeData {
    fn from(value: ColloscopeData) -> Self {
        collomatique_state_colloscopes::colloscopes::ColloscopeData {
            period_map: value
                .period_map
                .into_iter()
                .map(|(id, data)| (id.into(), data.into()))
                .collect(),
            group_lists: value
                .group_lists
                .into_iter()
                .map(|(id, data)| (id.into(), data.into()))
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopePeriodDesc {
    #[pyo3(set, get)]
    pub subject_map: BTreeMap<ColloscopeSubjectId, ColloscopeSubjectDesc>,
}

#[pymethods]
impl ColloscopePeriodDesc {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopePeriod> for ColloscopePeriodDesc {
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopePeriod) -> Self {
        ColloscopePeriodDesc {
            subject_map: value
                .subject_map
                .into_iter()
                .map(|(id, desc)| (id.into(), desc.into()))
                .collect(),
        }
    }
}

impl From<ColloscopePeriodDesc> for collomatique_state_colloscopes::colloscopes::ColloscopePeriod {
    fn from(value: ColloscopePeriodDesc) -> Self {
        collomatique_state_colloscopes::colloscopes::ColloscopePeriod {
            subject_map: value
                .subject_map
                .into_iter()
                .map(|(id, desc)| (id.into(), desc.into()))
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSubjectDesc {
    #[pyo3(set, get)]
    pub slots: BTreeMap<slots::ColloscopeSlotId, Vec<Option<ColloscopeInterrogation>>>,
}

#[pymethods]
impl ColloscopeSubjectDesc {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopeSubject>
    for ColloscopeSubjectDesc
{
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopeSubject) -> Self {
        ColloscopeSubjectDesc {
            slots: value
                .slots
                .into_iter()
                .map(|(id, desc)| {
                    (
                        id.into(),
                        desc.into_iter().map(|x| x.map(|y| y.into())).collect(),
                    )
                })
                .collect(),
        }
    }
}

impl From<ColloscopeSubjectDesc>
    for collomatique_state_colloscopes::colloscopes::ColloscopeSubject
{
    fn from(value: ColloscopeSubjectDesc) -> Self {
        collomatique_state_colloscopes::colloscopes::ColloscopeSubject {
            slots: value
                .slots
                .into_iter()
                .map(|(id, desc)| {
                    (
                        id.into(),
                        desc.into_iter().map(|x| x.map(|y| y.into())).collect(),
                    )
                })
                .collect(),
        }
    }
}

use std::collections::BTreeSet;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeInterrogation {
    #[pyo3(set, get)]
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
            assigned_groups: value.assigned_groups.clone(),
        }
    }
}

impl From<ColloscopeInterrogation>
    for collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation
{
    fn from(value: ColloscopeInterrogation) -> Self {
        collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation {
            assigned_groups: value.assigned_groups.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupListDesc {
    #[pyo3(set, get)]
    pub groups_for_students: BTreeMap<ColloscopeStudentId, Option<u32>>,
}

#[pymethods]
impl ColloscopeGroupListDesc {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::colloscopes::ColloscopeGroupList>
    for ColloscopeGroupListDesc
{
    fn from(value: collomatique_state_colloscopes::colloscopes::ColloscopeGroupList) -> Self {
        ColloscopeGroupListDesc {
            groups_for_students: value
                .groups_for_students
                .into_iter()
                .map(|(id, desc)| (id.into(), desc.clone()))
                .collect(),
        }
    }
}

impl From<ColloscopeGroupListDesc>
    for collomatique_state_colloscopes::colloscopes::ColloscopeGroupList
{
    fn from(value: ColloscopeGroupListDesc) -> Self {
        collomatique_state_colloscopes::colloscopes::ColloscopeGroupList {
            groups_for_students: value
                .groups_for_students
                .into_iter()
                .map(|(id, desc)| (id.into(), desc.clone()))
                .collect(),
        }
    }
}
