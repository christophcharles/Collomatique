//! Tables module
//!
//! This module defines generic id-indexed containers for entity storage
//! in [crate::InMemoryData] implementations:
//! - [Table] for entities without a meaningful order (id-sorted storage)
//! - [OrderedTable] for entities whose relative order is user-visible data
//!
//! Both containers guarantee primary-key uniqueness: a table never holds
//! two entries with the same ID. [Table] gets this structurally from its
//! map backend; [OrderedTable] enforces it through fallible construction
//! and insertion.
//!
//! The inner representation is deliberately opaque so it can change later
//! without touching consumer code. Consumers should only ever read tables;
//! the mutating methods are for the state layer itself.
//!
//! The serialized form is stable and independent of the opaque backend:
//! both [Table] and [OrderedTable] serialize exactly like a `Vec<(I, T)>` — a
//! sequence of `(id, value)` pairs in table order. (A sequence rather than a map
//! so that composite keys like `(PeriodId, SubjectId)`, which a JSON object
//! cannot key on, round-trip just as well as scalar ids.)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Key bound for [Table]
///
/// [Table] is `BTreeMap`-backed, so its key only needs to be cheaply copyable
/// and totally ordered. Every entity [`Id`](crate::ids::Id) satisfies this (its
/// supertraits already include `Copy + Ord`), and so does any tuple of ids —
/// which is what lets a junction table use a composite key like
/// `(PeriodId, SubjectId)`.
pub trait Key: Copy + Ord {}
impl<T: Copy + Ord> Key for T {}

/// Key bound for [OrderedTable]
///
/// [OrderedTable] is `Vec`-backed and finds entries by linear `==` scan, so its
/// key needs even less than [Key]: only `Copy + Eq`. `Debug` is required so
/// [DuplicatedIdError] can format the offending key.
pub trait OrderedKey: Copy + Eq + std::fmt::Debug {}
impl<T: Copy + Eq + std::fmt::Debug> OrderedKey for T {}

/// Error returned when an operation would insert an ID already present in the table
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated ID {0:?} in table")]
pub struct DuplicatedIdError<I: OrderedKey>(pub I);

/// Id-indexed table without user-visible ordering
///
/// Entries are stored (and iterated) in ID order. Primary-key uniqueness
/// is structural: inserting an existing ID replaces the previous value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table<I: Key, T> {
    inner: BTreeMap<I, T>,
}

/// Serializes as a sequence of `(id, value)` pairs in ID order.
///
/// Not a map: a JSON object cannot be keyed on a composite key like
/// `(PeriodId, SubjectId)`, so a sequence is used for every key type uniformly
/// (matching [OrderedTable]).
impl<I: Key + Serialize, T: Serialize> Serialize for Table<I, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.inner.iter())
    }
}

impl<'de, I, T> Deserialize<'de> for Table<I, T>
where
    I: Key + Deserialize<'de>,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<(I, T)>::deserialize(deserializer)?;
        Ok(Table {
            inner: entries.into_iter().collect(),
        })
    }
}

impl<I: Key, T> Default for Table<I, T> {
    fn default() -> Self {
        Table {
            inner: BTreeMap::new(),
        }
    }
}

impl<I: Key, T> Table<I, T> {
    /// Creates an empty table
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the entry with the given ID, if any
    pub fn get(&self, id: &I) -> Option<&T> {
        self.inner.get(id)
    }

    /// Returns `true` if the table contains an entry with the given ID
    pub fn contains(&self, id: &I) -> bool {
        self.inner.contains_key(id)
    }

