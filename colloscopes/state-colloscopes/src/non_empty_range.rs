//! A [`RangeInclusive`] that is non-empty by construction.

use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Error when a [`NonEmptyRangeInclusive`] would be empty (`start > end`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("range is empty (start > end)")]
pub struct EmptyRangeError;

/// A [`RangeInclusive`] that is non-empty by construction (`start <= end`).
///
/// Serialized exactly like [`RangeInclusive`] (`{"start": …, "end": …}`);
/// deserialization of an empty range is a hard error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RangeInclusive<T>", into = "RangeInclusive<T>")]
pub struct NonEmptyRangeInclusive<T: Ord + Clone>(RangeInclusive<T>);

impl<T: Ord + Clone> NonEmptyRangeInclusive<T> {
    /// Builds a non-empty range; returns `None` iff the range is empty.
    pub fn new(range: RangeInclusive<T>) -> Option<Self> {
        if range.is_empty() {
            return None;
        }
        Some(NonEmptyRangeInclusive(range))
    }
}

/// The document order: a range is an atom — its content is the endpoint
/// pair. Reading `[2..=3] ⊆ [1..=4]` as an order would compare the denoted
/// sets, which is exactly the semantic reading the document order forbids.
///
/// Hand-written because the type is generic, which `#[derive(ContentOrd)]`
/// deliberately does not support.
impl<T: Ord + Clone> collomatique_state::ContentOrd for NonEmptyRangeInclusive<T> {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        collomatique_state::partial_order::discrete(self, other)
    }
}

/// An atom's content equivalence is `==` by construction, so a range may
/// be matched by equality inside containers.
impl<T: Ord + Clone> collomatique_state::ContentIdentity for NonEmptyRangeInclusive<T> {}

/// The total order used to key ranges in sorted containers: lexicographic
/// on the endpoint pair `(start, end)`. This is a storage order, not a
/// semantic one — it says nothing about set inclusion (see [`ContentOrd`]
/// above).
///
/// Hand-written because `#[derive(Ord)]` cannot apply here: the wrapped
/// [`RangeInclusive`] implements `Eq` and `Hash` but not `PartialOrd`.
///
/// [`ContentOrd`]: collomatique_state::ContentOrd
impl<T: Ord + Clone> Ord for NonEmptyRangeInclusive<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .start()
            .cmp(other.0.start())
            .then_with(|| self.0.end().cmp(other.0.end()))
    }
}

impl<T: Ord + Clone> PartialOrd for NonEmptyRangeInclusive<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord + Clone> std::ops::Deref for NonEmptyRangeInclusive<T> {
    type Target = RangeInclusive<T>;
    fn deref(&self) -> &RangeInclusive<T> {
        &self.0
    }
}

impl<T: Ord + Clone> From<NonEmptyRangeInclusive<T>> for RangeInclusive<T> {
    fn from(value: NonEmptyRangeInclusive<T>) -> Self {
        value.0
    }
}

impl<T: Ord + Clone> TryFrom<RangeInclusive<T>> for NonEmptyRangeInclusive<T> {
    type Error = EmptyRangeError;
    fn try_from(range: RangeInclusive<T>) -> Result<Self, EmptyRangeError> {
        NonEmptyRangeInclusive::new(range).ok_or(EmptyRangeError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_range_is_rejected() {
        assert_eq!(NonEmptyRangeInclusive::new(3..=2), None);
    }

    #[test]
    fn singleton_and_proper_ranges_are_accepted() {
        let singleton = NonEmptyRangeInclusive::new(2..=2).expect("non-empty");
        assert_eq!(*singleton.start(), 2);
        assert_eq!(*singleton.end(), 2);

        let proper = NonEmptyRangeInclusive::new(2..=3).expect("non-empty");
        assert_eq!(*proper.start(), 2);
        assert_eq!(*proper.end(), 3);
    }

    #[test]
    fn try_from_reports_empty_range() {
        assert_eq!(
            NonEmptyRangeInclusive::try_from(3..=2),
            Err(EmptyRangeError)
        );
        assert!(NonEmptyRangeInclusive::try_from(2..=3).is_ok());
    }

    #[test]
    fn order_is_lexicographic_on_endpoints() {
        let r = |a: u32, b: u32| NonEmptyRangeInclusive::new(a..=b).expect("non-empty");

        // The start dominates...
        assert!(r(1, 3) < r(2, 2));
        // ...and the end breaks ties.
        assert!(r(1, 2) < r(1, 3));
        assert_eq!(r(1, 2).cmp(&r(1, 2)), std::cmp::Ordering::Equal);
    }

    #[test]
    fn round_trips_through_range_inclusive() {
        let range = NonEmptyRangeInclusive::new(2..=3).expect("non-empty");
        let back: RangeInclusive<u32> = range.into();
        assert_eq!(back, 2..=3);
    }
}
