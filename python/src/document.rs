//! Opening a colloscope file, and writing it back

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use pyo3::types::PyFrozenSet;

use collomatique_ops::{Desc, UpdateOp};
use collomatique_state::AppState;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::Data;
use collomatique_storage::Caveat;

use crate::collections::Periods;
use crate::errors::{
    CaveatedOverwrite, Error, IdCeilingExceeded, LoadError, NoOrigin, NothingToUndo, SaveError,
    UpdateError,
};
use crate::results::{OpResult, Warning};

/// Where a document came from, and where a bare `save()` writes
///
/// Fixed at creation and never changed: `save(path)` writes where it is told
/// and does not re-target a later `save()`. Pairing the path with its caveats
/// — mirroring the GUI's `FileName` — makes "caveats with no path"
/// unrepresentable, and gives the hosted document a slot of its own.
/// [Origin::Hosted] carries no caveats, because the handoff carries the `Data`
/// and not the host's caveat set (`docs/python/new_api_design.md` §9.2).
enum Origin {
    /// Never on disk and not hosted: `save()` has nowhere to write
    None,
    File {
        path: PathBuf,
        caveats: BTreeSet<Caveat>,
    },
    /// The document the application handed over
    ///
    /// Nothing builds one yet: `current_document()` is the only constructor
    /// and it lands with hosted mode. The variant is here now so the enum has
    /// its final shape and the arms that will need it are already written.
    #[allow(dead_code)]
    Hosted,
}

impl Origin {
    /// The file this came from, for [Document::source_path] and for `save()`
    ///
    /// `None` for [Origin::Hosted] too: a hosted document was never on disk,
    /// and `save()` sends it back to the application rather than writing.
    fn path(&self) -> Option<&Path> {
        match self {
            Origin::File { path, .. } => Some(path),
            Origin::None | Origin::Hosted => None,
        }
    }

    /// What could not be read, empty for everything but a file
    fn caveats(&self) -> &BTreeSet<Caveat> {
        static EMPTY: BTreeSet<Caveat> = BTreeSet::new();

        match self {
            Origin::File { caveats, .. } => caveats,
            Origin::None | Origin::Hosted => &EMPTY,
        }
    }
}

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
    origin: Origin,
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
        origin: Origin::File { path, caveats },
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
        origin: Origin::None,
    }
}

impl Document {
    /// The document as it is now, for the collections to read through
    pub(crate) fn data(&self) -> &Data {
        self.state.get_data()
    }

    /// Applies one composite op, and keeps the repairs it had to make
    ///
    /// The single door every mutator goes through. It is `dry_apply` rather
    /// than `apply`: `apply` exists for callers with no way of showing the
    /// cascade's repairs, and this api has one — every mutator hands them back
    /// on its [OpResult].
    pub(crate) fn update(&mut self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let result = op
            .dry_apply(&self.state)
            .map_err(|e| UpdateError::new_err(e.to_string()))?;

        // Rendered here, while `self.state` is still the state the op was
        // applied to: a warning names material this update may be about to
        // remove, so the pre-state is the only one it can be read against
        // (`ops/src/cascade.rs`).
        let warnings = result
            .warnings
            .iter()
            .map(|warning| {
                let text = warning.text(self.state.get_data()).map_err(|e| {
                    Error::new_err(format!(
                        "a repair named something the document does not hold ({e}); \
                         this is a bug in collomatique, not something the script did"
                    ))
                })?;
                Py::new(py, Warning::new(text))
            })
            .collect::<PyResult<Vec<_>>>()?;

        self.state = result.new_state;

        Ok(OpResult::new(warnings))
    }
}