    /// Iterates over the IDs in the table, in ID order
    ///
    /// Defined inherently so it shadows the `Deref` [`BTreeMap::keys`], yielding
    /// owned IDs (all IDs are `Copy`) rather than references.
    pub fn keys(&self) -> impl Iterator<Item = I> + '_ {
        self.inner.keys().copied()
    }

    /// Iterates over the `(id, value)` entries in the table, in ID order
    ///
    /// Defined inherently so it shadows the `Deref` [`BTreeMap::iter`], yielding
    /// an owned ID with each value rather than a reference pair.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.inner.iter().map(|(&id, value)| (id, value))
    }

    /// Iterates over the values in the table, in ID order
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.inner.values()
    }

    /// Returns the number of entries in the table
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the table has no entries
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Inserts an entry, returning the previous value for this ID, if any
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    pub fn insert(&mut self, id: I, value: T) -> Option<T> {
        self.inner.insert(id, value)
    }

    /// Removes the entry with the given ID, returning its value, if any
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    pub fn remove(&mut self, id: &I) -> Option<T> {
        self.inner.remove(id)
    }

    /// Returns a mutable reference to the entry with the given ID, if any
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    pub fn get_mut(&mut self, id: &I) -> Option<&mut T> {
        self.inner.get_mut(id)
    }

    /// Iterates over mutable references to the values, in ID order
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.inner.values_mut()
    }
}

impl<I: Key, T: Clone> Table<I, T> {
    /// Copies the table into a plain `BTreeMap`
    ///
    /// Compatibility-window helper for consumers that store their own
    /// `BTreeMap` copy of table data. Same lifecycle as the [`Deref`](std::ops::Deref)
    /// impl: scheduled for removal once consumers move to join patterns —
    /// do not use in new code.
    pub fn to_map(&self) -> BTreeMap<I, T> {
        self.inner.clone()
    }
}

impl<I: Key, T> From<BTreeMap<I, T>> for Table<I, T> {
    fn from(inner: BTreeMap<I, T>) -> Self {
        Table { inner }
    }
}

impl<I: Key, T> FromIterator<(I, T)> for Table<I, T> {
    fn from_iter<It: IntoIterator<Item = (I, T)>>(iter: It) -> Self {
        Table {
            inner: iter.into_iter().collect(),
        }
    }
}

/// Consuming iterator over the `(id, value)` entries, in ID order.
///
/// This is the dual of [`FromIterator`]: it drains the table without exposing
/// the backend representation, so it is part of the stable API (unlike the
/// `Deref` and `&Table` iteration compatibility shims).
impl<I: Key, T> IntoIterator for Table<I, T> {
    type Item = (I, T);
    type IntoIter = std::collections::btree_map::IntoIter<I, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// Compatibility window only: lets existing call sites keep using the map API
/// during the migration. Scheduled for removal — do not use in new code.
impl<I: Key, T> std::ops::Deref for Table<I, T> {
    type Target = BTreeMap<I, T>;

    fn deref(&self) -> &BTreeMap<I, T> {
        &self.inner
    }
}

/// Id-indexed table with user-visible ordering
///
/// Entries keep the order they were built/inserted in, and that order is
/// meaningful data (e.g. subject or period ordering). Primary-key uniqueness
/// is an enforced invariant: construction and insertion are fallible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct OrderedTable<I: OrderedKey, T> {
    inner: Vec<(I, T)>,
}

impl<I: OrderedKey, T> Default for OrderedTable<I, T> {
    fn default() -> Self {
        OrderedTable { inner: Vec::new() }
    }
}

impl<I: OrderedKey, T> TryFrom<Vec<(I, T)>> for OrderedTable<I, T> {
    type Error = DuplicatedIdError<I>;

    fn try_from(inner: Vec<(I, T)>) -> Result<Self, DuplicatedIdError<I>> {
        let mut seen: Vec<I> = Vec::new();
        for (id, _) in &inner {
            if seen.contains(id) {
                return Err(DuplicatedIdError(*id));
            }
            seen.push(*id);
        }
        Ok(OrderedTable { inner })
    }
}

impl<'de, I, T> Deserialize<'de> for OrderedTable<I, T>
where
    I: OrderedKey + Deserialize<'de>,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<(I, T)>::deserialize(deserializer)?;
        OrderedTable::try_from(entries).map_err(serde::de::Error::custom)
    }
}

impl<I: OrderedKey, T> OrderedTable<I, T> {
    /// Creates an empty table
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the entry with the given ID, if any
    pub fn get(&self, id: &I) -> Option<&T> {
        self.inner
            .iter()
            .find(|(entry_id, _)| entry_id == id)
            .map(|(_, value)| value)
    }

