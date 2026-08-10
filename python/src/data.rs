//! The value dataclasses, and the boundary they cross
//!
//! `docs/python/values.md` is the design. The classes themselves are written in
//! python, in `data.py`, for the reason §2 of `new_api_design.md` gives: a value
//! nests and holds mutable containers, and a pyo3 getter hands back a clone of
//! the struct it holds. This module is what compiles that file into the module
//! and what converts between the objects it defines and the model's own types.
//!
//! Two directions, both explicit, and neither of them a `FromPyObject` impl: a
//! field that names an entity has to be resolved against *this* document, and
//! `extract_bound` has nowhere to put one.

use std::collections::BTreeSet;
use std::ffi::CString;
use std::num::NonZeroU32;

use pyo3::PyClass;
use pyo3::exceptions::{PyAttributeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::boolean_struct::True;
use pyo3::types::{PyDict, PyFrozenSet, PyList, PySet, PyTuple};

use collomatique_state_colloscopes::{
    NonEmptyRangeInclusive, PersonWithContact, SubjectInterrogationParameters, SubjectPeriodicity,
    group_lists, incompats, pairings, slot_pairings, slots, students, subjects, teachers,
    week_patterns,
};

use crate::Document;
use crate::collections::{Period, Slot, Student, Subject, Teacher, Week, WeekPattern};
use crate::handles::{Handle, RawId, argument, shown};
use crate::ids::{
    IdClass, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId,
};
use crate::values;

/// The dataclasses, as `data.py` writes them
const DATA_PY: &str = include_str!("data.py");

/// The module `data.py` becomes, and the one a script never names
const MODULE: &str = "collomatique._data";

/// One python value class, and the model type behind it
///
/// The same shape [crate::handles::Handle] gives the handle classes, for the
/// same reason: the classes are uniform, and a trait is what stops them
/// drifting.
pub trait Value: Sized {
    /// The model type this value converts to
    ///
    /// The **entity**, per `values.md` §2.0, and not the op payload. They are
    /// the same type for eleven of the thirteen classes; where they are not, it
    /// is the ops mirror that takes the payload half out of the entity and says
    /// loudly what it cannot carry.
    type Model;

    /// The python class name — `TeacherData`
    const CLASS: &'static str;

    /// The entity one python value names
    ///
    /// Takes the document because a field that names an entity has to be
    /// resolved against it: a handle of another document names nothing here,
    /// and a dead id has to be refused. It borrows the document to ask, so a
    /// caller that means to write must extract *before* it takes its
    /// `borrow_mut`, never inside it.
    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<Self::Model>;

    /// The python value for one entity
    ///
    /// A fresh object every call, holding ids where the entity holds ids: a
    /// value that held handles would carry the document with it and keep it
    /// alive, which is the one thing a detached value must not do.
    fn to_py<'py>(py: Python<'py>, model: &Self::Model) -> PyResult<Bound<'py, PyAny>>;
}

/// The class object one value class is built from
///
/// Looked up through `sys.modules` at every call rather than cached in a
/// static. A `GILOnceCell` is per *process*, while `sys.modules` is per
/// *interpreter*, so a cached class object would be somebody else's class in a
/// second interpreter. The lookup is a dictionary hit.
fn class<'py>(py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    py.import(MODULE)?.getattr(name)
}

/// Where a value being read sits, for the sentence a refusal carries
///
/// A message names the line the script has in front of it. When a script hands
/// over a `SubjectData`, that is the class it wrote and the field it wrote is
/// `interrogation.duration` — even though what is being read at that moment is
/// an `InterrogationData` of its own. One level of nesting is all the values of
/// `docs/python/values.md` have: what nests holds leaves.
#[derive(Clone, Copy)]
struct Site<'a> {
    /// The class a script wrote down
    class: &'a str,
    /// The field of it this value sits in, when it sits in one
    nested_in: Option<&'a str>,
}

