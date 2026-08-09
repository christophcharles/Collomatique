//! The subjects of a document, and how their interrogations are laid out
//!
//! Reached as `doc.subjects`. A subject carries a name, the periods it does not
//! run in, and — when it holds interrogations at all — a whole set of parameters
//! for them, which is the [Interrogation] sub-view.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet};

use collomatique_state_colloscopes::SubjectId as RawSubjectId;
use collomatique_state_colloscopes::{InnerData, SubjectInterrogationParameters};

use crate::Document;
use crate::collections::periods::Period;
use crate::errors::StaleHandleError;
use crate::handles::{Handle, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, SubjectId};
use crate::values;

/// The subjects of one document, in user order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
#[pyclass(module = "collomatique", frozen)]
pub struct Subjects {
    doc: Py<Document>,
}

impl Subjects {
    /// Builds the view — `doc.subjects` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Subjects {
        Subjects { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The subject an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawSubjectId> {
        let id = named::<Subject>(&self.doc, key)?;
        self.with_data(py, |data| {
            data.params.subjects.find_subject_position(id).is_some()
        })
        .then_some(id)
    }
}

#[pymethods]
impl Subjects {
    /// How many subjects the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.subjects.ordered_subject_list.len())
    }

    /// The subjects, as handles, in user order
    fn __iter__(&self, py: Python<'_>) -> SubjectIter {
        let ids = self.with_data(py, |data| {
            data.params.subjects.ordered_subject_list.keys().collect()
        });
        SubjectIter::new(self.doc.clone_ref(py), ids)
    }

    /// The subject an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Subject> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("subject", key))?;
        Ok(Subject::mint(self.doc.clone_ref(py), id))
    }

    /// The subject an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Subject> {
        let id = self.resolve(py, key)?;
        Some(Subject::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Subjects count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The subjects of a collection, minted as the loop asks for them
    SubjectIter yielding Subject
}

/// One subject of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose subject has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
#[pyclass(module = "collomatique", frozen)]
pub struct Subject {
    doc: Py<Document>,
    id: RawSubjectId,
}

impl Handle for Subject {
    type IdClass = SubjectId;

    const CLASS: &'static str = "Subject";
    const NOUN: &'static str = "subject";

    fn mint(doc: Py<Document>, id: RawSubjectId) -> Subject {
        Subject { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawSubjectId {
        self.id
    }
}

#[pymethods]
impl Subject {
    /// The subject's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> SubjectId {
        SubjectId::wrap(self.id)
    }

    /// The subject's display position, 0-based
    #[getter]
    fn index(&self, py: Python<'_>) -> PyResult<usize> {
        self.read(py, |data| {
            data.params.subjects.find_subject_position(self.id)
        })
    }

    /// The subject's name
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .subjects
                .find_subject(self.id)
                .map(|subject| subject.parameters.name.clone())
        })
    }

    /// How this subject's interrogations are laid out, or `None`
    ///
    /// `None` means the subject holds no interrogations at all — the Quidditch
    /// practice that sits in the timetable without ever being a colle. What
    /// comes back otherwise is a live sub-view: asking again after the
    /// interrogations were switched off answers `None`, and the view handed out
    /// before that goes stale.
    #[getter]
    fn interrogation(&self, py: Python<'_>) -> PyResult<Option<Interrogation>> {
        let holds_them = self.read(py, |data| {
            let subject = data.params.subjects.find_subject(self.id)?;
            Some(subject.parameters.interrogation_parameters.is_some())
        })?;

        Ok(holds_them.then(|| Interrogation::mint(self.doc.clone_ref(py), self.id)))
    }

    /// The periods this subject does not run in, as a `frozenset` of [Period]
    ///
    /// A snapshot, built when it is asked for: the set does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn excluded_periods<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let ids = self.read(py, |data| {
            let subject = data.params.subjects.find_subject(self.id)?;
            Some(subject.excluded_periods.iter().copied().collect::<Vec<_>>())
        })?;

        let periods: Vec<_> = ids
            .into_iter()
            .map(|period_id| Period::mint(self.doc.clone_ref(py), period_id))
            .collect();
        PyFrozenSet::new(py, periods)
    }

    /// Whether two handles name the same subject of the same document
    ///
    /// Never reads the state, so it keeps working once the subject is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Subject>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let name = self.peek(py, |data| {
            data.params
                .subjects
                .find_subject(self.id)
                .map(|subject| subject.parameters.name.clone())
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}

