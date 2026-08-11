//! The settings of a document
//!
//! Reached as `doc.settings`: the
//! limits imposed on the resolution, as one global entry plus sparse
//! per-student overrides. The two are whole entries — an override replaces the
//! global entry **verbatim**, a `None` field in it disabling the corresponding
//! global limit rather than inheriting it — and the resolution itself stays in
//! the model (`Settings::limits_for`); python only ever sees resolved or raw
//! entries, never a merge it could get wrong.
//!
//! [Limits] is a live sub-view, bound to `(document, kind)`: it reads the
//! current state on every access and goes stale with what it is bound to. The
//! three ways to get one differ in exactly that — the global entry never goes
//! stale, a resolved view tracks an override appearing or vanishing, and a raw
//! override view goes stale when the override is removed.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::StudentId as RawStudentId;
use collomatique_state_colloscopes::settings::Limits as RawLimits;
use collomatique_state_colloscopes::settings::SoftParam;

use crate::Document;
use crate::collections::students::Student;
use crate::data::LimitsData;
use crate::data::Value as _;
use crate::errors::StaleHandleError;
use crate::handles::{Handle, argument};
use crate::ids::{IdClass, StudentId};
use crate::values::{Limit, limit, nonzero_limit};

/// The settings of one document
///
/// Frozen and holding nothing but the document: it is a view, so two of them
/// on the same document are interchangeable and neither can go stale. A
/// singleton view — the settings have no place in a collection and no id of
/// their own, so there is no collection protocol here, only the four members
/// below.
#[pyclass(module = "collomatique", frozen)]
pub struct Settings {
    doc: Py<Document>,
}

impl Settings {
    /// Builds the view — `doc.settings` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Settings {
        Settings { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }
}

#[pymethods]
impl Settings {
    /// The document-wide limits
    ///
    /// The entry every student inherits when they have no override of their
    /// own. The view is bound to the global entry itself, so it can never go
    /// stale.
    #[getter]
    fn global_limits(&self, py: Python<'_>) -> Limits {
        Limits::mint(self.doc.clone_ref(py), LimitsKind::Global)
    }

    /// What limits apply to one student
    ///
    /// The resolved entry: the student's override when they have one, the
    /// global entry otherwise — the model's own `Settings::limits_for` answer,
    /// never a merge python could get wrong.
    ///
    /// The view is live in the strong sense: it re-resolves on every read, so
    /// it tracks an override appearing or vanishing — the same [Limits] object
    /// answers differently once the document changes. It goes stale when the
    /// student does, and only then.
    ///
    /// Takes a [Student] handle or a [StudentId], as every argument of this
    /// api does; one this document does not hold raises `StaleHandleError`.
    fn limits_for(&self, py: Python<'_>, student: &Bound<'_, PyAny>) -> PyResult<Limits> {
        let student = argument::<Student>(&self.doc, student)?;
        Ok(Limits::mint(
            self.doc.clone_ref(py),
            LimitsKind::ForStudent(student),
        ))
    }

    /// A student's raw limits override, if one is set
    ///
    /// `None` — not an empty entry — when the student has no override at all
    /// and inherits the global limits. The view is bound to the override entry
    /// itself: it goes stale when the override is removed, or when the student
    /// is. A `student` this document does not hold raises `StaleHandleError`.
    fn override_for(&self, py: Python<'_>, student: &Bound<'_, PyAny>) -> PyResult<Option<Limits>> {
        let student = argument::<Student>(&self.doc, student)?;
        let overridden =
            self.with_data(py, |data| data.params.settings.students.contains(&student));
        Ok(overridden.then(|| Limits::mint(self.doc.clone_ref(py), LimitsKind::Override(student))))
    }

