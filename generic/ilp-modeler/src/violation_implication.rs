use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub trait ViolationImplication {
    type CategoryKey: Hash + Eq;

    fn violation_category(&self) -> Option<Self::CategoryKey>;
    fn violation_implies(&self, other: &Self) -> bool;
}

impl<T: ViolationImplication> ViolationImplication for &T {
    type CategoryKey = T::CategoryKey;

    fn violation_category(&self) -> Option<Self::CategoryKey> {
        (*self).violation_category()
    }

    fn violation_implies(&self, other: &&T) -> bool {
        (*self).violation_implies(*other)
    }
}

pub struct MinimalBlame<T: ViolationImplication + Hash + Eq> {
    categorized: HashMap<T::CategoryKey, HashSet<T>>,
    uncategorized: HashSet<T>,
}

impl<T: ViolationImplication + Hash + Eq> MinimalBlame<T> {
    pub fn new() -> Self {
        MinimalBlame {
            categorized: HashMap::new(),
            uncategorized: HashSet::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        match item.violation_category() {
            None => {
                self.uncategorized.insert(item);
            }
            Some(key) => {
                let bucket = self.categorized.entry(key).or_default();

                for existing in bucket.iter() {
                    if existing.violation_implies(&item) && !item.violation_implies(existing) {
                        return;
                    }
                }

                bucket.retain(|existing| {
                    !(item.violation_implies(existing) && !existing.violation_implies(&item))
                });
                bucket.insert(item);
            }
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len());
        result.extend(self.uncategorized);
        for (_, bucket) in self.categorized {
            result.extend(bucket);
        }
        result
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.uncategorized
            .iter()
            .chain(self.categorized.values().flat_map(|bucket| bucket.iter()))
    }

    pub fn len(&self) -> usize {
        self.uncategorized.len()
            + self
                .categorized
                .values()
                .map(|bucket| bucket.len())
                .sum::<usize>()
    }

    pub fn is_empty(&self) -> bool {
        self.uncategorized.is_empty() && self.categorized.values().all(|b| b.is_empty())
    }
}

impl<T: ViolationImplication + Hash + Eq> FromIterator<T> for MinimalBlame<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut blame = MinimalBlame::new();
        for item in iter {
            blame.push(item);
        }
        blame
    }
}