impl<'a> Site<'a> {
    /// The site of a value a script handed over whole
    fn whole(class: &'a str) -> Site<'a> {
        Site {
            class,
            nested_in: None,
        }
    }

    /// The site of a value that is itself one field of this one
    fn inside(self, name: &'a str) -> Site<'a> {
        Site {
            class: self.class,
            nested_in: Some(name),
        }
    }

    /// The class, with the article english wants in front of it
    ///
    /// Every class name here is an ascii identifier, so its first letter
    /// settles the question — « an InterrogationData », « a SubjectData ».
    fn expected(&self) -> String {
        let article = match self.class.chars().next() {
            Some('A' | 'E' | 'I' | 'O' | 'U') => "an",
            _ => "a",
        };
        format!("{article} {}", self.class)
    }

    /// The path from the class a script wrote down to one field read here
    fn path(&self, name: &str) -> String {
        match self.nested_in {
            Some(outer) => format!("{outer}.{name}"),
            None => name.to_owned(),
        }
    }

    /// How a message names one field read here
    fn field(&self, name: &str) -> String {
        format!("{}'s {}", self.expected(), self.path(name))
    }
}

/// One field of a value, by attribute access
///
/// Never `cast::<T>()`. A value is a *python* object, so anything carrying the
/// right attributes is one — a script may perfectly well subclass a dataclass,
/// and duck typing is the language's own convention for this shape. What is
/// refused is an object that does not have the field at all, and the refusal
/// names the class that was expected, the way an argument of the wrong kind
/// already does.
fn field<'py>(site: Site<'_>, name: &str, obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    obj.getattr(name).map_err(|e| {
        if e.is_instance_of::<PyAttributeError>(obj.py()) {
            PyTypeError::new_err(format!(
                "{} is expected here, and {} has no {}",
                site.expected(),
                shown(obj, "that object"),
                site.path(name),
            ))
        } else {
            e
        }
    })
}

/// A field the model types as a plain `String`
///
/// The empty string is a value here, not an absence: the model types
/// `PersonWithContact::surname` as a `String`, so python mirrors it.
fn plain_text(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = field(site, name, obj)?;
    value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a string, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })
}

/// A field the model types as an optional non-empty string
///
/// Generic over the model's own string type rather than naming it. That type is
/// foreign — it comes from `non_empty_string` — and the struct field the result
/// lands in is what says which one it is, so this crate needs no dependency of
/// its own on it and `Cargo.lock` does not move.
///
/// This is where `''` is refused, which is the whole difference between this
/// helper and [plain_text].
fn optional_text<T: TryFrom<String>>(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<T>> {
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    let text: String = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a string or None, and {} is neither",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    T::try_from(text).map(Some).map_err(|_| {
        PyValueError::new_err(format!(
            "{} is a non-empty string or None, and '' is neither",
            site.field(name),
        ))
    })
}

/// A field the model types as a `bool`
///
/// Strictly a `bool`: an int that happens to be truthy is refused, the way
/// every other field refuses a value of the wrong kind rather than guessing.
fn flag(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    let value = field(site, name, obj)?;
    value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is True or False, and {} is neither",
            site.field(name),
            shown(&value, "that value"),
        ))
    })
}

