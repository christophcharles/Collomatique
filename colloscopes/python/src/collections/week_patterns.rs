//! The week patterns of a document, and the weeks they switch off
//!
//! Reached as `doc.week_patterns`. A pattern is « les semaines paires » — a slot
//! that carries one only holds its colles on the weeks the pattern leaves on.
//! The model stores it as the set of weeks it *excludes*, so a pattern that
//! excludes nothing leaves every week alone, and python reads it the same way.
//!
//! Whether a week really carries interrogations is not a pattern's answer alone:
//! the week has a flag of its own. The two are merged by the [Document]'s own
//! `is_week_active`, which is where a script asks the question.
//!
//! Written through `add`, `update` and `remove`. Two things follow a pattern —
//! a slot, to know which weeks it runs on, and an incompatibility, to know
//! which weeks it applies on — and both hold it in a field whose empty value is
//! a legal one meaning « toutes les semaines ». So removing a pattern does not
//! take them with it: the cascade clears the reference, and the slot and the
//! incompatibility survive, running every week from then on. Each of those
//! repairs comes back on the `OpResult`, which is the only account a script
//! gets of a slot that has just changed meaning.
//!
//! The `update` cascades the other way about. Excluding a week from a pattern
//! makes interrogations impossible on that week for every slot following it, so
//! the colles already written there contradict the new pattern and the cascade
//! clears exactly those cells.
//!
//! The family keeps no refusal for the model. `WeekPatternsUpdateOp` can object
//! to two things — a pattern id the document does not hold, and an excluded
//! week that names nothing — and both are caught on this side, where the
//! message can say which argument was wrong: a dead pattern is the argument
//! convention's business ([crate::handles::argument]), and a dead week is the
//! value boundary's ([crate::data::WeekPatternData]). So nothing here raises
//! `WeekPatternsError`.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_ops::{UpdateOp, WeekPatternsUpdateOp};
use collomatique_state_colloscopes::WeekPatternId as RawWeekPatternId;
use collomatique_state_colloscopes::{InnerData, NewId};

use crate::Document;
use crate::collections::weeks::Week;
use crate::data::{Value as _, WeekPatternData};
use crate::handles::{Handle, argument, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, WeekPatternId};
use crate::results::{AddResult, OpResult};

/// The week patterns of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the patterns — the application lists
/// them as the table hands them over — so the order here is the ids', which is
/// the one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct WeekPatterns {
    doc: Py<Document>,
}

