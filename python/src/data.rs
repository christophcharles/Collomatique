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

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::num::NonZeroU32;

use pyo3::PyClass;
use pyo3::exceptions::{PyAttributeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::boolean_struct::True;
use pyo3::types::{PyDict, PyFrozenSet, PyList, PySet, PyTuple};

use collomatique_ops::ColloscopeContents;
use collomatique_state::{OrderedTable, Table};
use collomatique_state_colloscopes::export_config::{
    ColloscopeConfig as RawColloscopeConfig, Color as RawColor, ExportConfig as RawExportConfig,
    GlobalConfig as RawGlobalConfig, PageOrientation, PerGroupListConfig as RawPerGroupListConfig,
    PerStudentGroupsConfig as RawPerStudentGroupsConfig,
};
use collomatique_state_colloscopes::{
    InnerData, NonEmptyRangeInclusive, PersonWithContact, SubjectInterrogationParameters,
    SubjectPeriodicity, assignments, balancing, colloscope_params, colloscopes, group_lists,
    incompats, pairings, periods, settings, slot_pairings, slots, students, subjects, teachers,
    week_patterns, weeks,
};
use collomatique_time::WeekStart;

use crate::Document;
use crate::collections::{
    GroupList, Incompat, PairingRule, Period, Slot, SlotPairingRule, Student, Subject, Teacher,
    Week, WeekPattern,
};
use crate::handles::{Handle, RawId, argument, shown};
use crate::ids::{
    GroupListId, IdClass, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId,
    StudentId, SubjectId, TeacherId, WeekId, WeekPatternId,
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

/// A field the model stores as an optional limit on interrogations
///
/// A `Limit` leaf or `None`. The leaf was born whole, so nothing about its
/// range is checked here: a limit of zero is a value the per-week fields of
/// the model hold, and only the per-day field refuses it, in its own helper.
fn optional_limit(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<settings::SoftParam<u32>>> {
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    let limit: values::Limit = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a Limit or None, and {} is neither",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(Some(settings::SoftParam {
        soft: limit.enforcement.to_model(),
        value: limit.value,
    }))
}

/// The same, for a count the model stores as at least one
///
/// The per-day limit is the one field the model types non-zero, so a `Limit`
/// of zero is refused here, in the words `values.rs` already uses for that
/// rule — the promise `Limit`'s own docstring makes.
fn optional_nonzero_limit(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<settings::SoftParam<NonZeroU32>>> {
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    let limit: values::Limit = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a Limit or None, and {} is neither",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(Some(settings::SoftParam {
        soft: limit.enforcement.to_model(),
        value: values::at_least_one(&site.field(name), limit.value)?,
    }))
}

/// A field the model stores as an optional rotation goal
///
/// An `Enforcement` or `None` — the balancing goals carry no value of their
/// own, the model's `SoftParam<()>` is a three-state switch, so only the
/// enforcement crosses.
fn optional_enforcement(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<settings::SoftParam<()>>> {
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    let enforcement: values::Enforcement = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is an Enforcement or None, and {} is neither",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(Some(settings::SoftParam {
        soft: enforcement.to_model(),
        value: (),
    }))
}

/// A field holding a [values::Color]
///
/// The colors of the export configuration. The leaf was born whole, so its
/// three channels were checked when it was built; nothing is refused here but
/// an object that is not a color at all.
fn color(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<RawColor> {
    let value = field(site, name, obj)?;
    let color: values::Color = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a Color, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(color.to_model())
}

/// A field holding an orientation
///
/// The [values::Orientation] leaf was born whole, so nothing is refused here
/// but an object that is not an orientation at all.
fn orientation(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<PageOrientation> {
    let value = field(site, name, obj)?;
    let orientation: values::Orientation = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is an Orientation, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(orientation.to_model())
}

/// A field holding an orientation, or naming none
///
/// The auto-detect case of the per-student-groups sheets: `None` means the
/// orientation is chosen from the group count when the sheet is written, so it
/// is a value the model holds and not a field left unfilled.
fn optional_orientation(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<PageOrientation>> {
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    let orientation: values::Orientation = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is an Orientation or None, and {} is neither",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    Ok(Some(orientation.to_model()))
}

/// A field holding the extra cell colors, by the label that names them
///
/// Anything mapping-like is accepted — a dict, whatever has an `items()` —
/// and every pair must be a name and a [values::Color]. The leaf was born
/// whole, so nothing else can be refused here.
fn color_map(
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<BTreeMap<String, RawColor>> {
    let value = field(site, name, obj)?;
    let items = value.call_method0("items").map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a mapping of names to colors, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    let mut colors = BTreeMap::new();
    for item in items.try_iter()? {
        let item = item?;
        let (label, color): (String, values::Color) = item.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} holds pairs of a name and a Color, and {} is not one",
                site.field(name),
                shown(&item, "that pair"),
            ))
        })?;
        colors.insert(label, color.to_model());
    }

    Ok(colors)
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
    entity_members::<H>(doc, site, name, &value)
}

/// The members of one iterable of entities, each resolved like every entity
/// reference
///
/// The body of [entity_set], split out so that a set read *inside* another
/// value — the students of one `assignments` row of a `DocumentData` — goes
/// through the same door as a field that is itself a set.
fn entity_members<H>(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<BTreeSet<RawId<H>>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    let items = value.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a set of entities, and {} cannot be iterated over",
            site.field(name),
            shown(value, "that value"),
        ))
    })?;

    items.map(|item| argument::<H>(doc, &item?)).collect()
}

