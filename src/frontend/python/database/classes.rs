use super::*;

use pyo3::types::PyString;

use std::num::{NonZeroU32, NonZeroUsize};

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
                Some(val) => format!("{}..{}", val.0, val.1 as i64),
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

#[pymethods]
impl WeekPatternHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
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

#[pymethods]
impl TeacherHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
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

#[pymethods]
impl StudentHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
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

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectGroupHandle {
    pub handle: state::SubjectGroupHandle,
}

#[pymethods]
impl SubjectGroupHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::SubjectGroupHandle> for SubjectGroupHandle {
    fn from(value: &state::SubjectGroupHandle) -> Self {
        SubjectGroupHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::SubjectGroupHandle> for SubjectGroupHandle {
    fn from(value: state::SubjectGroupHandle) -> Self {
        SubjectGroupHandle::from(&value)
    }
}

impl From<&SubjectGroupHandle> for state::SubjectGroupHandle {
    fn from(value: &SubjectGroupHandle) -> Self {
        value.handle.clone()
    }
}

impl From<SubjectGroupHandle> for state::SubjectGroupHandle {
    fn from(value: SubjectGroupHandle) -> Self {
        state::SubjectGroupHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectGroup {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    optional: bool,
}

#[pymethods]
impl SubjectGroup {
    #[new]
    fn new(name: String) -> Self {
        SubjectGroup {
            name,
            optional: false,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{{ name = {}, optional = {} }}", self_.name, self_.optional,);

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::SubjectGroup> for SubjectGroup {
    fn from(value: &backend::SubjectGroup) -> Self {
        SubjectGroup {
            name: value.name.clone(),
            optional: value.optional,
        }
    }
}

impl From<backend::SubjectGroup> for SubjectGroup {
    fn from(value: backend::SubjectGroup) -> Self {
        SubjectGroup::from(&value)
    }
}

impl From<&SubjectGroup> for backend::SubjectGroup {
    fn from(value: &SubjectGroup) -> Self {
        backend::SubjectGroup {
            name: value.name.clone(),
            optional: value.optional,
        }
    }
}

impl From<SubjectGroup> for backend::SubjectGroup {
    fn from(value: SubjectGroup) -> Self {
        backend::SubjectGroup::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncompatHandle {
    pub handle: state::IncompatHandle,
}

#[pymethods]
impl IncompatHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::IncompatHandle> for IncompatHandle {
    fn from(value: &state::IncompatHandle) -> Self {
        IncompatHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::IncompatHandle> for IncompatHandle {
    fn from(value: state::IncompatHandle) -> Self {
        IncompatHandle::from(&value)
    }
}

impl From<&IncompatHandle> for state::IncompatHandle {
    fn from(value: &IncompatHandle) -> Self {
        value.handle.clone()
    }
}

impl From<IncompatHandle> for state::IncompatHandle {
    fn from(value: IncompatHandle) -> Self {
        state::IncompatHandle::from(&value)
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl std::fmt::Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Weekday::Monday => "Monday",
                Weekday::Tuesday => "Tuesday",
                Weekday::Wednesday => "Wednesday",
                Weekday::Thursday => "Thursday",
                Weekday::Friday => "Friday",
                Weekday::Saturday => "Saturday",
                Weekday::Sunday => "Sunday",
            }
        )
    }
}

#[pymethods]
impl Weekday {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = self_.to_string();

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&crate::time::Weekday> for Weekday {
    fn from(value: &crate::time::Weekday) -> Self {
        use crate::time::Weekday as W;
        match value {
            W::Monday => Weekday::Monday,
            W::Tuesday => Weekday::Tuesday,
            W::Wednesday => Weekday::Wednesday,
            W::Thursday => Weekday::Thursday,
            W::Friday => Weekday::Friday,
            W::Saturday => Weekday::Saturday,
            W::Sunday => Weekday::Sunday,
        }
    }
}

impl From<crate::time::Weekday> for Weekday {
    fn from(value: crate::time::Weekday) -> Self {
        Weekday::from(&value)
    }
}

impl From<&Weekday> for crate::time::Weekday {
    fn from(value: &Weekday) -> Self {
        use crate::time::Weekday as W;
        match value {
            Weekday::Monday => W::Monday,
            Weekday::Tuesday => W::Tuesday,
            Weekday::Wednesday => W::Wednesday,
            Weekday::Thursday => W::Thursday,
            Weekday::Friday => W::Friday,
            Weekday::Saturday => W::Saturday,
            Weekday::Sunday => W::Sunday,
        }
    }
}

impl From<Weekday> for crate::time::Weekday {
    fn from(value: Weekday) -> Self {
        crate::time::Weekday::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    #[pyo3(get)]
    hour: u32,
    #[pyo3(get)]
    minute: u32,
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute,)
    }
}

#[pymethods]
impl Time {
    #[new]
    fn new(hour: u32, minute: u32) -> PyResult<Self> {
        if hour >= 24 {
            return Err(PyValueError::new_err("Hour must be less or equal to 23"));
        }
        if minute >= 60 {
            return Err(PyValueError::new_err("Hour must be less or equal to 59"));
        }
        Ok(Time { hour, minute })
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        PyString::new_bound(self_.py(), self_.to_string().as_str())
    }

    #[setter]
    fn set_hour(&mut self, hour: u32) -> PyResult<()> {
        if hour >= 24 {
            return Err(PyValueError::new_err("Hour must be less or equal to 23"));
        }
        self.hour = hour;
        Ok(())
    }

    #[setter]
    fn set_minute(&mut self, minute: u32) -> PyResult<()> {
        if minute >= 60 {
            return Err(PyValueError::new_err("Hour must be less or equal to 59"));
        }
        self.minute = minute;
        Ok(())
    }
}

impl From<&crate::time::Time> for Time {
    fn from(value: &crate::time::Time) -> Self {
        Time {
            hour: value.get_hour(),
            minute: value.get_min(),
        }
    }
}

impl From<crate::time::Time> for Time {
    fn from(value: crate::time::Time) -> Self {
        Time::from(&value)
    }
}

impl From<&Time> for crate::time::Time {
    fn from(value: &Time) -> Self {
        crate::time::Time::from_hm(value.hour, value.minute)
            .expect("Time should always give valid hour and minute")
    }
}

impl From<Time> for crate::time::Time {
    fn from(value: Time) -> Self {
        crate::time::Time::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotStart {
    #[pyo3(set, get)]
    day: Weekday,
    #[pyo3(set, get)]
    time: Time,
}

impl std::fmt::Display for SlotStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ day = {}, time = {} }}", self.day, self.time,)
    }
}

#[pymethods]
impl SlotStart {
    #[new]
    fn new(day: Weekday, time: Time) -> Self {
        SlotStart { day, time }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        PyString::new_bound(self_.py(), self_.to_string().as_str())
    }
}

impl From<&backend::SlotStart> for SlotStart {
    fn from(value: &backend::SlotStart) -> Self {
        SlotStart {
            day: value.day.into(),
            time: value.time.clone().into(),
        }
    }
}

impl From<backend::SlotStart> for SlotStart {
    fn from(value: backend::SlotStart) -> Self {
        SlotStart::from(&value)
    }
}

impl From<&SlotStart> for backend::SlotStart {
    fn from(value: &SlotStart) -> Self {
        backend::SlotStart {
            day: value.day.into(),
            time: value.time.clone().into(),
        }
    }
}

impl From<SlotStart> for backend::SlotStart {
    fn from(value: SlotStart) -> Self {
        backend::SlotStart::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatSlot {
    #[pyo3(set, get)]
    week_pattern_handle: WeekPatternHandle,
    #[pyo3(set, get)]
    start: SlotStart,
    #[pyo3(set, get)]
    duration: NonZeroU32,
}

impl std::fmt::Display for IncompatSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ week_pattern_handle = {:?}, start = {}, duration = {} }}",
            self.week_pattern_handle, self.start, self.duration,
        )
    }
}

#[pymethods]
impl IncompatSlot {
    #[new]
    fn new(week_pattern_handle: WeekPatternHandle, start: SlotStart, duration: NonZeroU32) -> Self {
        IncompatSlot {
            week_pattern_handle,
            start,
            duration,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        PyString::new_bound(self_.py(), self_.to_string().as_str())
    }
}

impl From<&backend::IncompatSlot<state::WeekPatternHandle>> for IncompatSlot {
    fn from(value: &backend::IncompatSlot<state::WeekPatternHandle>) -> Self {
        IncompatSlot {
            week_pattern_handle: value.week_pattern_id.into(),
            start: value.start.clone().into(),
            duration: value.duration,
        }
    }
}

impl From<backend::IncompatSlot<state::WeekPatternHandle>> for IncompatSlot {
    fn from(value: backend::IncompatSlot<state::WeekPatternHandle>) -> Self {
        IncompatSlot::from(&value)
    }
}

impl From<&IncompatSlot> for backend::IncompatSlot<state::WeekPatternHandle> {
    fn from(value: &IncompatSlot) -> Self {
        backend::IncompatSlot {
            week_pattern_id: value.week_pattern_handle.clone().into(),
            start: value.start.clone().into(),
            duration: value.duration,
        }
    }
}

impl From<IncompatSlot> for backend::IncompatSlot<state::WeekPatternHandle> {
    fn from(value: IncompatSlot) -> Self {
        backend::IncompatSlot::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incompat {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    max_count: usize,
    #[pyo3(set, get)]
    groups: BTreeSet<BTreeSet<IncompatSlot>>,
}

#[pymethods]
impl Incompat {
    #[new]
    fn new(name: String) -> Self {
        Incompat {
            name,
            max_count: 0,
            groups: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let groups_strings: Vec<_> = self_
            .groups
            .iter()
            .map(|x| {
                let temp: Vec<_> = x.iter().map(|y| y.to_string()).collect();

                format!("[{}]", temp.join(","))
            })
            .collect();

        let output = format!(
            "{{ name = {}, max_count = {}, groups = [{}] }}",
            self_.name,
            self_.max_count,
            groups_strings.join(","),
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::Incompat<state::WeekPatternHandle>> for Incompat {
    fn from(value: &backend::Incompat<state::WeekPatternHandle>) -> Self {
        Incompat {
            name: value.name.clone(),
            max_count: value.max_count,
            groups: value
                .groups
                .iter()
                .map(|x| x.slots.iter().map(|y| y.into()).collect())
                .collect(),
        }
    }
}

impl From<backend::Incompat<state::WeekPatternHandle>> for Incompat {
    fn from(value: backend::Incompat<state::WeekPatternHandle>) -> Self {
        Incompat::from(&value)
    }
}

impl From<&Incompat> for backend::Incompat<state::WeekPatternHandle> {
    fn from(value: &Incompat) -> Self {
        backend::Incompat {
            name: value.name.clone(),
            max_count: value.max_count,
            groups: value
                .groups
                .iter()
                .map(|x| backend::IncompatGroup {
                    slots: x.iter().map(|y| y.into()).collect(),
                })
                .collect(),
        }
    }
}

impl From<Incompat> for backend::Incompat<state::WeekPatternHandle> {
    fn from(value: Incompat) -> Self {
        backend::Incompat::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupListHandle {
    pub handle: state::GroupListHandle,
}

#[pymethods]
impl GroupListHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::GroupListHandle> for GroupListHandle {
    fn from(value: &state::GroupListHandle) -> Self {
        GroupListHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::GroupListHandle> for GroupListHandle {
    fn from(value: state::GroupListHandle) -> Self {
        GroupListHandle::from(&value)
    }
}

impl From<&GroupListHandle> for state::GroupListHandle {
    fn from(value: &GroupListHandle) -> Self {
        value.handle.clone()
    }
}

impl From<GroupListHandle> for state::GroupListHandle {
    fn from(value: GroupListHandle) -> Self {
        state::GroupListHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    extendable: bool,
}

impl std::fmt::Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ name = {}, extendable = {} }}",
            self.name, self.extendable,
        )
    }
}

#[pymethods]
impl Group {
    #[new]
    fn new(name: String) -> Self {
        Group {
            name,
            extendable: false,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        PyString::new_bound(self_.py(), self_.to_string().as_str())
    }
}

impl From<&backend::Group> for Group {
    fn from(value: &backend::Group) -> Self {
        Group {
            name: value.name.clone(),
            extendable: value.extendable,
        }
    }
}

impl From<backend::Group> for Group {
    fn from(value: backend::Group) -> Self {
        Group::from(&value)
    }
}

impl From<&Group> for backend::Group {
    fn from(value: &Group) -> Self {
        backend::Group {
            name: value.name.clone(),
            extendable: value.extendable,
        }
    }
}

impl From<Group> for backend::Group {
    fn from(value: Group) -> Self {
        backend::Group::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupList {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    groups: Vec<Group>,
    #[pyo3(set, get)]
    students_mapping: BTreeMap<StudentHandle, usize>,
}

impl std::fmt::Display for GroupList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let groups_strings: Vec<_> = self.groups.iter().map(|g| g.to_string()).collect();

        let students_mapping_strings: Vec<_> = self
            .students_mapping
            .iter()
            .map(|(student_handle, group)| format!("{:?}: {}", student_handle, group))
            .collect();

        write!(
            f,
            "{{ name = {}, groups = [{}], students_mapping = {{ {} }} }}",
            self.name,
            groups_strings.join(","),
            students_mapping_strings.join(","),
        )
    }
}

#[pymethods]
impl GroupList {
    #[new]
    fn new(name: String) -> Self {
        GroupList {
            name,
            groups: Vec::new(),
            students_mapping: BTreeMap::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        PyString::new_bound(self_.py(), self_.to_string().as_str())
    }
}

impl From<&backend::GroupList<state::StudentHandle>> for GroupList {
    fn from(value: &backend::GroupList<state::StudentHandle>) -> Self {
        GroupList {
            name: value.name.clone(),
            groups: value.groups.iter().map(|x| x.into()).collect(),
            students_mapping: value
                .students_mapping
                .iter()
                .map(|(y, z)| (y.into(), *z))
                .collect(),
        }
    }
}

impl From<backend::GroupList<state::StudentHandle>> for GroupList {
    fn from(value: backend::GroupList<state::StudentHandle>) -> Self {
        GroupList::from(&value)
    }
}

impl From<&GroupList> for backend::GroupList<state::StudentHandle> {
    fn from(value: &GroupList) -> Self {
        backend::GroupList {
            name: value.name.clone(),
            groups: value.groups.iter().map(|x| x.into()).collect(),
            students_mapping: value
                .students_mapping
                .iter()
                .map(|(y, z)| (y.into(), *z))
                .collect(),
        }
    }
}

impl From<GroupList> for backend::GroupList<state::StudentHandle> {
    fn from(value: GroupList) -> Self {
        backend::GroupList::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectHandle {
    pub handle: state::SubjectHandle,
}

#[pymethods]
impl SubjectHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::SubjectHandle> for SubjectHandle {
    fn from(value: &state::SubjectHandle) -> Self {
        SubjectHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::SubjectHandle> for SubjectHandle {
    fn from(value: state::SubjectHandle) -> Self {
        SubjectHandle::from(&value)
    }
}

impl From<&SubjectHandle> for state::SubjectHandle {
    fn from(value: &SubjectHandle) -> Self {
        value.handle.clone()
    }
}

impl From<SubjectHandle> for state::SubjectHandle {
    fn from(value: SubjectHandle) -> Self {
        state::SubjectHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    subject_group_handle: SubjectGroupHandle,
    #[pyo3(set, get)]
    incompat_handle: Option<IncompatHandle>,
    #[pyo3(set, get)]
    group_list_handle: Option<GroupListHandle>,
    #[pyo3(set, get)]
    duration: NonZeroU32,
    #[pyo3(set, get)]
    students_per_group_range: (NonZeroUsize, NonZeroUsize),
    #[pyo3(set, get)]
    period: NonZeroU32,
    #[pyo3(set, get)]
    period_is_strict: bool,
    #[pyo3(set, get)]
    is_tutorial: bool,
    #[pyo3(set, get)]
    max_groups_per_slot: NonZeroUsize,
    #[pyo3(set, get)]
    balance_teachers: bool,
    #[pyo3(set, get)]
    balance_timeslots: bool,
}

#[pymethods]
impl Subject {
    #[new]
    fn new(name: String, subject_group_handle: SubjectGroupHandle) -> Self {
        Subject {
            name,
            subject_group_handle,
            incompat_handle: None,
            group_list_handle: None,
            duration: NonZeroU32::new(60).unwrap(),
            students_per_group_range: (
                NonZeroUsize::new(2).unwrap(),
                NonZeroUsize::new(3).unwrap(),
            ),
            period: NonZeroU32::new(32).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balance_teachers: false,
            balance_timeslots: false,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ name = {}, subject_group_handle = {:?}, incompat_handle = {}, group_list_handle = {}, duration = {}, students_per_group_range = {}..={}, period = {}, period_is_strict = {}, is_tutorial = {}, max_groups_per_slot = {}, balance_teachers = {}, balance_timeslots = {} }}",
            self_.name,
            self_.subject_group_handle,
            match &self_.incompat_handle {
                Some(handle) => {
                    format!("{:?}", handle)
                }
                None => {
                    "none".to_string()
                }
            },
            match &self_.group_list_handle {
                Some(handle) => {
                    format!("{:?}", handle)
                }
                None => {
                    "none".to_string()
                }
            },
            self_.duration,
            self_.students_per_group_range.0.get(),
            self_.students_per_group_range.1.get(),
            self_.period.get(),
            self_.period_is_strict,
            self_.is_tutorial,
            self_.max_groups_per_slot.get(),
            self_.balance_teachers,
            self_.balance_timeslots,

        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl
    From<
        &backend::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>,
    > for Subject
{
    fn from(
        value: &backend::Subject<
            state::SubjectGroupHandle,
            state::IncompatHandle,
            state::GroupListHandle,
        >,
    ) -> Self {
        Subject {
            name: value.name.clone(),
            subject_group_handle: value.subject_group_id.into(),
            incompat_handle: value.incompat_id.map(|x| x.into()),
            group_list_handle: value.group_list_id.map(|x| x.into()),
            duration: value.duration,
            students_per_group_range: (
                *value.students_per_group.start(),
                *value.students_per_group.end(),
            ),
            period: value.period,
            period_is_strict: value.period_is_strict,
            is_tutorial: value.is_tutorial,
            max_groups_per_slot: value.max_groups_per_slot,
            balance_teachers: value.balancing_requirements.teachers,
            balance_timeslots: value.balancing_requirements.timeslots,
        }
    }
}

impl
    From<backend::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>>
    for Subject
{
    fn from(
        value: backend::Subject<
            state::SubjectGroupHandle,
            state::IncompatHandle,
            state::GroupListHandle,
        >,
    ) -> Self {
        Subject::from(&value)
    }
}

impl From<&Subject>
    for backend::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>
{
    fn from(value: &Subject) -> Self {
        backend::Subject {
            name: value.name.clone(),
            subject_group_id: value.subject_group_handle.clone().into(),
            incompat_id: value.incompat_handle.clone().map(|x| x.into()),
            group_list_id: value.group_list_handle.clone().map(|x| x.into()),
            duration: value.duration,
            students_per_group: value.students_per_group_range.0..=value.students_per_group_range.1,
            period: value.period,
            period_is_strict: value.period_is_strict,
            is_tutorial: value.is_tutorial,
            max_groups_per_slot: value.max_groups_per_slot,
            balancing_requirements: backend::BalancingRequirements {
                teachers: value.balance_teachers,
                timeslots: value.balance_timeslots,
            },
        }
    }
}

impl From<Subject>
    for backend::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>
{
    fn from(value: Subject) -> Self {
        backend::Subject::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeSlotHandle {
    pub handle: state::TimeSlotHandle,
}

#[pymethods]
impl TimeSlotHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::TimeSlotHandle> for TimeSlotHandle {
    fn from(value: &state::TimeSlotHandle) -> Self {
        TimeSlotHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::TimeSlotHandle> for TimeSlotHandle {
    fn from(value: state::TimeSlotHandle) -> Self {
        TimeSlotHandle::from(&value)
    }
}

impl From<&TimeSlotHandle> for state::TimeSlotHandle {
    fn from(value: &TimeSlotHandle) -> Self {
        value.handle.clone()
    }
}

impl From<TimeSlotHandle> for state::TimeSlotHandle {
    fn from(value: TimeSlotHandle) -> Self {
        state::TimeSlotHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSlot {
    #[pyo3(set, get)]
    subject_handle: SubjectHandle,
    #[pyo3(set, get)]
    teacher_handle: TeacherHandle,
    #[pyo3(set, get)]
    start: SlotStart,
    #[pyo3(set, get)]
    week_pattern_handle: WeekPatternHandle,
    #[pyo3(set, get)]
    room: String,
}

#[pymethods]
impl TimeSlot {
    #[new]
    fn new(
        subject_handle: SubjectHandle,
        teacher_handle: TeacherHandle,
        week_pattern_handle: WeekPatternHandle,
    ) -> Self {
        TimeSlot {
            subject_handle,
            teacher_handle,
            start: SlotStart {
                day: Weekday::Monday,
                time: Time { hour: 8, minute: 0 },
            },
            week_pattern_handle,
            room: String::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ subject_handle = {:?}, teacher_handle = {:?}, start = {}, week_pattern_handle = {:?}, room = {} }}",
            self_.subject_handle,
            self_.teacher_handle,
            self_.start,
            self_.week_pattern_handle,
            self_.room,
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>>
    for TimeSlot
{
    fn from(
        value: &backend::TimeSlot<
            state::SubjectHandle,
            state::TeacherHandle,
            state::WeekPatternHandle,
        >,
    ) -> Self {
        TimeSlot {
            subject_handle: value.subject_id.clone().into(),
            teacher_handle: value.teacher_id.clone().into(),
            start: value.start.clone().into(),
            week_pattern_handle: value.week_pattern_id.clone().into(),
            room: value.room.clone(),
        }
    }
}

impl From<backend::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>>
    for TimeSlot
{
    fn from(
        value: backend::TimeSlot<
            state::SubjectHandle,
            state::TeacherHandle,
            state::WeekPatternHandle,
        >,
    ) -> Self {
        TimeSlot::from(&value)
    }
}

impl From<&TimeSlot>
    for backend::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>
{
    fn from(value: &TimeSlot) -> Self {
        backend::TimeSlot {
            subject_id: value.subject_handle.clone().into(),
            teacher_id: value.teacher_handle.clone().into(),
            start: value.start.clone().into(),
            week_pattern_id: value.week_pattern_handle.clone().into(),
            room: value.room.clone(),
        }
    }
}

impl From<TimeSlot>
    for backend::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>
{
    fn from(value: TimeSlot) -> Self {
        backend::TimeSlot::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupingHandle {
    pub handle: state::GroupingHandle,
}

#[pymethods]
impl GroupingHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::GroupingHandle> for GroupingHandle {
    fn from(value: &state::GroupingHandle) -> Self {
        GroupingHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::GroupingHandle> for GroupingHandle {
    fn from(value: state::GroupingHandle) -> Self {
        GroupingHandle::from(&value)
    }
}

impl From<&GroupingHandle> for state::GroupingHandle {
    fn from(value: &GroupingHandle) -> Self {
        value.handle.clone()
    }
}

impl From<GroupingHandle> for state::GroupingHandle {
    fn from(value: GroupingHandle) -> Self {
        state::GroupingHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grouping {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    slots: BTreeSet<TimeSlotHandle>,
}

#[pymethods]
impl Grouping {
    #[new]
    fn new(name: String) -> Self {
        Grouping {
            name,
            slots: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let slots_strings: Vec<_> = self_.slots.iter().map(|x| format!("{:?}", x)).collect();

        let output = format!(
            "{{ name = {}, slots = {{ {} }} }}",
            self_.name,
            slots_strings.join(","),
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::Grouping<state::TimeSlotHandle>> for Grouping {
    fn from(value: &backend::Grouping<state::TimeSlotHandle>) -> Self {
        Grouping {
            name: value.name.clone(),
            slots: value.slots.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<backend::Grouping<state::TimeSlotHandle>> for Grouping {
    fn from(value: backend::Grouping<state::TimeSlotHandle>) -> Self {
        Grouping::from(&value)
    }
}

impl From<&Grouping> for backend::Grouping<state::TimeSlotHandle> {
    fn from(value: &Grouping) -> Self {
        backend::Grouping {
            name: value.name.clone(),
            slots: value.slots.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<Grouping> for backend::Grouping<state::TimeSlotHandle> {
    fn from(value: Grouping) -> Self {
        backend::Grouping::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupingIncompatHandle {
    pub handle: state::GroupingIncompatHandle,
}

#[pymethods]
impl GroupingIncompatHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::GroupingIncompatHandle> for GroupingIncompatHandle {
    fn from(value: &state::GroupingIncompatHandle) -> Self {
        GroupingIncompatHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::GroupingIncompatHandle> for GroupingIncompatHandle {
    fn from(value: state::GroupingIncompatHandle) -> Self {
        GroupingIncompatHandle::from(&value)
    }
}

impl From<&GroupingIncompatHandle> for state::GroupingIncompatHandle {
    fn from(value: &GroupingIncompatHandle) -> Self {
        value.handle.clone()
    }
}

impl From<GroupingIncompatHandle> for state::GroupingIncompatHandle {
    fn from(value: GroupingIncompatHandle) -> Self {
        state::GroupingIncompatHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupingIncompat {
    #[pyo3(set, get)]
    max_count: NonZeroUsize,
    #[pyo3(set, get)]
    groupings: BTreeSet<GroupingHandle>,
}

#[pymethods]
impl GroupingIncompat {
    #[new]
    fn new(max_count: NonZeroUsize) -> Self {
        GroupingIncompat {
            max_count,
            groupings: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let groupings_strings: Vec<_> =
            self_.groupings.iter().map(|x| format!("{:?}", x)).collect();

        let output = format!(
            "{{ max_count = {}, groupings = {{ {} }} }}",
            self_.max_count.get(),
            groupings_strings.join(","),
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&backend::GroupingIncompat<state::GroupingHandle>> for GroupingIncompat {
    fn from(value: &backend::GroupingIncompat<state::GroupingHandle>) -> Self {
        GroupingIncompat {
            max_count: value.max_count,
            groupings: value.groupings.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<backend::GroupingIncompat<state::GroupingHandle>> for GroupingIncompat {
    fn from(value: backend::GroupingIncompat<state::GroupingHandle>) -> Self {
        GroupingIncompat::from(&value)
    }
}

impl From<&GroupingIncompat> for backend::GroupingIncompat<state::GroupingHandle> {
    fn from(value: &GroupingIncompat) -> Self {
        backend::GroupingIncompat {
            max_count: value.max_count,
            groupings: value.groupings.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<GroupingIncompat> for backend::GroupingIncompat<state::GroupingHandle> {
    fn from(value: GroupingIncompat) -> Self {
        backend::GroupingIncompat::from(&value)
    }
}
