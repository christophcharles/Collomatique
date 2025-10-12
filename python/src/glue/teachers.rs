use super::*;
use pyo3::types::PyString;

use std::collections::BTreeSet;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TeacherId {
    id: collomatique_state_colloscopes::TeacherId,
}

#[pymethods]
impl TeacherId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::TeacherId> for TeacherId {
    fn from(value: &collomatique_state_colloscopes::TeacherId) -> Self {
        TeacherId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::TeacherId> for TeacherId {
    fn from(value: collomatique_state_colloscopes::TeacherId) -> Self {
        TeacherId::from(&value)
    }
}

impl From<&TeacherId> for collomatique_state_colloscopes::TeacherId {
    fn from(value: &TeacherId) -> Self {
        value.id.clone()
    }
}

impl From<TeacherId> for collomatique_state_colloscopes::TeacherId {
    fn from(value: TeacherId) -> Self {
        collomatique_state_colloscopes::TeacherId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teacher {
    #[pyo3(set, get)]
    pub desc: PersonWithContact,
    #[pyo3(set, get)]
    pub subjects: BTreeSet<SubjectId>,
}

#[pymethods]
impl Teacher {
    #[new]
    fn new(firstname: String, surname: String) -> Self {
        Teacher {
            desc: PersonWithContact {
                firstname,
                surname,
                tel: String::new(),
                email: String::new(),
            },
            subjects: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    > for Teacher
{
    fn from(
        value: collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    ) -> Self {
        Teacher {
            desc: value.desc.into(),
            subjects: value.subjects.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<Teacher>
    for collomatique_state_colloscopes::teachers::Teacher<collomatique_state_colloscopes::SubjectId>
{
    fn from(value: Teacher) -> Self {
        collomatique_state_colloscopes::teachers::Teacher {
            desc: value.desc.into(),
            subjects: value.subjects.into_iter().map(|x| x.into()).collect(),
        }
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeTeacherId {
    id: collomatique_state_colloscopes::ColloscopeTeacherId,
}

#[pymethods]
impl ColloscopeTeacherId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeTeacherId> for ColloscopeTeacherId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeTeacherId) -> Self {
        ColloscopeTeacherId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeTeacherId> for ColloscopeTeacherId {
    fn from(value: collomatique_state_colloscopes::ColloscopeTeacherId) -> Self {
        ColloscopeTeacherId::from(&value)
    }
}

impl From<&ColloscopeTeacherId> for collomatique_state_colloscopes::ColloscopeTeacherId {
    fn from(value: &ColloscopeTeacherId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopeTeacherId> for collomatique_state_colloscopes::ColloscopeTeacherId {
    fn from(value: ColloscopeTeacherId) -> Self {
        collomatique_state_colloscopes::ColloscopeTeacherId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeTeacher {
    #[pyo3(set, get)]
    pub desc: PersonWithContact,
    #[pyo3(set, get)]
    pub subjects: BTreeSet<ColloscopeSubjectId>,
}

#[pymethods]
impl ColloscopeTeacher {
    #[new]
    fn new(firstname: String, surname: String) -> Self {
        ColloscopeTeacher {
            desc: PersonWithContact {
                firstname,
                surname,
                tel: String::new(),
                email: String::new(),
            },
            subjects: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::ColloscopeSubjectId,
        >,
    > for ColloscopeTeacher
{
    fn from(
        value: collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::ColloscopeSubjectId,
        >,
    ) -> Self {
        ColloscopeTeacher {
            desc: value.desc.into(),
            subjects: value.subjects.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<ColloscopeTeacher>
    for collomatique_state_colloscopes::teachers::Teacher<
        collomatique_state_colloscopes::ColloscopeSubjectId,
    >
{
    fn from(value: ColloscopeTeacher) -> Self {
        collomatique_state_colloscopes::teachers::Teacher {
            desc: value.desc.into(),
            subjects: value.subjects.into_iter().map(|x| x.into()).collect(),
        }
    }
}
