//! Time-related types for scheduling colloscopes (school timetables)
//!
//! This crate provides strongly-typed time primitives optimized for educational
//! scheduling contexts. All types enforce minute-level time granularity, which is
//! both practical for school schedules and necessary for the colloscope resolution
//! algorithm.
//!
//! # Key Design Decisions
//!
//! - **Minute granularity**: All times use whole minutes only (no seconds/nanoseconds)
//! - **Monday-first weeks**: Weeks start on Monday, following European convention
//! - **No day crossing**: Scheduling slots cannot cross midnight boundaries, as the
//!   resolution algorithm processes each day independently
//! - **French localization**: Display formatting uses French weekday names
//!
//! # Core Types
//!
//! - [`NonZeroDurationInMinutes`]: A duration of at least one minute
//! - [`Weekday`]: A weekday with Monday-first ordering
//! - [`WholeMinuteTime`]: A time of day with minute precision (no seconds)
//! - [`SlotStart`]: The beginning of a time slot (weekday + time)
//! - [`SlotWithDuration`]: A complete time slot with start and duration
//! - [`WeekStart`]: A date that is always a Monday, used to identify weeks
//!
//! # Example: Creating and checking slot overlaps
//!
//! ```
//! use collomatique_time::*;
//! use chrono::{Weekday as ChronoWeekday, NaiveTime};
//!
//! // Create a Monday 9:00-11:00 slot
//! let morning_slot = SlotWithDuration::new(
//!     SlotStart {
//!         weekday: Weekday(ChronoWeekday::Mon),
//!         start_time: WholeMinuteTime::new(
//!             NaiveTime::from_hms_opt(9, 0, 0).unwrap()
//!         ).unwrap(),
//!     },
//!     NonZeroDurationInMinutes::new(120).unwrap(),
//! ).unwrap();
//!
//! // Create a Monday 10:30-12:30 slot
//! let midday_slot = SlotWithDuration::new(
//!     SlotStart {
//!         weekday: Weekday(ChronoWeekday::Mon),
//!         start_time: WholeMinuteTime::new(
//!             NaiveTime::from_hms_opt(10, 30, 0).unwrap()
//!         ).unwrap(),
//!     },
//!     NonZeroDurationInMinutes::new(120).unwrap(),
//! ).unwrap();
//!
//! // Check if slots overlap
//! assert!(morning_slot.overlaps_with(&midday_slot));
//! ```
//!
//! # Example: Working with weeks
//!
//! ```
//! use collomatique_time::WeekStart;
//! use chrono::NaiveDate;
//!
//! // Get the current week
//! let this_week = WeekStart::from_today();
//!
//! // Get the week containing a specific date
//! let some_date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
//! let that_week = WeekStart::round_from(some_date);
//! ```

#[cfg(test)]
mod tests;

use std::num::NonZeroU32;

use chrono::Timelike;
use serde::{Deserialize, Serialize};

/// DurationInMinutes obviously represents a duration in minutes.
///
/// The type is useful because it gives information on the type of data
/// represented. Also, it garantees that the underlying duration is
/// at least one minute.
///
/// Crucially all durations have always positive span.
///
/// On top of that it implements a few utility functions to interact with chrono.
///
/// This type is typically used with [SlotStart] to construct a [SlotWithDuration].
///
/// # Example
///
/// ```
/// use collomatique_time::NonZeroDurationInMinutes;
///
/// // Create a 90-minute duration
/// let duration = NonZeroDurationInMinutes::new(90).unwrap();
/// assert_eq!(duration.to_string(), "1h30");
///
/// // Convert to chrono::TimeDelta for calculations
/// let time_delta = duration.time_delta();
///
/// // Zero minutes is not allowed
/// assert!(NonZeroDurationInMinutes::new(0).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    /// Returns the number of minutes as a [`NonZeroU32`]
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::NonZeroDurationInMinutes;
    ///
    /// let duration = NonZeroDurationInMinutes::new(90).unwrap();
    /// assert_eq!(duration.get().get(), 90);
    /// ```
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

