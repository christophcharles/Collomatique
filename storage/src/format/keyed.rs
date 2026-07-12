//! Keyed collections of the spec-2 format (spec §3)
//!
//! A keyed collection is a JSON array of items identified by a key; it is
//! sparse (any subset of keys may be present) but a duplicated key makes
//! the file invalid. These containers enforce the uniqueness rule at
//! deserialization time; everything else is a plain `Vec` passthrough.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt::Debug;

/// A row of a keyed collection, exposing its key
pub trait KeyedRow {
    type Key: Ord + Debug;

    fn key(&self) -> Self::Key;
}

/// A keyed collection: an array of rows with pairwise distinct keys
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct KeyedVec<R>(pub Vec<R>);

impl<R> Default for KeyedVec<R> {
    fn default() -> Self {
        KeyedVec(Vec::new())
    }
}

impl<'de, R: Deserialize<'de> + KeyedRow> Deserialize<'de> for KeyedVec<R> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = Vec::<R>::deserialize(deserializer)?;
        let mut keys = BTreeSet::new();
        for row in &rows {
            let key = row.key();
            if !keys.insert(row.key()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate key {key:?} in keyed collection"
                )));
            }
        }
        Ok(KeyedVec(rows))
    }
}

/// A set encoded as an array: elements must be pairwise distinct
///
/// Used for id sets (`excluded_periods`, teacher `subjects`, assignment
/// `students`…) and group-number sets, where a silent dedup would hide
/// invalid input.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct UniqueVec<T>(pub Vec<T>);

impl<T> Default for UniqueVec<T> {
    fn default() -> Self {
        UniqueVec(Vec::new())
    }
}

impl<'de, T: Deserialize<'de> + Ord + Debug> Deserialize<'de> for UniqueVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let elements = Vec::<T>::deserialize(deserializer)?;
        let mut seen = BTreeSet::new();
        for element in &elements {
            if !seen.insert(element) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate element {element:?} in set"
                )));
            }
        }
        Ok(UniqueVec(elements))
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
        assert_eq!(rows.0[0].id, 3);
        assert_eq!(rows.0[1].id, 1);
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
    fn unique_vec_accepts_distinct_elements() {
        let elements: UniqueVec<u64> = serde_json::from_value(json!([3, 1, 2])).unwrap();
        assert_eq!(elements.0, vec![3, 1, 2]);
    }

    #[test]
    fn unique_vec_rejects_duplicate_elements() {
        assert!(serde_json::from_value::<UniqueVec<u64>>(json!([1, 2, 1])).is_err());
    }
}
