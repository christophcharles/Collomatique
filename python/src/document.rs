//! Opening a colloscope file, and writing it back

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use pyo3::types::PyFrozenSet;

use collomatique_ops::Desc;
use collomatique_state::AppState;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::Data;
use collomatique_storage::Caveat;

use crate::errors::{CaveatedOverwrite, IdCeilingExceeded, LoadError, NoOrigin, SaveError};

/// An open colloscope document
///
/// The document owns its own state, so two documents in one script share
/// nothing and the module holds no global state. Its origin is fixed at
/// creation and never changes: `save(path)` writes where it is told and does
/// not re-target a later `save()`.
///
/// There is no constructor: [load] and [new_document] are the only ways to get
/// one, so `collomatique.Document()` raises `TypeError`.
#[pyclass(module = "collomatique")]
pub struct Document {
    state: AppState<Data, Desc>,
    source_path: Option<PathBuf>,
    /// What could not be read from the file this document came from
    ///
    /// Part of the origin, so like [Document::source_path] it is fixed at
    /// creation. An `Origin` enum pairing the two — mirroring the GUI's
    /// `FileName` — would make "caveats with no path" unrepresentable, but
    /// `docs/python/new_api_design.md` §9.2's hosted origin will reshape this
    /// field anyway, so the enum belongs there rather than being guessed at
    /// now.
    caveats: BTreeSet<Caveat>,
}

/// Opens a colloscope file
///
/// The argument is anything python calls a path: a `str`, a `pathlib.Path`, or
/// any other `os.PathLike`.
#[pyfunction]
pub fn load(path: PathBuf) -> PyResult<Document> {
    let fail = |e: &dyn std::fmt::Display| LoadError::new_err(format!("{}: {e}", path.display()));

    let content = std::fs::read_to_string(&path).map_err(|e| fail(&e))?;

    // `deserialize_data` hands back an `InnerData`; its docs are explicit that
    // the caller owns the invariant gate, as `gtk4`'s file loader does too.
    let (inner_data, caveats) =
        collomatique_storage::deserialize_data(&content).map_err(|e| fail(&e))?;
    let data = Data::from_inner_data(inner_data).map_err(|e| fail(&e))?;

    // Caveats are handed back, not announced: nothing is printed and no
    // `warnings.warn` is raised. The GUI shows a modal because a human is
    // sitting in front of it; a script has nobody, and a library writing to
    // stderr on its own is a nuisance in a cron job. What was skipped was, by
    // construction, something this build cannot use anyway — the loss only
    // happens when the file is written back.
    Ok(Document {
        state: AppState::new(data),
        source_path: Some(path),
        caveats,
    })
}

/// An empty document, with no origin
///
/// This is the state of a brand new file. It has nowhere to write, so `save()`
/// with no argument raises `NoOrigin` until it is given a path.
#[pyfunction]
pub fn new_document() -> Document {
    Document {
        state: AppState::new(Data::new()),
        source_path: None,
        caveats: BTreeSet::new(),
    }
}

#[pymethods]
impl Document {
    /// Where this document came from, as a `pathlib.Path`, or `None`
    ///
    /// `None` means the document was never on disk — it came from
    /// [new_document]. Saving it once does not give it an origin: the origin
    /// is where the document *came from*, not where it was last written.
    /// pyo3 converts a rust path to a `pathlib.Path`, which is the type
    /// `docs/python/new_api_design.md` §1 promises, so nothing is built by
    /// hand here — but the script asserts the `isinstance`, because it is the
    /// promise and not an implementation detail we happen to inherit.
    #[getter]
    fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// What could not be read from the file, as a `frozenset` of [Caveat]s
    ///
    /// Empty for a clean file and for [new_document]. Like the origin it is
    /// fixed at creation: writing a copy of the document does not make the
    /// file it came from any more readable, so nothing clears it. (The GUI
    /// *does* clear its `FileName::CaveatFile` after a save, but that field is
    /// a save *target*, which is a different thing from an origin.)
    ///
    /// Each element says what was dropped; `str()` on one is an english
    /// sentence, and the classes are in the module, so a script can test for
    /// the caveat it knows how to handle:
    ///
    /// ```python
    /// if clm.UnknownEntry("colloscope", 3) in doc.caveats:
    ///     ...
    /// ```
    #[getter]
    fn caveats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let caveats = self
            .caveats
            .iter()
            .map(|caveat| crate::caveats::to_python(py, caveat))
            .collect::<PyResult<Vec<_>>>()?;
        PyFrozenSet::new(py, &caveats)
    }

    /// Writes the document out
    ///
    /// With a path, writes that file. Without one, writes back to the origin,
    /// and raises `NoOrigin` when there is none: it is never a silent no-op.
    /// The origin does not move — `save(other)` does not re-target a later
    /// `save()`.
    ///
    /// Writing back over a file that was loaded with caveats raises
    /// `CaveatedOverwrite` instead, because whatever could not be read is
    /// dropped by the rewrite. The four forms are:
    ///
    /// ```python
    /// doc.save()                      # raises, when doc.caveats is non-empty
    /// doc.save("copy.collomatique")   # writes; the suspect original survives
    /// doc.save(doc.source_path)       # writes; the script named the target
    /// doc.save(ignore_caveats=True)   # writes; deliberate
    /// ```
    ///
    /// The rule keys on *no path argument*, not on whether the path equals the
    /// origin. Path equality is fragile — symlinks, relative paths, hard links
    /// — and the GUI does not test it either: its "Enregistrer" on a caveated
    /// file opens a Save-As dialog defaulting to that same file, and a user who
    /// picks it overwrites it, because they chose. Naming the path from python
    /// is that same choice. `save()` with no argument is the one form that
    /// writes somewhere the script never named, so it is the one that is loud.
    ///
    /// `ignore_caveats` does nothing when a path is given, since that form
    /// never raises; it is accepted there so a script can pass it uniformly.
    #[pyo3(signature = (path=None, *, ignore_caveats=false))]
    fn save(&self, path: Option<PathBuf>, ignore_caveats: bool) -> PyResult<()> {
        let target = match path {
            Some(path) => path,
            None => {
                let origin = self.source_path.clone().ok_or_else(|| {
                    NoOrigin::new_err("this document has no origin: pass a path to save()")
                })?;
                if !ignore_caveats && !self.caveats.is_empty() {
                    return Err(CaveatedOverwrite::new_err(format!(
                        "{}: this file was loaded with caveats, so part of it could not be \
                         read and writing back would drop it ({}); pass a path to write \
                         elsewhere, or ignore_caveats=True to overwrite it anyway",
                        origin.display(),
                        self.caveats
                            .iter()
                            .map(|caveat| caveat.to_string())
                            .collect::<Vec<_>>()
                            .join("; "),
                    )));
                }
                origin
            }
        };

        let content = collomatique_storage::serialize_data(self.state.get_data().get_inner_data())
            .map_err(|collomatique_storage::EncodeError::IdAboveCeiling { id }| {
                IdCeilingExceeded::new_err(format!(
                    "id {id} exceeds the file-format ceiling; the document must be \
                     compacted before it can be written"
                ))
            })?;

        std::fs::write(&target, content)
            .map_err(|e| SaveError::new_err(format!("{}: {e}", target.display())))
    }
}
