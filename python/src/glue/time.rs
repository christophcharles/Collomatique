use super::*;

use std::num::NonZeroU32;

use collomatique_time::NonZeroMinutes;
use pyo3::{exceptions::PyValueError, types::PyString};

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct NaiveMondayDate {
    internal: collomatique_time::WeekStart,
}

#[pymethods]
impl NaiveMondayDate {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{}", self_.internal.format("%Y-%m-%d"));
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(year: i32, month: u32, day: u32) -> PyResult<Self> {
        let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
            return Err(PyValueError::new_err(format!("Invalid date")));
        };

        let Some(internal) = collomatique_time::WeekStart::new(date) else {
            return Err(PyValueError::new_err(format!("Not a monday")));
        };

        Ok(NaiveMondayDate { internal })
    }
}

impl From<collomatique_time::WeekStart> for NaiveMondayDate {
    fn from(value: collomatique_time::WeekStart) -> Self {
        NaiveMondayDate { internal: value }
    }
}

impl From<NaiveMondayDate> for collomatique_time::WeekStart {
    fn from(value: NaiveMondayDate) -> Self {
        value.internal
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct NaiveDate {
    internal: chrono::NaiveDate,
}

#[pymethods]
impl NaiveDate {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{}", self_.internal.format("%Y-%m-%d"));
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(year: i32, month: u32, day: u32) -> PyResult<Self> {
        let Some(internal) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
            return Err(PyValueError::new_err(format!("Invalid date")));
        };

        Ok(NaiveDate { internal })
    }

    fn round_to_week(self_: PyRef<'_, Self>) -> NaiveMondayDate {
        NaiveMondayDate {
            internal: collomatique_time::WeekStart::round_from(self_.internal),
        }
    }
}

impl From<chrono::NaiveDate> for NaiveDate {
    fn from(value: chrono::NaiveDate) -> Self {
        NaiveDate { internal: value }
    }
}

impl From<NaiveDate> for chrono::NaiveDate {
    fn from(value: NaiveDate) -> Self {
        value.internal
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct Time {
    internal: collomatique_time::WholeMinuteTime,
}

#[pymethods]
impl Time {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", self_.internal);
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(h: u32, m: u32) -> PyResult<Self> {
        let Some(time) = chrono::NaiveTime::from_hms_milli_opt(h, m, 0, 0) else {
            return Err(PyValueError::new_err(format!("Invalid time")));
        };

        let internal =
            collomatique_time::WholeMinuteTime::new(time).expect("Time should be on minute");
        Ok(Time { internal })
    }
}

impl From<collomatique_time::WholeMinuteTime> for Time {
    fn from(value: collomatique_time::WholeMinuteTime) -> Self {
        Time { internal: value }
    }
}

impl From<Time> for collomatique_time::WholeMinuteTime {
    fn from(value: Time) -> Self {
        value.internal
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[pymethods]
impl Weekday {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_time::Weekday> for Weekday {
    fn from(value: collomatique_time::Weekday) -> Self {
        match value.0 {
            chrono::Weekday::Mon => Weekday::Monday,
            chrono::Weekday::Tue => Weekday::Tuesday,
            chrono::Weekday::Wed => Weekday::Wednesday,
            chrono::Weekday::Thu => Weekday::Thursday,
            chrono::Weekday::Fri => Weekday::Friday,
            chrono::Weekday::Sat => Weekday::Saturday,
            chrono::Weekday::Sun => Weekday::Sunday,
        }
    }
}

impl From<Weekday> for collomatique_time::Weekday {
    fn from(value: Weekday) -> Self {
        collomatique_time::Weekday(match value {
            Weekday::Monday => chrono::Weekday::Mon,
            Weekday::Tuesday => chrono::Weekday::Tue,
            Weekday::Wednesday => chrono::Weekday::Wed,
            Weekday::Thursday => chrono::Weekday::Thu,
            Weekday::Friday => chrono::Weekday::Fri,
            Weekday::Saturday => chrono::Weekday::Sat,
            Weekday::Sunday => chrono::Weekday::Sun,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct SlotStart {
    #[pyo3(set, get)]
    pub start_time: Time,
    #[pyo3(set, get)]
    pub weekday: Weekday,
}

#[pymethods]
impl SlotStart {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", self_);
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(weekday: Weekday, start_time: Time) -> Self {
        SlotStart {
            start_time,
            weekday,
        }
    }
}

impl From<collomatique_time::SlotStart> for SlotStart {
    fn from(value: collomatique_time::SlotStart) -> Self {
        SlotStart {
            weekday: value.weekday.into(),
            start_time: value.start_time.into(),
        }
    }
}

impl From<SlotStart> for collomatique_time::SlotStart {
    fn from(value: SlotStart) -> Self {
        collomatique_time::SlotStart {
            weekday: value.weekday.into(),
            start_time: value.start_time.internal,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct SlotWithDuration {
    #[pyo3(set, get)]
    pub start_time: SlotStart,
    #[pyo3(set, get)]
    pub duration_in_minutes: NonZeroU32,
}

#[pymethods]
impl SlotWithDuration {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", self_);
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(start_time: SlotStart, duration_in_minutes: NonZeroU32) -> Self {
        SlotWithDuration {
            start_time,
            duration_in_minutes,
        }
    }
}

impl From<collomatique_time::SlotWithDuration> for SlotWithDuration {
    fn from(value: collomatique_time::SlotWithDuration) -> Self {
        SlotWithDuration {
            start_time: value.start().clone().into(),
            duration_in_minutes: value.duration().get(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlotWithDurationError {
    SlotOverlapsWithNextDay,
}

impl TryFrom<SlotWithDuration> for collomatique_time::SlotWithDuration {
    type Error = SlotWithDurationError;
    fn try_from(value: SlotWithDuration) -> Result<Self, Self::Error> {
        let slot_start = collomatique_time::SlotStart {
            weekday: value.start_time.weekday.into(),
            start_time: value.start_time.start_time.into(),
        };
        let duration = NonZeroMinutes::from(value.duration_in_minutes);
        value.duration_in_minutes;
        collomatique_time::SlotWithDuration::new(slot_start, duration)
            .ok_or(SlotWithDurationError::SlotOverlapsWithNextDay)
    }
}
