//! The exceptions the `collomatique` module raises
//!
//! Every one of them descends from [Error], so a script that only wants "the
//! collomatique call failed" has one thing to catch. The tree below is the
//! seed of the one described in `docs/python/new_api_design.md` §6: the design
//! names `Error`, `NoOrigin` and `IdCeilingExceeded`, and leaves the ordinary
//! ways reading and writing a file fail unnamed — hence [LoadError] and
//! [SaveError].
//!
//! §6's per-family write errors (`SubjectsError`, `TeachersError`, …) are
//! below, and they all subclass [UpdateError], so a script that catches the
//! general one keeps catching all of them. They are not written out one by
//! one: the class comes from the family name the model's own error carries,
//! and everything under it — which op, which case, which entities — is walked
//! structurally ([crate::payload]), so an error variant added in `ops/` reaches a
//! script with no change here. A family this module has never heard of lands
//! on the base [UpdateError] rather than on a panic.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyTuple, PyType};

#[cfg(test)]
mod tests;

create_exception!(
    collomatique,
    Error,
    PyException,
    "Base class for every error the collomatique module raises."
);

create_exception!(
    collomatique,
    LoadError,
    Error,
    "A document could not be read from a file."
);

create_exception!(
    collomatique,
    SaveError,
    Error,
    "A document could not be written out, to a file or back to the application."
);

create_exception!(
    collomatique,
    NoOrigin,
    Error,
    "save() was called on a document that has nowhere to write."
);

create_exception!(
    collomatique,
    UpdateError,
    Error,
    "A write was refused: the document could not be made sense of that way."
);

create_exception!(
    collomatique,
    ExportError,
    Error,
    "An export could not be produced or written."
);

create_exception!(
    collomatique,
    ModelBuildError,
    Error,
    "The colloscope model could not be built."
);

create_exception!(
    collomatique,
    NothingToUndo,
    Error,
    "undo() or redo() was called with nothing left in that direction."
);

create_exception!(
    collomatique,
    NotHosted,
    Error,
    "A call that needs an application to talk to was made by a standalone script."
);

create_exception!(
    collomatique,
    NoDocument,
    Error,
    "There was no document to open, and nothing left to try."
);

create_exception!(
    collomatique,
    Cancelled,
    Error,
    "The user dismissed a dialog rather than answering it."
);

create_exception!(
    collomatique,
    DialogUnavailable,
    Error,
    "A dialog was asked for on a machine that cannot show one."
);

create_exception!(
    collomatique,
    StaleHandleError,
    Error,
    "A handle was read after the entity it names left the document."
);

create_exception!(
    collomatique,
    IdCeilingExceeded,
    SaveError,
    "The document holds an id the file format cannot represent."
);

create_exception!(
    collomatique,
    CaveatedOverwrite,
    SaveError,
    "save() with no path on a document that was loaded with caveats."
);

/// Declares the fifteen per-family write errors, and the table that finds one
///
/// The names on the left are `collomatique_ops::UpdateError`'s own variants,
/// spelled as serde writes them — the walk reads one off the error and this is
/// what turns it into a class. It is the only hand-written part of the mapping:
/// below the family level nothing here knows the vocabulary, so an op or a case
/// added in `ops/` reaches a script with no change on this side.
macro_rules! family_errors {
    ($($family:ident => $class:ident, $what:literal;)*) => {
        $(
            create_exception!(
                collomatique,
                $class,
                UpdateError,
                concat!("A write to ", $what, " was refused.")
            );
        )*

        /// The class one family's refusals raise, when python knows the family
        ///
        /// `None` for a family `ops/` has grown since this list was written;
        /// the caller answers that with the base [UpdateError] rather than with
        /// a panic (`docs/python/new_api_design.md` §6).
        fn family_class<'py>(py: Python<'py>, family: &str) -> Option<Bound<'py, PyType>> {
            match family {
                $(stringify!($family) => Some(py.get_type::<$class>()),)*
                _ => None,
            }
        }

        /// Adds the family classes to the module
        fn register_families(m: &Bound<'_, PyModule>) -> PyResult<()> {
            $(m.add(stringify!($class), m.py().get_type::<$class>())?;)*
            Ok(())
        }
    };
}