/// A field holding a whole section keyed by entity ids
///
/// The tree of `DocumentData` is a dict per entity section. The keys follow
/// §2.3 like every other entity reference — a handle and an id name the same
/// entity, against this document — and each entry is a value of its own class,
/// extracted the way every other value is, at the site of the section it
/// sits in.
///
/// The two spellings give a mapping one more way to go wrong: a handle and an
/// id of one entity are *different* dict keys, so a section can name the same
/// entity twice. Keeping the last entry would be a silent loss, so the double
/// naming is refused instead, naming the id both spellings resolve to.
fn entity_dict<H, V>(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Vec<(RawId<H>, V::Model)>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
    V: Value,
{
    let value = field(site, name, obj)?;
    let items = value.call_method0("items").map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a mapping of entities to values, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    let mut entries = Vec::new();
    for item in items.try_iter()? {
        let item = item?;
        let (key, entry): (Bound<'_, PyAny>, Bound<'_, PyAny>) = item.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} holds pairs of an entity and a value, and {} is not one",
                site.field(name),
                shown(&item, "that pair"),
            ))
        })?;
        let id = argument::<H>(doc, &key)?;
        if entries.iter().any(|(seen, _)| *seen == id) {
            return Err(PyValueError::new_err(format!(
                "{} names {} twice",
                site.field(name),
                <H::IdClass as IdClass>::text(id),
            )));
        }
        entries.push((id, V::from_py(doc, &entry)?));
    }

    Ok(entries)
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

/// The python limit for one model limit whose count is a plain int
fn limit_to_py<'py>(
    py: Python<'py>,
    soft: &settings::SoftParam<u32>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(Py::new(py, values::limit(soft))?.into_bound(py).into_any())
}

/// The same, for a count the model stores as at least one
fn nonzero_limit_to_py<'py>(
    py: Python<'py>,
    soft: &settings::SoftParam<NonZeroU32>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(Py::new(py, values::nonzero_limit(soft))?
        .into_bound(py)
        .into_any())
}

/// One whole limits entry — the three limits, or the fields that disable them
///
/// The whole-entry rule is the model's: a field set to `None` does not mean
/// "inherit", it **disables** the corresponding limit of the entry the student
/// inherits from. That semantic stays in the model — this boundary only ever
/// carries a whole raw entry across, never a merge, and the dataclass is dumb
/// about what a `None` means.
pub struct LimitsData;

impl Value for LimitsData {
    type Model = settings::Limits;

    const CLASS: &'static str = "LimitsData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(_doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<settings::Limits> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names.
        Ok(settings::Limits {
            interrogations_per_week_min: optional_limit(site, "interrogations_per_week_min", obj)?,
            interrogations_per_week_max: optional_limit(site, "interrogations_per_week_max", obj)?,
            max_interrogations_per_day: optional_nonzero_limit(
                site,
                "max_interrogations_per_day",
                obj,
            )?,
        })
    }

    fn to_py<'py>(py: Python<'py>, limits: &settings::Limits) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "interrogations_per_week_min",
            limits
                .interrogations_per_week_min
                .as_ref()
                .map(|soft| limit_to_py(py, soft))
                .transpose()?,
        )?;
        kwargs.set_item(
            "interrogations_per_week_max",
            limits
                .interrogations_per_week_max
                .as_ref()
                .map(|soft| limit_to_py(py, soft))
                .transpose()?,
        )?;
        kwargs.set_item(
            "max_interrogations_per_day",
            limits
                .max_interrogations_per_day
                .as_ref()
                .map(|soft| nonzero_limit_to_py(py, soft))
                .transpose()?,
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One whole balancing entry — the rotation goals, and the fairness switches
///
/// Like a [LimitsData], a whole-entry override record: a goal set to `None` is
/// *not pursued*, and that is a choice, never a fallback to the inherited
/// entry. The semantic stays in the model; this boundary carries whole raw
/// entries across.
pub struct BalancingData;

impl Value for BalancingData {
    type Model = balancing::BalancingOptions;

    const CLASS: &'static str = "BalancingData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(
        _doc: &Py<Document>,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<balancing::BalancingOptions> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names.
        Ok(balancing::BalancingOptions {
            teacher_rotation: optional_enforcement(site, "teacher_rotation", obj)?,
            slot_rotation: optional_enforcement(site, "slot_rotation", obj)?,
            avoid_twice_in_a_row: optional_enforcement(site, "avoid_twice_in_a_row", obj)?,
            year_teacher_rotation: flag(site, "year_teacher_rotation", obj)?,
            period_teacher_rotation: flag(site, "period_teacher_rotation", obj)?,
        })
    }

    fn to_py<'py>(
        py: Python<'py>,
        options: &balancing::BalancingOptions,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "teacher_rotation",
            options
                .teacher_rotation
                .as_ref()
                .map(|goal| values::Enforcement::from_model(goal.soft)),
        )?;
        kwargs.set_item(
            "slot_rotation",
            options
                .slot_rotation
                .as_ref()
                .map(|goal| values::Enforcement::from_model(goal.soft)),
        )?;
        kwargs.set_item(
            "avoid_twice_in_a_row",
            options
                .avoid_twice_in_a_row
                .as_ref()
                .map(|goal| values::Enforcement::from_model(goal.soft)),
        )?;
        kwargs.set_item("year_teacher_rotation", options.year_teacher_rotation)?;
        kwargs.set_item("period_teacher_rotation", options.period_teacher_rotation)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The settings shared by every sheet of the export, read at the site the
