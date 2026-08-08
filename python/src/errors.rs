//! The exceptions the `collomatique` module raises
//!
//! Every one of them descends from [Error], so a script that only wants "the
//! collomatique call failed" has one thing to catch. The tree below is the
//! seed of the one described in `docs/python/new_api_design.md` §6: the design
//! names `Error`, `NoOrigin` and `IdCeilingExceeded`, and leaves the ordinary
//! ways reading and writing a file fail unnamed — hence [LoadError] and
//! [SaveError].
//!
//! §6's per-family write errors (`SubjectsError`, `TeachersError`, …) will
//! subclass [UpdateError] when the write surface lands, so a script that
//! catches the general one keeps catching all of them.

use pyo3::create_exception;
use pyo3::exceptions::PyException;

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
