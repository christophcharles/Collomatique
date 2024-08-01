use super::*;

use pyo3::types::PyString;

use std::num::NonZeroU32;

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralData {
    #[pyo3(get, set)]
    interrogations_per_week_range: Option<(u32, u32)>,
    #[pyo3(get, set)]
    max_interrogations_per_day: Option<NonZeroU32>,
    #[pyo3(get, set)]
    week_count: NonZeroU32,
}

#[pymethods]
impl GeneralData {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ interrogations_per_week_range = {}, max_interrogations_per_day = {}, week_count = {} }}",
            match self_.interrogations_per_week_range {
                Some(val) => format!("{}..{}", val.0, val.1),
                None => String::from("none"),
            },
            match self_.max_interrogations_per_day {
                Some(val) => val.to_string(),
                None => String::from("none"),
            },
            self_.week_count,
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::GeneralData> for GeneralData {
    fn from(value: &backend::GeneralData) -> Self {
        GeneralData {
            interrogations_per_week_range: value
                .interrogations_per_week
                .clone()
                .map(|range| (range.start, range.end)),
            max_interrogations_per_day: value.max_interrogations_per_day,
            week_count: value.week_count,
        }
    }
}

impl From<backend::GeneralData> for GeneralData {
    fn from(value: backend::GeneralData) -> Self {
        GeneralData::from(&value)
    }
}

impl From<&GeneralData> for backend::GeneralData {
    fn from(value: &GeneralData) -> Self {
        backend::GeneralData {
            interrogations_per_week: value
                .interrogations_per_week_range
                .map(|tuple| tuple.0..tuple.1),
            max_interrogations_per_day: value.max_interrogations_per_day,
            week_count: value.week_count,
        }
    }
}

impl From<GeneralData> for backend::GeneralData {
    fn from(value: GeneralData) -> Self {
        backend::GeneralData::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekPatternHandle {
    pub handle: state::WeekPatternHandle,
}

impl From<&state::WeekPatternHandle> for WeekPatternHandle {
    fn from(value: &state::WeekPatternHandle) -> Self {
        WeekPatternHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::WeekPatternHandle> for WeekPatternHandle {
    fn from(value: state::WeekPatternHandle) -> Self {
        WeekPatternHandle::from(&value)
    }
}

impl From<&WeekPatternHandle> for state::WeekPatternHandle {
    fn from(value: &WeekPatternHandle) -> Self {
        value.handle.clone()
    }
}

impl From<WeekPatternHandle> for state::WeekPatternHandle {
    fn from(value: WeekPatternHandle) -> Self {
        state::WeekPatternHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekPattern {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    weeks: BTreeSet<u32>,
}

#[pymethods]
impl WeekPattern {
    #[new]
    fn new(name: String) -> Self {
        WeekPattern {
            name,
            weeks: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let weeks_vec: Vec<_> = self_.weeks.iter().map(|x| x.to_string()).collect();
        let output = format!(
            "{{ name = {}, weeks = [{}] }}",
            self_.name,
            weeks_vec.join(","),
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::WeekPattern> for WeekPattern {
    fn from(value: &backend::WeekPattern) -> Self {
        WeekPattern {
            name: value.name.clone(),
            weeks: value.weeks.iter().map(|w| w.get()).collect(),
        }
    }
}

impl From<backend::WeekPattern> for WeekPattern {
    fn from(value: backend::WeekPattern) -> Self {
        WeekPattern::from(&value)
    }
}

impl From<&WeekPattern> for backend::WeekPattern {
    fn from(value: &WeekPattern) -> Self {
        backend::WeekPattern {
            name: value.name.clone(),
            weeks: value.weeks.iter().map(|x| backend::Week::new(*x)).collect(),
        }
    }
}

impl From<WeekPattern> for backend::WeekPattern {
    fn from(value: WeekPattern) -> Self {
        backend::WeekPattern::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TeacherHandle {
    pub handle: state::TeacherHandle,
}

impl From<&state::TeacherHandle> for TeacherHandle {
    fn from(value: &state::TeacherHandle) -> Self {
        TeacherHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::TeacherHandle> for TeacherHandle {
    fn from(value: state::TeacherHandle) -> Self {
        TeacherHandle::from(&value)
    }
}

impl From<&TeacherHandle> for state::TeacherHandle {
    fn from(value: &TeacherHandle) -> Self {
        value.handle.clone()
    }
}

impl From<TeacherHandle> for state::TeacherHandle {
    fn from(value: TeacherHandle) -> Self {
        state::TeacherHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teacher {
    #[pyo3(set, get)]
    surname: String,
    #[pyo3(set, get)]
    firstname: String,
    #[pyo3(set, get)]
    contact: String,
}

#[pymethods]
impl Teacher {
    #[new]
    fn new(surname: String, firstname: String) -> Self {
        Teacher {
            surname,
            firstname,
            contact: String::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ surname = {}, firstname = {}, contact = {} }}",
            self_.surname, self_.firstname, self_.contact,
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::Teacher> for Teacher {
    fn from(value: &backend::Teacher) -> Self {
        Teacher {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            contact: value.contact.clone(),
        }
    }
}

impl From<backend::Teacher> for Teacher {
    fn from(value: backend::Teacher) -> Self {
        Teacher::from(&value)
    }
}

impl From<&Teacher> for backend::Teacher {
    fn from(value: &Teacher) -> Self {
        backend::Teacher {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            contact: value.contact.clone(),
        }
    }
}

impl From<Teacher> for backend::Teacher {
    fn from(value: Teacher) -> Self {
        backend::Teacher::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudentHandle {
    pub handle: state::StudentHandle,
}

impl From<&state::StudentHandle> for StudentHandle {
    fn from(value: &state::StudentHandle) -> Self {
        StudentHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::StudentHandle> for StudentHandle {
    fn from(value: state::StudentHandle) -> Self {
        StudentHandle::from(&value)
    }
}

impl From<&StudentHandle> for state::StudentHandle {
    fn from(value: &StudentHandle) -> Self {
        value.handle.clone()
    }
}

impl From<StudentHandle> for state::StudentHandle {
    fn from(value: StudentHandle) -> Self {
        state::StudentHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Student {
    #[pyo3(set, get)]
    surname: String,
    #[pyo3(set, get)]
    firstname: String,
    #[pyo3(set, get)]
    email: Option<String>,
    #[pyo3(set, get)]
    phone: Option<String>,
}

#[pymethods]
impl Student {
    #[new]
    fn new(firstname: String, surname: String) -> Self {
        Student {
            surname,
            firstname,
            email: None,
            phone: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ surname = {}, firstname = {}, email = {}, phone = {} }}",
            self_.surname,
            self_.firstname,
            match &self_.email {
                Some(email) => email.clone(),
                None => "none".to_string(),
            },
            match &self_.phone {
                Some(phone) => phone.clone(),
                None => "none".to_string(),
            },
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::Student> for Student {
    fn from(value: &backend::Student) -> Self {
        Student {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            email: value.email.clone(),
            phone: value.phone.clone(),
        }
    }
}

impl From<backend::Student> for Student {
    fn from(value: backend::Student) -> Self {
        Student::from(&value)
    }
}

impl From<&Student> for backend::Student {
    fn from(value: &Student) -> Self {
        backend::Student {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            email: value.email.clone(),
            phone: value.phone.clone(),
        }
    }
}

impl From<Student> for backend::Student {
    fn from(value: Student) -> Self {
        backend::Student::from(&value)
    }
}