/// script wrote them
///
/// Split out of [ExportGlobalConfigData::from_py] because an `ExportConfigData`
/// holds one of these and reads it at a site of its own: what a script wrote
/// there is an `ExportConfigData`, so that is the class a refusal names — « an
/// ExportConfigData's global_config.background_color ».
fn global_config(site: Site<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RawGlobalConfig> {
    Ok(RawGlobalConfig {
        background_color: color(site, "background_color", obj)?,
        stripes_color_enabled: flag(site, "stripes_color_enabled", obj)?,
        stripes_color: color(site, "stripes_color", obj)?,
    })
}

/// The settings shared by every sheet of the export
///
/// One of the five classes of `docs/python/values.md` §3.9, and one of the
/// two of them that no entity lies behind at all: the whole configuration is
/// pure value data, so nothing here resolves against the document, and the
/// extraction cannot go stale — the views' `to_data()` is the only source of
/// one of these in this milestone.
pub struct ExportGlobalConfigData;

impl Value for ExportGlobalConfigData {
    type Model = RawGlobalConfig;

    const CLASS: &'static str = "ExportGlobalConfigData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(_doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<RawGlobalConfig> {
        global_config(Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(py: Python<'py>, config: &RawGlobalConfig) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "background_color",
            values::Color::from_model(&config.background_color),
        )?;
        kwargs.set_item("stripes_color_enabled", config.stripes_color_enabled)?;
        kwargs.set_item(
            "stripes_color",
            values::Color::from_model(&config.stripes_color),
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The settings of the colloscope sheet, read at the site the script wrote
/// them
///
/// Split out of [ExportColloscopeConfigData::from_py] for the same reason
/// [global_config] is: an `ExportConfigData` holds one of these and reads it
/// at a site of its own.
fn colloscope_config(site: Site<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RawColloscopeConfig> {
    Ok(RawColloscopeConfig {
        sheet_name: plain_text(site, "sheet_name", obj)?,
        extra_info_column_enabled: flag(site, "extra_info_column_enabled", obj)?,
        extra_info_column_name: plain_text(site, "extra_info_column_name", obj)?,
        teacher_email_enabled: flag(site, "teacher_email_enabled", obj)?,
        teacher_email: plain_text(site, "teacher_email", obj)?,
        teacher_tel_enabled: flag(site, "teacher_tel_enabled", obj)?,
        teacher_tel: plain_text(site, "teacher_tel", obj)?,
        orientation: orientation(site, "orientation", obj)?,
        display_week_dates: flag(site, "display_week_dates", obj)?,
        display_annotations: flag(site, "display_annotations", obj)?,
        no_interrogation_color: color(site, "no_interrogation_color", obj)?,
        annotation_color_enabled: flag(site, "annotation_color_enabled", obj)?,
        annotation_color: color(site, "annotation_color", obj)?,
        extra_colors: color_map(site, "extra_colors", obj)?,
    })
}

/// The settings of the colloscope sheet
///
/// The one export class with a container field: the extra cell colors, read
/// out as a plain dict — a value is written as well as read, and the read
/// surface's `mappingproxy` has no place in a builder.
pub struct ExportColloscopeConfigData;

impl Value for ExportColloscopeConfigData {
    type Model = RawColloscopeConfig;

    const CLASS: &'static str = "ExportColloscopeConfigData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(_doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<RawColloscopeConfig> {
        colloscope_config(Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(py: Python<'py>, config: &RawColloscopeConfig) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("sheet_name", &config.sheet_name)?;
        kwargs.set_item(
            "extra_info_column_enabled",
            config.extra_info_column_enabled,
        )?;
        kwargs.set_item("extra_info_column_name", &config.extra_info_column_name)?;
        kwargs.set_item("teacher_email_enabled", config.teacher_email_enabled)?;
        kwargs.set_item("teacher_email", &config.teacher_email)?;
        kwargs.set_item("teacher_tel_enabled", config.teacher_tel_enabled)?;
        kwargs.set_item("teacher_tel", &config.teacher_tel)?;
        kwargs.set_item(
            "orientation",
            values::Orientation::from_model(&config.orientation),
        )?;
        kwargs.set_item("display_week_dates", config.display_week_dates)?;
        kwargs.set_item("display_annotations", config.display_annotations)?;
        kwargs.set_item(
            "no_interrogation_color",
            values::Color::from_model(&config.no_interrogation_color),
        )?;
        kwargs.set_item("annotation_color_enabled", config.annotation_color_enabled)?;
        kwargs.set_item(
            "annotation_color",
            values::Color::from_model(&config.annotation_color),
        )?;

        let extra_colors = PyDict::new(py);
        for (name, color) in &config.extra_colors {
            extra_colors.set_item(name, values::Color::from_model(color))?;
        }
        kwargs.set_item("extra_colors", extra_colors)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The settings of one per-student-groups sheet, read at the site the script
/// wrote them
///
/// Split out of [ExportStudentGroupsConfigData::from_py] for the same reason
/// [global_config] is: an `ExportConfigData` holds three of these and reads
/// them at sites of their own.
fn student_groups_config(
    site: Site<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<RawPerStudentGroupsConfig> {
    Ok(RawPerStudentGroupsConfig {
        sheet_name: plain_text(site, "sheet_name", obj)?,
        orientation: optional_orientation(site, "orientation", obj)?,
        show_emails: flag(site, "show_emails", obj)?,
        show_tel: flag(site, "show_tel", obj)?,
    })
}

/// The settings of one per-student-groups sheet
///
/// The model has three constructors rather than one `Default` —
/// `default_all_groups` and its two siblings — so the dataclass mirrors them
/// as three classmethods and takes a required `sheet_name`: the one field
/// that says *which* sheet a value is for (`docs/python/values.md` §3.9).
pub struct ExportStudentGroupsConfigData;

impl Value for ExportStudentGroupsConfigData {
    type Model = RawPerStudentGroupsConfig;

    const CLASS: &'static str = "ExportStudentGroupsConfigData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(_doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<RawPerStudentGroupsConfig> {
        student_groups_config(Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(
        py: Python<'py>,
        config: &RawPerStudentGroupsConfig,
    ) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("sheet_name", &config.sheet_name)?;
        kwargs.set_item(
            "orientation",
            config
                .orientation
                .as_ref()
                .map(values::Orientation::from_model),
        )?;
        kwargs.set_item("show_emails", config.show_emails)?;
        kwargs.set_item("show_tel", config.show_tel)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The settings of the per-group-list sheets, read at the site the script
/// wrote them
///
/// Split out of [ExportGroupListConfigData::from_py] for the same reason
/// [global_config] is: an `ExportConfigData` holds one of these and reads it
/// at a site of its own.
fn group_list_config(site: Site<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RawPerGroupListConfig> {
    Ok(RawPerGroupListConfig {
        orientation: orientation(site, "orientation", obj)?,
        show_emails: flag(site, "show_emails", obj)?,
        show_tel: flag(site, "show_tel", obj)?,
        center_vertically: flag(site, "center_vertically", obj)?,
    })
}

/// The settings of the per-group-list sheets
///
/// The smallest of the export classes, and the one with the only model field
/// the whole configuration has that a `PerStudentGroupsConfig` field is not:
/// `center_vertically`.
pub struct ExportGroupListConfigData;

impl Value for ExportGroupListConfigData {
    type Model = RawPerGroupListConfig;

    const CLASS: &'static str = "ExportGroupListConfigData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(_doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<RawPerGroupListConfig> {
        group_list_config(Site::whole(Self::CLASS), obj)
    }

    fn to_py<'py>(py: Python<'py>, config: &RawPerGroupListConfig) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "orientation",
            values::Orientation::from_model(&config.orientation),
        )?;
        kwargs.set_item("show_emails", config.show_emails)?;
        kwargs.set_item("show_tel", config.show_tel)?;
        kwargs.set_item("center_vertically", config.center_vertically)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The whole export configuration
///
/// The tree `doc.export_config.to_data()` assembles, and the value §8's
/// `DocumentData` will hold one of. No op takes it: the eleven export
/// mutators each patch one field of the document's own configuration, so
/// nothing in this milestone consumes one — its extraction has no caller
/// until the coarse door lands, which is what the dataclass's docstring says
/// rather than hiding.
pub struct ExportConfigData;

impl Value for ExportConfigData {
    type Model = RawExportConfig;

    const CLASS: &'static str = "ExportConfigData";

    /// Names no entity, so the document goes unused here — the one asymmetry
    /// §2.2 of the design accepts, since two shapes for one boundary would be
    /// worse than one shape carrying an argument it sometimes ignores.
    fn from_py(_doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<RawExportConfig> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names. The four nested
        // configs are fetched first and read at sites of their own, so a
        // refusal names the class a script wrote down: « an ExportConfigData's
        // colloscope_config.teacher_email ».
        let global = field(site, "global_config", obj)?;
        let colloscope = field(site, "colloscope_config", obj)?;
        let all_groups = field(site, "all_groups_config", obj)?;
        let automatic = field(site, "automatic_groups_config", obj)?;
        let prefilled = field(site, "prefilled_groups_config", obj)?;
        let per_group_list = field(site, "per_group_list_config", obj)?;

        Ok(RawExportConfig {
            global: global_config(site.inside("global_config"), &global)?,
            colloscope_enabled: flag(site, "colloscope_enabled", obj)?,
            all_groups_enabled: flag(site, "all_groups_enabled", obj)?,
            automatic_groups_enabled: flag(site, "automatic_groups_enabled", obj)?,
            prefilled_groups_enabled: flag(site, "prefilled_groups_enabled", obj)?,
            per_group_list_enabled: flag(site, "per_group_list_enabled", obj)?,
            colloscope_config: colloscope_config(site.inside("colloscope_config"), &colloscope)?,
            all_groups_config: student_groups_config(
                site.inside("all_groups_config"),
                &all_groups,
            )?,
            automatic_groups_config: student_groups_config(
                site.inside("automatic_groups_config"),
                &automatic,
            )?,
            prefilled_groups_config: student_groups_config(
                site.inside("prefilled_groups_config"),
                &prefilled,
            )?,
            per_group_list_config: group_list_config(
                site.inside("per_group_list_config"),
                &per_group_list,
            )?,
        })
    }

    fn to_py<'py>(py: Python<'py>, config: &RawExportConfig) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "global_config",
            ExportGlobalConfigData::to_py(py, &config.global)?,
        )?;
        kwargs.set_item("colloscope_enabled", config.colloscope_enabled)?;
        kwargs.set_item("all_groups_enabled", config.all_groups_enabled)?;
        kwargs.set_item("automatic_groups_enabled", config.automatic_groups_enabled)?;
        kwargs.set_item("prefilled_groups_enabled", config.prefilled_groups_enabled)?;
        kwargs.set_item("per_group_list_enabled", config.per_group_list_enabled)?;
        kwargs.set_item(
            "colloscope_config",
            ExportColloscopeConfigData::to_py(py, &config.colloscope_config)?,
        )?;
        kwargs.set_item(
            "all_groups_config",
            ExportStudentGroupsConfigData::to_py(py, &config.all_groups_config)?,
        )?;
        kwargs.set_item(
            "automatic_groups_config",
            ExportStudentGroupsConfigData::to_py(py, &config.automatic_groups_config)?,
        )?;
        kwargs.set_item(
            "prefilled_groups_config",
            ExportStudentGroupsConfigData::to_py(py, &config.prefilled_groups_config)?,
        )?;
        kwargs.set_item(
            "per_group_list_config",
            ExportGroupListConfigData::to_py(py, &config.per_group_list_config)?,
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The interrogation rows of a colloscope value — the cells, and their groups
///
/// The same shape as `ColloscopeContents`'s field of the same name, written
/// out as a name so the boundary's two directions agree with the model and
/// with each other.
type InterrogationRows = BTreeMap<(RawId<Slot>, RawId<Week>), BTreeSet<u32>>;

/// The placements rows of a colloscope value — the lists, and their placements
///
/// The same shape as `ColloscopeContents`'s field of the same name, written
/// out as a name so the boundary's two directions agree with the model and
/// with each other.
type PlacementRows = BTreeMap<RawId<GroupList>, BTreeMap<RawId<Student>, u32>>;

/// A field holding the interrogation rows of a colloscope
///
/// A mapping of `(slot, week)` pairs to sets of group numbers — one of the
/// two sparse tables of the colloscope, read out as a value. Each end of a
/// pair resolves like every other entity reference: a handle or an id,
/// against this document, so a foreign handle and a dead id are refused
/// here, by the same [crate::handles::argument] check every method uses.
/// The group numbers are plain ints, and an empty set is the "no row" the
/// payload promises rather than a shape to correct.
fn interrogation_rows(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<InterrogationRows> {
    let value = field(site, name, obj)?;
    let items = value.call_method0("items").map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a mapping of (slot, week) pairs to sets of group numbers, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    let mut rows = BTreeMap::new();
    for item in items.try_iter()? {
        let item = item?;
        let (key, groups): (Bound<'_, PyAny>, Bound<'_, PyAny>) = item.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} holds pairs of a (slot, week) pair and a set of group numbers, and {} is \
                 not one",
                site.field(name),
                shown(&item, "that pair"),
            ))
        })?;
        let (slot, week): (Bound<'_, PyAny>, Bound<'_, PyAny>) = key.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} holds (slot, week) pairs, and {} is not one",
                site.field(name),
                shown(&key, "that key"),
            ))
        })?;
        let groups: BTreeSet<u32> = groups.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "{} holds sets of group numbers, and {} is not one",
                site.field(name),
                shown(&groups, "that value"),
            ))
        })?;

        let cell = (argument::<Slot>(doc, &slot)?, argument::<Week>(doc, &week)?);
        if rows.insert(cell, groups).is_some() {
            return Err(PyValueError::new_err(format!(
                "{} names the ({}, {}) cell twice",
                site.field(name),
                SlotId::text(cell.0),
                WeekId::text(cell.1),
            )));
        }
    }

    Ok(rows)
}

