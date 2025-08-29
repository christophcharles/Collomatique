//! Time related types
//!
//! This crate defines a few useful types for handling time in
//! the context of colloscopes.

#[cfg(test)]
mod tests;

use std::num::NonZeroU32;

/// DurationInMinutes obviously represents a duration in minutes.
///
/// The type is useful because it gives information on the type of data
/// represented. Also, it garantees that the underlying duration is
/// at least one minute.
///
/// Crucially all durations have always positive span.
///
/// On top of that it implements a few utility functions to interact with chrono.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonZeroDurationInMinutes(NonZeroU32);

impl NonZeroDurationInMinutes {
    /// Creates a new [NonZeroDurationInMinutes] from a number of minutes
    ///
    /// Returns `None` if the value is zero.
    pub fn new(value: u32) -> Option<NonZeroDurationInMinutes> {
        Some(NonZeroDurationInMinutes(NonZeroU32::new(value)?))
    }

    /// Returns the corresponding [chrono::TimeDelta].
    pub fn time_delta(self) -> chrono::TimeDelta {
        chrono::TimeDelta::minutes(self.0.get().into())
    }

    /// Returns the number of minutes
    pub fn get(&self) -> NonZeroU32 {
        self.0
    }
}

impl From<NonZeroDurationInMinutes> for chrono::TimeDelta {
    fn from(value: NonZeroDurationInMinutes) -> Self {
        value.time_delta()
    }
}

impl From<NonZeroU32> for NonZeroDurationInMinutes {
    fn from(value: NonZeroU32) -> Self {
        NonZeroDurationInMinutes(value)
    }
}

/// Encapsulates a [chrono::Weekday] and gives it a default ordering
/// (monday is the lowest and sunday the biggest day)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Weekday(pub chrono::Weekday);

impl PartialOrd for Weekday {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Weekday {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let num = self.num_days_from_monday();
        let other_num = other.num_days_from_monday();
        num.cmp(&other_num)
    }
}

impl From<chrono::Weekday> for Weekday {
    fn from(value: chrono::Weekday) -> Self {
        Weekday(value)
    }
}

impl std::fmt::Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            chrono::Weekday::Mon => write!(f, "lundi"),
            chrono::Weekday::Tue => write!(f, "mardi"),
            chrono::Weekday::Wed => write!(f, "mercredi"),
            chrono::Weekday::Thu => write!(f, "jeudi"),
            chrono::Weekday::Fri => write!(f, "vendredi"),
            chrono::Weekday::Sat => write!(f, "samedi"),
            chrono::Weekday::Sun => write!(f, "Dimanche"),
        }
    }
}

impl Weekday {
    pub fn inner(&self) -> &chrono::Weekday {
        &self.0
    }

    pub fn into_inner(self) -> chrono::Weekday {
        self.0
    }

    pub fn inner_mut(&mut self) -> &mut chrono::Weekday {
        &mut self.0
    }
}

impl std::ops::Deref for Weekday {
    type Target = chrono::Weekday;
    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl std::ops::DerefMut for Weekday {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_mut()
    }
}

/// Type representing the beginning of a slot in time
///
/// A slot starts on a given weekday at a certain time.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotStart {
    /// Weekday the slot starts on
    pub weekday: Weekday,

    /// The time the slot starts
    ///
    /// The time uses [chrono::NaiveTime] as we don't
    /// really need localization at this point in the algorithm
    /// Obviously, all the times in the colloscope should be
    /// expressed in the same timezone.
    pub start_time: chrono::NaiveTime,
}

/// Type representing a slot in time, with it its start time but
/// also the corresponding duration.
///
/// The resolution is at most the minute for the duration and a slot
/// cannot be zero minutes long.
///
/// A slot cannot cross a day boundary
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotWithDuration {
    /// start of the slot described by a [SlotStart]
    start: SlotStart,

    /// duration of the slot
    ///
    /// It is expressed in minutes using [time::NonZeroDurationInMinutes]
    /// and cannot be zero.
    duration: NonZeroDurationInMinutes,
}

impl SlotWithDuration {
    /// Creates a new slot with duration
    ///
    /// Returns `None` if the duration is too big and the slot would cross the
    /// day boundary
    pub fn new(start: SlotStart, duration: NonZeroDurationInMinutes) -> Option<SlotWithDuration> {
        let (end, ignored_seconds) = start.start_time.overflowing_add_signed(duration.into());

        // We might have overflowed
        if ignored_seconds != 0 {
            // If we overflow by something other than a day (3600*24 = 86400)
            // that's bad
            if ignored_seconds != 86_400 {
                return None;
            }

            // if we ignore exactly a full day and end is midnight, we technically did not cross
            // but anything else, we should avoid
            if end != chrono::NaiveTime::from_hms_opt(0, 0, 0).expect("Midnight should be possible")
            {
                return None;
            }
        }

        Some(SlotWithDuration { start, duration })
    }

    /// Returns the end time of a slot
    ///
    /// This end time is just past the end of the slot. So
    /// if this slot is an interrogation, the student is actually free
    /// at the end time.
    pub fn end_time(&self) -> chrono::NaiveTime {
        self.start.start_time + self.duration.time_delta()
    }

    /// Returns the duration of a slot
    pub fn duration(&self) -> NonZeroDurationInMinutes {
        self.duration
    }

    /// Checks if two slots (with duration) overlap
    pub fn overlaps_with(&self, other: &SlotWithDuration) -> bool {
        if self.start.weekday != other.start.weekday {
            return false;
        }

        if self.start.start_time <= other.start.start_time {
            self.end_time() > other.start.start_time
        } else {
            self.start.start_time < other.end_time()
        }
    }
}

/// Represents a date that is necessarily a monday
///
/// This is useful for instance to represent the beginning
/// of a specific week.
///
/// Internally it is just a [chrono::NaiveDate].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveMondayDate(chrono::NaiveDate);

impl NaiveMondayDate {
    /// Builds a [NaiveMondayDate] from a [chrono::NaiveDate]
    ///
    /// Returns `None` if the date is not a monday.
    pub fn new(date: chrono::NaiveDate) -> Option<NaiveMondayDate> {
        let week = date.week(chrono::Weekday::Mon);
        let first_day = week.checked_first_day()?;
        if first_day != date {
            return None;
        }

        Some(NaiveMondayDate(date))
    }

    /// Builds a [NaiveMondayDate] from a [chrono::NaiveDate].
    ///
    /// The monday is the last monday before (including) the date
    /// given as parameter
    pub fn round_from(date: chrono::NaiveDate) -> NaiveMondayDate {
        let week = date.week(chrono::Weekday::Mon);
        let first_day = week.checked_first_day().expect("Date should be valid");
        NaiveMondayDate(first_day)
    }

    /// Builds a [NaiveMondayDate] from the current date
    ///
    /// The monday is the last monday before (including) today
    pub fn from_today() -> NaiveMondayDate {
        let today = chrono::Local::now();
        let naive = today.naive_local();

        let date = naive.date();
        Self::round_from(date)
    }
}

impl NaiveMondayDate {
    pub fn inner(&self) -> &chrono::NaiveDate {
        &self.0
    }

    pub fn into_inner(self) -> chrono::NaiveDate {
        self.0
    }
}

impl std::ops::Deref for NaiveMondayDate {
    type Target = chrono::NaiveDate;
    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
