//! The opaque ids — one class per id kind
//!
//! An id compares and hashes, orders against
//! its own kind, prints readably, and does nothing else. There is no
//! constructor, no `int()` and no serialization — an id is a token the document
//! handed out, not a number a script writes down.
//!
//! The eleven classes are uniform by design, so they come out of one macro
//! rather than being written eleven times: eleven hand-written copies would
//! drift.

use pyo3::prelude::*;

use collomatique_state::ids::Id as _;

/// One python id class, and the rust id behind it
///
/// The handle plumbing ([crate::handles]) is written against this rather than
/// against eleven concrete types: it is how a handle knows which class to mint
/// for its `.id`, and how a `StaleHandleError` names what it could not find.
pub(crate) trait IdClass: Sized {
    /// The rust id this class wraps
    type Inner: collomatique_state::ids::Id;

    /// The python class name — `SubjectId`
    const CLASS: &'static str;

    /// Wraps a rust id for python
    fn wrap(inner: Self::Inner) -> Self;

    /// The rust id back
    fn raw(&self) -> Self::Inner;

    /// `<SubjectId 3>` — the `repr`, and what a stale message names
    ///
    /// The angle brackets are deliberate: `SubjectId(3)` would read as an
    /// expression a script could paste back, and there is no such constructor.
    fn text(inner: Self::Inner) -> String {
        format!("<{} {}>", Self::CLASS, inner.inner())
    }
}

/// Declares the id classes, which differ only in their name and their kind
macro_rules! id_classes {
    ($($name:ident wrapping $inner:ty, one $what:literal;)*) => {
        /// The id class a serde newtype-struct name stands for, around `inner`
        ///
        /// The rust ids are newtype structs (`PeriodId(u64)`), so a serde walk
        /// over anything holding one is handed the struct's name — and the name
        /// is the only thing that still tells an id from a plain number. The
        /// walk over the model's refusals (`errors::payload`) asks here
        /// with it.
        ///
        /// `inner` comes back untouched when the name is not an id class, or
        /// when what it holds is not the `u64` an id is: the caller is
        /// structural, and it must not lose what it cannot recognise.
        pub(crate) fn from_serde<'py>(
            py: Python<'py>,
            name: &str,
            inner: Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let Ok(value) = inner.extract::<u64>() else {
                return Ok(inner);
            };

            match name {
                $(stringify!($name) => {
                    // `Id::new` is unsafe because a *document* must not be
                    // handed an id it did not mint. Nothing here reaches one:
                    // this is the id the model just named in a refusal, minted
                    // so the script can print it and compare it with the ids it
                    // already holds.
                    let raw = unsafe { <$inner as collomatique_state::ids::Id>::new(value) };
                    Ok(Bound::new(py, <$name as IdClass>::wrap(raw))?.into_any())
                })*
                _ => Ok(inner),
            }
        }

        $(
        #[doc = concat!("The identity of one ", $what, ", inside one run")]
        ///
        /// Opaque: it compares and hashes, orders against ids of its own kind,
        /// and prints. There is no constructor, no `int()` and no way to write
        /// one down — comparing it with an id of another kind is `False`, and
        /// ordering it against one raises `TypeError`.
        ///
        /// An id does not know its document. Two documents open in one script
        /// can hand back ids that compare equal while naming unrelated things,
        /// so an id only means something inside the document that produced it —
        /// and only during the run that produced it, since every load
        /// renumbers. What carries between documents is content (names,
        /// matching); what is safe inside one document is the handle, which
        /// *is* bound to it.
        #[pyclass(module = "collomatique", frozen, eq, ord, hash)]
        #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            id: $inner,
        }

        #[pymethods]
        impl $name {
            fn __repr__(&self) -> String {
                <$name as IdClass>::text(self.id)
            }
        }

        impl IdClass for $name {
            type Inner = $inner;

            const CLASS: &'static str = stringify!($name);

            fn wrap(inner: $inner) -> Self {
                $name { id: inner }
            }

            fn raw(&self) -> $inner {
                self.id
            }
        }
    )* };
}

id_classes! {
    PeriodId wrapping collomatique_state_colloscopes::PeriodId, one "period";
    WeekId wrapping collomatique_state_colloscopes::WeekId, one "week";
    SubjectId wrapping collomatique_state_colloscopes::SubjectId, one "subject";
    TeacherId wrapping collomatique_state_colloscopes::TeacherId, one "teacher";
    StudentId wrapping collomatique_state_colloscopes::StudentId, one "student";
    WeekPatternId wrapping collomatique_state_colloscopes::WeekPatternId, one "week pattern";
    SlotId wrapping collomatique_state_colloscopes::SlotId, one "slot";
    IncompatId wrapping collomatique_state_colloscopes::IncompatId, one "incompatibility";
    GroupListId wrapping collomatique_state_colloscopes::GroupListId, one "group list";
    PairingRuleId wrapping collomatique_state_colloscopes::PairingRuleId, one "pairing rule";
    SlotPairingRuleId
        wrapping collomatique_state_colloscopes::SlotPairingRuleId,
        one "slot pairing rule";
}

/// Adds the id classes to the module
///
/// All eleven land together even though only two have a collection handing them
/// out yet: the macro makes them uniform, so splitting them across commits
/// would buy nothing and leave the module half-populated in between.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PeriodId>()?;
    m.add_class::<WeekId>()?;
    m.add_class::<SubjectId>()?;
    m.add_class::<TeacherId>()?;
    m.add_class::<StudentId>()?;
    m.add_class::<WeekPatternId>()?;
    m.add_class::<SlotId>()?;
    m.add_class::<IncompatId>()?;
    m.add_class::<GroupListId>()?;
    m.add_class::<PairingRuleId>()?;
    m.add_class::<SlotPairingRuleId>()?;
    Ok(())
}