const MINUTES_PER_HOUR: u32 = 60;

impl std::fmt::Display for NonZeroDurationInMinutes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}h{:02}",
            self.0.get() / MINUTES_PER_HOUR,
            self.0.get() % MINUTES_PER_HOUR,
        )
    }
}

/// Encapsulates a [chrono::Weekday] with a canonical ordering where Monday is the
/// first day of the week and Sunday is the last.
///
/// This ordering (Monday < Tuesday < ... < Sunday) is the standard European week layout
/// and is used throughout the colloscope system.
/// This defines a default ordering for [chrono::Weekday] which does have one by default.
///
/// # Example
///
/// ```
/// use collomatique_time::Weekday;
/// use chrono::Weekday as ChronoWeekday;
///
/// let monday = Weekday(ChronoWeekday::Mon);
/// let friday = Weekday(ChronoWeekday::Fri);
/// let sunday = Weekday(ChronoWeekday::Sun);
///
/// assert!(monday < friday);
/// assert!(friday < sunday);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Returns the capitalized French name of the weekday.
    ///
    /// The uncapitalized name is returned by the [std::fmt::Display]
    /// implementation. So you can get it with `to_string`.
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::Weekday;
    /// use chrono::Weekday as ChronoWeekday;
    ///
    /// let monday = Weekday(ChronoWeekday::Mon);
    /// assert_eq!(monday.capitalize(), "Lundi");
    /// ```
    ///
    /// And to get a non-capitalized text:
    /// ```
    /// use collomatique_time::Weekday;
    /// use chrono::Weekday as ChronoWeekday;
    ///
    /// let monday = Weekday(ChronoWeekday::Mon);
    /// assert_eq!(&monday.to_string(), "lundi");
    /// ```
    pub fn capitalize(&self) -> &'static str {
        match self.0 {
            chrono::Weekday::Mon => "Lundi",
            chrono::Weekday::Tue => "Mardi",
            chrono::Weekday::Wed => "Mercredi",
            chrono::Weekday::Thu => "Jeudi",
            chrono::Weekday::Fri => "Vendredi",
            chrono::Weekday::Sat => "Samedi",
            chrono::Weekday::Sun => "Dimanche",
        }
    }

    /// Returns an iterator over all weekdays in order (Monday through Sunday)
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::Weekday;
    ///
    /// let days: Vec<_> = Weekday::iter().collect();
    /// assert_eq!(days.len(), 7);
    /// ```
    pub fn iter() -> impl Iterator<Item = Weekday> {
        [
            Weekday(chrono::Weekday::Mon),
            Weekday(chrono::Weekday::Tue),
            Weekday(chrono::Weekday::Wed),
            Weekday(chrono::Weekday::Thu),
            Weekday(chrono::Weekday::Fri),
            Weekday(chrono::Weekday::Sat),
            Weekday(chrono::Weekday::Sun),
        ]
        .into_iter()
    }

    /// Returns a reference to the inner [`chrono::Weekday`]
    pub fn inner(&self) -> &chrono::Weekday {
        &self.0
    }

    /// Consumes `self` and returns the inner [`chrono::Weekday`]
    pub fn into_inner(self) -> chrono::Weekday {
        self.0
    }

    /// Returns a mutable reference to the inner [`chrono::Weekday`]
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

/// Represents a time of day with minute-level precision.
///
/// This type wraps [`chrono::NaiveTime`] but enforces that seconds and
/// nanoseconds are always zero. This ensures all times in the colloscope
/// system use a consistent granularity of one minute, which is both
/// appropriate for scheduling contexts and necessary for the resolution
/// algorithm.
///
/// # Example
///
/// ```
/// use collomatique_time::WholeMinuteTime; // or whatever name we choose
/// use chrono::NaiveTime;
///
/// // Valid: whole minutes
/// let time = WholeMinuteTime::new(
///     NaiveTime::from_hms_opt(14, 30, 0).unwrap()
/// ).unwrap();
///
/// // Invalid: has seconds
/// let time_with_seconds = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
/// assert!(WholeMinuteTime::new(time_with_seconds).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct WholeMinuteTime(chrono::NaiveTime);

