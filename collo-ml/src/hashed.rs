use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A wrapper that caches the hash of its inner value at construction time.
///
/// This is useful for types that are expensive to hash (e.g., deeply nested trees)
/// but are used as keys in `HashMap`/`HashSet`. The hash is computed once via
/// `DefaultHasher` and then fed into whatever `Hasher` the container provides.
///
/// Equality and ordering delegate to the inner value, not the cached hash.
pub struct Hashed<V> {
    inner: V,
    cached_hash: u64,
}

impl<V: Hash> Hashed<V> {
    pub fn new(inner: V) -> Self {
        let mut hasher = DefaultHasher::new();
        inner.hash(&mut hasher);
        let cached_hash = hasher.finish();
        Hashed { inner, cached_hash }
    }
}

impl<V> Hashed<V> {
    pub fn inner(&self) -> &V {
        &self.inner
    }

    pub fn into_inner(self) -> V {
        self.inner
    }
}

impl<V: Hash> Hash for Hashed<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cached_hash.hash(state);
    }
}

impl<V: PartialEq> PartialEq for Hashed<V> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<V: Eq> Eq for Hashed<V> {}

impl<V: PartialOrd> PartialOrd for Hashed<V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<V: Ord> Ord for Hashed<V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<V: fmt::Debug> fmt::Debug for Hashed<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<V: fmt::Display> fmt::Display for Hashed<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<V: Clone + Hash> Clone for Hashed<V> {
    fn clone(&self) -> Self {
        Hashed {
            inner: self.inner.clone(),
            cached_hash: self.cached_hash,
        }
    }
}

impl<V: Hash> From<V> for Hashed<V> {
    fn from(inner: V) -> Self {
        Hashed::new(inner)
    }
}

impl<V> Deref for Hashed<V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.inner
    }
}
