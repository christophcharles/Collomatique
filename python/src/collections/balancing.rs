//! The balancing of a document
//!
//! Reached as `doc.balancing`: how the
//! resolution is asked to balance interrogations — rotation of teachers and of
//! slots, and fairness over the year and within a period — as one global entry
//! plus sparse per-subject overrides. The two are whole entries, exactly like
//! the settings: an override replaces the global entry **verbatim**, and the
//! resolution stays in the model (`Balancing::options_for`).
//!
//! [BalancingOptions] is a live sub-view, bound to `(document, kind)` and
//! reading the current state on every access — the structural twin of the
//! settings [Limits] view, with the same three ways to get one and the same
//! rules for which of them goes stale when.
//!
//! [Limits]: crate::collections::settings::Limits

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::SubjectId as RawSubjectId;
use collomatique_state_colloscopes::balancing::BalancingOptions as RawBalancingOptions;

use crate::Document;
use crate::collections::subjects::Subject;
use crate::data::BalancingData;
use crate::data::Value as _;
use crate::errors::StaleHandleError;
use crate::handles::{Handle, argument};
use crate::ids::{IdClass, SubjectId};
use crate::values::Enforcement;

/// The balancing of one document
///
/// Frozen and holding nothing but the document: it is a view, so two of them
/// on the same document are interchangeable and neither can go stale. A
/// singleton view — the balancing has no place in a collection and no id of
/// its own, so there is no collection protocol here, only the four members
/// below.
#[pyclass(module = "collomatique", frozen)]
pub struct Balancing {
    doc: Py<Document>,
}

impl Balancing {
    /// Builds the view — `doc.balancing` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Balancing {
        Balancing { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }
}

#[pymethods]
impl Balancing {
    /// The document-wide balancing options
    ///
    /// The entry every subject inherits when they have no override of their
    /// own. The view is bound to the global entry itself, so it can never go
    /// stale.
    #[getter]
    fn global_options(&self, py: Python<'_>) -> BalancingOptions {
        BalancingOptions::mint(self.doc.clone_ref(py), BalancingKind::Global)
    }

    /// What balancing options apply to one subject
    ///
    /// The resolved entry: the subject's override when they have one, the
    /// global entry otherwise — the model's own `Balancing::options_for`
    /// answer, never a merge python could get wrong.
    ///
    /// The view is live in the strong sense: it re-resolves on every read, so
    /// it tracks an override appearing or vanishing — the same [BalancingOptions]
    /// object answers differently once the document changes. It goes stale
    /// when the subject does, and only then.
    ///
    /// Takes a [Subject] handle or a [SubjectId], as every argument of this
    /// api does; one this document does not hold raises `StaleHandleError`.
    fn options_for(
        &self,
        py: Python<'_>,
        subject: &Bound<'_, PyAny>,
    ) -> PyResult<BalancingOptions> {
        let subject = argument::<Subject>(&self.doc, subject)?;
        Ok(BalancingOptions::mint(
            self.doc.clone_ref(py),
            BalancingKind::ForSubject(subject),
        ))
    }

    /// A subject's raw balancing override, if one is set
    ///
    /// `None` — not an empty entry — when the subject has no override at all
    /// and inherits the global options. The view is bound to the override
    /// entry itself: it goes stale when the override is removed, or when the
    /// subject is. A `subject` this document does not hold raises
    /// `StaleHandleError`.
    fn override_for(
        &self,
        py: Python<'_>,
        subject: &Bound<'_, PyAny>,
    ) -> PyResult<Option<BalancingOptions>> {
        let subject = argument::<Subject>(&self.doc, subject)?;
        let overridden =
            self.with_data(py, |data| data.params.balancing.subjects.contains(&subject));
        Ok(overridden.then(|| {
            BalancingOptions::mint(self.doc.clone_ref(py), BalancingKind::Override(subject))
        }))
    }

    /// The stored overrides, as `(Subject, BalancingOptions)` pairs, in id order
    ///
    /// The rows are read when the call is made; the pairs in them are live —
    /// the [Subject] handles and the [BalancingOptions] views keep reading the
    /// document, and a view goes stale when its entry is removed.
    fn overrides<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let rows = self.with_data(py, |data| {
            data.params
                .balancing
                .subjects
                .iter()
                .map(|(subject, _)| subject)
                .collect::<Vec<_>>()
        });
        let rows = rows
            .into_iter()
            .map(|subject| {
                Ok((
                    Subject::mint(self.doc.clone_ref(py), subject),
                    BalancingOptions::mint(
                        self.doc.clone_ref(py),
                        BalancingKind::Override(subject),
                    ),
                ))
            })
            .collect::<PyResult<Vec<_>>>()?;
        PyTuple::new(py, rows)
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let count = self.with_data(py, |data| data.params.balancing.subjects.len());
        format!("<collomatique.Balancing overrides={count}>")
    }
}