/// A field the model counts in whole minutes, at least one
fn minutes(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<collomatique_time::NonZeroMinutes> {
    let value = field(site, name, obj)?;
    let count: u32 = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a number of minutes, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(collomatique_time::NonZeroMinutes::from(
        values::at_least_one(&site.field(name), count)?,
    ))
}

/// A `(min, max)` field the model counts from one
///
/// The checks are `values.rs`'s own, so a range written in a dataclass is
/// refused for the same reasons and in the same words as one written in a leaf
/// value.
fn nonzero_range(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<NonEmptyRangeInclusive<NonZeroU32>> {
    let value = field(site, name, obj)?;
    let bounds: values::Range = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a (min, max) pair of counts, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    values::nonzero_bounds(&site.field(name), bounds)
}

/// A field holding one of the four periodicities
///
/// The leaf values check themselves when they are built, so there is nothing
/// left to refuse here but an object that is not a periodicity at all.
fn periodicity(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<SubjectPeriodicity> {
    let value = field(site, name, obj)?;
    values::model_periodicity(&value).ok_or_else(|| {
        PyTypeError::new_err(format!(
            "{} is a Periodicity, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })
}

/// A field holding one of the seven days
///
/// The days are leaf values of `values.rs`, so there is nothing to refuse here
/// but an object that is not a day at all — the seven members are the only
/// things that ever cast to one.
fn weekday(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<collomatique_time::Weekday> {
    let value = field(site, name, obj)?;
    let day: values::Weekday = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a Weekday, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(day.to_model())
}

/// A field the model stores as a time of day, with minute precision
///
/// The refusal is the model's own rule, in the words [values::TimeSlot]'s
/// constructor already uses for the same one: python's `datetime.time` counts
/// microseconds, and the model does not, so a time carrying any is not a time
/// this document can hold rather than one it would round.
fn whole_minute(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<collomatique_time::WholeMinuteTime> {
    let value = field(site, name, obj)?;
    let time: chrono::NaiveTime = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a time of day, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    collomatique_time::WholeMinuteTime::new(time).ok_or_else(|| {
        PyValueError::new_err(format!(
            "{} is a whole minute, with no seconds or microseconds",
            site.field(name),
        ))
    })
}

/// A field the model types as a plain signed count
///
/// The slot's cost is the one field of this shape: zero leaves the solver
/// alone, a positive number tells it to avoid the slot and a negative one to
/// favour it. So every whole number means something, and nothing is refused
/// but a value that is not one.
fn cost(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<i32> {
    let value = field(site, name, obj)?;
    value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a whole number, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })
}

/// A field the model stores as "at least one"
///
/// The incompatibility's `minimum_free_slots` is the one field of this shape:
/// it has no model default, and 1 is the neutral one — an incompatibility that
/// could spare every window would be no incompatibility at all. The check is
/// `values.rs`'s own, so a count written in a dataclass is refused for the same
/// reason and in the same words as one written in a leaf value.
fn non_zero_count(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<NonZeroU32> {
    let value = field(site, name, obj)?;
    let count: u32 = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a number of slots, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    values::at_least_one(&site.field(name), count)
}

/// A field naming one entity
///
/// The single one's version of [entity_set], and it defers to the same
/// [crate::handles::argument]: a handle and an id are interchangeable, a handle
/// of another document is refused, and an id this document no longer holds is
/// refused. The message is `argument`'s own, so a script meets the same
/// sentence wherever it passes a dead reference.
fn entity<H>(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<RawId<H>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    let value = field(site, name, obj)?;
    argument::<H>(doc, &value)
}

/// A field naming one entity, or naming none
///
/// `None` is a value here rather than an absence: a slot with no week pattern
/// is one that runs every week, which is a state the model holds and not a
/// field left unfilled.
fn optional_entity<H>(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<RawId<H>>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    argument::<H>(doc, &value).map(Some)
}

/// A field naming a set of entities
///
/// Anything iterable is accepted — a set, a list, a generator — and every item
/// goes through [crate::handles::argument], so a handle and an id are the same
/// thing here, a handle of another document is refused, and an id this document
/// no longer holds is refused. The message is `argument`'s own, so a script
/// meets the same sentence wherever it passes a dead reference.
fn entity_set<H>(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<BTreeSet<RawId<H>>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    let value = field(site, name, obj)?;
    let items = value.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a set of entities, and {} cannot be iterated over",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    items.map(|item| argument::<H>(doc, &item?)).collect()
}

/// A field holding the busy windows of an incompatibility
///
/// Anything iterable is accepted, and every item must be a [values::TimeSlot] —
/// the leaf value the read surface already hands the windows out as. Nothing
/// else can be refused here: a `TimeSlot` was born whole, so a window that
/// crosses midnight or lasts zero minutes never existed in the first place.
fn time_slots(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Vec<collomatique_time::SlotWithDuration>> {
    let value = field(site, name, obj)?;
    let items = value.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a list of TimeSlot values, and {} cannot be iterated over",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    items
        .map(|item| {
            let item = item?;
            let slot: values::TimeSlot = item.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} holds TimeSlot values, and {} is not one",
                    site.field(name),
                    shown(&item, "that value"),
                ))
            })?;

            Ok(slot.to_model())
        })
        .collect()
}

/// A field holding the group names of a group list
///
/// A list of optional non-empty strings, entry `i` naming group `i` and
/// `None` naming none. The empty string is refused the way every optional
/// text is — absent is `None`, never `""` — and the generic bound is the same
/// trick [optional_text] uses, so this crate keeps no dependency of its own
/// on the model's string type.
fn group_names<T: TryFrom<String>>(
    site: Site<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Vec<Option<T>>> {
    let value = field(site, "group_names", obj)?;
    let items = value.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a list of names, and {} cannot be iterated over",
            site.field("group_names"),
            shown(&value, "that value"),
        ))
    })?;

    items
        .map(|item| {
            let item = item?;
            if item.is_none() {
                return Ok(None);
            }

            let text: String = item.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} holds non-empty strings or None, and {} is neither",
                    site.field("group_names"),
                    shown(&item, "that value"),
                ))
            })?;

            T::try_from(text).map(Some).map_err(|_| {
                PyValueError::new_err(format!(
                    "{} holds non-empty strings or None, and '' is neither",
                    site.field("group_names"),
                ))
            })
        })
        .collect()
}

