//! References module
//!
//! This module defines the generic [References] trait through which a value
//! enumerates the typed IDs ("foreign keys") it contains. It is the runtime
//! side of the relationship registry: entity structs declare their FK fields
//! once with `#[derive(References)]` and generic code walks them without
//! knowing the concrete shapes.

use std::collections::BTreeSet;

/// Trait for values containing references to entities
///
/// `K` is a union type covering every ID kind a value might reference
/// (each concrete ID converts into it via `From`). Leaf implementations on
/// ID newtypes come from `#[derive(EntityId)]`; struct implementations from
/// `#[derive(References)]`. Manual implementations should only be needed
/// for genuinely irregular shapes.
pub trait References<K> {
    /// Calls `f` on every entity ID referenced by `self`
    ///
    /// Fields are visited in declaration order, depth-first through nested
    /// structures and containers.
    fn for_each_ref(&self, f: &mut dyn FnMut(K));
}

impl<K, T: References<K>> References<K> for Option<T> {
    fn for_each_ref(&self, f: &mut dyn FnMut(K)) {
        if let Some(value) = self {
            value.for_each_ref(f);
        }
    }
}

impl<K, T: References<K>> References<K> for Vec<T> {
    fn for_each_ref(&self, f: &mut dyn FnMut(K)) {
        for value in self {
            value.for_each_ref(f);
        }
    }
}

impl<K, T: References<K>> References<K> for BTreeSet<T> {
    fn for_each_ref(&self, f: &mut dyn FnMut(K)) {
        for value in self {
            value.for_each_ref(f);
        }
    }
}
