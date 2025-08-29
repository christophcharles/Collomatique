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
    #[pyo3(get, set)]
    periodicity_cuts: BTreeSet<NonZeroU32>,
    #[pyo3(get, set)]
    max_interrogations_per_day_for_single_student_cost: i32,
    #[pyo3(get, set)]
    max_interrogations_per_day_for_all_students_cost: i32,
    #[pyo3(get, set)]
    interrogations_per_week_range_for_single_student_cost: i32,
    #[pyo3(get, set)]
    interrogations_per_week_range_for_all_students_cost: i32,
    #[pyo3(get, set)]
    balancing_cost: i32,
    #[pyo3(get, set)]
    consecutive_slots_cost: i32,
}

#[pymethods]
impl GeneralData {
    #[new]
    fn new(week_count: NonZeroU32) -> Self {
        GeneralData {
            interrogations_per_week_range: None,
            max_interrogations_per_day: None,
            week_count,
            periodicity_cuts: BTreeSet::new(),
            max_interrogations_per_day_for_single_student_cost: 1,
            max_interrogations_per_day_for_all_students_cost: 1,
            interrogations_per_week_range_for_single_student_cost: 1,
            interrogations_per_week_range_for_all_students_cost: 1,
            balancing_cost: 1,
            consecutive_slots_cost: 1,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let periodicity_cuts_strings: Vec<_> = self_
            .periodicity_cuts
            .iter()
            .map(|x| x.to_string())
            .collect();
        let output = format!(
            "{{ interrogations_per_week_range = {}, max_interrogations_per_day = {}, week_count = {}, periodicity_cuts = [{}], max_interrogations_per_day_for_single_student_cost = {}, max_interrogations_per_day_for_all_students_cost = {}, interrogations_per_week_range_for_single_student_cost = {}, interrogations_per_week_range_for_all_students_cost = {}, balancing_cost = {}, consecutive_slots_cost = {} }}",
            match self_.interrogations_per_week_range {
                Some(val) => format!("{}..{}", val.0, val.1 as i64),
                None => String::from("none"),
            },
            match self_.max_interrogations_per_day {
                Some(val) => val.to_string(),
                None => String::from("none"),
            },
            self_.week_count,
            periodicity_cuts_strings.join(","),
            self_.max_interrogations_per_day_for_single_student_cost,
            self_.max_interrogations_per_day_for_all_students_cost,
            self_.interrogations_per_week_range_for_single_student_cost,
            self_.interrogations_per_week_range_for_all_students_cost,
            self_.balancing_cost,
            self_.consecutive_slots_cost,
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&json::GeneralData> for GeneralData {
    fn from(value: &json::GeneralData) -> Self {
        GeneralData {
            interrogations_per_week_range: value
                .interrogations_per_week
                .clone()
                .map(|range| (range.start, range.end)),
            max_interrogations_per_day: value.max_interrogations_per_day,
            week_count: value.week_count,
            periodicity_cuts: value.periodicity_cuts.clone(),
            max_interrogations_per_day_for_single_student_cost: value
                .costs_adjustments
                .max_interrogations_per_day_for_single_student,
            max_interrogations_per_day_for_all_students_cost: value
                .costs_adjustments
                .max_interrogations_per_day_for_all_students,
            interrogations_per_week_range_for_single_student_cost: value
                .costs_adjustments
                .interrogations_per_week_range_for_single_student,
            interrogations_per_week_range_for_all_students_cost: value
                .costs_adjustments
                .interrogations_per_week_range_for_all_students,
            balancing_cost: value.costs_adjustments.balancing,
            consecutive_slots_cost: value.costs_adjustments.consecutive_slots,
        }
    }
}

impl From<json::GeneralData> for GeneralData {
    fn from(value: json::GeneralData) -> Self {
        GeneralData::from(&value)
    }
}

impl From<&GeneralData> for json::GeneralData {
    fn from(value: &GeneralData) -> Self {
        json::GeneralData {
            interrogations_per_week: value
                .interrogations_per_week_range
                .map(|tuple| tuple.0..tuple.1),
            max_interrogations_per_day: value.max_interrogations_per_day,
            week_count: value.week_count,
            periodicity_cuts: value.periodicity_cuts.clone(),
            costs_adjustments: json::CostsAdjustments {
                max_interrogations_per_day_for_single_student: value
                    .max_interrogations_per_day_for_single_student_cost,
                max_interrogations_per_day_for_all_students: value
                    .max_interrogations_per_day_for_all_students_cost,
                interrogations_per_week_range_for_single_student: value
                    .interrogations_per_week_range_for_single_student_cost,
                interrogations_per_week_range_for_all_students: value
                    .interrogations_per_week_range_for_all_students_cost,
                balancing: value.balancing_cost,
                consecutive_slots: value.consecutive_slots_cost,
            },
        }
    }
}

impl From<GeneralData> for json::GeneralData {
    fn from(value: GeneralData) -> Self {
        json::GeneralData::from(&value)
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

impl From<&json::WeekPattern> for WeekPattern {
    fn from(value: &json::WeekPattern) -> Self {
        WeekPattern {
            name: value.name.clone(),
            weeks: value.weeks.iter().map(|w| w.get()).collect(),
        }
    }
}

impl From<json::WeekPattern> for WeekPattern {
    fn from(value: json::WeekPattern) -> Self {
        WeekPattern::from(&value)
    }
}

impl From<&WeekPattern> for json::WeekPattern {
    fn from(value: &WeekPattern) -> Self {
        json::WeekPattern {
            name: value.name.clone(),
            weeks: value.weeks.iter().map(|x| json::Week::new(*x)).collect(),
        }
    }
}

impl From<WeekPattern> for json::WeekPattern {
    fn from(value: WeekPattern) -> Self {
        json::WeekPattern::from(&value)
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

impl From<&json::Teacher> for Teacher {
    fn from(value: &json::Teacher) -> Self {
        Teacher {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            contact: value.contact.clone(),
        }
    }
}

impl From<json::Teacher> for Teacher {
    fn from(value: json::Teacher) -> Self {
        Teacher::from(&value)
    }
}

impl From<&Teacher> for json::Teacher {
    fn from(value: &Teacher) -> Self {
        json::Teacher {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            contact: value.contact.clone(),
        }
    }
}

impl From<Teacher> for json::Teacher {
    fn from(value: Teacher) -> Self {
        json::Teacher::from(&value)
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
    #[pyo3(set, get)]
    no_consecutive_slots: bool,
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
            no_consecutive_slots: false,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ surname = {}, firstname = {}, email = {}, phone = {}, no_consecutive_slots = {} }}",
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
            self_.no_consecutive_slots,
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&json::Student> for Student {
    fn from(value: &json::Student) -> Self {
        Student {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            email: value.email.clone(),
            phone: value.phone.clone(),
            no_consecutive_slots: value.no_consecutive_slots,
        }
    }
}

impl From<json::Student> for Student {
    fn from(value: json::Student) -> Self {
        Student::from(&value)
    }
}

impl From<&Student> for json::Student {
    fn from(value: &Student) -> Self {
        json::Student {
            surname: value.surname.clone(),
            firstname: value.firstname.clone(),
            email: value.email.clone(),
            phone: value.phone.clone(),
            no_consecutive_slots: value.no_consecutive_slots,
        }
    }
}

impl From<Student> for json::Student {
    fn from(value: Student) -> Self {
        json::Student::from(&value)
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

impl From<&json::SubjectGroup> for SubjectGroup {
    fn from(value: &json::SubjectGroup) -> Self {
        SubjectGroup {
            name: value.name.clone(),
            optional: value.optional,
        }
    }
}

impl From<json::SubjectGroup> for SubjectGroup {
    fn from(value: json::SubjectGroup) -> Self {
        SubjectGroup::from(&value)
    }
}

impl From<&SubjectGroup> for json::SubjectGroup {
    fn from(value: &SubjectGroup) -> Self {
        json::SubjectGroup {
            name: value.name.clone(),
            optional: value.optional,
        }
    }
}

impl From<SubjectGroup> for json::SubjectGroup {
    fn from(value: SubjectGroup) -> Self {
        json::SubjectGroup::from(&value)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl From<&json::SlotStart> for SlotStart {
    fn from(value: &json::SlotStart) -> Self {
        SlotStart {
            day: value.day.into(),
            time: value.time.clone().into(),
        }
    }
}

impl From<json::SlotStart> for SlotStart {
    fn from(value: json::SlotStart) -> Self {
        SlotStart::from(&value)
    }
}

impl From<&SlotStart> for json::SlotStart {
    fn from(value: &SlotStart) -> Self {
        json::SlotStart {
            day: value.day.into(),
            time: value.time.clone().into(),
        }
    }
}

impl From<SlotStart> for json::SlotStart {
    fn from(value: SlotStart) -> Self {
        json::SlotStart::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncompatSlot {
    #[pyo3(get)]
    week_pattern_handle: WeekPatternHandle,
    #[pyo3(get)]
    start: SlotStart,
    #[pyo3(get)]
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

impl From<&json::IncompatSlot<state::WeekPatternHandle>> for IncompatSlot {
    fn from(value: &json::IncompatSlot<state::WeekPatternHandle>) -> Self {
        IncompatSlot {
            week_pattern_handle: value.week_pattern_id.into(),
            start: value.start.clone().into(),
            duration: value.duration,
        }
    }
}

impl From<json::IncompatSlot<state::WeekPatternHandle>> for IncompatSlot {
    fn from(value: json::IncompatSlot<state::WeekPatternHandle>) -> Self {
        IncompatSlot::from(&value)
    }
}

impl From<&IncompatSlot> for json::IncompatSlot<state::WeekPatternHandle> {
    fn from(value: &IncompatSlot) -> Self {
        json::IncompatSlot {
            week_pattern_id: value.week_pattern_handle.clone().into(),
            start: value.start.clone().into(),
            duration: value.duration,
        }
    }
}

impl From<IncompatSlot> for json::IncompatSlot<state::WeekPatternHandle> {
    fn from(value: IncompatSlot) -> Self {
        json::IncompatSlot::from(&value)
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
    groups: Vec<BTreeSet<IncompatSlot>>,
}

#[pymethods]
impl Incompat {
    #[new]
    fn new(name: String) -> Self {
        Incompat {
            name,
            max_count: 0,
            groups: Vec::new(),
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

impl From<&json::Incompat<state::WeekPatternHandle>> for Incompat {
    fn from(value: &json::Incompat<state::WeekPatternHandle>) -> Self {
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

impl From<json::Incompat<state::WeekPatternHandle>> for Incompat {
    fn from(value: json::Incompat<state::WeekPatternHandle>) -> Self {
        Incompat::from(&value)
    }
}

impl From<&Incompat> for json::Incompat<state::WeekPatternHandle> {
    fn from(value: &Incompat) -> Self {
        json::Incompat {
            name: value.name.clone(),
            max_count: value.max_count,
            groups: value
                .groups
                .iter()
                .map(|x| json::IncompatGroup {
                    slots: x.iter().map(|y| y.into()).collect(),
                })
                .collect(),
        }
    }
}

impl From<Incompat> for json::Incompat<state::WeekPatternHandle> {
    fn from(value: Incompat) -> Self {
        json::Incompat::from(&value)
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

impl From<&json::Group> for Group {
    fn from(value: &json::Group) -> Self {
        Group {
            name: value.name.clone(),
            extendable: value.extendable,
        }
    }
}

impl From<json::Group> for Group {
    fn from(value: json::Group) -> Self {
        Group::from(&value)
    }
}

impl From<&Group> for json::Group {
    fn from(value: &Group) -> Self {
        json::Group {
            name: value.name.clone(),
            extendable: value.extendable,
        }
    }
}

impl From<Group> for json::Group {
    fn from(value: Group) -> Self {
        json::Group::from(&value)
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

impl From<&json::GroupList<state::StudentHandle>> for GroupList {
    fn from(value: &json::GroupList<state::StudentHandle>) -> Self {
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

impl From<json::GroupList<state::StudentHandle>> for GroupList {
    fn from(value: json::GroupList<state::StudentHandle>) -> Self {
        GroupList::from(&value)
    }
}

impl From<&GroupList> for json::GroupList<state::StudentHandle> {
    fn from(value: &GroupList) -> Self {
        json::GroupList {
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

impl From<GroupList> for json::GroupList<state::StudentHandle> {
    fn from(value: GroupList) -> Self {
        json::GroupList::from(&value)
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

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BalancingConstraints {
    OptimizeOnly,
    OverallOnly,
    StrictWithCuts,
    StrictWithCutsAndOverall,
    Strict,
    OptimizeAndConsecutiveDifferentTeachers,
    OverallAndConsecutiveDifferentTeachers,
    StrictWithCutsAndConsecutiveDifferentTeachers,
    StrictWithCutsAndOverallAndConsecutiveDifferentTeachers,
}

impl std::fmt::Display for BalancingConstraints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                BalancingConstraints::OptimizeOnly => "OptimizeOnly",
                BalancingConstraints::OverallOnly => "OverallOnly",
                BalancingConstraints::StrictWithCuts => "StrictWithCuts",
                BalancingConstraints::StrictWithCutsAndOverall => "StrictWithCutsAndOverall",
                BalancingConstraints::Strict => "Strict",
                BalancingConstraints::OptimizeAndConsecutiveDifferentTeachers =>
                    "OptimizeAndConsecutiveDifferentTeachers",
                BalancingConstraints::OverallAndConsecutiveDifferentTeachers =>
                    "OverallAndConsecutiveDifferentTeachers",
                BalancingConstraints::StrictWithCutsAndConsecutiveDifferentTeachers =>
                    "StrictWithCutsAndConsecutiveDifferentTeachers",
                BalancingConstraints::StrictWithCutsAndOverallAndConsecutiveDifferentTeachers =>
                    "StrictWithCutsAndOverallAndConsecutiveDifferentTeachers",
            }
        )
    }
}

#[pymethods]
impl BalancingConstraints {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = self_.to_string();

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&crate::json::BalancingConstraints> for BalancingConstraints {
    fn from(value: &crate::json::BalancingConstraints) -> Self {
        use crate::json::BalancingConstraints as BC;
        match value {
            BC::OptimizeOnly => BalancingConstraints::OptimizeOnly,
            BC::OverallOnly => BalancingConstraints::OverallOnly,
            BC::StrictWithCuts => BalancingConstraints::StrictWithCuts,
            BC::StrictWithCutsAndOverall => BalancingConstraints::StrictWithCutsAndOverall,
            BC::Strict => BalancingConstraints::Strict,
            BC::OptimizeAndConsecutiveDifferentTeachers => {
                BalancingConstraints::OptimizeAndConsecutiveDifferentTeachers
            }
            BC::OverallAndConsecutiveDifferentTeachers => {
                BalancingConstraints::OverallAndConsecutiveDifferentTeachers
            }
            BC::StrictWithCutsAndConsecutiveDifferentTeachers => {
                BalancingConstraints::StrictWithCutsAndConsecutiveDifferentTeachers
            }
            BC::StrictWithCutsAndOverallAndConsecutiveDifferentTeachers => {
                BalancingConstraints::StrictWithCutsAndOverallAndConsecutiveDifferentTeachers
            }
        }
    }
}

impl From<crate::json::BalancingConstraints> for BalancingConstraints {
    fn from(value: crate::json::BalancingConstraints) -> Self {
        BalancingConstraints::from(&value)
    }
}

impl From<&BalancingConstraints> for crate::json::BalancingConstraints {
    fn from(value: &BalancingConstraints) -> Self {
        use crate::json::BalancingConstraints as BC;
        match value {
            BalancingConstraints::OptimizeOnly => BC::OptimizeOnly,
            BalancingConstraints::OverallOnly => BC::OverallOnly,
            BalancingConstraints::StrictWithCuts => BC::StrictWithCuts,
            BalancingConstraints::StrictWithCutsAndOverall => BC::StrictWithCutsAndOverall,
            BalancingConstraints::Strict => BC::Strict,
            BalancingConstraints::OptimizeAndConsecutiveDifferentTeachers => {
                BC::OptimizeAndConsecutiveDifferentTeachers
            }
            BalancingConstraints::OverallAndConsecutiveDifferentTeachers => {
                BC::OverallAndConsecutiveDifferentTeachers
            }
            BalancingConstraints::StrictWithCutsAndConsecutiveDifferentTeachers => {
                BC::StrictWithCutsAndConsecutiveDifferentTeachers
            }
            BalancingConstraints::StrictWithCutsAndOverallAndConsecutiveDifferentTeachers => {
                BC::StrictWithCutsAndOverallAndConsecutiveDifferentTeachers
            }
        }
    }
}

impl From<BalancingConstraints> for crate::json::BalancingConstraints {
    fn from(value: BalancingConstraints) -> Self {
        crate::json::BalancingConstraints::from(&value)
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BalancingSlotSelections {
    TeachersAndTimeSlots,
    Teachers,
    TimeSlots,
    Manual,
}

impl std::fmt::Display for BalancingSlotSelections {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                BalancingSlotSelections::Manual => "Manual",
                BalancingSlotSelections::Teachers => "Teachers",
                BalancingSlotSelections::TimeSlots => "TimeSlots",
                BalancingSlotSelections::TeachersAndTimeSlots => "TeachersAndTimeSlots",
            }
        )
    }
}

#[pymethods]
impl BalancingSlotSelections {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = self_.to_string();

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&crate::json::BalancingSlotSelections> for BalancingSlotSelections {
    fn from(value: &crate::json::BalancingSlotSelections) -> Self {
        use crate::json::BalancingSlotSelections as BSS;
        match value {
            BSS::Manual => BalancingSlotSelections::Manual,
            BSS::Teachers => BalancingSlotSelections::Teachers,
            BSS::TimeSlots => BalancingSlotSelections::TimeSlots,
            BSS::TeachersAndTimeSlots => BalancingSlotSelections::TeachersAndTimeSlots,
        }
    }
}

impl From<crate::json::BalancingSlotSelections> for BalancingSlotSelections {
    fn from(value: crate::json::BalancingSlotSelections) -> Self {
        BalancingSlotSelections::from(&value)
    }
}

impl From<&BalancingSlotSelections> for crate::json::BalancingSlotSelections {
    fn from(value: &BalancingSlotSelections) -> Self {
        use crate::json::BalancingSlotSelections as BSS;
        match value {
            BalancingSlotSelections::Manual => BSS::Manual,
            BalancingSlotSelections::Teachers => BSS::Teachers,
            BalancingSlotSelections::TimeSlots => BSS::TimeSlots,
            BalancingSlotSelections::TeachersAndTimeSlots => BSS::TeachersAndTimeSlots,
        }
    }
}

impl From<BalancingSlotSelections> for crate::json::BalancingSlotSelections {
    fn from(value: BalancingSlotSelections) -> Self {
        crate::json::BalancingSlotSelections::from(&value)
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
    balancing_constraints: BalancingConstraints,
    #[pyo3(set, get)]
    balancing_slot_selections: BalancingSlotSelections,
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
            balancing_constraints: BalancingConstraints::OptimizeOnly,
            balancing_slot_selections: BalancingSlotSelections::TeachersAndTimeSlots,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ name = {}, subject_group_handle = {:?}, incompat_handle = {}, group_list_handle = {}, duration = {}, students_per_group_range = {}..={}, period = {}, period_is_strict = {}, is_tutorial = {}, max_groups_per_slot = {}, balancing_constraints = {}, balancing_slot_selections = {} }}",
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
            self_.balancing_constraints,
            self_.balancing_slot_selections,

        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&json::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>>
    for Subject
{
    fn from(
        value: &json::Subject<
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
            balancing_constraints: value.balancing_requirements.constraints.into(),
            balancing_slot_selections: value.balancing_requirements.slot_selections.into(),
        }
    }
}

impl From<json::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>>
    for Subject
{
    fn from(
        value: json::Subject<
            state::SubjectGroupHandle,
            state::IncompatHandle,
            state::GroupListHandle,
        >,
    ) -> Self {
        Subject::from(&value)
    }
}

impl From<&Subject>
    for json::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>
{
    fn from(value: &Subject) -> Self {
        json::Subject {
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
            balancing_requirements: json::BalancingRequirements {
                constraints: value.balancing_constraints.into(),
                slot_selections: value.balancing_slot_selections.into(),
            },
        }
    }
}

impl From<Subject>
    for json::Subject<state::SubjectGroupHandle, state::IncompatHandle, state::GroupListHandle>
{
    fn from(value: Subject) -> Self {
        json::Subject::from(&value)
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
    #[pyo3(set, get)]
    cost: u32,
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
            cost: 0,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!(
            "{{ subject_handle = {:?}, teacher_handle = {:?}, start = {}, week_pattern_handle = {:?}, room = {}, cost = {} }}",
            self_.subject_handle,
            self_.teacher_handle,
            self_.start,
            self_.week_pattern_handle,
            self_.room,
            self_.cost,
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&json::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>>
    for TimeSlot
{
    fn from(
        value: &json::TimeSlot<
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
            cost: value.cost,
        }
    }
}

impl From<json::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>>
    for TimeSlot
{
    fn from(
        value: json::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>,
    ) -> Self {
        TimeSlot::from(&value)
    }
}

impl From<&TimeSlot>
    for json::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>
{
    fn from(value: &TimeSlot) -> Self {
        json::TimeSlot {
            subject_id: value.subject_handle.clone().into(),
            teacher_id: value.teacher_handle.clone().into(),
            start: value.start.clone().into(),
            week_pattern_id: value.week_pattern_handle.clone().into(),
            room: value.room.clone(),
            cost: value.cost,
        }
    }
}

impl From<TimeSlot>
    for json::TimeSlot<state::SubjectHandle, state::TeacherHandle, state::WeekPatternHandle>
{
    fn from(value: TimeSlot) -> Self {
        json::TimeSlot::from(&value)
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

impl From<&json::Grouping<state::TimeSlotHandle>> for Grouping {
    fn from(value: &json::Grouping<state::TimeSlotHandle>) -> Self {
        Grouping {
            name: value.name.clone(),
            slots: value.slots.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<json::Grouping<state::TimeSlotHandle>> for Grouping {
    fn from(value: json::Grouping<state::TimeSlotHandle>) -> Self {
        Grouping::from(&value)
    }
}

impl From<&Grouping> for json::Grouping<state::TimeSlotHandle> {
    fn from(value: &Grouping) -> Self {
        json::Grouping {
            name: value.name.clone(),
            slots: value.slots.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<Grouping> for json::Grouping<state::TimeSlotHandle> {
    fn from(value: Grouping) -> Self {
        json::Grouping::from(&value)
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

impl From<&json::GroupingIncompat<state::GroupingHandle>> for GroupingIncompat {
    fn from(value: &json::GroupingIncompat<state::GroupingHandle>) -> Self {
        GroupingIncompat {
            max_count: value.max_count,
            groupings: value.groupings.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<json::GroupingIncompat<state::GroupingHandle>> for GroupingIncompat {
    fn from(value: json::GroupingIncompat<state::GroupingHandle>) -> Self {
        GroupingIncompat::from(&value)
    }
}

impl From<&GroupingIncompat> for json::GroupingIncompat<state::GroupingHandle> {
    fn from(value: &GroupingIncompat) -> Self {
        json::GroupingIncompat {
            max_count: value.max_count,
            groupings: value.groupings.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<GroupingIncompat> for json::GroupingIncompat<state::GroupingHandle> {
    fn from(value: GroupingIncompat) -> Self {
        json::GroupingIncompat::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotGroup {
    #[pyo3(set, get)]
    count: usize,
    #[pyo3(set, get)]
    slots: BTreeSet<TimeSlotHandle>,
}

#[pymethods]
impl SlotGroup {
    #[new]
    fn new(count: usize) -> Self {
        SlotGroup {
            count,
            slots: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let slots_strings: Vec<_> = self_.slots.iter().map(|x| format!("{:?}", x)).collect();

        let output = format!(
            "{{ count = {}, slots = {{ {} }} }}",
            self_.count,
            slots_strings.join(","),
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&json::SlotGroup<state::TimeSlotHandle>> for SlotGroup {
    fn from(value: &json::SlotGroup<state::TimeSlotHandle>) -> Self {
        SlotGroup {
            count: value.count,
            slots: value.slots.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<json::SlotGroup<state::TimeSlotHandle>> for SlotGroup {
    fn from(value: json::SlotGroup<state::TimeSlotHandle>) -> Self {
        SlotGroup::from(&value)
    }
}

impl From<&SlotGroup> for json::SlotGroup<state::TimeSlotHandle> {
    fn from(value: &SlotGroup) -> Self {
        json::SlotGroup {
            count: value.count,
            slots: value.slots.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<SlotGroup> for json::SlotGroup<state::TimeSlotHandle> {
    fn from(value: SlotGroup) -> Self {
        json::SlotGroup::from(&value)
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotSelectionHandle {
    pub handle: state::SlotSelectionHandle,
}

#[pymethods]
impl SlotSelectionHandle {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&state::SlotSelectionHandle> for SlotSelectionHandle {
    fn from(value: &state::SlotSelectionHandle) -> Self {
        SlotSelectionHandle {
            handle: value.clone(),
        }
    }
}

impl From<state::SlotSelectionHandle> for SlotSelectionHandle {
    fn from(value: state::SlotSelectionHandle) -> Self {
        SlotSelectionHandle::from(&value)
    }
}

impl From<&SlotSelectionHandle> for state::SlotSelectionHandle {
    fn from(value: &SlotSelectionHandle) -> Self {
        value.handle.clone()
    }
}

impl From<SlotSelectionHandle> for state::SlotSelectionHandle {
    fn from(value: SlotSelectionHandle) -> Self {
        state::SlotSelectionHandle::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotSelection {
    #[pyo3(set, get)]
    subject_handle: SubjectHandle,
    #[pyo3(set, get)]
    slot_groups: Vec<SlotGroup>,
}

#[pymethods]
impl SlotSelection {
    #[new]
    fn new(subject_handle: SubjectHandle) -> Self {
        SlotSelection {
            subject_handle,
            slot_groups: Vec::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let slot_groups_strings: Vec<_> = self_
            .slot_groups
            .iter()
            .map(|x| format!("{:?}", x))
            .collect();

        let output = format!(
            "{{ subject_handle = {:?}, slot_groups = {{ {} }} }}",
            self_.subject_handle,
            slot_groups_strings.join(","),
        );

        PyString::new_bound(self_.py(), output.as_str())
    }
}

impl From<&json::SlotSelection<state::SubjectHandle, state::TimeSlotHandle>> for SlotSelection {
    fn from(value: &json::SlotSelection<state::SubjectHandle, state::TimeSlotHandle>) -> Self {
        SlotSelection {
            subject_handle: value.subject_id.into(),
            slot_groups: value.slot_groups.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<json::SlotSelection<state::SubjectHandle, state::TimeSlotHandle>> for SlotSelection {
    fn from(value: json::SlotSelection<state::SubjectHandle, state::TimeSlotHandle>) -> Self {
        SlotSelection::from(&value)
    }
}

impl From<&SlotSelection> for json::SlotSelection<state::SubjectHandle, state::TimeSlotHandle> {
    fn from(value: &SlotSelection) -> Self {
        json::SlotSelection {
            subject_id: (&value.subject_handle).into(),
            slot_groups: value.slot_groups.iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<SlotSelection> for json::SlotSelection<state::SubjectHandle, state::TimeSlotHandle> {
    fn from(value: SlotSelection) -> Self {
        json::SlotSelection::from(&value)
    }
}