impl<'de> serde::Deserialize<'de> for WholeMinuteTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let naive_time = chrono::NaiveTime::deserialize(deserializer)?;
        use serde::de::Error;
        match Self::new(naive_time) {
            Some(t) => Ok(t),
            None => Err(D::Error::custom(
                "time must have zero seconds and nanoseconds (whole minute only)",
            )),
        }
    }
}

impl WholeMinuteTime {
    /// Creates a new [`WholeMinuteTime`] from a [`chrono::NaiveTime`]
    ///
    /// Returns `None` if the time has non-zero seconds or nanoseconds.
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::WholeMinuteTime;
    /// use chrono::NaiveTime;
    ///
    /// let time = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    /// assert!(WholeMinuteTime::new(time).is_some());
    ///
    /// let time_with_seconds = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
    /// assert!(WholeMinuteTime::new(time_with_seconds).is_none());
    /// ```
    pub fn new(naive_time: chrono::NaiveTime) -> Option<Self> {
        if naive_time.second() != 0 || naive_time.nanosecond() != 0 {
            return None;
        }

        Some(WholeMinuteTime(naive_time))
    }

    /// Returns a reference to the inner [`chrono::NaiveTime`]
    pub fn inner(&self) -> &chrono::NaiveTime {
        &self.0
    }

    /// Consumes `self` and returns the inner [`chrono::NaiveTime`]
    pub fn into_inner(self) -> chrono::NaiveTime {
        self.0
    }
}

impl std::fmt::Display for WholeMinuteTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format("%Hh%M"),)
    }
}

use thiserror::Error;

/// Error returned when attempting to convert a [`chrono::NaiveTime`] to a
/// [`WholeMinuteTime`] that has non-zero seconds or nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("time must have zero seconds and nanoseconds")]
pub struct NotWholeMinuteError;

impl TryFrom<chrono::NaiveTime> for WholeMinuteTime {
    type Error = NotWholeMinuteError;

    fn try_from(naive_time: chrono::NaiveTime) -> Result<Self, Self::Error> {
        WholeMinuteTime::new(naive_time).ok_or(NotWholeMinuteError)
    }
}

impl std::ops::Deref for WholeMinuteTime {
    type Target = chrono::NaiveTime;
    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

/// Type representing the beginning of a slot in time
///
/// A slot starts on a given weekday at a certain time. This type is typically
/// paired with a [`NonZeroDurationInMinutes`] to create a [`SlotWithDuration`].
///
/// # Example
///
/// ```
/// use collomatique_time::{SlotStart, Weekday, WholeMinuteTime};
/// use chrono::NaiveTime;
///
/// let start = SlotStart {
///     weekday: Weekday(chrono::Weekday::Mon),
///     start_time: WholeMinuteTime::new(
///         NaiveTime::from_hms_opt(9, 0, 0).unwrap()
///     ).unwrap(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SlotStart {
    /// Weekday the slot starts on
    pub weekday: Weekday,

    /// The time the slot starts, with minute-level precision
    ///
    /// All times in the colloscope should be expressed in the same timezone.
    pub start_time: WholeMinuteTime,
}

impl SlotStart {
    /// Returns a capitalized string representation suitable for display
    ///
    /// Uses capitalized French weekday names. For a non-capitalized version,
    /// use the [std::fmt::Display] implementation.
    pub fn capitalize(&self) -> String {
        format!("{} {}", self.weekday.capitalize(), self.start_time,)
    }
}

impl std::fmt::Display for SlotStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.weekday, self.start_time,)
    }
}

