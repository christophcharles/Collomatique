use super::*;
use pyo3::types::PyString;

use std::collections::BTreeSet;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudentId {
    id: collomatique_state_colloscopes::StudentId,
}

#[pymethods]
impl StudentId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::StudentId> for StudentId {
    fn from(value: &collomatique_state_colloscopes::StudentId) -> Self {
        StudentId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::StudentId> for StudentId {
    fn from(value: collomatique_state_colloscopes::StudentId) -> Self {
        StudentId::from(&value)
    }
}

impl From<&StudentId> for collomatique_state_colloscopes::StudentId {
    fn from(value: &StudentId) -> Self {
        value.id.clone().into()
    }
}

impl From<StudentId> for collomatique_state_colloscopes::StudentId {
    fn from(value: StudentId) -> Self {
        collomatique_state_colloscopes::StudentId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Student {
    #[pyo3(set, get)]
    pub desc: PersonWithContact,
    #[pyo3(set, get)]
    pub excluded_periods: BTreeSet<PeriodId>,
}

#[pymethods]
impl Student {
    #[new]
    fn new(firstname: String, surname: String) -> Self {
        Student {
            desc: PersonWithContact {
                firstname,
                surname,
                tel: String::new(),
                email: String::new(),
            },
            excluded_periods: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::students::Student<collomatique_state_colloscopes::PeriodId>,
    > for Student
{
    fn from(
        value: collomatique_state_colloscopes::students::Student<
            collomatique_state_colloscopes::PeriodId,
        >,
    ) -> Self {
        Student {
            desc: value.desc.into(),
            excluded_periods: value
                .excluded_periods
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

impl From<Student>
    for collomatique_state_colloscopes::students::Student<collomatique_state_colloscopes::PeriodId>
{
    fn from(value: Student) -> Self {
        collomatique_state_colloscopes::students::Student {
            desc: value.desc.into(),
            excluded_periods: value
                .excluded_periods
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeStudentId {
    id: collomatique_state_colloscopes::ColloscopeStudentId,
}

#[pymethods]
impl ColloscopeStudentId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeStudentId> for ColloscopeStudentId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeStudentId) -> Self {
        ColloscopeStudentId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeStudentId> for ColloscopeStudentId {
    fn from(value: collomatique_state_colloscopes::ColloscopeStudentId) -> Self {
        ColloscopeStudentId::from(&value)
    }
}

impl From<&ColloscopeStudentId> for collomatique_state_colloscopes::ColloscopeStudentId {
    fn from(value: &ColloscopeStudentId) -> Self {
        value.id.clone().into()
    }
}

impl From<ColloscopeStudentId> for collomatique_state_colloscopes::ColloscopeStudentId {
    fn from(value: ColloscopeStudentId) -> Self {
        collomatique_state_colloscopes::ColloscopeStudentId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeStudent {
    #[pyo3(set, get)]
    pub desc: PersonWithContact,
    #[pyo3(set, get)]
    pub excluded_periods: BTreeSet<ColloscopePeriodId>,
}

#[pymethods]
impl ColloscopeStudent {
    #[new]
    fn new(firstname: String, surname: String) -> Self {
        ColloscopeStudent {
            desc: PersonWithContact {
                firstname,
                surname,
                tel: String::new(),
                email: String::new(),
            },
            excluded_periods: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::students::Student<
            collomatique_state_colloscopes::ColloscopePeriodId,
        >,
    > for ColloscopeStudent
{
    fn from(
        value: collomatique_state_colloscopes::students::Student<
            collomatique_state_colloscopes::ColloscopePeriodId,
        >,
    ) -> Self {
        ColloscopeStudent {
            desc: value.desc.into(),
            excluded_periods: value
                .excluded_periods
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

impl From<ColloscopeStudent>
    for collomatique_state_colloscopes::students::Student<
        collomatique_state_colloscopes::ColloscopePeriodId,
    >
{
    fn from(value: ColloscopeStudent) -> Self {
        collomatique_state_colloscopes::students::Student {
            desc: value.desc.into(),
            excluded_periods: value
                .excluded_periods
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}