impl WeekPatterns {
    /// Builds the view — `doc.week_patterns` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> WeekPatterns {
        WeekPatterns { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The pattern an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawWeekPatternId> {
        let id = named::<WeekPattern>(&self.doc, key)?;
        self.with_data(py, |data| WeekPattern::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl WeekPatterns {
    /// How many week patterns the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.week_patterns.week_pattern_map.len())
    }

    /// The week patterns, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> WeekPatternIter {
        let ids = self.with_data(py, |data| {
            data.params.week_patterns.week_pattern_map.keys().collect()
        });
        WeekPatternIter::new(self.doc.clone_ref(py), ids)
    }

    /// The week pattern an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<WeekPattern> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("week pattern", key))?;
        Ok(WeekPattern::mint(self.doc.clone_ref(py), id))
    }

    /// The week pattern an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<WeekPattern> {
        let id = self.resolve(py, key)?;
        Some(WeekPattern::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// Adds a week pattern, and hands back the handle of the new one
    ///
    /// Takes a `WeekPatternData` — the whole of what a pattern is, since the
    /// entity and the op payload are the same type here — and answers an
    /// `AddResult`, whose `created` is the `WeekPattern` the document just
    /// minted.
    ///
    /// ```python
    /// doc.week_patterns.add(collomatique.WeekPatternData(
    ///     "Semaines paires", excluded_weeks={first_week}))
    /// ```
    ///
    /// A new pattern is followed by nothing — a slot picks one up afterwards —
    /// so there is nothing for the cascade to repair: the answer's `warnings`
    /// is empty, whatever the pattern switches off.
    fn add(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<AddResult>> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask
        // (`docs/python/new_api_design.md` §5).
        let pattern = WeekPatternData::from_py(&self.doc, data)?;

        crate::results::created::<WeekPattern>(
            py,
            &self.doc,
            UpdateOp::WeekPatterns(WeekPatternsUpdateOp::AddNewWeekPattern(pattern)),
            |new_id| match new_id {
                NewId::WeekPatternId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Rewrites a week pattern whole
    ///
    /// The op carries the whole value, so this replaces both fields at once:
    /// what the `WeekPatternData` says is what the pattern becomes, the name and
    /// the excluded weeks together. The id stays, and so does every handle
    /// naming it — and so does every slot following it, which is the point of
    /// rewriting rather than replacing.
    ///
    /// Excluding a week is a write like any other, and the cascade repairs what
    /// it broke: a slot following the pattern holds no interrogation on a week
    /// the pattern switches off, so the colles already written in those cells
    /// go, and the warnings say so. Switching a week back on breaks nothing —
    /// an empty cell is what a week that never ran looks like.
    ///
    /// The pattern is resolved before the value is read, so a call that is wrong
    /// about both says which pattern it could not find rather than what was
    /// wrong with a value meant for nothing.
    fn update(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let id = argument::<WeekPattern>(&self.doc, key)?;
        let pattern = WeekPatternData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::WeekPatterns(WeekPatternsUpdateOp::UpdateWeekPattern(id, pattern)),
        )
    }

    /// Removes a week pattern
    ///
    /// What followed the pattern lets go of it and stays: a slot that named it
    /// now runs every week, and so does an incompatibility that named it, since
    /// « pas de modèle » is a legal value for both. Nothing is removed but the
    /// pattern itself, so no handle but the ones naming it goes stale — and the
    /// `OpResult` carries one repair per site, which is where a script reads
    /// what has just changed meaning.
    fn remove(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<WeekPattern>(&self.doc, key)?;

        self.write(
            py,
            UpdateOp::WeekPatterns(WeekPatternsUpdateOp::DeleteWeekPattern(id)),
        )
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.WeekPatterns count={}>", self.__len__(py))
    }
}

impl WeekPatterns {
    /// Writes through the document the view came from
    ///
    /// The two mutators that create nothing end here. The creating one ends in
    /// [crate::results::created], which takes the same borrow and keeps the id
    /// the op issued as well.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

handle_iterator! {
    /// The week patterns of a collection, minted as the loop asks for them
    WeekPatternIter yielding WeekPattern
}

/// One week pattern of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose pattern has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
///
/// A pattern is stored as the weeks it switches off, and that is how it reads
/// here: `excluded_weeks` is the whole of it. Which weeks are left is a question
/// about the weeks as well as about the pattern, so it is the [Document]'s own
/// `is_week_active` that answers it.
#[pyclass(module = "collomatique", frozen)]
pub struct WeekPattern {
    doc: Py<Document>,
    id: RawWeekPatternId,
}

impl Handle for WeekPattern {
    type IdClass = WeekPatternId;

    const CLASS: &'static str = "WeekPattern";
    const NOUN: &'static str = "week pattern";

    fn mint(doc: Py<Document>, id: RawWeekPatternId) -> WeekPattern {
        WeekPattern { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawWeekPatternId {
        self.id
    }

    fn exists(data: &InnerData, id: RawWeekPatternId) -> bool {
        data.params.week_patterns.week_pattern_map.contains(&id)
    }
}

#[pymethods]
impl WeekPattern {
    /// The pattern's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> WeekPatternId {
        WeekPatternId::wrap(self.id)
    }

    /// The pattern's name — « Semaines paires » and the like
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .week_patterns
                .week_pattern_map
                .get(&self.id)
                .map(|pattern| pattern.name.clone())
        })
    }

    /// The weeks this pattern switches off, as a `frozenset` of [Week]
    ///
    /// The exception set, which is the whole of what a pattern is: every week
    /// not in here is one the pattern leaves alone. A week that holds no
    /// interrogations of its own may perfectly well be in it — the model keeps
    /// the two apart, so that switching a week back on brings back the pattern
    /// it had.
    ///
    /// A snapshot, built when it is asked for: the set does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn excluded_weeks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let ids = self.read(py, |data| {
            let pattern = data.params.week_patterns.week_pattern_map.get(&self.id)?;
            Some(pattern.excluded_weeks.iter().copied().collect::<Vec<_>>())
        })?;

        let weeks: Vec<_> = ids
            .into_iter()
            .map(|week_id| Week::mint(self.doc.clone_ref(py), week_id))
            .collect();
        PyFrozenSet::new(py, weeks)
    }

    /// What points at this week pattern — every site whose coordinates name it,
    /// as a tuple of `RefSite` values, in the registry's walk order. An empty
    /// tuple means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::week_pattern_references(py, self)
    }

    /// This pattern, detached — a `WeekPatternData` holding what the handle
    /// shows
    ///
    /// A fresh object every call: two calls give two values that compare equal
    /// and share nothing. The excluded weeks come out as `WeekId`s rather than
    /// as handles, because a value holding handles would carry this document
    /// around with it and keep it alive.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let pattern = self.read(py, |data| {
            data.params
                .week_patterns
                .week_pattern_map
                .get(&self.id)
                .cloned()
        })?;

        crate::data::WeekPatternData::to_py(py, &pattern)
    }

    /// Whether two handles name the same pattern of the same document
    ///
    /// Never reads the state, so it keeps working once the pattern is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<WeekPattern>() {
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
                .week_patterns
                .week_pattern_map
                .get(&self.id)
                .map(|pattern| pattern.name.clone())
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}