    /// The stored overrides, as `(Student, Limits)` pairs, in id order
    ///
    /// The rows are read when the call is made; the pairs in them are live —
    /// the [Student] handles and the [Limits] views keep reading the document,
    /// and a view goes stale when its entry is removed.
    fn overrides<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let rows = self.with_data(py, |data| {
            data.params
                .settings
                .students
                .iter()
                .map(|(student, _)| student)
                .collect::<Vec<_>>()
        });
        let rows = rows
            .into_iter()
            .map(|student| {
                Ok((
                    Student::mint(self.doc.clone_ref(py), student),
                    Limits::mint(self.doc.clone_ref(py), LimitsKind::Override(student)),
                ))
            })
            .collect::<PyResult<Vec<_>>>()?;
        PyTuple::new(py, rows)
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let count = self.with_data(py, |data| data.params.settings.students.len());
        format!("<collomatique.Settings overrides={count}>")
    }
}

/// What a [Limits] view is bound to, which is what it reads and what kills it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LimitsKind {
    /// `doc.settings.global_limits` — the global entry, which never goes stale
    Global,
    /// `doc.settings.limits_for(s)` — the resolved entry, which follows an
    /// override appearing or vanishing and goes stale when the student does
    ForStudent(RawStudentId),
    /// `doc.settings.override_for(s)` — the raw entry, which goes stale when
    /// the override is removed, and when the student is
    Override(RawStudentId),
}

/// The limits a student's interrogation schedule is held to
///
/// A live sub-view, which is a handle in everything but the `.id`: it is
/// bound to `(document, kind)`, reads
/// the current state on every access, and goes stale with what it is bound to.
/// The three ways to get one — `doc.settings.global_limits`, `limits_for(s)`
/// and `override_for(s)` — differ in exactly that: a global view never goes
/// stale, a resolved view tracks an override appearing or vanishing, and an
/// override view goes stale when its entry is removed.
///
/// The kind is part of the view's identity: a resolved view and an override
/// view of the same student are different views with different lives, even
/// while they read the same entry, and they do not compare equal — like the
/// two ends of a pairing rule.
///
/// What it reads is the three limits, each a [Limit] value or `None` — `None`
/// meaning the limit is not set at all, not that it is somehow zero.
#[pyclass(module = "collomatique", frozen)]
pub struct Limits {
    doc: Py<Document>,
    kind: LimitsKind,
}

impl Limits {
    /// Builds the view — `doc.settings`'s four members are the only ways to
    /// get one
    pub(crate) fn mint(doc: Py<Document>, kind: LimitsKind) -> Limits {
        Limits { doc, kind }
    }

    /// Borrows the document and reads the entry the view is bound to
    ///
    /// The view's two ways of dying are told apart by the kind: a resolved
    /// view dies with its student, an override view dies with its student or
    /// with the removal of its entry — and the message says which, as the
    /// `Interrogation` view's does, because a script meeting the error wants
    /// to know what happened.
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RawLimits) -> R) -> PyResult<R> {
        let doc = self.doc.borrow(py);
        let data = doc.data().get_inner_data();
        let settings = &data.params.settings;

        let limits = match self.kind {
            LimitsKind::Global => &settings.global,
            LimitsKind::ForStudent(student) => {
                if !data.params.students.student_map.contains(&student) {
                    return Err(self.stale_student(student));
                }
                settings.limits_for(student)
            }
            LimitsKind::Override(student) => {
                if !data.params.students.student_map.contains(&student) {
                    return Err(self.stale_student(student));
                }
                settings
                    .students
                    .get(&student)
                    .ok_or_else(|| self.stale_override(student))?
            }
        };

        Ok(f(limits))
    }

    /// Reads without saying anything about liveness — for `repr`, which never
    /// raises
    fn peek<R>(&self, py: Python<'_>, f: impl FnOnce(&RawLimits) -> R) -> Option<R> {
        let doc = self.doc.borrow(py);
        let data = doc.data().get_inner_data();
        let settings = &data.params.settings;

        let limits = match self.kind {
            LimitsKind::Global => &settings.global,
            LimitsKind::ForStudent(student) => {
                if !data.params.students.student_map.contains(&student) {
                    return None;
                }
                settings.limits_for(student)
            }
            LimitsKind::Override(student) => settings.students.get(&student)?,
        };
        Some(f(limits))
    }

    /// The error a read through a view whose student is gone raises
    fn stale_student(&self, student: RawStudentId) -> PyErr {
        StaleHandleError::new_err(format!(
            "this Limits view is stale: student {} is no longer in the document",
            StudentId::text(student),
        ))
    }

    /// The error a read through a view whose override was removed raises
    fn stale_override(&self, student: RawStudentId) -> PyErr {
        StaleHandleError::new_err(format!(
            "this Limits view is stale: student {} has no limits override anymore",
            StudentId::text(student),
        ))
    }

    /// The view's anchor in its repr — « (global) » or « #<student id> »
    fn anchor(&self) -> String {
        match self.kind {
            LimitsKind::Global => "(global)".to_owned(),
            LimitsKind::ForStudent(student) | LimitsKind::Override(student) => {
                use collomatique_state::ids::Id as _;
                format!("#{}", student.inner())
            }
        }
    }
}