    /// Returns `true` if the table contains an entry with the given ID
    pub fn contains(&self, id: &I) -> bool {
        self.inner.iter().any(|(entry_id, _)| entry_id == id)
    }

    /// Returns the `(id, value)` entry at the given position, if any
    pub fn get_at(&self, pos: usize) -> Option<(I, &T)> {
        self.inner.get(pos).map(|(id, value)| (*id, value))
    }

    /// Returns the position of the entry with the given ID, if any
    pub fn position_of(&self, id: &I) -> Option<usize> {
        self.inner.iter().position(|(entry_id, _)| entry_id == id)
    }

    /// Iterates over the IDs in the table, in table order
    ///
    /// Defined inherently so it shadows the `Deref` slice API, yielding owned
    /// IDs (all IDs are `Copy`).
    pub fn keys(&self) -> impl Iterator<Item = I> + '_ {
        self.inner.iter().map(|(id, _)| *id)
    }

    /// Iterates over the `(id, value)` entries in the table, in table order
    ///
    /// Defined inherently so it shadows the `Deref` slice [`slice::iter`],
    /// yielding an owned ID with each value rather than a reference to the
    /// backing `(id, value)` pair.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.inner.iter().map(|(id, value)| (*id, value))
    }

    /// Iterates over the values in the table, in table order
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.inner.iter().map(|(_, value)| value)
    }

    /// Returns the number of entries in the table
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the table has no entries
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Inserts an entry at the given position (`pos == len` appends)
    ///
    /// Fails if the ID is already present in the table.
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    ///
    /// # Panics
    ///
    /// Panics if `pos > len`.
    pub fn insert_at(&mut self, pos: usize, id: I, value: T) -> Result<(), DuplicatedIdError<I>> {
        if self.contains(&id) {
            return Err(DuplicatedIdError(id));
        }
        self.inner.insert(pos, (id, value));
        Ok(())
    }

    /// Removes and returns the entry at the given position
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    ///
    /// # Panics
    ///
    /// Panics if `pos >= len`.
    pub fn remove_at(&mut self, pos: usize) -> (I, T) {
        self.inner.remove(pos)
    }

    /// Replaces the value at the given position (the ID is unchanged),
    /// returning the previous value
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    ///
    /// # Panics
    ///
    /// Panics if `pos >= len`.
    pub fn replace_value_at(&mut self, pos: usize, value: T) -> T {
        std::mem::replace(&mut self.inner[pos].1, value)
    }

    /// Moves the entry at position `from` so that it ends up at position `to`
    ///
    /// This is a state-layer-internal mutator: consumer code should treat
    /// tables as read-only.
    ///
    /// # Panics
    ///
    /// Panics if `from >= len` or `to >= len`.
    pub fn move_entry(&mut self, from: usize, to: usize) {
        let entry = self.inner.remove(from);
        self.inner.insert(to, entry);
    }
}

/// Consuming iterator over the `(id, value)` entries, in table order.
///
/// This is the dual of the [`TryFrom<Vec<(I, T)>>`](OrderedTable::try_from)
/// constructor: it drains the table without exposing the backend
/// representation, so it is part of the stable API (unlike the `Deref` and
/// `&OrderedTable` iteration compatibility shims).
impl<I: OrderedKey, T> IntoIterator for OrderedTable<I, T> {
    type Item = (I, T);
    type IntoIter = std::vec::IntoIter<(I, T)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<I: OrderedKey, T: Clone> OrderedTable<I, T> {
    /// Copies the table into a plain `Vec<(I, T)>`, in table order
    ///
    /// Compatibility-window helper for consumers that store their own
    /// `Vec` copy of ordered table data. Same lifecycle as the
    /// [`Deref`](std::ops::Deref) impl: scheduled for removal once consumers
    /// move to join patterns — do not use in new code.
    pub fn to_vec(&self) -> Vec<(I, T)> {
        self.inner.clone()
    }
}

/// Compatibility window only: lets existing call sites keep using the slice API
/// during the migration. Scheduled for removal — do not use in new code.
impl<I: OrderedKey, T> std::ops::Deref for OrderedTable<I, T> {
    type Target = [(I, T)];

