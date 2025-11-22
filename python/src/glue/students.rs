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

impl From<collomatique_state_colloscopes::students::Student> for Student {
    fn from(value: collomatique_state_colloscopes::students::Student) -> Self {
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

impl From<Student> for collomatique_state_colloscopes::students::Student {
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