/// A field holding one of the two fillings of a group list
///
/// The sum keeps its two shapes in python — [values::AutomaticGroups] and
/// [values::PrefilledGroups], under the [values::Filling] base — and the two
/// are told apart by their class, the way the four periodicities are. The
/// students inside resolve like every other entity reference: a handle or an
/// id, against this document, so a foreign handle and a dead id are refused
/// here. Nothing else can be refused: the group count and the duplicate check
/// belong to the pair `{params, filling}` and stay in the model's own
/// constructor, which the caller runs.
fn filling(
    doc: &Py<Document>,
    site: Site<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<group_lists::GroupListFilling> {
    let value = field(site, "filling", obj)?;

    if let Ok(filling) = value.cast::<values::AutomaticGroups>() {
        let excluded = filling.get().excluded_students.bind(obj.py());
        let items = excluded.try_iter().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} is a set of students, and {} cannot be iterated over",
                site.field("filling"),
                shown(excluded, "that value"),
            ))
        })?;
        let excluded_students = items
            .map(|student| argument::<Student>(doc, &student?))
            .collect::<PyResult<BTreeSet<_>>>()?;

        return Ok(group_lists::GroupListFilling::Automatic { excluded_students });
    }

    if let Ok(filling) = value.cast::<values::PrefilledGroups>() {
        let groups = filling.get().groups.bind(obj.py());
        let items = groups.try_iter().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} is a collection of groups, and {} cannot be iterated over",
                site.field("filling"),
                shown(groups, "that value"),
            ))
        })?;
        let mut prefilled = Vec::new();
        for group in items {
            let group = group?;
            let members = group.try_iter().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} holds groups of students, and {} is not one",
                    site.field("filling"),
                    shown(&group, "that value"),
                ))
            })?;
            let students = members
                .map(|student| argument::<Student>(doc, &student?))
                .collect::<PyResult<BTreeSet<_>>>()?;
            prefilled.push(group_lists::PrefilledGroup { students });
        }

        return Ok(group_lists::GroupListFilling::Prefilled { groups: prefilled });
    }

    Err(PyTypeError::new_err(format!(
        "{} is a Filling, and {} is not one",
        site.field("filling"),
        shown(&value, "that value"),
    )))
}

/// The python filling for one model filling
///
/// The students come out as ids, like every entity reference of a value
/// (§2.3 of the design), inside the leaf value's frozen containers.
fn filling_to_py<'py>(
    py: Python<'py>,
    filling: &group_lists::GroupListFilling,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(match filling {
        group_lists::GroupListFilling::Automatic { excluded_students } => Py::new(
            py,
            values::AutomaticGroups {
                excluded_students: PyFrozenSet::new(
                    py,
                    excluded_students.iter().map(|id| StudentId::wrap(*id)),
                )?
                .into(),
            }
            .init(),
        )?
        .into_bound(py)
        .into_any(),
        group_lists::GroupListFilling::Prefilled { groups } => {
            let groups: Vec<Bound<'py, PyAny>> = groups
                .iter()
                .map(|group| {
                    PyFrozenSet::new(py, group.students.iter().map(|id| StudentId::wrap(*id)))
                        .map(|set| set.into_any())
                })
                .collect::<PyResult<_>>()?;
            Py::new(
                py,
                values::PrefilledGroups {
                    groups: PyTuple::new(py, groups)?.into(),
                }
                .init(),
            )?
            .into_bound(py)
            .into_any()
        }
    })
}

/// The person card the two classes share, read off a value
///
/// The fields are read in the order they are declared in, so the first bad one
/// is the one named — rust evaluates a struct literal's fields in the order they
/// are written, which is why `firstname` comes first here although the model
/// declares `surname` first.
fn person(site: Site<'_>, obj: &Bound<'_, PyAny>) -> PyResult<PersonWithContact> {
    Ok(PersonWithContact {
        firstname: plain_text(site, "firstname", obj)?,
        surname: plain_text(site, "surname", obj)?,
        tel: optional_text(site, "tel", obj)?,
        email: optional_text(site, "email", obj)?,
    })
}

/// The same card, as the four keyword arguments a value is built from
fn card<'py>(py: Python<'py>, desc: &PersonWithContact) -> PyResult<Bound<'py, PyDict>> {
    let kwargs = PyDict::new(py);
    kwargs.set_item("firstname", &desc.firstname)?;
    kwargs.set_item("surname", &desc.surname)?;
    kwargs.set_item("tel", desc.tel.as_ref().map(|tel| tel.to_string()))?;
    kwargs.set_item("email", desc.email.as_ref().map(|email| email.to_string()))?;
    Ok(kwargs)
}

/// One teacher — a name, contact details, and the subjects they interrogate in
pub struct TeacherData;

impl Value for TeacherData {
    type Model = teachers::Teacher;

    const CLASS: &'static str = "TeacherData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<teachers::Teacher> {
        let site = Site::whole(Self::CLASS);

