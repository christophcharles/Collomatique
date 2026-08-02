//! Scalar encodings of the spec-2 format (spec §3)
//!
//! Ids are plain `u64` on purpose: the spec's "at most 2⁶³ − 1" rule is
//! enforced at layer 3 with its dedicated error (`EndOfTheUniverse`), and
//! a layer-1 serde check would only shadow it with a generic message.

use serde::{Deserialize, Deserializer, Serialize};
use std::num::NonZeroU32;

/// Deserialize an optional record field that must be written explicitly
///
/// The spec requires every record field to be present, optional values
/// being an explicit `null`. Plain `Option<T>` fields do not enforce
/// this: serde's missing-field path silently produces `None` for any
/// type accepting "none" (an implicit default that even a type alias or
/// a transparent newtype cannot defeat). Attaching this function with
/// `#[serde(deserialize_with = "...")]` switches serde to its strict
/// missing-field handling: a missing field is a hard error, while an
/// explicit `null` still deserializes to `None`.
pub fn explicit_option<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer)
}

/// Error when building a [WeekStartDate] out of a non-Monday date
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAMonday(pub chrono::NaiveDate);

impl std::fmt::Display for NotAMonday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "week start date {} is not a Monday", self.0)
    }
}

impl std::error::Error for NotAMonday {}

/// A "week start" date: an ISO `"YYYY-MM-DD"` date that must be a Monday
///
/// The encoding is strict: the date must be zero-padded (`"2026-8-31"` is
/// invalid even though chrono would accept it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WeekStartDate(chrono::NaiveDate);

impl WeekStartDate {
    /// Build a week start date, checking that it is a Monday
    pub fn new(date: chrono::NaiveDate) -> Result<Self, NotAMonday> {
        use chrono::Datelike;
        if date.weekday() != chrono::Weekday::Mon {
            return Err(NotAMonday(date));
        }
        Ok(WeekStartDate(date))
    }

    pub fn date(&self) -> chrono::NaiveDate {
        self.0
    }
}

impl TryFrom<String> for WeekStartDate {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let date = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|e| format!("invalid week start date {s:?}: {e}"))?;
        // chrono accepts non-padded numbers; the re-format round-trip
        // enforces the strict "YYYY-MM-DD" encoding
        if date.format("%Y-%m-%d").to_string() != s {
            return Err(format!(
                "week start date {s:?} is not in strict \"YYYY-MM-DD\" form"
            ));
        }
        WeekStartDate::new(date).map_err(|e| e.to_string())
    }
}

impl From<WeekStartDate> for String {
    fn from(date: WeekStartDate) -> String {
        date.0.format("%Y-%m-%d").to_string()
    }
}

/// Error when building a [TimeOfDay] out of an out-of-range hour or minute
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeOutOfRange {
    pub hour: u8,
    pub minute: u8,
}

impl std::fmt::Display for TimeOutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "time of day {:02}:{:02} is out of range",
            self.hour, self.minute
        )
    }
}

impl std::error::Error for TimeOutOfRange {}

/// A time of day: 24-hour `"HH:MM"`, zero-padded, minute precision
///
/// The encoding is strict: `"9:00"` and `"09:00:00"` are invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
}

impl TimeOfDay {
    /// Build a time of day, checking that the hour and minute are in
    /// range
    pub fn new(hour: u8, minute: u8) -> Result<Self, TimeOutOfRange> {
        if hour >= 24 || minute >= 60 {
            return Err(TimeOutOfRange { hour, minute });
        }
        Ok(TimeOfDay { hour, minute })
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }
}

impl TryFrom<String> for TimeOfDay {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        let valid_shape = bytes.len() == 5
            && bytes[2] == b':'
            && [bytes[0], bytes[1], bytes[3], bytes[4]]
                .iter()
                .all(u8::is_ascii_digit);
        if !valid_shape {
            return Err(format!("time of day {s:?} is not in \"HH:MM\" form"));
        }
        let hour = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
        let minute = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
        TimeOfDay::new(hour, minute).map_err(|_| format!("time of day {s:?} is out of range"))
    }
}

impl From<TimeOfDay> for String {
    fn from(time: TimeOfDay) -> String {
        format!("{:02}:{:02}", time.hour, time.minute)
    }
}

/// A weekday, encoded as a lowercase English name
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// A duration in integer minutes, at least 1
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DurationMinutes(NonZeroU32);

