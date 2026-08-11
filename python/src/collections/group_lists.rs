//! The group lists of a document, and which subjects they serve
//!
//! Reached as `doc.group_lists`. A group list is either *prefilled* — its
//! groups are fixed sets of students — or *automatic*, filled by the solver,
//! whose placements then live in the colloscope. The two shapes read
//! differently, and the
//! `None`-for-inapplicable rule keeps a script from ever reading an empty set
//! where the question did not apply.
//!
//! Which group list a subject uses on a period is a separate table, the
//! associations keyed by `(period, subject)` — the hop every script makes
//! between a colloscope cell and the names it should print.

use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_state_colloscopes::GroupListId as RawGroupListId;
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::SubjectId as RawSubjectId;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

use crate::Document;
use crate::collections::periods::Period;
use crate::collections::students::Student;
use crate::collections::subjects::Subject;
use crate::handles::{Handle, argument, handle_iterator, named, no_such, quoted};
use crate::ids::{GroupListId, IdClass};
use crate::values::nonzero_range;

/// The group lists of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the group lists — the application lists
/// them as the table hands them over — so the order here is the ids', which is
/// the one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct GroupLists {
    doc: Py<Document>,
}

impl GroupLists {
    /// Builds the view — `doc.group_lists` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> GroupLists {
        GroupLists { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The group list an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawGroupListId> {
        let id = named::<GroupList>(&self.doc, key)?;
        self.with_data(py, |data| GroupList::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl GroupLists {
    /// How many group lists the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.group_lists.group_list_map.len())
    }

    /// The group lists, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> GroupListIter {
        let ids = self.with_data(py, |data| {
            data.params.group_lists.group_list_map.keys().collect()
        });
        GroupListIter::new(self.doc.clone_ref(py), ids)
    }

    /// The group list an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<GroupList> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("group list", key))?;
        Ok(GroupList::mint(self.doc.clone_ref(py), id))
    }

    /// The group list an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<GroupList> {
        let id = self.resolve(py, key)?;
        Some(GroupList::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// The group list a subject uses on a period, or `None`
    ///
    /// The `(period, subject) → group list` hop of the design: the one every
    /// script makes between a colloscope cell and the names it should print.
    ///
    /// The read is total over valid addresses, like the assignments': a period
    /// or a subject the document does not hold raises `StaleHandleError`, and
    /// a valid pair the model stores no association for answers `None` — the
    /// design's rule for a missing junction row. A `GroupList` handle never
    /// answers this question; the same list may serve many pairs.
    fn association_for(
        &self,
        py: Python<'_>,
        period: &Bound<'_, PyAny>,
        subject: &Bound<'_, PyAny>,
    ) -> PyResult<Option<GroupList>> {
        let period_id = argument::<Period>(&self.doc, period)?;
        let subject_id = argument::<Subject>(&self.doc, subject)?;

        let group_list_id = self.with_data(py, |data| {
            data.params
                .group_lists
                .subjects_associations
                .get(&(period_id, subject_id))
                .copied()
        });
        Ok(group_list_id
            .map(|group_list_id| GroupList::mint(self.doc.clone_ref(py), group_list_id)))
    }

    /// The stored associations, as `(Period, Subject, GroupList)` triples, in
    /// key order
    ///
    /// Yields only the rows the model stores — the pairs a group list was
    /// actually associated to. `association_for` is the total read; this is the
    /// content.
    fn associations(&self, py: Python<'_>) -> GroupListAssociationIter {
        let rows = self.with_data(py, |data| {
            data.params
                .group_lists
                .subjects_associations
                .iter()
                .map(|((period, subject), group_list)| (period, subject, *group_list))
                .collect()
        });
        GroupListAssociationIter::new(self.doc.clone_ref(py), rows)
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.GroupLists count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The group lists of a collection, minted as the loop asks for them
    GroupListIter yielding GroupList
}

/// The associations of a collection, minted as the loop asks for them
///
/// A row is a triple: the `Period` and `Subject` handles of the key, and the
/// `GroupList` handle the pair is associated to. The ids were snapshotted when
/// the iteration started, so a removal in
/// the middle leaves the ids standing and the handles minted for a dead entity
/// raise `StaleHandleError` on the first read.
#[pyclass]
pub struct GroupListAssociationIter {
    doc: Py<Document>,
    rows: std::vec::IntoIter<(RawPeriodId, RawSubjectId, RawGroupListId)>,
}

impl GroupListAssociationIter {
    /// Builds the iterator over an already-taken snapshot of the rows
    pub(crate) fn new(
        doc: Py<Document>,
        rows: Vec<(RawPeriodId, RawSubjectId, RawGroupListId)>,
    ) -> GroupListAssociationIter {
        GroupListAssociationIter {
            doc,
            rows: rows.into_iter(),
        }
    }
}

#[pymethods]
impl GroupListAssociationIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> Option<(Period, Subject, GroupList)> {
        let (period, subject, group_list) = self.rows.next()?;
        Some((
            Period::mint(self.doc.clone_ref(py), period),
            Subject::mint(self.doc.clone_ref(py), subject),
            GroupList::mint(self.doc.clone_ref(py), group_list),
        ))
    }
}

/// One group list of the document
///
/// A live view: every attribute reads the document as it stands now. Reading
/// one whose list has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
///
/// The two filling shapes are read side by side: a prefilled list answers
/// `.groups` and `None` for `.excluded_students`, an automatic one `.groups =
/// None` and its exclusions — the question that does not apply to the list's
/// filling kind is the one that reads `None`, never an empty set.
#[pyclass(module = "collomatique", frozen)]
pub struct GroupList {
    doc: Py<Document>,
    id: RawGroupListId,
}

impl Handle for GroupList {
    type IdClass = GroupListId;

    const CLASS: &'static str = "GroupList";
    const NOUN: &'static str = "group list";

    fn mint(doc: Py<Document>, id: RawGroupListId) -> GroupList {
        GroupList { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawGroupListId {
        self.id
    }

    fn exists(data: &InnerData, id: RawGroupListId) -> bool {
        data.params.group_lists.group_list_map.contains(&id)
    }
}

#[pymethods]
impl GroupList {
    /// The group list's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> GroupListId {
        GroupListId::wrap(self.id)
    }

    /// The group list's name — « Liste principale » and the like
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| group_list.params().name.clone())
        })
    }

    /// How many students one group holds, as a `(min, max)` range
    #[getter]
    fn students_per_group(&self, py: Python<'_>) -> PyResult<(u32, u32)> {
        self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| nonzero_range(&group_list.params().students_per_group))
        })
    }

    /// How many groups the list can hold
    ///
    /// The length of `.group_names`, and the bound every group number is
    /// measured against: the colloscope's cell values are indices into
    /// `range(group_count)`.
    #[getter]
    fn group_count(&self, py: Python<'_>) -> PyResult<usize> {
        self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| group_list.params().group_names.len())
        })
    }

    /// The group names, as a tuple, `None` where a group is unnamed
    ///
    /// The raw names, `group_name`'s source: entry `i` names group `i`, and a
    /// `None` entry is a group that shows its number. The names are optional
    /// non-empty strings in the model, so they read as `str` or `None`, never
    /// `""`.
    #[getter]
    fn group_names<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let names = self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| {
                    group_list
                        .params()
                        .group_names
                        .iter()
                        .map(|name| name.as_ref().map(|name| name.to_string()))
                        .collect::<Vec<_>>()
                })
        })?;

        PyTuple::new(py, names)
    }

    /// Whether the groups are filled by hand rather than by the solver
    #[getter]
    fn is_prefilled(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| group_list.is_prefilled())
        })
    }

    /// The prefilled groups, as a tuple of `frozenset` of [Student], or `None`
    ///
    /// One frozenset per group, in group order, for a prefilled list. `None` —
    /// not the empty tuple — for an automatic list, whose placements live in
    /// the colloscope (`doc.colloscope.group_list`); the question does not
    /// apply. The handles in a frozenset stay live.
    #[getter]
    fn groups<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let groups = self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| match group_list.filling() {
                    GroupListFilling::Prefilled { groups } => Some(groups.clone()),
                    GroupListFilling::Automatic { .. } => None,
                })
        })?;

        let Some(groups) = groups else {
            return Ok(None);
        };
        let groups: Vec<Bound<'py, PyAny>> = groups
            .into_iter()
            .map(|group| {
                students_frozenset(py, &self.doc, group.students.into_iter())
                    .map(|set| set.into_any())
            })
            .collect::<PyResult<_>>()?;
        Ok(Some(PyTuple::new(py, groups)?))
    }

    /// The students an automatic filling must skip, or `None`
    ///
    /// The exclusion set of an automatic list, as a `frozenset` of [Student]
    /// handles — empty for a list that excludes nobody. `None` for a prefilled
    /// list, whose filling never consults it.
    #[getter]
    fn excluded_students<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyFrozenSet>>> {
        let excluded = self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| match group_list.filling() {
                    GroupListFilling::Automatic { excluded_students } => {
                        Some(excluded_students.clone())
                    }
                    GroupListFilling::Prefilled { .. } => None,
                })
        })?;

        match excluded {
            Some(excluded) => Ok(Some(students_frozenset(
                py,
                &self.doc,
                excluded.into_iter(),
            )?)),
            None => Ok(None),
        }
    }

    /// The name of a group, the way the application writes it
    ///
    /// ```python
    /// gl.group_name(2)   # 'Serdaigle', or 'Groupe 3' when unnamed
    /// ```
    ///
    /// The stored name when the group has one; otherwise the 1-based fallback
    /// « Groupe N » that the gui shows for an unnamed group
    /// (`gtk4/src/editor/colloscope.rs` — the number always displays, with the
    /// name after a colon when there is one). An index past the list's
    /// `group_count` raises `IndexError`.
    fn group_name(&self, py: Python<'_>, index: usize) -> PyResult<String> {
        let (name, count) = self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| {
                    let names = &group_list.params().group_names;
                    (names.get(index).cloned(), names.len())
                })
        })?;

        let Some(name) = name else {
            return Err(PyIndexError::new_err(format!(
                "group {index} is out of range: this group list has {count} groups"
            )));
        };
        Ok(name
            .map(|name| name.to_string())
            .unwrap_or_else(|| format!("Groupe {}", index + 1)))
    }

    /// What points at this group list — every site whose coordinates name it, as
    /// a tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::group_list_references(py, self)
    }

    /// This group list, detached — a `GroupListData` holding what the handle
    /// shows
    ///
    /// A fresh object every call. The filling comes out as the matching leaf
    /// value — `PrefilledGroups` or `AutomaticGroups` — because the value
    /// keeps the sum the model keeps; the students inside come out as ids
    /// rather than as handles, since a value holding handles would carry this
    /// document around with it and keep it alive.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let group_list = self.read(py, |data| {
            data.params
                .group_lists
                .group_list_map
                .get(&self.id)
                .cloned()
        })?;

        crate::data::GroupListData::to_py(py, &group_list)
    }

    /// Whether two handles name the same group list of the same document
    ///
    /// Never reads the state, so it keeps working once the group list is gone —
    /// a dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<GroupList>() {
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
                .group_lists
                .group_list_map
                .get(&self.id)
                .map(|group_list| group_list.params().name.clone())
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}

/// A group of students, as a `frozenset` of live [Student] handles
///
/// Both the prefilled groups of a list and its excluded students read this way,
/// which is why it takes the ids as a plain iterator: a `frozenset` is a
/// snapshot of the membership at call time,
/// and the handles in it stay live.
fn students_frozenset<'py>(
    py: Python<'py>,
    doc: &Py<Document>,
    ids: impl IntoIterator<Item = collomatique_state_colloscopes::StudentId>,
) -> PyResult<Bound<'py, PyFrozenSet>> {
    let students: Vec<Bound<'py, PyAny>> = ids
        .into_iter()
        .map(|id| {
            Py::new(py, Student::mint(doc.clone_ref(py), id))
                .map(|student| student.into_bound(py).into_any())
        })
        .collect::<PyResult<_>>()?;
    PyFrozenSet::new(py, &students)
}