/// A field holding the placements rows of a colloscope
///
/// The second of the two sparse tables: a mapping of group lists to student
/// placements. The outer keys resolve like every other entity reference —
/// a `GroupList` handle or a `GroupListId` — and so do the inner ones, the
/// placed students. A prefilled list never appears here: it has groups of
/// its own, so the model never fills it. An empty placement map is the "no
/// row" the payload promises rather than a shape to correct.
fn placement_rows(
    doc: &Py<Document>,
    site: Site<'_>,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<PlacementRows> {
    let value = field(site, name, obj)?;
    let items = value.call_method0("items").map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a mapping of group lists to student placements, and {} is not one",
            site.field(name),
            shown(&value, "that value"),
        ))
    })?;

    let mut rows = BTreeMap::new();
    for item in items.try_iter()? {
        let item = item?;
        let (group_list, placements): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
            item.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} holds pairs of a group list and a student placement, and {} is not one",
                    site.field(name),
                    shown(&item, "that pair"),
                ))
            })?;
        let group_list = argument::<GroupList>(doc, &group_list)?;

        let placed_items = placements.call_method0("items").map_err(|_| {
            PyTypeError::new_err(format!(
                "{} holds a mapping of students to group numbers, and {} is not one",
                site.field(name),
                shown(&placements, "that value"),
            ))
        })?;
        let mut placed = BTreeMap::new();
        for entry in placed_items.try_iter()? {
            let entry = entry?;
            let (student, group): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                entry.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "{} holds pairs of a student and a group number, and {} is not one",
                        site.field(name),
                        shown(&entry, "that pair"),
                    ))
                })?;
            let group: u32 = group.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} holds group numbers, and {} is not one",
                    site.field(name),
                    shown(&group, "that value"),
                ))
            })?;

            let student = argument::<Student>(doc, &student)?;
            if placed.insert(student, group).is_some() {
                return Err(PyValueError::new_err(format!(
                    "{} names {} twice in one placement",
                    site.field(name),
                    StudentId::text(student),
                )));
            }
        }

        if rows.insert(group_list, placed).is_some() {
            return Err(PyValueError::new_err(format!(
                "{} names {} twice",
                site.field(name),
                GroupListId::text(group_list),
            )));
        }
    }

    Ok(rows)
}

