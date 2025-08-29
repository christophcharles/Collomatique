use super::*;
use pyo3::types::PyString;

use std::collections::BTreeSet;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudentId {
    id: crate::rpc::cmd_msg::MsgStudentId,
}

#[pymethods]
impl StudentId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgStudentId> for StudentId {
    fn from(value: &crate::rpc::cmd_msg::MsgStudentId) -> Self {
        StudentId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgStudentId> for StudentId {
    fn from(value: crate::rpc::cmd_msg::MsgStudentId) -> Self {
        StudentId::from(&value)
    }
}

impl From<&StudentId> for crate::rpc::cmd_msg::MsgStudentId {
    fn from(value: &StudentId) -> Self {
        value.id.clone()
    }
}

impl From<StudentId> for crate::rpc::cmd_msg::MsgStudentId {
    fn from(value: StudentId) -> Self {
        crate::rpc::cmd_msg::MsgStudentId::from(&value)
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
                .map(|x| MsgPeriodId::from(x).into())
                .collect(),
        }
    }
}

impl From<Student> for crate::rpc::cmd_msg::students::StudentMsg {
    fn from(value: Student) -> Self {
        use crate::rpc::cmd_msg::students::StudentMsg;
        StudentMsg {
            desc: value.desc.into(),
            excluded_periods: value
                .excluded_periods
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}