        Ok(teachers::Teacher {
            desc: person(site, obj)?,
            subjects: entity_set::<Subject>(doc, site, "subjects", obj)?,
        })
    }

    fn to_py<'py>(py: Python<'py>, teacher: &teachers::Teacher) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = card(py, &teacher.desc)?;
        kwargs.set_item(
            "subjects",
            PySet::new(py, teacher.subjects.iter().map(|id| SubjectId::wrap(*id)))?,
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One student — a name, contact details, and the periods they sit out
pub struct StudentData;

impl Value for StudentData {
    type Model = students::Student;

    const CLASS: &'static str = "StudentData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<students::Student> {
        let site = Site::whole(Self::CLASS);

        Ok(students::Student {
            desc: person(site, obj)?,
            excluded_periods: entity_set::<Period>(doc, site, "excluded_periods", obj)?,
        })
    }

    fn to_py<'py>(py: Python<'py>, student: &students::Student) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = card(py, &student.desc)?;
        kwargs.set_item(
            "excluded_periods",
            PySet::new(
                py,
                student
                    .excluded_periods
                    .iter()
                    .map(|id| PeriodId::wrap(*id)),
            )?,
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The interrogation parameters, read at the site the script wrote them
///
/// Split out of [InterrogationData::from_py] because a `SubjectData` holds one
/// of these and reads it at a site of its own: what a script wrote there is a
/// `SubjectData`, so that is the class a refusal names — « a SubjectData's
/// interrogation.duration ».
fn interrogation(
    site: Site<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<SubjectInterrogationParameters> {
    Ok(SubjectInterrogationParameters {
        students_per_group: nonzero_range(site, "students_per_group", obj)?,
        groups_per_interrogation: nonzero_range(site, "groups_per_interrogation", obj)?,
        duration: minutes(site, "duration", obj)?,
        take_duration_into_account: flag(site, "take_duration_into_account", obj)?,
        periodicity: periodicity(site, "periodicity", obj)?,
    })
}

/// The same parameters, as the keyword arguments a value is built from
fn interrogation_kwargs<'py>(
    py: Python<'py>,
    params: &SubjectInterrogationParameters,
) -> PyResult<Bound<'py, PyDict>> {
    let kwargs = PyDict::new(py);
    kwargs.set_item(
        "students_per_group",
        values::nonzero_range(&params.students_per_group),
    )?;
    kwargs.set_item(
        "groups_per_interrogation",
        values::nonzero_range(&params.groups_per_interrogation),
    )?;
    kwargs.set_item("duration", params.duration.get().get())?;
    kwargs.set_item(
        "take_duration_into_account",
        params.take_duration_into_account,
    )?;
    kwargs.set_item("periodicity", values::periodicity(py, &params.periodicity)?)?;
    Ok(kwargs)
}

/// How one subject's interrogations are laid out
pub struct InterrogationData;

impl Value for InterrogationData {
    type Model = SubjectInterrogationParameters;