/// Type representing a slot in time with both start time and duration.
///
/// A slot cannot cross the midnight boundary (from one day to the next). This
/// constraint is both practical for school scheduling contexts and essential for
/// the resolution algorithm, which operates day-by-day and needs to reason about
/// each day independently.
///
/// The resolution is at most the minute for the duration, and a slot
/// cannot be zero minutes long.
///
/// # Example
///
/// ```
/// use collomatique_time::{SlotStart, SlotWithDuration, Weekday, WholeMinuteTime, NonZeroDurationInMinutes};
/// use chrono::{Weekday as ChronoWeekday, NaiveTime};
///
/// let start = SlotStart {
///     weekday: Weekday(ChronoWeekday::Mon),
///     start_time: WholeMinuteTime::new(
///         NaiveTime::from_hms_opt(14, 0, 0).unwrap()
///     ).unwrap(),
/// };
///
/// // Valid: 2-hour slot ending at 16:00
/// let slot = SlotWithDuration::new(
///     start.clone(),
///     NonZeroDurationInMinutes::new(120).unwrap()
/// ).unwrap();
///
/// // Invalid: would end at 02:00 the next day, crossing midnight
/// let too_long = SlotWithDuration::new(
///     start,
///     NonZeroDurationInMinutes::new(720).unwrap() // 12 hours
/// );
/// assert!(too_long.is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SlotWithDuration {
    /// start of the slot described by a [SlotStart]
    start: SlotStart,

    /// duration of the slot
    ///
    /// It is expressed in minutes using [NonZeroDurationInMinutes]
    /// and cannot be zero.
    duration: NonZeroDurationInMinutes,
}

impl SlotWithDuration {
    /// Creates a new slot with duration
    ///
    /// Returns `None` if the duration would cause the slot to cross the midnight
    /// boundary into the next day. A slot ending exactly at midnight (00:00) is
    /// considered valid as it doesn't cross into the next day.
    ///
    /// # Examples
    ///
    /// ```
    /// use collomatique_time::{SlotStart, SlotWithDuration, Weekday, WholeMinuteTime, NonZeroDurationInMinutes};
    /// use chrono::{Weekday as ChronoWeekday, NaiveTime};
    ///
    /// let start_morning = SlotStart {
    ///     weekday: Weekday(ChronoWeekday::Mon),
    ///     start_time: WholeMinuteTime::new(
    ///         NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    ///     ).unwrap(),
    /// };
    ///
    /// // Valid: 2-hour slot from 9:00 to 11:00
    /// let slot = SlotWithDuration::new(
    ///     start_morning,
    ///     NonZeroDurationInMinutes::new(120).unwrap()
    /// );
    /// assert!(slot.is_some());
    ///
    /// // Edge case: slot ending exactly at midnight is valid
    /// let start_evening = SlotStart {
    ///     weekday: Weekday(ChronoWeekday::Mon),
    ///     start_time: WholeMinuteTime::new(
    ///         NaiveTime::from_hms_opt(22, 0, 0).unwrap()
    ///     ).unwrap(),
    /// };
    /// let until_midnight = SlotWithDuration::new(
    ///     start_evening,
    ///     NonZeroDurationInMinutes::new(120).unwrap() // 22:00 + 2h = 00:00
    /// );
    /// assert!(until_midnight.is_some());
    ///
    /// // Invalid: would cross midnight into the next day
    /// let start_late = SlotStart {
    ///     weekday: Weekday(ChronoWeekday::Mon),
    ///     start_time: WholeMinuteTime::new(
    ///         NaiveTime::from_hms_opt(23, 30, 0).unwrap()
    ///     ).unwrap(),
    /// };
    /// let crosses_midnight = SlotWithDuration::new(
    ///     start_late,
    ///     NonZeroDurationInMinutes::new(60).unwrap() // 23:30 + 1h = 00:30
    /// );
    /// assert!(crosses_midnight.is_none());
    /// ```
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

    /// Returns the start time of a slot
    pub fn start(&self) -> &SlotStart {
        &self.start
    }

    /// Returns the end time of the slot as a [`chrono::NaiveTime`]
    ///
    /// This end time is just past the end of the slot. For example,
    /// if this slot represents a class period, the student is actually free
    /// at the end time.
    pub fn naive_end_time(&self) -> chrono::NaiveTime {
        *self.start.start_time.inner() + self.duration.time_delta()
    }

