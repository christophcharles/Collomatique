/// Transform an iterator of `T` values into a collection of `Self` values.
///
/// The identity impl allows any type to pass through its own values.
/// Container types like `Option<T>` implement this to wrap inner values.
pub trait EnumerateFrom<T>: Sized {
    fn enumerate_from(inner: impl IntoIterator<Item = T>) -> Vec<Self>;
}

/// Enumerate all possible values of a type without external input.
///
/// Only implement this for types with a known finite set of values
/// (e.g. `bool`). The invariant is that `enumerate_all()` returns
/// every valid value — no fixing is needed for these types.
pub trait EnumerateAll: Sized {
    fn enumerate_all() -> Vec<Self>;
}

// ---------------------------------------------------------------------------
// EnumerateFrom impls
// ---------------------------------------------------------------------------

impl<T> EnumerateFrom<T> for T {
    fn enumerate_from(inner: impl IntoIterator<Item = T>) -> Vec<Self> {
        inner.into_iter().collect()
    }
}

impl<T> EnumerateFrom<T> for Option<T> {
    fn enumerate_from(inner: impl IntoIterator<Item = T>) -> Vec<Self> {
        std::iter::once(None)
            .chain(inner.into_iter().map(Some))
            .collect()
    }
}

impl<T> EnumerateFrom<T> for Option<Option<T>> {
    fn enumerate_from(inner: impl IntoIterator<Item = T>) -> Vec<Self> {
        let middle = <Option<T> as EnumerateFrom<T>>::enumerate_from(inner);
        <Self as EnumerateFrom<Option<T>>>::enumerate_from(middle)
    }
}

impl<T> EnumerateFrom<T> for Option<Option<Option<T>>> {
    fn enumerate_from(inner: impl IntoIterator<Item = T>) -> Vec<Self> {
        let middle = <Option<Option<T>> as EnumerateFrom<T>>::enumerate_from(inner);
        <Self as EnumerateFrom<Option<Option<T>>>>::enumerate_from(middle)
    }
}

// ---------------------------------------------------------------------------
// EnumerateAll impls
// ---------------------------------------------------------------------------

impl EnumerateAll for bool {
    fn enumerate_all() -> Vec<Self> {
        vec![false, true]
    }
}

impl<T: EnumerateAll> EnumerateAll for Option<T> {
    fn enumerate_all() -> Vec<Self> {
        <Self as EnumerateFrom<T>>::enumerate_from(T::enumerate_all())
    }
}