impl DurationMinutes {
    /// Build a duration; `NonZeroU32` already carries the "at least 1"
    /// invariant, so this cannot fail
    pub fn new(minutes: NonZeroU32) -> Self {
        DurationMinutes(minutes)
    }

    pub fn get(&self) -> NonZeroU32 {
        self.0
    }
}

/// A weekday plus a time of day (the `start` record of a slot)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayTime {
    pub day: Weekday,
    pub time: TimeOfDay,
}

/// Error when building a [Range] with `min` greater than `max`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidRange<T> {
    pub min: T,
    pub max: T,
}

impl<T: std::fmt::Debug> std::fmt::Display for InvalidRange<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid range: min {:?} is greater than max {:?}",
            self.min, self.max
        )
    }
}

impl<T: std::fmt::Debug> std::error::Error for InvalidRange<T> {}

/// An integer range `{"min": n, "max": n}` with `min <= max`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "RawRange<T>",
    bound(deserialize = "T: serde::Deserialize<'de> + Ord + std::fmt::Debug")
)]
pub struct Range<T> {
    min: T,
    max: T,
}

impl<T: Ord> Range<T> {
    /// Build a range, checking that `min <= max`
    pub fn new(min: T, max: T) -> Result<Self, InvalidRange<T>> {
        if min > max {
            return Err(InvalidRange { min, max });
        }
        Ok(Range { min, max })
    }
}

impl<T> Range<T> {
    // Part of the accessor API alongside `into_min_max`; the decoder
    // consumes ranges whole, so only tests exercise these
    #[allow(dead_code)]
    pub fn min(&self) -> &T {
        &self.min
    }

    #[allow(dead_code)]
    pub fn max(&self) -> &T {
        &self.max
    }

    pub fn into_min_max(self) -> (T, T) {
        (self.min, self.max)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRange<T> {
    min: T,
    max: T,
}

impl<T: Ord> TryFrom<RawRange<T>> for Range<T> {
    type Error = InvalidRange<T>;

    fn try_from(raw: RawRange<T>) -> Result<Self, Self::Error> {
        Range::new(raw.min, raw.max)
    }
}

/// An RGB color
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// A soft parameter carrying a value: `{"soft": bool, "value": ...}`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftParam<T> {
    pub soft: bool,
    pub value: T,
}

/// A valueless soft parameter: `{"soft": bool}`
///
/// The spec drops the `value` field when a soft parameter carries no
/// value; this being a separate type, no custom serde is needed and
/// `deny_unknown_fields` rejects a stray `value`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftFlag {
    pub soft: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn week_start_date_accepts_a_strict_monday() {
        let date: WeekStartDate = serde_json::from_value(json!("2026-08-31")).unwrap();
        assert_eq!(
            date.date(),
            chrono::NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
        );
        assert_eq!(serde_json::to_value(date).unwrap(), json!("2026-08-31"));
    }

    #[test]
    fn week_start_date_new_checks_the_invariant() {
        let monday = chrono::NaiveDate::from_ymd_opt(2026, 8, 31).unwrap();
        let date = WeekStartDate::new(monday).unwrap();
        assert_eq!(date.date(), monday);

        let tuesday = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(WeekStartDate::new(tuesday), Err(NotAMonday(tuesday)));
    }

    #[test]
    fn week_start_date_rejects_non_mondays() {
        assert!(serde_json::from_value::<WeekStartDate>(json!("2026-09-01")).is_err());
    }

    #[test]
    fn week_start_date_rejects_non_padded_dates() {
        assert!(serde_json::from_value::<WeekStartDate>(json!("2026-8-31")).is_err());
    }

    #[test]
    fn week_start_date_rejects_wrong_field_order() {
        assert!(serde_json::from_value::<WeekStartDate>(json!("31-08-2026")).is_err());
    }

    #[test]
    fn time_of_day_accepts_strict_form() {
        let time: TimeOfDay = serde_json::from_value(json!("09:05")).unwrap();
        assert_eq!(time, TimeOfDay::new(9, 5).unwrap());
        assert_eq!(serde_json::to_value(time).unwrap(), json!("09:05"));

        let time: TimeOfDay = serde_json::from_value(json!("23:59")).unwrap();
        assert_eq!(time, TimeOfDay::new(23, 59).unwrap());
    }