    /// Returns the end time of the slot as a [`WholeMinuteTime`]
    ///
    /// This is a convenience wrapper around [`naive_end_time()`](Self::naive_end_time)
    /// that returns the domain type.
    pub fn end_time(&self) -> WholeMinuteTime {
        WholeMinuteTime::new(self.naive_end_time()).unwrap()
    }

    /// Returns the duration of a slot
    pub fn duration(&self) -> NonZeroDurationInMinutes {
        self.duration
    }

    /// Checks if two slots overlap in time
    ///
    /// Two slots overlap if they occur on the same weekday and their time ranges
    /// intersect. Note that the end time of a slot is *not included* in the slot
    /// itself - it represents the first moment when the slot is over.
    ///
    /// This means that two adjacent slots (where one ends exactly when the other
    /// begins) do *not* overlap.
    ///
    /// Slots on different weekdays never overlap, regardless of their times.
    ///
    /// # Examples
    ///
    /// ```
    /// use collomatique_time::{SlotStart, SlotWithDuration, Weekday, WholeMinuteTime, NonZeroDurationInMinutes};
    /// use chrono::NaiveTime;
    ///
    /// let monday = Weekday(chrono::Weekday::Mon);
    /// let tuesday = Weekday(chrono::Weekday::Tue);
    ///
    /// // Slot 1: Monday 9:00-11:00
    /// let slot1 = SlotWithDuration::new(
    ///     SlotStart {
    ///         weekday: monday,
    ///         start_time: WholeMinuteTime::new(
    ///             NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    ///         ).unwrap(),
    ///     },
    ///     NonZeroDurationInMinutes::new(120).unwrap(),
    /// ).unwrap();
    ///
    /// // Slot 2: Monday 10:00-12:00 - overlaps with slot1
    /// let slot2 = SlotWithDuration::new(
    ///     SlotStart {
    ///         weekday: monday,
    ///         start_time: WholeMinuteTime::new(
    ///             NaiveTime::from_hms_opt(10, 0, 0).unwrap()
    ///         ).unwrap(),
    ///     },
    ///     NonZeroDurationInMinutes::new(120).unwrap(),
    /// ).unwrap();
    /// assert!(slot1.overlaps_with(&slot2));
    ///
    /// // Slot 3: Monday 11:00-13:00 - does NOT overlap (adjacent, end time not included)
    /// let slot3 = SlotWithDuration::new(
    ///     SlotStart {
    ///         weekday: monday,
    ///         start_time: WholeMinuteTime::new(
    ///             NaiveTime::from_hms_opt(11, 0, 0).unwrap()
    ///         ).unwrap(),
    ///     },
    ///     NonZeroDurationInMinutes::new(120).unwrap(),
    /// ).unwrap();
    /// assert!(!slot1.overlaps_with(&slot3));
    ///
    /// // Slot 4: Tuesday 9:00-11:00 - does NOT overlap (different day)
    /// let slot4 = SlotWithDuration::new(
    ///     SlotStart {
    ///         weekday: tuesday,
    ///         start_time: WholeMinuteTime::new(
    ///             NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    ///         ).unwrap(),
    ///     },
    ///     NonZeroDurationInMinutes::new(120).unwrap(),
    /// ).unwrap();
    /// assert!(!slot1.overlaps_with(&slot4));
    /// ```
    pub fn overlaps_with(&self, other: &SlotWithDuration) -> bool {
        if self.start.weekday != other.start.weekday {
            return false;
        }

        if self.start.start_time <= other.start.start_time {
            self.naive_end_time() > *other.start.start_time.inner()
        } else {
            *self.start.start_time.inner() < other.naive_end_time()
        }
    }

    /// Returns a capitalized string representation suitable for display
    ///
    /// Uses capitalized French weekday names. For a non-capitalized version,
    /// use the [std::fmt::Display] implementation.
    pub fn capitalize(&self) -> String {
        let end_time_on_minute = self.end_time();
        format!(
            "{} {}-{}",
            self.start.weekday.capitalize(),
            self.start.start_time,
            end_time_on_minute,
        )
    }
}