/// The whole colloscope — the two sparse tables, detached
///
/// The result of a resolution: the assigned group numbers per `(slot, week)`
/// cell, and the placements of each automatic group list. The op payload and
/// the entity are the same shape here — [ColloscopeContents] is the
/// plain-map twin of the state's sparse `Colloscope`, and
/// `InstallColloscope` takes it whole — so §2.0 of the design says nothing
/// new. The group *numbers* are not ids. An empty group set or an empty
/// placement map means "no row", which is what the payload promises its
/// callers: the boundary is dumb, and the canonical form stays with the
/// write that reads the value back.
pub struct ColloscopeData;

impl Value for ColloscopeData {
    type Model = ColloscopeContents;

    const CLASS: &'static str = "ColloscopeData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<ColloscopeContents> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names.
        Ok(ColloscopeContents {
            interrogations: interrogation_rows(doc, site, "interrogations", obj)?,
            group_lists: placement_rows(doc, site, "group_lists", obj)?,
        })
    }

    fn to_py<'py>(py: Python<'py>, contents: &ColloscopeContents) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);

        let interrogations = PyDict::new(py);
        for ((slot, week), groups) in &contents.interrogations {
            interrogations.set_item(
                (SlotId::wrap(*slot), WeekId::wrap(*week)),
                PySet::new(py, groups)?,
            )?;
        }
        kwargs.set_item("interrogations", interrogations)?;

        let group_lists = PyDict::new(py);
        for (group_list, placements) in &contents.group_lists {
            let placed = PyDict::new(py);
            for (student, group) in placements {
                placed.set_item(StudentId::wrap(*student), group)?;
            }
            group_lists.set_item(GroupListId::wrap(*group_list), placed)?;
        }
        kwargs.set_item("group_lists", group_lists)?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One week — the period it belongs to, whether colles run on it, and its label