/// What a [BalancingOptions] view is bound to, which is what it reads and what
/// kills it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BalancingKind {
    /// `doc.balancing.global_options` — the global entry, which never goes stale
    Global,
    /// `doc.balancing.options_for(s)` — the resolved entry, which follows an
    /// override appearing or vanishing and goes stale when the subject does
    ForSubject(RawSubjectId),
    /// `doc.balancing.override_for(s)` — the raw entry, which goes stale when
    /// the override is removed, and when the subject is
    Override(RawSubjectId),
}

/// The balancing options one subject's interrogations are scheduled under
///
/// A live sub-view, which is a handle in everything but the `.id`: it is
/// bound to `(document, kind)`, reads
/// the current state on every access, and goes stale with what it is bound to.
/// The three ways to get one — `doc.balancing.global_options`, `options_for(s)`
/// and `override_for(s)` — differ in exactly that: a global view never goes
/// stale, a resolved view tracks an override appearing or vanishing, and an
/// override view goes stale when its entry is removed.
///
/// The kind is part of the view's identity, as it is for the settings [Limits]
/// view: a resolved view and an override view of the same subject are
/// different views with different lives, and they do not compare equal.
///
/// What it reads is the model's three-state `Option<SoftParam<()>>` for the
/// rotation goals — `None` (not pursued), `Enforcement.OBJECTIVE` (optimize
/// for it) or `Enforcement.STRICT` (a hard constraint) — plus the two boolean
/// fairness switches.
///
/// [Limits]: crate::collections::settings::Limits
#[pyclass(module = "collomatique", frozen)]
pub struct BalancingOptions {
    doc: Py<Document>,
    kind: BalancingKind,
}

impl BalancingOptions {
    /// Builds the view — `doc.balancing`'s four members are the only ways to
    /// get one
    pub(crate) fn mint(doc: Py<Document>, kind: BalancingKind) -> BalancingOptions {
        BalancingOptions { doc, kind }
    }

    /// Borrows the document and reads the entry the view is bound to
    ///
    /// The view's two ways of dying are told apart by the kind, as the
    /// settings [Limits] view's are: a resolved view dies with its subject, an
    /// override view dies with its subject or with the removal of its entry —
    /// and the message says which.
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RawBalancingOptions) -> R) -> PyResult<R> {
        let doc = self.doc.borrow(py);
        let data = doc.data().get_inner_data();
        let balancing = &data.params.balancing;

        let options = match self.kind {
            BalancingKind::Global => &balancing.global,
            BalancingKind::ForSubject(subject) => {
                if data.params.subjects.find_subject(subject).is_none() {
                    return Err(self.stale_subject(subject));
                }
                balancing.options_for(subject)
            }
            BalancingKind::Override(subject) => {
                if data.params.subjects.find_subject(subject).is_none() {
                    return Err(self.stale_subject(subject));
                }
                balancing
                    .subjects
                    .get(&subject)
                    .ok_or_else(|| self.stale_override(subject))?
            }
        };

        Ok(f(options))
    }

    /// Reads without saying anything about liveness — for `repr`, which never
    /// raises
    fn peek<R>(&self, py: Python<'_>, f: impl FnOnce(&RawBalancingOptions) -> R) -> Option<R> {
        let doc = self.doc.borrow(py);
        let data = doc.data().get_inner_data();
        let balancing = &data.params.balancing;

        let options = match self.kind {
            BalancingKind::Global => &balancing.global,
            BalancingKind::ForSubject(subject) => {
                data.params.subjects.find_subject(subject)?;
                balancing.options_for(subject)
            }
            BalancingKind::Override(subject) => balancing.subjects.get(&subject)?,
        };
        Some(f(options))
    }

    /// The error a read through a view whose subject is gone raises
    fn stale_subject(&self, subject: RawSubjectId) -> PyErr {
        StaleHandleError::new_err(format!(
            "this BalancingOptions view is stale: subject {} is no longer in the document",
            SubjectId::text(subject),
        ))
    }

    /// The error a read through a view whose override was removed raises
    fn stale_override(&self, subject: RawSubjectId) -> PyErr {
        StaleHandleError::new_err(format!(
            "this BalancingOptions view is stale: subject {} has no balancing override anymore",
            SubjectId::text(subject),
        ))
    }

    /// The view's anchor in its repr — « (global) » or « #<subject id> »
    fn anchor(&self) -> String {
        match self.kind {
            BalancingKind::Global => "(global)".to_owned(),
            BalancingKind::ForSubject(subject) | BalancingKind::Override(subject) => {
                use collomatique_state::ids::Id as _;
                format!("#{}", subject.inner())
            }
        }
    }
}