    #[test]
    fn time_of_day_new_checks_the_invariant() {
        let time = TimeOfDay::new(9, 5).unwrap();
        assert_eq!((time.hour(), time.minute()), (9, 5));

        assert_eq!(
            TimeOfDay::new(24, 0),
            Err(TimeOutOfRange {
                hour: 24,
                minute: 0
            })
        );
        assert_eq!(
            TimeOfDay::new(9, 60),
            Err(TimeOutOfRange {
                hour: 9,
                minute: 60
            })
        );
    }

    #[test]
    fn time_of_day_rejects_loose_forms() {
        for bad in ["9:00", "09:00:00", "24:00", "09:60", "0900", "09-00"] {
            assert!(
                serde_json::from_value::<TimeOfDay>(json!(bad)).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn weekday_is_lowercase_english() {
        let day: Weekday = serde_json::from_value(json!("monday")).unwrap();
        assert_eq!(day, Weekday::Monday);
        assert_eq!(serde_json::to_value(day).unwrap(), json!("monday"));

        assert!(serde_json::from_value::<Weekday>(json!("Monday")).is_err());
        assert!(serde_json::from_value::<Weekday>(json!("lundi")).is_err());
    }

    #[test]
    fn duration_is_a_positive_number_of_minutes() {
        let duration: DurationMinutes = serde_json::from_value(json!(60)).unwrap();
        assert_eq!(duration.get().get(), 60);
        assert_eq!(serde_json::to_value(duration).unwrap(), json!(60));
        assert_eq!(duration, DurationMinutes::new(NonZeroU32::new(60).unwrap()));

        assert!(serde_json::from_value::<DurationMinutes>(json!(0)).is_err());
        assert!(serde_json::from_value::<DurationMinutes>(json!(-5)).is_err());
    }

    #[test]
    fn range_round_trips() {
        let range: Range<NonZeroU32> =
            serde_json::from_value(json!({ "min": 2, "max": 3 })).unwrap();
        assert_eq!(
            serde_json::to_value(range).unwrap(),
            json!({ "min": 2, "max": 3 })
        );
    }

    #[test]
    fn range_new_checks_the_invariant() {
        let range = Range::new(2u32, 3).unwrap();
        assert_eq!((*range.min(), *range.max()), (2, 3));
        assert_eq!(range.into_min_max(), (2, 3));

        assert_eq!(Range::new(3u32, 2), Err(InvalidRange { min: 3, max: 2 }));
    }

    #[test]
    fn range_rejects_min_greater_than_max() {
        assert!(serde_json::from_value::<Range<u32>>(json!({ "min": 3, "max": 2 })).is_err());
    }

    #[test]
    fn range_rejects_unknown_fields() {
        assert!(
            serde_json::from_value::<Range<u32>>(json!({ "min": 2, "max": 3, "step": 1 })).is_err()
        );
    }

    #[test]
    fn range_rejects_missing_fields() {
        assert!(serde_json::from_value::<Range<u32>>(json!({ "min": 2 })).is_err());
    }

    #[test]
    fn day_time_round_trips_and_is_strict() {
        let start: DayTime =
            serde_json::from_value(json!({ "day": "monday", "time": "14:00" })).unwrap();
        assert_eq!(
            serde_json::to_value(start).unwrap(),
            json!({ "day": "monday", "time": "14:00" })
        );

        assert!(
            serde_json::from_value::<DayTime>(
                json!({ "day": "monday", "time": "14:00", "room": "101" })
            )
            .is_err()
        );
        assert!(serde_json::from_value::<DayTime>(json!({ "day": "monday" })).is_err());
    }

    #[test]
    fn soft_param_requires_its_value() {
        let param: SoftParam<u32> =
            serde_json::from_value(json!({ "soft": true, "value": 3 })).unwrap();
        assert_eq!(
            param,
            SoftParam {
                soft: true,
                value: 3
            }
        );

        assert!(serde_json::from_value::<SoftParam<u32>>(json!({ "soft": true })).is_err());
    }

    #[test]
    fn soft_flag_rejects_a_stray_value() {
        let flag: SoftFlag = serde_json::from_value(json!({ "soft": false })).unwrap();
        assert_eq!(flag, SoftFlag { soft: false });

        assert!(serde_json::from_value::<SoftFlag>(json!({ "soft": true, "value": 3 })).is_err());
    }
}