    fn deref(&self) -> &[(I, T)] {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    struct ToyId(u64);

    impl Id for ToyId {
        fn inner(&self) -> u64 {
            self.0
        }

        unsafe fn new(value: u64) -> ToyId {
            ToyId(value)
        }
    }

    fn table_fixture() -> (Table<ToyId, String>, BTreeMap<ToyId, String>) {
        let map = BTreeMap::from([
            (ToyId(3), "three".to_string()),
            (ToyId(1), "one".to_string()),
            (ToyId(2), "two".to_string()),
        ]);
        (Table::from(map.clone()), map)
    }

    fn ordered_fixture() -> (OrderedTable<ToyId, String>, Vec<(ToyId, String)>) {
        let entries = vec![
            (ToyId(3), "three".to_string()),
            (ToyId(1), "one".to_string()),
            (ToyId(2), "two".to_string()),
        ];
        (
            OrderedTable::try_from(entries.clone()).expect("no duplicates"),
            entries,
        )
    }

    #[test]
    fn table_wire_format_matches_vec_of_pairs() {
        let (table, map) = table_fixture();

        // Serializes as a sequence of `(id, value)` pairs in ID order, not as a
        // map — so composite keys round-trip too (see `Table`'s serde impls).
        let expected: Vec<(ToyId, String)> = map.iter().map(|(&id, v)| (id, v.clone())).collect();

        assert_eq!(
            serde_json::to_value(&table).expect("table serializes"),
            serde_json::to_value(&expected).expect("vec serializes"),
        );
    }

    #[test]
    fn table_round_trips_through_serde() {
        let (table, _) = table_fixture();

        let json = serde_json::to_string(&table).expect("table serializes");
        let back: Table<ToyId, String> = serde_json::from_str(&json).expect("table deserializes");

        assert_eq!(back, table);
    }

    #[test]
    fn table_iterates_in_id_order() {
        let (table, _) = table_fixture();

        let ids: Vec<_> = table.keys().collect();

        assert_eq!(ids, vec![ToyId(1), ToyId(2), ToyId(3)]);
    }

    #[test]
    fn table_insert_replaces_existing_entry() {
        let (mut table, _) = table_fixture();

        let previous = table.insert(ToyId(2), "TWO".to_string());

        assert_eq!(previous, Some("two".to_string()));
        assert_eq!(table.get(&ToyId(2)), Some(&"TWO".to_string()));
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn table_to_map_copies_backing_store() {
        let (table, map) = table_fixture();

        assert_eq!(table.to_map(), map);
    }

    #[test]
    fn ordered_table_wire_format_matches_vec() {
        let (table, entries) = ordered_fixture();

        assert_eq!(
            serde_json::to_value(&table).expect("table serializes"),
            serde_json::to_value(&entries).expect("vec serializes"),
        );
    }

    #[test]
    fn ordered_table_round_trips_through_serde() {
        let (table, _) = ordered_fixture();

        let json = serde_json::to_string(&table).expect("table serializes");
        let back: OrderedTable<ToyId, String> =
            serde_json::from_str(&json).expect("table deserializes");

        assert_eq!(back, table);
    }

    #[test]
    fn ordered_table_to_vec_copies_entries_in_order() {
        let (table, entries) = ordered_fixture();

        assert_eq!(table.to_vec(), entries);
    }

    #[test]
    fn ordered_table_preserves_insertion_order() {
        let (table, entries) = ordered_fixture();

        let ids: Vec<_> = table.keys().collect();

        assert_eq!(ids, entries.iter().map(|(id, _)| *id).collect::<Vec<_>>());
        assert_eq!(table.get_at(0), Some((ToyId(3), &"three".to_string())));
        assert_eq!(table.position_of(&ToyId(2)), Some(2));
    }

    #[test]
    fn ordered_table_try_from_rejects_duplicated_ids() {
        let entries = vec![
            (ToyId(1), "one".to_string()),
            (ToyId(2), "two".to_string()),
            (ToyId(1), "one again".to_string()),
        ];

        let result = OrderedTable::try_from(entries);

        assert_eq!(result.err(), Some(DuplicatedIdError(ToyId(1))));
    }

    #[test]
    fn ordered_table_deserialize_rejects_duplicated_ids() {
        let json = r#"[[1, "one"], [2, "two"], [1, "one again"]]"#;

        let result: Result<OrderedTable<ToyId, String>, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    #[test]
    fn ordered_table_insert_at_rejects_existing_id() {
        let (mut table, entries) = ordered_fixture();

        let result = table.insert_at(0, ToyId(2), "again".to_string());

        assert_eq!(result, Err(DuplicatedIdError(ToyId(2))));
        assert_eq!(
            table.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            entries.iter().map(|(id, _)| *id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ordered_table_insert_at_len_appends() {
        let (mut table, _) = ordered_fixture();

        table
            .insert_at(table.len(), ToyId(4), "four".to_string())
            .expect("new id");

        assert_eq!(table.get_at(3), Some((ToyId(4), &"four".to_string())));
    }

    #[test]
    fn ordered_table_positional_mutators() {
        let (mut table, _) = ordered_fixture();

        let removed = table.remove_at(1);
        assert_eq!(removed, (ToyId(1), "one".to_string()));
        assert_eq!(table.keys().collect::<Vec<_>>(), vec![ToyId(3), ToyId(2)]);

        let previous = table.replace_value_at(0, "THREE".to_string());
        assert_eq!(previous, "three".to_string());
        assert_eq!(table.get(&ToyId(3)), Some(&"THREE".to_string()));

        table.move_entry(0, 1);
        assert_eq!(table.keys().collect::<Vec<_>>(), vec![ToyId(2), ToyId(3)]);
    }

    #[test]
    fn ordered_table_move_entry_lands_at_target_position() {
        let entries = vec![
            (ToyId(1), "a".to_string()),
            (ToyId(2), "b".to_string()),
            (ToyId(3), "c".to_string()),
            (ToyId(4), "d".to_string()),
        ];
        let mut table = OrderedTable::try_from(entries).expect("no duplicates");

        table.move_entry(0, 2);
        assert_eq!(
            table.keys().collect::<Vec<_>>(),
            vec![ToyId(2), ToyId(3), ToyId(1), ToyId(4)]
        );

        table.move_entry(2, 0);
        assert_eq!(
            table.keys().collect::<Vec<_>>(),
            vec![ToyId(1), ToyId(2), ToyId(3), ToyId(4)]
        );
    }

    fn composite_fixture() -> Table<(ToyId, ToyId), String> {
        let mut table: Table<(ToyId, ToyId), String> = Table::new();
        table.insert((ToyId(1), ToyId(2)), "a".to_string());
        table.insert((ToyId(1), ToyId(1)), "b".to_string());
        table.insert((ToyId(2), ToyId(1)), "c".to_string());
        table
    }

    #[test]
    fn table_supports_composite_tuple_key() {
        // A tuple of ids is `Copy + Ord`, so the `Key` blanket impl makes it a
        // valid `Table` key — this is what junction tables like assignments and
        // associations rely on.
        let table = composite_fixture();

        assert_eq!(table.get(&(ToyId(1), ToyId(2))), Some(&"a".to_string()));
        assert!(table.contains(&(ToyId(2), ToyId(1))));
        assert_eq!(table.get(&(ToyId(2), ToyId(2))), None);

        // Iterates in tuple (lexicographic) key order.
        assert_eq!(
            table.keys().collect::<Vec<_>>(),
            vec![
                (ToyId(1), ToyId(1)),
                (ToyId(1), ToyId(2)),
                (ToyId(2), ToyId(1)),
            ]
        );
    }

    #[test]
    fn table_composite_tuple_key_round_trips_through_serde() {
        // The array-of-pairs wire format lets a composite key survive JSON,
        // which a map (object) form cannot — this is exercised for real when
        // `InnerData` (carrying these tables) is round-tripped over rpc.
        let table = composite_fixture();

        let json = serde_json::to_string(&table).expect("table serializes");
        let back: Table<(ToyId, ToyId), String> =
            serde_json::from_str(&json).expect("table deserializes");

        assert_eq!(back, table);
    }
}
