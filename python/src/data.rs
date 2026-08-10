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

use pyo3::PyClass;
use pyo3::exceptions::{PyAttributeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::boolean_struct::True;
use pyo3::types::{PyDict, PySet};

use collomatique_state_colloscopes::{PersonWithContact, students, teachers};

use crate::Document;
use crate::collections::{Period, Subject};
use crate::handles::{Handle, RawId, argument, shown};
use crate::ids::{IdClass, PeriodId, SubjectId};

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

/// One field of a value, by attribute access
///
/// Never `cast::<T>()`. A value is a *python* object, so anything carrying the
/// right attributes is one — a script may perfectly well subclass a dataclass,
/// and duck typing is the language's own convention for this shape. What is
/// refused is an object that does not have the field at all, and the refusal
/// names the class that was expected, the way an argument of the wrong kind
/// already does.
fn field<'py>(class: &str, name: &str, obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    obj.getattr(name).map_err(|e| {
        if e.is_instance_of::<PyAttributeError>(obj.py()) {
            PyTypeError::new_err(format!(
                "a {class} is expected here, and {} has no {name}",
                shown(obj, "that object"),
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
fn plain_text(class: &str, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = field(class, name, obj)?;
    value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "a {class}'s {name} is a string, and {} is not one",
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
    class: &str,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Option<T>> {
    let value = field(class, name, obj)?;
    if value.is_none() {
        return Ok(None);
    }

    let text: String = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "a {class}'s {name} is a string or None, and {} is neither",
            shown(&value, "that value"),
        ))
    })?;

    T::try_from(text).map(Some).map_err(|_| {
        PyValueError::new_err(format!(
            "a {class}'s {name} is a non-empty string or None, and '' is neither"
        ))
    })
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
    class: &str,
    name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<BTreeSet<RawId<H>>>
where
    H: Handle + PyClass<Frozen = True> + Sync,
    H::IdClass: PyClass<Frozen = True> + Sync,
{
    let value = field(class, name, obj)?;
    let items = value.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "a {class}'s {name} is a set of entities, and {} cannot be iterated over",
            shown(&value, "that value"),
        ))
    })?;

    items.map(|item| argument::<H>(doc, &item?)).collect()
}

/// The person card the two classes share, read off a value
///
/// The fields are read in the order they are declared in, so the first bad one
/// is the one named — rust evaluates a struct literal's fields in the order they
/// are written, which is why `firstname` comes first here although the model
/// declares `surname` first.
fn person(class: &str, obj: &Bound<'_, PyAny>) -> PyResult<PersonWithContact> {
    Ok(PersonWithContact {
        firstname: plain_text(class, "firstname", obj)?,
        surname: plain_text(class, "surname", obj)?,
        tel: optional_text(class, "tel", obj)?,
        email: optional_text(class, "email", obj)?,
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
        Ok(teachers::Teacher {
            desc: person(Self::CLASS, obj)?,
            subjects: entity_set::<Subject>(doc, Self::CLASS, "subjects", obj)?,
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
        Ok(students::Student {
            desc: person(Self::CLASS, obj)?,
            excluded_periods: entity_set::<Period>(doc, Self::CLASS, "excluded_periods", obj)?,
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
