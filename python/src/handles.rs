//! What every handle class is built out of
//!
//! A handle is a live, read-only view bound to `(document, id)` and holding
//! nothing else (`docs/python/handle_api.md` §2.2). Every attribute access
//! borrows the document, resolves the id, reads and lets go, so a handle read
//! always sees the current state — through undo, redo and transactions alike.
//!
//! This module holds the three pieces every handle class needs: the
//! borrow-resolve-read helper, the `StaleHandleError` that names the kind and
//! the id it could not find, and the extraction that lets a method take an id
//! or a handle interchangeably.

use pyo3::PyClass;
use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::pyclass::boolean_struct::True;

use collomatique_state_colloscopes::InnerData;

use crate::Document;
use crate::errors::StaleHandleError;
use crate::ids::IdClass;

/// The rust id a handle is bound to
pub(crate) type RawId<H> = <<H as Handle>::IdClass as IdClass>::Inner;

/// A live, read-only view bound to `(document, id)`
///
/// Implementing this is what makes a pyclass a handle: it brings the reads, the
/// stale error, and the `==` / `hash` that work on `(document, id)` alone —
/// never on the state, so a dict holding handles does not blow up when an
/// entity dies.
pub(crate) trait Handle: Sized {
    /// The python class of this handle's `.id`
    type IdClass: IdClass;

    /// The python class name — `Period`, `Week`, …
    const CLASS: &'static str;

    /// The noun a stale message uses — `period`, `week`, …
    const NOUN: &'static str;

    /// Builds the handle
    ///
    /// The collections and the navigation attributes are the only callers: a
    /// handle is something the document hands out, so the python classes have
    /// no constructor.
    fn mint(doc: Py<Document>, id: RawId<Self>) -> Self;

    /// The document this handle reads through
    fn document(&self) -> &Py<Document>;

    /// The id this handle is bound to
    fn raw_id(&self) -> RawId<Self>;

    /// Whether a document still holds the entity an id names
    ///
    /// The liveness question, asked without a handle because both of the places
    /// that ask it have only an id: a collection's `resolve`, on its way to the
    /// mapping conventions, and [argument], on its way to `StaleHandleError`.
    /// It lives on the trait so that the two cannot answer differently — the
    /// whole point of §2.4 is that they differ in what they *do* about the
    /// answer, not in what the answer is.
    fn exists(data: &InnerData, id: RawId<Self>) -> bool;

    /// Borrows the document, reads through it, and lets go
    ///
    /// `f` answers `None` when the entity is gone, which is the one thing every
    /// handle read has to say the same way. An attribute whose *value* is
    /// optional nests the two: the outer `None` is staleness, the inner one is
    /// the absent value.
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> Option<R>) -> PyResult<R> {
        let doc = self.document().borrow(py);
        f(doc.data().get_inner_data()).ok_or_else(|| self.stale())
    }

    /// Reads without saying anything about liveness — for `repr`, which never
    /// raises
    fn peek<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.document().borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The error a read through a dead handle raises
    fn stale(&self) -> PyErr {
        StaleHandleError::new_err(format!(
            "this {} handle is stale: {} {} is no longer in the document",
            Self::CLASS,
            Self::NOUN,
            Self::IdClass::text(self.raw_id()),
        ))
    }

    /// Whether two handles name the same thing
    ///
    /// `(document identity, id)`, and nothing else: this keeps working on a
    /// stale handle because it never touches the state.
    fn same_as(&self, other: &Self) -> bool {
        std::ptr::eq(self.document().as_ptr(), other.document().as_ptr())
            && self.raw_id() == other.raw_id()
    }

    /// The hash of the same pair `==` compares
    fn hash_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.document().as_ptr() as usize).hash(&mut hasher);
        self.raw_id().hash(&mut hasher);
        hasher.finish()
    }

    /// `<Period #3 …>` — the repr shape every handle shares
    ///
    /// `what` is what the handle could read about itself, or `None` when it is
    /// stale; a repr is for logging, and logging a dead handle is exactly when
    /// it matters that it says so.
    fn repr_text(&self, what: Option<String>) -> String {
        use collomatique_state::ids::Id as _;

        match what {
            Some(what) => format!("<{} #{} {what}>", Self::CLASS, self.raw_id().inner()),
            None => format!("<{} #{} (stale)>", Self::CLASS, self.raw_id().inner()),
        }
    }
}