impl<T: ViolationImplication + Hash + Eq> Default for MinimalBlame<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct Interval {
        group: u32,
        lo: u32,
        hi: u32,
    }

    impl ViolationImplication for Interval {
        type CategoryKey = u32;

        fn violation_category(&self) -> Option<Self::CategoryKey> {
            Some(self.group)
        }

        fn violation_implies(&self, other: &Self) -> bool {
            self.group == other.group && self.lo >= other.lo && self.hi <= other.hi
        }
    }

    fn iv(group: u32, lo: u32, hi: u32) -> Interval {
        Interval { group, lo, hi }
    }

    #[test]
    fn empty() {
        let blame: MinimalBlame<Interval> = MinimalBlame::new();
        assert!(blame.is_empty());
        assert_eq!(blame.len(), 0);
        assert_eq!(blame.into_vec().len(), 0);
    }

    #[test]
    fn single_element() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 1, 5));
        assert_eq!(blame.len(), 1);
    }

    #[test]
    fn subset_discards_superset() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 0, 10));
        blame.push(iv(0, 2, 5));
        assert_eq!(blame.len(), 1);
        assert!(blame.iter().any(|i| i.lo == 2 && i.hi == 5));
    }

    #[test]
    fn superset_discarded_by_existing_subset() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 2, 5));
        blame.push(iv(0, 0, 10));
        assert_eq!(blame.len(), 1);
        assert!(blame.iter().any(|i| i.lo == 2 && i.hi == 5));
    }

    #[test]
    fn incomparable_kept() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 0, 5));
        blame.push(iv(0, 3, 8));
        assert_eq!(blame.len(), 2);
    }

    #[test]
    fn different_groups_kept() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 2, 5));
        blame.push(iv(1, 0, 10));
        assert_eq!(blame.len(), 2);
    }

    #[test]
    fn chain_keeps_only_minimal() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 0, 10));
        blame.push(iv(0, 1, 9));
        blame.push(iv(0, 2, 8));
        assert_eq!(blame.len(), 1);
        assert!(blame.iter().any(|i| i.lo == 2 && i.hi == 8));
    }

    #[test]
    fn exact_duplicates_deduped() {
        let mut blame = MinimalBlame::new();
        blame.push(iv(0, 1, 5));
        blame.push(iv(0, 1, 5));
        blame.push(iv(0, 1, 5));
        assert_eq!(blame.len(), 1);
    }

    #[test]
    fn equivalent_but_distinct_kept() {
        #[derive(Debug, Clone, Hash, PartialEq, Eq)]
        struct Tagged {
            group: u32,
            value: u32,
            tag: &'static str,
        }

        impl ViolationImplication for Tagged {
            type CategoryKey = u32;

            fn violation_category(&self) -> Option<Self::CategoryKey> {
                Some(self.group)
            }

            fn violation_implies(&self, other: &Self) -> bool {
                self.group == other.group && self.value == other.value
            }
        }

        let mut blame = MinimalBlame::new();
        blame.push(Tagged {
            group: 0,
            value: 1,
            tag: "alpha",
        });
        blame.push(Tagged {
            group: 0,
            value: 1,
            tag: "beta",
        });
        assert_eq!(blame.len(), 2);
    }

    #[test]
    fn from_iterator() {
        let items = vec![iv(0, 0, 10), iv(0, 2, 5), iv(1, 0, 3)];
        let blame: MinimalBlame<_> = items.into_iter().collect();
        assert_eq!(blame.len(), 2);
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct Uncategorized(u32);

    impl ViolationImplication for Uncategorized {
        type CategoryKey = u32;

        fn violation_category(&self) -> Option<Self::CategoryKey> {
            None
        }

        fn violation_implies(&self, other: &Self) -> bool {
            self == other
        }
    }

    #[test]
    fn uncategorized_deduped() {
        let mut blame = MinimalBlame::new();
        blame.push(Uncategorized(1));
        blame.push(Uncategorized(1));
        blame.push(Uncategorized(2));
        assert_eq!(blame.len(), 2);
    }

    #[test]
    fn uncategorized_all_distinct_kept() {
        let mut blame = MinimalBlame::new();
        blame.push(Uncategorized(1));
        blame.push(Uncategorized(2));
        blame.push(Uncategorized(3));
        assert_eq!(blame.len(), 3);
    }

    #[test]
    fn mixed_categorized_and_uncategorized() {
        #[derive(Debug, Clone, Hash, PartialEq, Eq)]
        enum Mixed {
            Cat(Interval),
            Uncat(u32),
        }

        impl ViolationImplication for Mixed {
            type CategoryKey = u32;

            fn violation_category(&self) -> Option<Self::CategoryKey> {
                match self {
                    Mixed::Cat(i) => Some(i.group),
                    Mixed::Uncat(_) => None,
                }
            }

            fn violation_implies(&self, other: &Self) -> bool {
                match (self, other) {
                    (Mixed::Cat(a), Mixed::Cat(b)) => a.violation_implies(b),
                    _ => self == other,
                }
            }
        }

        let mut blame = MinimalBlame::new();
        blame.push(Mixed::Cat(iv(0, 0, 10)));
        blame.push(Mixed::Uncat(42));
        blame.push(Mixed::Cat(iv(0, 2, 5)));
        blame.push(Mixed::Uncat(42));
        assert_eq!(blame.len(), 2);
    }

    #[test]
    fn new_item_removes_multiple_supersets() {
        let mut blame = MinimalBlame::new();
        // Three pairwise-incomparable intervals
        blame.push(iv(0, 0, 5));
        blame.push(iv(0, 3, 8));
        blame.push(iv(0, 6, 11));
        assert_eq!(blame.len(), 3);
        // [3, 5] implies [0, 5] and [3, 8] (subset of both), but not [6, 11]
        blame.push(iv(0, 3, 5));
        assert_eq!(blame.len(), 2);
        assert!(blame.iter().any(|i| i.lo == 3 && i.hi == 5));
        assert!(blame.iter().any(|i| i.lo == 6 && i.hi == 11));
    }
}