/// How one subject's interrogations are laid out
///
/// A sub-view, which is a handle in everything but the `.id`
/// (`docs/python/handle_api.md` §1): it is bound to its subject, reads the
/// current state on every access, and goes stale with it. `subject.interrogation`
/// asked again always answers the current truth.
///
/// It goes stale in two ways, and both mean the same thing — what the view was
/// viewing is gone: the subject was removed, or its interrogations were switched
/// off.
///
/// The number its repr shows is the subject's, since that is what the view is
/// bound to.
#[pyclass(module = "collomatique", frozen)]
pub struct Interrogation {
    doc: Py<Document>,
    id: RawSubjectId,
}

impl Handle for Interrogation {
    type IdClass = SubjectId;

    const CLASS: &'static str = "Interrogation";
    const NOUN: &'static str = "subject";

    fn mint(doc: Py<Document>, id: RawSubjectId) -> Interrogation {
        Interrogation { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawSubjectId {
        self.id
    }
}

impl Interrogation {
    /// Borrows the document, finds the parameters the view is about, and reads
    ///
    /// The two ways of being stale are told apart here rather than folded into
    /// one message: a subject that was removed and a subject that stopped
    /// holding colles are different things to have done, and a script reading
    /// the error wants to know which.
    fn read<R>(
        &self,
        py: Python<'_>,
        f: impl FnOnce(&SubjectInterrogationParameters) -> R,
    ) -> PyResult<R> {
        let doc = self.doc.borrow(py);
        let subjects = &doc.data().get_inner_data().params.subjects;

        let subject = subjects
            .find_subject(self.id)
            .ok_or_else(|| <Interrogation as Handle>::stale(self))?;

        let params = subject
            .parameters
            .interrogation_parameters
            .as_ref()
            .ok_or_else(|| {
                StaleHandleError::new_err(format!(
                    "this Interrogation view is stale: subject {} no longer holds interrogations",
                    SubjectId::text(self.id),
                ))
            })?;

        Ok(f(params))
    }
}

#[pymethods]
impl Interrogation {
    /// How many students one group holds, as a `(min, max)` range
    #[getter]
    fn students_per_group(&self, py: Python<'_>) -> PyResult<values::Range> {
        self.read(py, |params| {
            values::nonzero_range(&params.students_per_group)
        })
    }

    /// How many groups sit one interrogation together, as a `(min, max)` range
    #[getter]
    fn groups_per_interrogation(&self, py: Python<'_>) -> PyResult<values::Range> {
        self.read(py, |params| {
            values::nonzero_range(&params.groups_per_interrogation)
        })
    }

    /// How long one interrogation lasts, in minutes
    #[getter]
    fn duration(&self, py: Python<'_>) -> PyResult<u32> {
        self.read(py, |params| params.duration.get().get())
    }

    /// Whether this time counts against the limits on a student's week
    #[getter]
    fn take_duration_into_account(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |params| params.take_duration_into_account)
    }

    /// How often the interrogations come round, as one of the [Periodicity]
    /// values
    ///
    /// [Periodicity]: crate::values::Periodicity
    #[getter]
    fn periodicity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let periodicity = self.read(py, |params| params.periodicity.clone())?;
        values::periodicity(py, &periodicity)
    }

    /// Whether two views are about the same subject of the same document
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Interrogation>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let duration = self.read(py, |params| params.duration.get().get()).ok();
        self.repr_text(duration.map(|duration| format!("duration={duration}")))
    }
}