#[pymethods]
impl BalancingOptions {
    /// How the resolution rotates the teachers across the groups of one
    /// subject
    #[getter]
    fn teacher_rotation(&self, py: Python<'_>) -> PyResult<Option<Enforcement>> {
        self.read(py, |options| enforcement(&options.teacher_rotation))
    }

    /// How the resolution rotates the slots across the groups of one subject
    #[getter]
    fn slot_rotation(&self, py: Python<'_>) -> PyResult<Option<Enforcement>> {
        self.read(py, |options| enforcement(&options.slot_rotation))
    }

    /// How the resolution keeps the same teacher away from one group's
    /// consecutive interrogations
    #[getter]
    fn avoid_twice_in_a_row(&self, py: Python<'_>) -> PyResult<Option<Enforcement>> {
        self.read(py, |options| enforcement(&options.avoid_twice_in_a_row))
    }

    /// Whether each teacher is asked to see the same number of interrogations
    /// over the whole year
    #[getter]
    fn year_teacher_rotation(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |options| options.year_teacher_rotation)
    }

    /// Whether each teacher is asked to see the same number of interrogations
    /// within each period
    #[getter]
    fn period_teacher_rotation(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |options| options.period_teacher_rotation)
    }

    /// This entry, detached — a `BalancingData` holding what the view shows
    ///
    /// A fresh object every call, the whole entry as the document holds it: a
    /// goal the entry does not pursue comes out as `None`, which is the
    /// whole-entry override rule — not pursued means disabled, never inherited
    /// — and that meaning stays with the write, not with this value
    /// (`docs/python/values.md` §3.8).
    ///
    /// What the view is bound to is what comes out: the resolved view of a
    /// subject without an override hands back the global entry. A stale handle
    /// raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let options = self.read(py, |options| options.clone())?;

        BalancingData::to_py(py, &options)
    }

    /// Whether two views are bound to the same thing of the same document
    ///
    /// The kind is part of the identity: a resolved view and an override view
    /// of one subject never compare equal. Never reads the state, so it keeps
    /// working once what the view is bound to is gone.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<BalancingOptions>() {
            Ok(other) => {
                let other = other.get();
                std::ptr::eq(self.doc.as_ptr(), other.doc.as_ptr()) && self.kind == other.kind
            }
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut hasher);
        self.kind.hash(&mut hasher);
        hasher.finish()
    }

    /// `<BalancingOptions #2 teacher_rotation=Enforcement.OBJECTIVE …>` — what
    /// the view is bound to, then the five options as they read now. A view
    /// whose binding is gone prints `(périmé)`, like the handle reprs do.
    fn __repr__(&self, py: Python<'_>) -> String {
        let reading = self.peek(py, |options| {
            let one = |what: &Option<collomatique_state_colloscopes::settings::SoftParam<()>>,
                       word: &str| match what {
                Some(soft) => format!("{word}={}", Enforcement::from_model(soft.soft)),
                None => format!("{word}=None"),
            };
            format!(
                "{} {} {} year_teacher_rotation={} period_teacher_rotation={}",
                one(&options.teacher_rotation, "teacher_rotation"),
                one(&options.slot_rotation, "slot_rotation"),
                one(&options.avoid_twice_in_a_row, "avoid_twice_in_a_row"),
                options.year_teacher_rotation,
                options.period_teacher_rotation,
            )
        });
        match reading {
            Some(reading) => format!("<BalancingOptions {} {reading}>", self.anchor()),
            None => format!("<BalancingOptions {} (périmé)>", self.anchor()),
        }
    }
}

/// The python enforcement for one model rotation goal
///
/// The rotation goals carry no value — the model's `SoftParam<()>` is a
/// three-state switch — so only the soft flag crosses.
fn enforcement(
    soft: &Option<collomatique_state_colloscopes::settings::SoftParam<()>>,
) -> Option<Enforcement> {
    soft.as_ref().map(|soft| Enforcement::from_model(soft.soft))
}