family_errors! {
    GeneralPlanning => GeneralPlanningError, "the periods and the weeks";
    Subjects => SubjectsError, "the subjects";
    Teachers => TeachersError, "the teachers";
    Students => StudentsError, "the students";
    Assignments => AssignmentsError, "the subject assignments";
    WeekPatterns => WeekPatternsError, "the week patterns";
    Slots => SlotsError, "the slots";
    Incompatibilities => IncompatibilitiesError, "the incompatibilities";
    Pairings => PairingsError, "the pairing rules";
    SlotPairings => SlotPairingsError, "the slot pairing rules";
    GroupLists => GroupListsError, "the group lists";
    Settings => SettingsError, "the settings";
    Balancing => BalancingError, "the balancing options";
    Colloscope => ColloscopeError, "the colloscope";
    // Declared although `ExportConfigUpdateError` has no variants at all today:
    // the family is one of the fifteen the write surface mirrors, and an empty
    // enum now is not a promise about later.
    ExportConfig => ExportConfigError, "the export configuration";
}

/// The exception one refused write raises
///
/// The class comes from the family, the attributes from the two levels below
/// it, and the message is the model's own sentence — the same one the generic
/// [UpdateError] used to carry alone.
pub(crate) fn refused(py: Python<'_>, error: &collomatique_ops::UpdateError) -> PyErr {
    let message = error.to_string();

    match crate::payload::to_py(py, error) {
        Ok(data) => from_data(py, message, &data),
        // Nothing in `ops/` can fail to serialize — the error types are derived
        // over numbers and ids — but serde's contract allows it, and the
        // sentence has to reach the script either way.
        Err(_) => UpdateError::new_err(message),
    }
}

/// The exception one walked-over error becomes
///
/// Three levels, and each one may be the last: the walk keeps whatever it
/// reached rather than insisting on the shape the fifteen families have today.
fn from_data(py: Python<'_>, message: String, data: &Bound<'_, PyAny>) -> PyErr {
    let mut class = None;
    let mut op = None;
    let mut case = None;
    let mut details = data.clone();

    if let Some((family, payload)) = crate::payload::peel(data) {
        class = family_class(py, &family);
        details = payload.clone();

        if let Some((name, payload)) = descend(&payload) {
            op = Some(name);
            details = payload.clone();

            if let Some((name, payload)) = descend(&payload) {
                case = Some(name);
                details = payload;
            }
        }
    }

    let class = class.unwrap_or_else(|| py.get_type::<UpdateError>());
    build(class, message, op, case, details)
}

/// The level below a wrapper variant, peeled in its turn
fn descend<'py>(payload: &Bound<'py, PyAny>) -> Option<(String, Bound<'py, PyAny>)> {
    crate::payload::peel(&inside(payload)?)
}

/// What a wrapper variant wraps
///
/// The upper two levels of `UpdateError` each hold one thing — the family holds
/// its family error, which holds its op error — and [crate::payload] writes every
/// payload as a tuple, so descending is taking the only element.
fn inside<'py>(payload: &Bound<'py, PyAny>) -> Option<Bound<'py, PyAny>> {
    let tuple = payload.cast::<PyTuple>().ok()?;
    if tuple.len() != 1 {
        return None;
    }

    tuple.get_item(0).ok()
}

/// The exception object itself, with the three attributes on it
fn build(
    class: Bound<'_, PyType>,
    message: String,
    op: Option<String>,
    case: Option<String>,
    details: Bound<'_, PyAny>,
) -> PyErr {
    let built = (|| -> PyResult<PyErr> {
        let instance = class.call1((message.clone(),))?;
        instance.setattr("op", op)?;
        instance.setattr("case", case)?;
        instance.setattr("details", details)?;
        Ok(PyErr::from_value(instance))
    })();

    // Building an exception out of a class this module declared and three
    // attributes cannot fail; if python says otherwise, the sentence still has
    // to reach the script.
    built.unwrap_or_else(|_| UpdateError::new_err(message))
}

/// Adds the exception classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    m.add("Error", py.get_type::<Error>())?;
    m.add("LoadError", py.get_type::<LoadError>())?;
    m.add("SaveError", py.get_type::<SaveError>())?;
    m.add("NoOrigin", py.get_type::<NoOrigin>())?;
    m.add("UpdateError", py.get_type::<UpdateError>())?;
    m.add("ExportError", py.get_type::<ExportError>())?;
    m.add("ModelBuildError", py.get_type::<ModelBuildError>())?;
    m.add("NothingToUndo", py.get_type::<NothingToUndo>())?;
    m.add("StaleHandleError", py.get_type::<StaleHandleError>())?;
    m.add("NotHosted", py.get_type::<NotHosted>())?;
    m.add("NoDocument", py.get_type::<NoDocument>())?;
    m.add("Cancelled", py.get_type::<Cancelled>())?;
    m.add("DialogUnavailable", py.get_type::<DialogUnavailable>())?;
    m.add("IdCeilingExceeded", py.get_type::<IdCeilingExceeded>())?;
    m.add("CaveatedOverwrite", py.get_type::<CaveatedOverwrite>())?;

    register_families(m)
}