///
/// The one value class no op consumes: the two week ops carry a single field
/// each (`UpdateWeekStatus` a bool, `UpdateWeekAnnotation` an annotation),
/// addressed by `(period, index)`. So the class exists for `week.to_data()`
/// and for the whole-document snapshot of the next commit, and the boundary
/// mirrors the model's stored `weeks::Week` — the entity, per §2.0 — whole.
pub struct WeekData;

impl Value for WeekData {
    type Model = weeks::Week;

    const CLASS: &'static str = "WeekData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<weeks::Week> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names.
        Ok(weeks::Week {
            period_id: entity::<Period>(doc, site, "period", obj)?,
            interrogations: flag(site, "interrogations", obj)?,
            annotation: optional_text(site, "annotation", obj)?,
        })
    }

    fn to_py<'py>(py: Python<'py>, week: &weeks::Week) -> PyResult<Bound<'py, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("period", PeriodId::wrap(week.period_id))?;
        kwargs.set_item("interrogations", week.interrogations)?;
        kwargs.set_item(
            "annotation",
            week.annotation.as_ref().map(|text| text.to_string()),
        )?;

        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// The weeks of a whole-document tree, regrouped by period
///
/// The shape [weeks::Weeks::from_period_rows] wants: one row per period, its
/// weeks in tree order. Written out as a name so the reconstruction agrees
/// with the model's constructor and the two sides cannot drift.
type WeekRows = Vec<(RawId<Period>, Vec<(RawId<Week>, weeks::WeekDesc)>)>;

/// The slots of a whole-document tree, regrouped by subject
///
/// The shape [slots::Slots::from_subject_rows] wants, the twin of [WeekRows].
type SlotRows = Vec<(RawId<Subject>, Vec<(RawId<Slot>, slots::Slot)>)>;

/// The whole document, detached — one value holding every section
///
/// The tree `doc.snapshot()` assembles (§3.12 of `docs/python/values.md`):
/// every section of `InnerData` as a field, with the user orders carried by
/// the python containers themselves. No op takes one and nothing reads one
/// back in this milestone — `replace_all`, the coarse door's other half, is
/// step 4's, and it will take this same tree through `Data::from_inner_data` —
/// but the value is full two-direction like every other, and the round-trip
/// test drives the inbound half through the same public door.
pub struct DocumentData;

impl Value for DocumentData {
    /// The whole document, and [InnerData] rather than [crate::Document]'s
    /// `Data`: the snapshot is a pure read of what the document holds, and the
    /// invariants `Data` adds on top are the coarse door's business.
    type Model = InnerData;