    const CLASS: &'static str = "InterrogationData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(
        _doc: &Py<Document>,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<SubjectInterrogationParameters> {
        interrogation(Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(
        py: Python<'py>,
        params: &SubjectInterrogationParameters,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = interrogation_kwargs(py, params)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One subject — a name, how its colles run, and the periods it sits out
///
/// The model splits the id-free parameters from the `#[fk]` exclusions, because
/// its reference walk visits only the second. Python has no such walk, so the
/// value is flat: `d.name` and `d.excluded_periods` sit side by side, the way
/// the handle already shows them.
pub struct SubjectData;

impl Value for SubjectData {
    /// The **entity**, `values.md` §2.0: the subject ops take the `parameters`
    /// half alone, and it is the ops mirror that takes it out and refuses to
    /// discard the exclusions quietly.
    type Model = subjects::Subject;

    const CLASS: &'static str = "SubjectData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<subjects::Subject> {
        let site = Site::whole(Self::CLASS);

        let name = plain_text(site, "name", obj)?;
        let value = field(site, "interrogation", obj)?;
        let interrogation_parameters = if value.is_none() {
            // The subject that holds no colles at all — the quidditch practice
            // that sits in the timetable without ever being one.
            None
        } else {
            Some(interrogation(site.inside("interrogation"), &value)?)
        };

        Ok(subjects::Subject {
            parameters: subjects::SubjectParameters {
                name,
                interrogation_parameters,
            },
            excluded_periods: entity_set::<Period>(doc, site, "excluded_periods", obj)?,
        })
    }

    fn to_py<'py>(py: Python<'py>, subject: &subjects::Subject) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("name", &subject.parameters.name)?;
        kwargs.set_item(
            "interrogation",
            match &subject.parameters.interrogation_parameters {
                Some(params) => InterrogationData::to_py(py, params)?,
                None => py.None().into_bound(py),
            },
        )?;
        kwargs.set_item(
            "excluded_periods",
            PySet::new(
                py,
                subject
                    .excluded_periods
                    .iter()
                    .map(|id| PeriodId::wrap(*id)),
            )?,
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One week pattern — a name, and the weeks it switches off
pub struct WeekPatternData;

impl Value for WeekPatternData {
    type Model = week_patterns::WeekPattern;

    const CLASS: &'static str = "WeekPatternData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<week_patterns::WeekPattern> {
        let site = Site::whole(Self::CLASS);

        Ok(week_patterns::WeekPattern {
            name: plain_text(site, "name", obj)?,
            excluded_weeks: entity_set::<Week>(doc, site, "excluded_weeks", obj)?,
        })
    }

    fn to_py<'py>(
        py: Python<'py>,
        pattern: &week_patterns::WeekPattern,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("name", &pattern.name)?;
        kwargs.set_item(
            "excluded_weeks",
            PySet::new(
                py,
                pattern.excluded_weeks.iter().map(|id| WeekId::wrap(*id)),
            )?,
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One slot — a subject, a teacher, when it happens, and which weeks it runs on
///
/// The model packs the day and the time into a `SlotStart`, because the pair is
/// what its own time crate has a type for. Python flattens it, the way the
/// handle already shows it: `d.weekday` and `d.start_time` sit side by side.
pub struct SlotData;

impl Value for SlotData {
    /// The **entity**, `values.md` §2.0: the add op overwrites the subject with
    /// a separate argument of its own and the update op refuses a slot whose
    /// subject changed, so no slot op really carries the field. It is here all
    /// the same, because `doc.snapshot()` would otherwise lose which subject
    /// each slot belongs to, and it is the ops mirror that says loudly what it
    /// cannot carry.
    type Model = slots::Slot;

    const CLASS: &'static str = "SlotData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<slots::Slot> {
        let site = Site::whole(Self::CLASS);

        Ok(slots::Slot {
            subject_id: entity::<Subject>(doc, site, "subject", obj)?,
            teacher_id: entity::<Teacher>(doc, site, "teacher", obj)?,
            start_time: collomatique_time::SlotStart {
                weekday: weekday(site, "weekday", obj)?,
                start_time: whole_minute(site, "start_time", obj)?,
            },
            extra_info: plain_text(site, "extra_info", obj)?,
            week_pattern: optional_entity::<WeekPattern>(doc, site, "week_pattern", obj)?,
            cost: cost(site, "cost", obj)?,
        })
    }

    fn to_py<'py>(py: Python<'py>, slot: &slots::Slot) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("subject", SubjectId::wrap(slot.subject_id))?;
        kwargs.set_item("teacher", TeacherId::wrap(slot.teacher_id))?;
        kwargs.set_item(
            "weekday",
            values::Weekday::from_model(slot.start_time.weekday),
        )?;
        kwargs.set_item("start_time", *slot.start_time.start_time.inner())?;
        kwargs.set_item("extra_info", &slot.extra_info)?;
        kwargs.set_item("week_pattern", slot.week_pattern.map(WeekPatternId::wrap))?;
        kwargs.set_item("cost", slot.cost)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One incompatibility — a name, a subject, the busy windows, and the count
///
/// One of the eleven classes whose entity and op payload are the same type:
/// `AddNewIncompat` and `UpdateIncompat` carry the whole `Incompatibility`, so
/// §2.0 of the design says nothing new here. The subject is deliberately not
/// required to hold interrogations — the edge's whole point — so the value
/// takes any live subject, and the refusal stays where the model keeps it, in
/// the write.
pub struct IncompatData;

impl Value for IncompatData {
    type Model = incompats::Incompatibility;

    const CLASS: &'static str = "IncompatData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<incompats::Incompatibility> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names.
        Ok(incompats::Incompatibility {
            name: plain_text(site, "name", obj)?,
            subject_id: entity::<Subject>(doc, site, "subject", obj)?,
            slots: time_slots(site, "slots", obj)?,
            minimum_free_slots: non_zero_count(site, "minimum_free_slots", obj)?,
            week_pattern_id: optional_entity::<WeekPattern>(doc, site, "week_pattern", obj)?,
        })
    }

    fn to_py<'py>(
        py: Python<'py>,
        incompat: &incompats::Incompatibility,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("name", &incompat.name)?;
        kwargs.set_item("subject", SubjectId::wrap(incompat.subject_id))?;
        kwargs.set_item(
            "slots",
            PyList::new(py, incompat.slots.iter().map(values::TimeSlot::from_model))?,
        )?;
        kwargs.set_item("minimum_free_slots", incompat.minimum_free_slots.get())?;
        kwargs.set_item(
            "week_pattern",
            incompat.week_pattern_id.map(WeekPatternId::wrap),
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One group list — a name, a student range, the group names, and the filling
///
/// The model splits the id-free parameters from the `#[fk]` filling, for its
/// reference walk; python flattens them, the way the handle already shows
/// them. What stays a sum is the filling itself: `PrefilledGroups` or
/// `AutomaticGroups`, two leaf classes under the `Filling` base — the
/// `{params, filling}` pair is sealed in the model, and the boundary calls
/// the model's own constructor, which is where the group-count and
/// duplicate-student checks stay.
pub struct GroupListData;

impl Value for GroupListData {
    /// The **entity**, and the op payload too: the group list ops carry the
    /// whole sealed `GroupList`, so §2.0 of the design says nothing new here.
    type Model = group_lists::GroupList;

    const CLASS: &'static str = "GroupListData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<group_lists::GroupList> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names. The two value-
        // internal invariants — a prefilled filling has exactly as many groups
        // as `group_names`, and no student appears in two — are the model's
        // own, and their message is the one a script meets.
        let params = group_lists::GroupListParameters {
            name: plain_text(site, "name", obj)?,
            students_per_group: nonzero_range(site, "students_per_group", obj)?,
            group_names: group_names(site, obj)?,
        };
        let filling = filling(doc, site, obj)?;

        group_lists::GroupList::new(params, filling)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn to_py<'py>(
        py: Python<'py>,
        group_list: &group_lists::GroupList,
    ) -> PyResult<Bound<'py, PyAny>> {
        let params = group_list.params();
        let kwargs = PyDict::new(py);
        kwargs.set_item("name", &params.name)?;
        kwargs.set_item(
            "students_per_group",
            values::nonzero_range(&params.students_per_group),
        )?;
        kwargs.set_item(
            "group_names",
            PyList::new(
                py,
                params
                    .group_names
                    .iter()
                    .map(|name| name.as_ref().map(|name| name.to_string())),
            )?,
        )?;
        kwargs.set_item("filling", filling_to_py(py, group_list.filling())?)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One end of a pairing rule, read at the site the script wrote it
///
/// Split out of [PairingRuleSideData::from_py] because a `PairingRuleData`
/// holds two of these and reads them at sites of their own: what a script
/// wrote there is a `PairingRuleData`, so that is the class a refusal names —
/// « a PairingRuleData's antecedent.subject ».
fn rule_side(
    doc: &Py<Document>,
    site: Site<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<pairings::RulePart> {
    // The fields are read in the order they are declared in the dataclass, so
    // the first bad one is the one a refusal names.
    Ok(pairings::RulePart {
        subject_id: entity::<Subject>(doc, site, "subject", obj)?,
        should_have: flag(site, "should_have", obj)?,
    })
}

/// One end of a pairing rule, and the python value for it
pub struct PairingRuleSideData;

impl Value for PairingRuleSideData {
    type Model = pairings::RulePart;

    const CLASS: &'static str = "PairingRuleSideData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<pairings::RulePart> {
        rule_side(doc, Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(py: Python<'py>, part: &pairings::RulePart) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("subject", SubjectId::wrap(part.subject_id))?;
        kwargs.set_item("should_have", part.should_have)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One pairing rule — two sides, the periods it skips, and the softness
///
/// The model's rule is sealed, with the one value-internal invariant — the
/// two ends must name different subjects, since an implication from a subject
/// to itself is meaningless — enforced by `PairingRule::new`. The boundary
/// calls it, so its message is the one a script meets. That both subjects
/// hold interrogations is a statement about the document, and it stays with
/// the write.
pub struct PairingRuleData;

impl Value for PairingRuleData {
    /// The **entity**, and the op payload too: the pairing rule ops carry the
    /// whole sealed `PairingRule`, so §2.0 of the design says nothing new
    /// here.
    type Model = pairings::PairingRule;

    const CLASS: &'static str = "PairingRuleData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<pairings::PairingRule> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the
        // dataclass, so the first bad one is the one a refusal names. The
        // one value-internal invariant — distinct subjects in the two parts —
        // is the model's own, and its message is the one a script meets.
        let antecedent = field(site, "antecedent", obj)?;
        let antecedent = rule_side(doc, site.inside("antecedent"), &antecedent)?;
        let consequent = field(site, "consequent", obj)?;
        let consequent = rule_side(doc, site.inside("consequent"), &consequent)?;
        let excluded_periods = entity_set::<Period>(doc, site, "excluded_periods", obj)?;
        let soft = flag(site, "soft", obj)?;

        pairings::PairingRule::new(antecedent, consequent, excluded_periods, soft)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn to_py<'py>(py: Python<'py>, rule: &pairings::PairingRule) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "antecedent",
            PairingRuleSideData::to_py(py, rule.antecedent())?,
        )?;
        kwargs.set_item(
            "consequent",
            PairingRuleSideData::to_py(py, rule.consequent())?,
        )?;
        kwargs.set_item(
            "excluded_periods",
            PySet::new(
                py,
                rule.excluded_periods().iter().map(|id| PeriodId::wrap(*id)),
            )?,
        )?;
        kwargs.set_item("soft", rule.soft())?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One end of a slot pairing rule, read at the site the script wrote it
///
/// The slots' twin of [rule_side], with a slot in place of a subject.
fn slot_rule_side(
    doc: &Py<Document>,
    site: Site<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<slot_pairings::SlotRulePart> {
    Ok(slot_pairings::SlotRulePart {
        slot_id: entity::<Slot>(doc, site, "slot", obj)?,
        should_have: flag(site, "should_have", obj)?,
    })
}

/// One end of a slot pairing rule, and the python value for it
pub struct SlotPairingRuleSideData;

impl Value for SlotPairingRuleSideData {
    type Model = slot_pairings::SlotRulePart;

    const CLASS: &'static str = "SlotPairingRuleSideData";

    fn from_py(
        doc: &Py<Document>,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<slot_pairings::SlotRulePart> {
        slot_rule_side(doc, Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(
        py: Python<'py>,
        part: &slot_pairings::SlotRulePart,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("slot", SlotId::wrap(part.slot_id))?;
        kwargs.set_item("should_have", part.should_have)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One slot pairing rule — two sides, the periods it skips, and the softness
///
/// The slots' twin of [PairingRuleData], sealed the same way: the two ends
/// must name different slots, and that both slots belong to one subject is a
/// statement about the document that stays with the write.
pub struct SlotPairingRuleData;

impl Value for SlotPairingRuleData {
    /// The **entity**, and the op payload too: the slot pairing rule ops
    /// carry the whole sealed `SlotPairingRule`, so §2.0 of the design says
    /// nothing new here.
    type Model = slot_pairings::SlotPairingRule;

    const CLASS: &'static str = "SlotPairingRuleData";

    fn from_py(
        doc: &Py<Document>,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<slot_pairings::SlotPairingRule> {
        let site = Site::whole(Self::CLASS);

        let antecedent = field(site, "antecedent", obj)?;
        let antecedent = slot_rule_side(doc, site.inside("antecedent"), &antecedent)?;
        let consequent = field(site, "consequent", obj)?;
        let consequent = slot_rule_side(doc, site.inside("consequent"), &consequent)?;
        let excluded_periods = entity_set::<Period>(doc, site, "excluded_periods", obj)?;
        let soft = flag(site, "soft", obj)?;

        slot_pairings::SlotPairingRule::new(antecedent, consequent, excluded_periods, soft)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn to_py<'py>(
        py: Python<'py>,
        rule: &slot_pairings::SlotPairingRule,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "antecedent",
            SlotPairingRuleSideData::to_py(py, rule.antecedent())?,
        )?;
        kwargs.set_item(
            "consequent",
            SlotPairingRuleSideData::to_py(py, rule.consequent())?,
        )?;
        kwargs.set_item(
            "excluded_periods",
            PySet::new(
                py,
                rule.excluded_periods().iter().map(|id| PeriodId::wrap(*id)),
            )?,
        )?;
        kwargs.set_item("soft", rule.soft())?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// Compiles `data.py` and puts the classes it defines in the module
///
/// Compiling from a string rather than shipping a package is what makes the
/// hosted path need no filesystem at all (`new_api_design.md` §12), and the
/// same code runs for the wheel — one mechanism rather than one per build
/// shape. The compilation happens once per interpreter, when `collomatique` is
/// first imported.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    let code = CString::new(DATA_PY).expect("data.py has no interior nul");
    let data = PyModule::from_code(py, &code, c"collomatique/_data.py", c"collomatique._data")?;

    // A submodule hung off its parent is not one python can `import`, the way
    // `dialogs` already is not; and a module in `sys.modules` that its parent
    // does not carry as an attribute is one `import collomatique._data` binds
    // and then cannot reach. Both halves are needed, and both are cheap.
    py.import("sys")?
        .getattr("modules")?
        .set_item(MODULE, &data)?;
    m.add("_data", &data)?;

    // `__all__` is the list, so a class added in a later commit is added in
    // `data.py` alone and the two sides cannot drift apart.
    for name in data.getattr("__all__")?.extract::<Vec<String>>()? {
        let class = data.getattr(name.as_str())?;

        // Where the class says it lives, and where a script really finds it.
        // `collomatique._data` is an implementation detail §2.1 of the design
        // says a script never names, and every rust class in this module
        // already claims `collomatique` through `#[pyclass(module = ...)]`.
        class.setattr("__module__", "collomatique")?;

        m.add(name.as_str(), class)?;
    }

    Ok(())
}