#[pymethods]
impl Document {
    /// The periods of the document, and the date the colles start
    ///
    /// A view on the document, not a copy: `doc.periods` twice gives two
    /// objects that read and write the same document.
    #[getter]
    fn periods(slf: Py<Self>) -> Periods {
        Periods::new(slf)
    }

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
        self.origin.path()
    }

    /// What could not be read from the file, as a `frozenset` of [Caveat]s
    ///
    /// Empty for a clean file and for [new_document]. It is part of the
    /// origin, so it is fixed at creation like the path is: writing a copy of
    /// the document does not make the file it came from any more readable, so
    /// nothing clears it. (The GUI *does* clear its `FileName::CaveatFile`
    /// after a save, but that field is a save *target*, which is a different
    /// thing from an origin.)
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
            .origin
            .caveats()
            .iter()
            .map(|caveat| crate::caveats::to_python(py, caveat))
            .collect::<PyResult<Vec<_>>>()?;
        PyFrozenSet::new(py, &caveats)
    }

    /// Takes back the last write
    ///
    /// Raises `NothingToUndo` when there is nothing left to take back, rather
    /// than doing nothing quietly: a script that undoes more than it wrote is
    /// mistaken about its own document, and silence would hide that.
    ///
    /// The history belongs to the document and never leaves the script. In
    /// hosted mode the script works on a copy, so an undo is invisible to the
    /// application: only `send_to_host` crosses back, and what it carries is
    /// the document as it stands, not the way it got there.
    fn undo(&mut self) -> PyResult<()> {
        self.state
            .undo()
            .map_err(|_| NothingToUndo::new_err("this document has nothing left to undo"))?;
        Ok(())
    }

    /// Puts back the last write [Document::undo] took away
    ///
    /// Raises `NothingToUndo` when there is nothing to put back. A new write
    /// empties the redo stack, as it does everywhere else: the history is a
    /// line, not a tree.
    fn redo(&mut self) -> PyResult<()> {
        self.state
            .redo()
            .map_err(|_| NothingToUndo::new_err("this document has nothing left to redo"))?;
        Ok(())
    }

    /// Whether [Document::undo] would do something rather than raise
    #[getter]
    fn can_undo(&self) -> bool {
        self.state.can_undo()
    }

    /// Whether [Document::redo] would do something rather than raise
    #[getter]
    fn can_redo(&self) -> bool {
        self.state.can_redo()
    }

    /// What [Document::undo] would take back, in french, or `None`
    ///
    /// The same short label the application shows in its Undo menu — « Changer
    /// le début des colles » and the like. It is a label for a human to read,
    /// so a script should not branch on it; `can_undo` is the question with a
    /// stable answer.
    ///
    /// The category the operation also carries is not exposed: it tells the
    /// GUI which screen to open, which is nothing a script can use.
    #[getter]
    fn undo_name(&self) -> Option<String> {
        self.state.get_undo_name().map(|(_, name)| name.clone())
    }

    /// What [Document::redo] would put back, in french, or `None`
    #[getter]
    fn redo_name(&self) -> Option<String> {
        self.state.get_redo_name().map(|(_, name)| name.clone())
    }

    /// A copy of the document with dense ids and no undo history
    ///
    /// The document itself is untouched: the copy is a new document, so
    /// nothing a script holds is invalidated, and "this clears the undo
    /// history" is not a warning — a new document simply has none.
    ///
    /// This is the way out of `IdCeilingExceeded`. The file format has a
    /// ceiling on the ids it can write and `save` never renumbers on its own,
    /// so the rescue is explicit:
    ///
    /// ```python
    /// clm.load("big_ids.collomatique").compacted().save()
    /// ```
    ///
    /// The copy inherits the file it came from, and with it the caveats — the
    /// script above overwrites the file it read, and a caveated file still
    /// refuses the bare `save()`, so compaction is not a laundering route
    /// around that guard.
    fn compacted(&self) -> PyResult<Document> {
        let inner_data = self.state.get_data().get_inner_data().clone().compact_ids();

        // Renumbering is injective and monotone, so it repairs nothing and
        // breaks nothing: a document that satisfied the invariants still does
        // (`state-colloscopes/src/compact.rs`, pinned by
        // `state-colloscopes/tests/compact_ids.rs`). The arm is still written
        // out rather than unwrapped, because §6 says a script never gets a
        // panic.
        let data = Data::from_inner_data(inner_data).map_err(|e| {
            Error::new_err(format!(
                "compacting the document produced an invalid one: {e}"
            ))
        })?;

        Ok(Document {
            state: AppState::new(data),
            origin: match &self.origin {
                Origin::None => Origin::None,
                Origin::File { path, caveats } => Origin::File {
                    path: path.clone(),
                    caveats: caveats.clone(),
                },
                // Compaction cannot travel to the host: an `Op::GlobalUpdate`
                // only pushes the id issuer forward, so the application would
                // end up with dense ids, an issuer still at its old high-water
                // mark, and a Ctrl-Z bringing the big ids back. So a compacted
                // copy of the hosted document is an ordinary origin-less one.
                Origin::Hosted => Origin::None,
            },
        })
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
                // `Origin::Hosted` answers `None` here and so shares the
                // `NoOrigin` arm. Sending a hosted document back to the
                // application is `docs/python/new_api_design.md` §9.2's job
                // and lands with the rest of hosted mode.
                let origin = self.origin.path().ok_or_else(|| {
                    NoOrigin::new_err("this document has no origin: pass a path to save()")
                })?;
                let caveats = self.origin.caveats();
                if !ignore_caveats && !caveats.is_empty() {
                    return Err(CaveatedOverwrite::new_err(format!(
                        "{}: this file was loaded with caveats, so part of it could not be \
                         read and writing back would drop it ({}); pass a path to write \
                         elsewhere, or ignore_caveats=True to overwrite it anyway",
                        origin.display(),
                        caveats
                            .iter()
                            .map(|caveat| caveat.to_string())
                            .collect::<Vec<_>>()
                            .join("; "),
                    )));
                }
                origin.to_path_buf()
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