    const CLASS: &'static str = "DocumentData";

    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<InnerData> {
        let site = Site::whole(Self::CLASS);

        // The fields are read in the order they are declared in the dataclass,
        // so the first bad one is the one a refusal names.
        let first_week = field(site, "first_week", obj)?;
        let first_week = if first_week.is_none() {
            None
        } else {
            let date: chrono::NaiveDate = first_week.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} is a date or None, and {} is neither",
                    site.field("first_week"),
                    shown(&first_week, "that value"),
                ))
            })?;
            // The model only ever stores a Monday, so a tree that names another
            // day is refused here, in the words `set_first_week` already uses
            // for the same rule.
            Some(WeekStart::new(date).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "{} is a Monday, and {date} is not one",
                    site.field("first_week"),
                ))
            })?)
        };

        let periods = field(site, "periods", obj)?;
        let period_ids: Vec<RawId<Period>> = periods
            .try_iter()
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "{} is a list of periods, and {} is not one",
                    site.field("periods"),
                    shown(&periods, "that value"),
                ))
            })?
            .map(|item| argument::<Period>(doc, &item?))
            .collect::<PyResult<_>>()?;
        let periods = periods::Periods::from_ordered_ids(first_week, period_ids)
            .map_err(|e| PyValueError::new_err(format!("a DocumentData's periods: {e}")))?;

        let weeks = entity_dict::<Week, WeekData>(doc, site, "weeks", obj)?;
        // Regrouped by period, in order of first appearance: a tree whose weeks
        // run in global week order — every snapshot — rebuilds the same walk,
        // each period keeping its own order and the periods keeping the display
        // order the `periods` list carried.
        let mut week_rows: WeekRows = Vec::new();
        for (week_id, week) in weeks {
            let period = week.period_id;
            match week_rows
                .iter_mut()
                .find(|(period_id, _)| *period_id == period)
            {
                Some((_, list)) => list.push((week_id, week.desc())),
                None => week_rows.push((period, vec![(week_id, week.desc())])),
            }
        }
        let weeks = weeks::Weeks::from_period_rows(week_rows)
            .map_err(|e| PyValueError::new_err(format!("a DocumentData's weeks: {e}")))?;

        let subjects = entity_dict::<Subject, SubjectData>(doc, site, "subjects", obj)?;
        let ordered_subject_list: OrderedTable<_, _> = subjects
            .try_into()
            .map_err(|e| PyValueError::new_err(format!("a DocumentData's subjects: {e}")))?;
        let subjects = subjects::Subjects {
            ordered_subject_list,
        };

        let teachers = entity_dict::<Teacher, TeacherData>(doc, site, "teachers", obj)?
            .into_iter()
            .collect::<Table<_, _>>();
        let students = entity_dict::<Student, StudentData>(doc, site, "students", obj)?
            .into_iter()
            .collect::<Table<_, _>>();

        let assignments_value = field(site, "assignments", obj)?;
        let items = assignments_value.call_method0("items").map_err(|_| {
            PyTypeError::new_err(format!(
                "{} is a mapping of (period, subject) pairs to sets of students, and {} is not \
                 one",
                site.field("assignments"),
                shown(&assignments_value, "that value"),
            ))
        })?;
        let mut assignments = Table::new();
        for item in items.try_iter()? {
            let item = item?;
            let (key, students): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                item.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "{} holds pairs of a (period, subject) pair and a set of students, and \
                         {} is not one",
                        site.field("assignments"),
                        shown(&item, "that pair"),
                    ))
                })?;
            let (period, subject): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                key.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "{} holds (period, subject) pairs, and {} is not one",
                        site.field("assignments"),
                        shown(&key, "that key"),
                    ))
                })?;
            let row = (
                argument::<Period>(doc, &period)?,
                argument::<Subject>(doc, &subject)?,
            );
            let members = entity_members::<Student>(doc, site, "assignments", &students)?;
            if assignments.insert(row, members).is_some() {
                return Err(PyValueError::new_err(format!(
                    "{} names the ({}, {}) row twice",
                    site.field("assignments"),
                    PeriodId::text(row.0),
                    SubjectId::text(row.1),
                )));
            }
        }

        let week_patterns =
            entity_dict::<WeekPattern, WeekPatternData>(doc, site, "week_patterns", obj)?
                .into_iter()
                .collect::<Table<_, _>>();

        let slots = entity_dict::<Slot, SlotData>(doc, site, "slots", obj)?;
        // Regrouped by subject the same way the weeks are: subject-then-position
        // order in the tree rebuilds the same per-subject order and the same
        // walk.
        let mut slot_rows: SlotRows = Vec::new();
        for (slot_id, slot) in slots {
            let subject = slot.subject_id;
            match slot_rows
                .iter_mut()
                .find(|(subject_id, _)| *subject_id == subject)
            {
                Some((_, list)) => list.push((slot_id, slot)),
                None => slot_rows.push((subject, vec![(slot_id, slot)])),
            }
        }
        let slots = slots::Slots::from_subject_rows(slot_rows)
            .map_err(|e| PyValueError::new_err(format!("a DocumentData's slots: {e}")))?;

        let incompats = entity_dict::<Incompat, IncompatData>(doc, site, "incompats", obj)?
            .into_iter()
            .collect::<Table<_, _>>();

        let group_lists_value =
            entity_dict::<GroupList, GroupListData>(doc, site, "group_lists", obj)?;
        let group_list_map = group_lists_value.into_iter().collect::<Table<_, _>>();
        let associations_value = field(site, "group_list_associations", obj)?;
        let items = associations_value.call_method0("items").map_err(|_| {
            PyTypeError::new_err(format!(
                "{} is a mapping of (period, subject) pairs to group lists, and {} is not one",
                site.field("group_list_associations"),
                shown(&associations_value, "that value"),
            ))
        })?;
        let mut subjects_associations = Table::new();
        for item in items.try_iter()? {
            let item = item?;
            let (key, group_list): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                item.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "{} holds pairs of a (period, subject) pair and a group list, and {} is \
                         not one",
                        site.field("group_list_associations"),
                        shown(&item, "that pair"),
                    ))
                })?;
            let (period, subject): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                key.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "{} holds (period, subject) pairs, and {} is not one",
                        site.field("group_list_associations"),
                        shown(&key, "that key"),
                    ))
                })?;
            let row = (
                argument::<Period>(doc, &period)?,
                argument::<Subject>(doc, &subject)?,
            );
            let association = argument::<GroupList>(doc, &group_list)?;
            if subjects_associations.insert(row, association).is_some() {
                return Err(PyValueError::new_err(format!(
                    "{} names the ({}, {}) row twice",
                    site.field("group_list_associations"),
                    PeriodId::text(row.0),
                    SubjectId::text(row.1),
                )));
            }
        }
        let group_lists = group_lists::GroupLists {
            group_list_map,
            subjects_associations,
        };

        let pairings = entity_dict::<PairingRule, PairingRuleData>(doc, site, "pairings", obj)?
            .into_iter()
            .collect::<Table<_, _>>();
        let slot_pairings =
            entity_dict::<SlotPairingRule, SlotPairingRuleData>(doc, site, "slot_pairings", obj)?
                .into_iter()
                .collect::<Table<_, _>>();

        let global_limits = LimitsData::from_py(doc, &field(site, "global_limits", obj)?)?;
        let student_limits = entity_dict::<Student, LimitsData>(doc, site, "student_limits", obj)?
            .into_iter()
            .collect::<Table<_, _>>();

        let global_balancing = BalancingData::from_py(doc, &field(site, "global_balancing", obj)?)?;
        let subject_balancing =
            entity_dict::<Subject, BalancingData>(doc, site, "subject_balancing", obj)?
                .into_iter()
                .collect::<Table<_, _>>();

        let colloscope_value = field(site, "colloscope", obj)?;
        let contents = ColloscopeData::from_py(doc, &colloscope_value)?;
        let mut colloscope = colloscopes::Colloscope::default();
        for ((slot, week), groups) in &contents.interrogations {
            colloscope.set_interrogation(*slot, *week, groups.clone());
        }
        for (group_list, placements) in &contents.group_lists {
            colloscope.set_group_list(*group_list, placements.clone());
        }

        let export_config = ExportConfigData::from_py(doc, &field(site, "export_config", obj)?)?;

        Ok(InnerData {
            params: colloscope_params::Parameters {
                periods,
                weeks,
                subjects,
                teachers: teachers::Teachers {
                    teacher_map: teachers,
                },
                students: students::Students {
                    student_map: students,
                },
                assignments: assignments::Assignments { map: assignments },
                week_patterns: week_patterns::WeekPatterns {
                    week_pattern_map: week_patterns,
                },
                slots,
                incompats: incompats::Incompats {
                    incompat_map: incompats,
                },
                group_lists,
                settings: settings::Settings {
                    global: global_limits,
                    students: student_limits,
                },
                pairings: pairings::Pairings {
                    pairing_rule_map: pairings,
                },
                slot_pairings: slot_pairings::SlotPairings {
                    slot_pairing_rule_map: slot_pairings,
                },
                balancing: balancing::Balancing {
                    global: global_balancing,
                    subjects: subject_balancing,
                },
            },
            colloscope,
            export_config,
        })
    }

    fn to_py<'py>(py: Python<'py>, inner: &InnerData) -> PyResult<Bound<'py, PyAny>> {
        let params = &inner.params;
        let kwargs = PyDict::new(py);

        kwargs.set_item(
            "first_week",
            params
                .periods
                .first_week
                .as_ref()
                .map(|week| *week.monday()),
        )?;
        kwargs.set_item(
            "periods",
            PyList::new(py, params.periods.period_ids().map(PeriodId::wrap))?,
        )?;

        let weeks = PyDict::new(py);
        for (_, week_id, week) in params.walk_weeks() {
            weeks.set_item(WeekId::wrap(week_id), WeekData::to_py(py, week)?)?;
        }
        kwargs.set_item("weeks", weeks)?;

        let subjects = PyDict::new(py);
        for (subject_id, subject) in params.subjects.ordered_subject_list.iter() {
            subjects.set_item(
                SubjectId::wrap(subject_id),
                SubjectData::to_py(py, subject)?,
            )?;
        }
        kwargs.set_item("subjects", subjects)?;

        let teachers = PyDict::new(py);
        for (teacher_id, teacher) in params.teachers.teacher_map.iter() {
            teachers.set_item(
                TeacherId::wrap(teacher_id),
                TeacherData::to_py(py, teacher)?,
            )?;
        }
        kwargs.set_item("teachers", teachers)?;

        let students = PyDict::new(py);
        for (student_id, student) in params.students.student_map.iter() {
            students.set_item(
                StudentId::wrap(student_id),
                StudentData::to_py(py, student)?,
            )?;
        }
        kwargs.set_item("students", students)?;

        let assignments = PyDict::new(py);
        for ((period, subject), students) in params.assignments.map.iter() {
            assignments.set_item(
                (PeriodId::wrap(period), SubjectId::wrap(subject)),
                PySet::new(py, students.iter().map(|id| StudentId::wrap(*id)))?,
            )?;
        }
        kwargs.set_item("assignments", assignments)?;

        let week_patterns = PyDict::new(py);
        for (pattern_id, pattern) in params.week_patterns.week_pattern_map.iter() {
            week_patterns.set_item(
                WeekPatternId::wrap(pattern_id),
                WeekPatternData::to_py(py, pattern)?,
            )?;
        }
        kwargs.set_item("week_patterns", week_patterns)?;

        let slots = PyDict::new(py);
        for subject_id in params.subjects.ordered_subject_list.keys() {
            if let Some(list) = params.slots.slots_for_subject(subject_id) {
                for (slot_id, slot) in list {
                    slots.set_item(SlotId::wrap(*slot_id), SlotData::to_py(py, slot)?)?;
                }
            }
        }
        kwargs.set_item("slots", slots)?;

        let incompats = PyDict::new(py);
        for (incompat_id, incompat) in params.incompats.incompat_map.iter() {
            incompats.set_item(
                IncompatId::wrap(incompat_id),
                IncompatData::to_py(py, incompat)?,
            )?;
        }
        kwargs.set_item("incompats", incompats)?;

        let group_lists = PyDict::new(py);
        for (group_list_id, group_list) in params.group_lists.group_list_map.iter() {
            group_lists.set_item(
                GroupListId::wrap(group_list_id),
                GroupListData::to_py(py, group_list)?,
            )?;
        }
        kwargs.set_item("group_lists", group_lists)?;

        let associations = PyDict::new(py);
        for ((period, subject), group_list) in params.group_lists.subjects_associations.iter() {
            associations.set_item(
                (PeriodId::wrap(period), SubjectId::wrap(subject)),
                GroupListId::wrap(*group_list),
            )?;
        }
        kwargs.set_item("group_list_associations", associations)?;

        let pairings = PyDict::new(py);
        for (rule_id, rule) in params.pairings.pairing_rule_map.iter() {
            pairings.set_item(
                PairingRuleId::wrap(rule_id),
                PairingRuleData::to_py(py, rule)?,
            )?;
        }
        kwargs.set_item("pairings", pairings)?;

        let slot_pairings = PyDict::new(py);
        for (rule_id, rule) in params.slot_pairings.slot_pairing_rule_map.iter() {
            slot_pairings.set_item(
                SlotPairingRuleId::wrap(rule_id),
                SlotPairingRuleData::to_py(py, rule)?,
            )?;
        }
        kwargs.set_item("slot_pairings", slot_pairings)?;

        kwargs.set_item(
            "global_limits",
            LimitsData::to_py(py, &params.settings.global)?,
        )?;
        let student_limits = PyDict::new(py);
        for (student_id, limits) in params.settings.students.iter() {
            student_limits.set_item(StudentId::wrap(student_id), LimitsData::to_py(py, limits)?)?;
        }
        kwargs.set_item("student_limits", student_limits)?;

        kwargs.set_item(
            "global_balancing",
            BalancingData::to_py(py, &params.balancing.global)?,
        )?;
        let subject_balancing = PyDict::new(py);
        for (subject_id, options) in params.balancing.subjects.iter() {
            subject_balancing.set_item(
                SubjectId::wrap(subject_id),
                BalancingData::to_py(py, options)?,
            )?;
        }
        kwargs.set_item("subject_balancing", subject_balancing)?;

        kwargs.set_item(
            "colloscope",
            ColloscopeData::to_py(py, &ColloscopeContents::from(&inner.colloscope))?,
        )?;
        kwargs.set_item(
            "export_config",
            ExportConfigData::to_py(py, &inner.export_config)?,
        )?;

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
