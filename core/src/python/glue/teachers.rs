use super::*;
use pyo3::types::PyString;

use std::collections::BTreeSet;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TeacherId {
    id: crate::rpc::cmd_msg::MsgTeacherId,
}

#[pymethods]
impl TeacherId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgTeacherId> for TeacherId {
    fn from(value: &crate::rpc::cmd_msg::MsgTeacherId) -> Self {
        TeacherId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgTeacherId> for TeacherId {
    fn from(value: crate::rpc::cmd_msg::MsgTeacherId) -> Self {
        TeacherId::from(&value)
    }
}

impl From<&TeacherId> for crate::rpc::cmd_msg::MsgTeacherId {
    fn from(value: &TeacherId) -> Self {
        value.id.clone()
    }
}

impl From<TeacherId> for crate::rpc::cmd_msg::MsgTeacherId {
    fn from(value: TeacherId) -> Self {
        crate::rpc::cmd_msg::MsgTeacherId::from(&value)
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
            subjects: value
                .subjects
                .into_iter()
                .map(|x| MsgSubjectId::from(x).into())
                .collect(),
        }
    }
}

impl From<Teacher> for crate::rpc::cmd_msg::teachers::TeacherMsg {
    fn from(value: Teacher) -> Self {
        use crate::rpc::cmd_msg::teachers::TeacherMsg;
        TeacherMsg {
            desc: value.desc.into(),
            subjects: value.subjects.into_iter().map(|x| x.into()).collect(),
        }
    }
}