/// The entity a script named, when this document is the one that holds it
///
/// An id resolves on its own say-so: it carries no document, so the only thing
/// to do with it is to look it up here. A handle carries its document, so a
/// handle bound to *another* one names nothing here, whatever its id says
/// (§2.1) — which is what makes `x in c` and `c[x]` answer `False` / `KeyError`
/// for a foreign handle.
///
/// The answer says only that the script named an id of the right kind. Whether
/// the document still holds it is the caller's question, and the two lookup
/// conventions of §2.4 differ in exactly what they do about it.
pub(crate) fn named<H>(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> Option<RawId<H>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    if let Ok(id) = obj.cast::<H::IdClass>() {
        return Some(id.get().raw());
    }

    if let Ok(handle) = obj.cast::<H>() {
        let handle = handle.get();
        return std::ptr::eq(handle.document().as_ptr(), doc.as_ptr()).then(|| handle.raw_id());
    }

    None
}

/// The entity an id-or-handle *argument* names, or the refusal §2.4 calls for
///
/// This is the other lookup convention. A mapping position answers `KeyError` /
/// `None` / `False`, because asking a lookup is legitimate; an argument that
/// names nothing raises `StaleHandleError` instead, because the question was
/// malformed before it had an answer. The model's own forgiving replies — its
/// `is_week_active` says `false` for a week it does not hold — are deliberately
/// not mirrored: a script passing a dead reference is mistaken about its own
/// document, and `false` would hide that.
///
/// The whole rule sits here: the kind, the document, and the liveness. The
/// liveness question is [Handle::exists], the same one a collection's `resolve`
/// asks, so the two conventions can never disagree about what is in the
/// document — only about what to do when nothing is. It borrows the document to
/// ask, so an argument is checked before the read it guards rather than inside
/// one.
///
/// Anything that is neither an id nor a handle of the kind wanted is a
/// `TypeError`: it was never a reference to this document at all.
pub(crate) fn argument<H>(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<RawId<H>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    let id = if let Ok(id) = obj.cast::<H::IdClass>() {
        id.get().raw()
    } else if let Ok(handle) = obj.cast::<H>() {
        let handle = handle.get();

        // A handle carries its document, so one bound to another document names
        // nothing here whatever its id says (§2.1) — and its id must not be
        // borrowed to ask about this document, since the number may well be
        // somebody else's here.
        if !std::ptr::eq(handle.document().as_ptr(), doc.as_ptr()) {
            return Err(somebody_elses::<H>(handle.raw_id()));
        }
        handle.raw_id()
    } else {
        return Err(PyTypeError::new_err(format!(
            "a {} argument takes a {} or a {}, and {} is neither",
            H::NOUN,
            H::CLASS,
            <H::IdClass as IdClass>::CLASS,
            shown(obj, "that object"),
        )));
    };

    let held = {
        let doc = doc.borrow(obj.py());
        H::exists(doc.data().get_inner_data(), id)
    };

    held.then_some(id).ok_or_else(|| not_here::<H>(id))
}

/// What an argument this document does not hold raises
fn not_here<H: Handle>(id: RawId<H>) -> PyErr {
    StaleHandleError::new_err(format!(
        "this {} argument names nothing here: {} {} is not in this document",
        H::CLASS,
        H::NOUN,
        H::IdClass::text(id),
    ))
}

