//! Keyed collections of the spec-2 format (spec §3)
//!
//! A keyed collection is a JSON array of items identified by a key; it is
//! sparse (any subset of keys may be present) but a duplicated key makes
//! the file invalid. These containers enforce the uniqueness rule at
//! construction time — their content is private so it cannot be mutated
//! into breaking the invariant — and are otherwise plain `Vec`
//! passthroughs.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::ops::Deref;

/// A row of a keyed collection, exposing its key
pub trait KeyedRow {
    type Key: Ord + Debug;

    fn key(&self) -> Self::Key;
}

/// Error when building a [KeyedVec] out of rows with a duplicated key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateKey<K>(pub K);

impl<K: Debug> std::fmt::Display for DuplicateKey<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "duplicate key {:?} in keyed collection", self.0)
    }
}

impl<K: Debug> std::error::Error for DuplicateKey<K> {}

/// A keyed collection: an array of rows with pairwise distinct keys
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct KeyedVec<R>(Vec<R>);

impl<R: KeyedRow> KeyedVec<R> {
    /// Build a keyed collection, checking that keys are pairwise
    /// distinct
    pub fn new(rows: Vec<R>) -> Result<Self, DuplicateKey<R::Key>> {
        let mut keys = BTreeSet::new();
        for row in &rows {
            if !keys.insert(row.key()) {
                return Err(DuplicateKey(row.key()));
            }
        }
        Ok(KeyedVec(rows))
    }
}

impl<R> KeyedVec<R> {
    // Part of the container API alongside `into_inner`; reads go
    // through `Deref` in the decoder, so only tests exercise it
    #[allow(dead_code)]
    pub fn inner(&self) -> &Vec<R> {
        &self.0
    }

    pub fn into_inner(self) -> Vec<R> {
        self.0
    }
}

impl<R> Deref for KeyedVec<R> {
    type Target = [R];

    fn deref(&self) -> &[R] {
        &self.0
    }
}

impl<R> Default for KeyedVec<R> {
    fn default() -> Self {
        KeyedVec(Vec::new())
    }
}

impl<'de, R: Deserialize<'de> + KeyedRow> Deserialize<'de> for KeyedVec<R> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = Vec::<R>::deserialize(deserializer)?;
        KeyedVec::new(rows).map_err(serde::de::Error::custom)
    }
}

/// Error when building a [UniqueVec] out of a duplicated element
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateElement<T>(pub T);

impl<T: Debug> std::fmt::Display for DuplicateElement<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "duplicate element {:?} in set", self.0)
    }
}

impl<T: Debug> std::error::Error for DuplicateElement<T> {}

/// A set encoded as an array: elements must be pairwise distinct
///
/// Used for id sets (`excluded_periods`, teacher `subjects`, assignment
/// `students`…) and group-number sets, where a silent dedup would hide
/// invalid input.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct UniqueVec<T>(Vec<T>);

impl<T: Ord + Clone> UniqueVec<T> {
    /// Build a set, checking that elements are pairwise distinct
    pub fn new(elements: Vec<T>) -> Result<Self, DuplicateElement<T>> {
        let mut seen = BTreeSet::new();
        for element in &elements {
            if !seen.insert(element) {
                return Err(DuplicateElement(element.clone()));
            }
        }
        Ok(UniqueVec(elements))
    }
}

impl<T> UniqueVec<T> {
    // Part of the container API alongside `into_inner`; reads go
    // through `Deref` in the decoder, so only tests exercise it
    #[allow(dead_code)]
    pub fn inner(&self) -> &Vec<T> {
        &self.0
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> Deref for UniqueVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T> Default for UniqueVec<T> {
    fn default() -> Self {
        UniqueVec(Vec::new())
    }
}

impl<'de, T: Deserialize<'de> + Ord + Clone + Debug> Deserialize<'de> for UniqueVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let elements = Vec::<T>::deserialize(deserializer)?;
        UniqueVec::new(elements).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestRow {
        id: u64,
        payload: String,
    }

    impl KeyedRow for TestRow {
        type Key = u64;

        fn key(&self) -> u64 {
            self.id
        }
    }

    #[test]
    fn keyed_vec_accepts_distinct_keys_and_preserves_order() {
        let value = json!([
            { "id": 3, "payload": "c" },
            { "id": 1, "payload": "a" }
        ]);
        let rows: KeyedVec<TestRow> = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(rows[0].id, 3);
        assert_eq!(rows[1].id, 1);
        assert_eq!(serde_json::to_value(&rows).unwrap(), value);
    }

    #[test]
    fn keyed_vec_rejects_duplicate_keys() {
        let value = json!([
            { "id": 1, "payload": "a" },
            { "id": 1, "payload": "b" }
        ]);
        assert!(serde_json::from_value::<KeyedVec<TestRow>>(value).is_err());
    }

    #[test]
    fn keyed_vec_new_checks_the_invariant() {
        let row = |id: u64| TestRow {
            id,
            payload: String::new(),
        };

        let rows = KeyedVec::new(vec![row(3), row(1)]).unwrap();
        assert_eq!(rows.inner().len(), 2);
        assert_eq!(rows.into_inner().len(), 2);

        assert_eq!(KeyedVec::new(vec![row(1), row(1)]), Err(DuplicateKey(1)));
    }

    #[test]
    fn unique_vec_accepts_distinct_elements() {
        let elements: UniqueVec<u64> = serde_json::from_value(json!([3, 1, 2])).unwrap();
        assert_eq!(*elements, [3, 1, 2]);
    }

    #[test]
    fn unique_vec_rejects_duplicate_elements() {
        assert!(serde_json::from_value::<UniqueVec<u64>>(json!([1, 2, 1])).is_err());
    }

    #[test]
    fn unique_vec_new_checks_the_invariant() {
        let elements = UniqueVec::new(vec![3u64, 1, 2]).unwrap();
        assert_eq!(elements.inner(), &vec![3, 1, 2]);
        assert_eq!(elements.into_inner(), vec![3, 1, 2]);

        assert_eq!(UniqueVec::new(vec![1u64, 2, 1]), Err(DuplicateElement(1)));
    }
}