#[pymethods]
impl Limits {
    /// How many interrogations a student may hold at the least in a week
    #[getter]
    fn interrogations_per_week_min(&self, py: Python<'_>) -> PyResult<Option<Limit>> {
        self.read(py, |limits| {
            limits.interrogations_per_week_min.as_ref().map(limit)
        })
    }

    /// How many interrogations a student may hold at the most in a week
    #[getter]
    fn interrogations_per_week_max(&self, py: Python<'_>) -> PyResult<Option<Limit>> {
        self.read(py, |limits| {
            limits.interrogations_per_week_max.as_ref().map(limit)
        })
    }

    /// How many interrogations a student may hold at the most in one day
    #[getter]
    fn max_interrogations_per_day(&self, py: Python<'_>) -> PyResult<Option<Limit>> {
        self.read(py, |limits| {
            limits
                .max_interrogations_per_day
                .as_ref()
                .map(nonzero_limit)
        })
    }

    /// This entry, detached — a `LimitsData` holding what the view shows
    ///
    /// A fresh object every call, the whole entry as the document holds it: a
    /// field the entry leaves unset comes out as `None`, which is the
    /// whole-entry override rule — it disables the inherited limit rather than
    /// inheriting it — and that meaning stays with the write, not with this
    /// value (`docs/python/values.md` §3.8).
    ///
    /// What the view is bound to is what comes out: the resolved view of a
    /// student without an override hands back the global entry. A stale handle
    /// raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let limits = self.read(py, |limits| limits.clone())?;

        LimitsData::to_py(py, &limits)
    }

    /// Whether two views are bound to the same thing of the same document
    ///
    /// The kind is part of the identity: a resolved view and an override view
    /// of one student never compare equal. Never reads the state, so it keeps
    /// working once what the view is bound to is gone.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Limits>() {
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

    /// `<Limits #100 interrogations_per_week_min=2 …>` — what the view is
    /// bound to, then the three limits as they read now. A view whose binding
    /// is gone prints `(périmé)`, like the handle reprs do.
    fn __repr__(&self, py: Python<'_>) -> String {
        let reading = self.peek(py, |limits| {
            let one = |what: &Option<SoftParam<u32>>, word: &str| match what {
                Some(soft) => format!("{word}={}", soft.value),
                None => format!("{word}=None"),
            };
            let day = |what: &Option<SoftParam<std::num::NonZeroU32>>| match what {
                Some(soft) => format!("max_interrogations_per_day={}", soft.value),
                None => "max_interrogations_per_day=None".to_owned(),
            };
            format!(
                "{} {} {}",
                one(
                    &limits.interrogations_per_week_min,
                    "interrogations_per_week_min"
                ),
                one(
                    &limits.interrogations_per_week_max,
                    "interrogations_per_week_max"
                ),
                day(&limits.max_interrogations_per_day),
            )
        });
        match reading {
            Some(reading) => format!("<Limits {} {reading}>", self.anchor()),
            None => format!("<Limits {} (périmé)>", self.anchor()),
        }
    }
}