impl std::fmt::Display for SlotWithDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let end_time_on_minute = self.end_time();
        write!(
            f,
            "{} {}-{}",
            self.start.weekday, self.start.start_time, end_time_on_minute,
        )
    }
}

/// Represents the start of a week, which is always a Monday.
///
/// This type enforces that a date is a Monday, making it useful for representing
/// and reasoning about specific weeks in the colloscope system. Since the system
/// uses Monday as the first day of the week (see [`Weekday`]), this type ensures
/// consistency throughout.
///
/// Internally wraps a [`chrono::NaiveDate`].
///
/// # Example
///
/// ```
/// use collomatique_time::WeekStart;
/// use chrono::NaiveDate;
///
/// // Create from a specific Monday
/// let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // A Monday
/// let week = WeekStart::new(monday).unwrap();
///
/// // Create from any date (rounds down to the Monday of that week)
/// let wednesday = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
/// let week = WeekStart::round_from(wednesday);
///
/// // Create from today's date
/// let current_week = WeekStart::from_today();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct WeekStart(chrono::NaiveDate);

impl<'de> serde::Deserialize<'de> for WeekStart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let naive_date = chrono::NaiveDate::deserialize(deserializer)?;
        use serde::de::Error;
        match Self::new(naive_date) {
            Some(d) => Ok(d),
            None => Err(D::Error::custom("date must be a Monday (week start)")),
        }
    }
}

impl WeekStart {
    /// Creates a [`WeekStart`] from a [`chrono::NaiveDate`]
    ///
    /// Returns `None` if the date is not a Monday.
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::WeekStart;
    /// use chrono::NaiveDate;
    ///
    /// let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    /// assert!(WeekStart::new(monday).is_some());
    ///
    /// let tuesday = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    /// assert!(WeekStart::new(tuesday).is_none());
    /// ```
    pub fn new(date: chrono::NaiveDate) -> Option<WeekStart> {
        let week = date.week(chrono::Weekday::Mon);
        let first_day = week.checked_first_day()?;
        if first_day != date {
            return None;
        }

        Some(WeekStart(date))
    }

    /// Creates a [`WeekStart`] from a [`chrono::NaiveDate`] by rounding down
    ///
    /// Returns the start of the week containing the given date. If the date
    /// is already a Monday (the week start), returns that date unchanged.
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::WeekStart;
    /// use chrono::NaiveDate;
    ///
    /// // Wednesday January 3, 2024
    /// let wednesday = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
    /// let week = WeekStart::round_from(wednesday);
    ///
    /// // Returns the start of that week (Monday January 1, 2024)
    /// assert_eq!(*week.monday(), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    /// ```
    pub fn round_from(date: chrono::NaiveDate) -> WeekStart {
        let week = date.week(chrono::Weekday::Mon);
        let first_day = week.checked_first_day().expect("Date should be valid");
        WeekStart(first_day)
    }

    /// Creates a [`WeekStart`] from the current date
    ///
    /// Returns the start of the week containing today's date. Uses the system's
    /// local timezone to determine "today".
    ///
    /// # Example
    ///
    /// ```
    /// use collomatique_time::WeekStart;
    ///
    /// let current_week = WeekStart::from_today();
    /// // Returns the start of the current week
    /// ```
    pub fn from_today() -> WeekStart {
        let today = chrono::Local::now();
        let naive = today.naive_local();

        let date = naive.date();
        Self::round_from(date)
    }
}

impl WeekStart {
    /// Returns a reference to the Monday date as a [`chrono::NaiveDate`]
    pub fn monday(&self) -> &chrono::NaiveDate {
        &self.0
    }

    /// Consumes `self` and returns the Monday date as a [`chrono::NaiveDate`]
    pub fn into_monday(self) -> chrono::NaiveDate {
        self.0
    }
}

impl std::ops::Deref for WeekStart {
    type Target = chrono::NaiveDate;
    fn deref(&self) -> &Self::Target {
        self.monday()
    }
}
