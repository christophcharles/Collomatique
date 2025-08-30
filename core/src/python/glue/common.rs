use super::*;
use pyo3::types::PyString;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonWithContact {
    #[pyo3(set, get)]
    pub firstname: String,
    #[pyo3(set, get)]
    pub surname: String,
    #[pyo3(set, get)]
    pub tel: String,
    #[pyo3(set, get)]
    pub email: String,
}

impl From<collomatique_state_colloscopes::PersonWithContact> for PersonWithContact {
    fn from(value: collomatique_state_colloscopes::PersonWithContact) -> Self {
        PersonWithContact {
            firstname: value.firstname,
            surname: value.surname,
            tel: match value.tel {
                Some(v) => v.into_inner(),
                None => String::new(),
            },
            email: match value.email {
                Some(v) => v.into_inner(),
                None => String::new(),
            },
        }
    }
}

impl From<PersonWithContact> for crate::rpc::cmd_msg::common::PersonWithContactMsg {
    fn from(value: PersonWithContact) -> Self {
        use crate::rpc::cmd_msg::common::PersonWithContactMsg;
        PersonWithContactMsg {
            firstname: value.firstname,
            surname: value.surname,
            tel: non_empty_string::NonEmptyString::new(value.tel).ok(),
            email: non_empty_string::NonEmptyString::new(value.email).ok(),
        }
    }
}

#[pymethods]
impl PersonWithContact {
    #[new]
    fn new(firstname: String, surname: String) -> Self {
        PersonWithContact {
            firstname,
            surname,
            tel: String::new(),
            email: String::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeU32 {
    #[pyo3(set, get)]
    pub min: u32,
    #[pyo3(set, get)]
    pub max: u32,
}

impl From<std::ops::Range<u32>> for RangeU32 {
    fn from(value: std::ops::Range<u32>) -> Self {
        RangeU32 {
            min: value.start,
            max: value.end,
        }
    }
}

impl From<RangeU32> for std::ops::Range<u32> {
    fn from(value: RangeU32) -> Self {
        value.min..value.max
    }
}

#[pymethods]
impl RangeU32 {
    #[new]
    fn new(min: u32, max: u32) -> Self {
        RangeU32 { min, max }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}
