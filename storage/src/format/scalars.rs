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

/// A "week start" date: an ISO `"YYYY-MM-DD"` date that must be a Monday
///
/// The encoding is strict: the date must be zero-padded (`"2026-8-31"` is
/// invalid even though chrono would accept it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WeekStartDate(pub chrono::NaiveDate);

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
        use chrono::Datelike;
        if date.weekday() != chrono::Weekday::Mon {
            return Err(format!("week start date {s:?} is not a Monday"));
        }
        Ok(WeekStartDate(date))
    }
}

impl From<WeekStartDate> for String {
    fn from(date: WeekStartDate) -> String {
        date.0.format("%Y-%m-%d").to_string()
    }
}

/// A time of day: 24-hour `"HH:MM"`, zero-padded, minute precision
///
/// The encoding is strict: `"9:00"` and `"09:00:00"` are invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
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
        if hour >= 24 || minute >= 60 {
            return Err(format!("time of day {s:?} is out of range"));
        }
        Ok(TimeOfDay { hour, minute })
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
pub struct DurationMinutes(pub NonZeroU32);

/// A weekday plus a time of day (the `start` record of a slot)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayTime {
    pub day: Weekday,
    pub time: TimeOfDay,
}

/// An integer range `{"min": n, "max": n}` with `min <= max`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "RawRange<T>",
    bound(deserialize = "T: serde::Deserialize<'de> + Ord")
)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRange<T> {
    min: T,
    max: T,
}

impl<T: Ord> TryFrom<RawRange<T>> for Range<T> {
    type Error = String;

    fn try_from(raw: RawRange<T>) -> Result<Self, Self::Error> {
        if raw.min > raw.max {
            return Err("invalid range: min is greater than max".to_string());
        }
        Ok(Range {
            min: raw.min,
            max: raw.max,
        })
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
            date.0,
            chrono::NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
        );
        assert_eq!(serde_json::to_value(date).unwrap(), json!("2026-08-31"));
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
        assert_eq!(time, TimeOfDay { hour: 9, minute: 5 });
        assert_eq!(serde_json::to_value(time).unwrap(), json!("09:05"));

        let time: TimeOfDay = serde_json::from_value(json!("23:59")).unwrap();
        assert_eq!(
            time,
            TimeOfDay {
                hour: 23,
                minute: 59
            }
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
        assert_eq!(duration.0.get(), 60);
        assert_eq!(serde_json::to_value(duration).unwrap(), json!(60));

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