/// What an argument that is another document's handle raises
///
/// A different failure from [not_here], and it must say so: two documents hand
/// out ids that coincide (§2.1), so the id in the message may perfectly well
/// name something here — something else. Claiming it is "not in this document"
/// would be the one falsehood available, and it would send a script looking for
/// a removal that never happened.
fn somebody_elses<H: Handle>(id: RawId<H>) -> PyErr {
    StaleHandleError::new_err(format!(
        "this {} argument belongs to another document: {} {} is that document's, \
         and an id names nothing outside the one that handed it out",
        H::CLASS,
        H::NOUN,
        H::IdClass::text(id),
    ))
}

/// What `collection[x]` raises when `x` names nothing in the document
///
/// A mapping position follows python's mapping protocol (§2.4): asking a lookup
/// is legitimate, so the mapping vocabulary is the right answer. The key is
/// shown as python would print it, whatever it turned out to be.
pub(crate) fn no_such(kind: &str, key: &Bound<'_, PyAny>) -> PyErr {
    PyKeyError::new_err(format!(
        "{} names no {kind} in this document",
        shown(key, "that key")
    ))
}

/// An object as python would print it, for the messages that name what was given
///
/// `fallback` is what to call it when even that fails — a key in a mapping, a
/// mere object in an argument. It exists because an error message must not fail
/// to be built; reaching it takes an object whose `repr` raises.
pub(crate) fn shown(obj: &Bound<'_, PyAny>, fallback: &str) -> String {
    obj.repr()
        .map(|repr| repr.to_string())
        .unwrap_or_else(|_| fallback.to_owned())
}

/// A string as python would print it, for the reprs that name an entity
///
/// `<Subject #3 'Maths'>` — python's own quoting, so a name holding a quote of
/// its own comes out readable rather than merely escaped the rust way. The
/// fallback exists because a repr never raises; reaching it takes a python that
/// cannot repr one of its own strings.
pub(crate) fn quoted(py: Python<'_>, text: &str) -> String {
    pyo3::types::PyString::new(py, text)
        .repr()
        .map(|repr| repr.to_string())
        .unwrap_or_else(|_| format!("{text:?}"))
}

/// Which end of a pairing rule a side sub-view is bound to
///
/// `rule.antecedent` and `rule.consequent` hand out the two side sub-views of
/// §3.11 / §3.12 (`docs/python/handle_api.md`). The side is part of the view's
/// identity — a view is bound to `(document, rule_id, side)` — so the two ends
/// of one rule never compare equal, and each keeps its own place in a dict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RuleSide {
    Antecedent,
    Consequent,
}

/// Declares a collection's iterator class
///
/// Iteration snapshots the ids when it starts, in the collection's order, and
/// mints the handles as the loop asks for them (§2.5). Removing an entity
/// mid-iteration is therefore safe and loud: the loop still sees the id, and
/// the handle minted for it raises `StaleHandleError` on the first read.
///
/// The iterator classes are not registered in the module: they are what `iter()`
/// hands back, not something a script names.
macro_rules! handle_iterator {
    ($(#[$meta:meta])* $name:ident yielding $handle:ty) => {
        $(#[$meta])*
        #[pyclass]
        pub struct $name {
            doc: pyo3::Py<$crate::Document>,
            ids: std::vec::IntoIter<$crate::handles::RawId<$handle>>,
        }

        impl $name {
            /// Builds the iterator over an already-taken snapshot of the ids
            pub(crate) fn new(
                doc: pyo3::Py<$crate::Document>,
                ids: Vec<$crate::handles::RawId<$handle>>,
            ) -> Self {
                $name {
                    doc,
                    ids: ids.into_iter(),
                }
            }
        }

        #[pymethods]
        impl $name {
            fn __iter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> {
                slf
            }

            fn __next__(&mut self, py: pyo3::Python<'_>) -> Option<$handle> {
                use $crate::handles::Handle as _;

                self.ids
                    .next()
                    .map(|id| <$handle>::mint(self.doc.clone_ref(py), id))
            }
        }
    };
}

pub(crate) use handle_iterator;
