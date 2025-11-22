use std::cell::{Ref, RefMut};
use std::marker::PhantomData;

/// The interface is very similar to [`std::cell::RefCell`].
/// Ownership is calculated at runtime, so you have to borrow the
/// value explicitly which might panic if done incorrectly.
#[derive(Debug, Clone)]
pub struct ColumnItem<T> {
    inner: libadwaita::glib::BoxedAnyObject,
    _ty: PhantomData<*const T>,
}

impl<T: 'static> ColumnItem<T> {
    pub(super) fn new(inner: libadwaita::glib::BoxedAnyObject) -> Self {
        Self {
            inner,
            _ty: PhantomData,
        }
    }

    // rustdoc-stripper-ignore-next
    /// Immutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple
    /// immutable borrows can be taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    ///
    /// For a non-panicking variant, use
    /// [`try_borrow`](#method.try_borrow).
    #[must_use]
    pub fn borrow(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    // rustdoc-stripper-ignore-next
    /// Mutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `RefMut` or all `RefMut`s derived
    /// from it exit scope. The value cannot be borrowed while this borrow is
    /// active.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// For a non-panicking variant, use
    /// [`try_borrow_mut`](#method.try_borrow_mut).
    #[must_use]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
