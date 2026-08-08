//! Opening a colloscope file, and writing it back

use std::path::{Path, PathBuf};

use pyo3::prelude::*;

use collomatique_ops::Desc;
use collomatique_state::AppState;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::Data;

use crate::errors::{IdCeilingExceeded, LoadError, NoOrigin, SaveError};

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
    let (inner_data, _caveats) =
        collomatique_storage::deserialize_data(&content).map_err(|e| fail(&e))?;
    let data = Data::from_inner_data(inner_data).map_err(|e| fail(&e))?;

    Ok(Document {
        state: AppState::new(data),
        source_path: Some(path),
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

    /// Writes the document out
    ///
    /// With a path, writes that file. Without one, writes back to the origin,
    /// and raises `NoOrigin` when there is none: it is never a silent no-op.
    /// The origin does not move — `save(other)` does not re-target a later
    /// `save()`.
    #[pyo3(signature = (path=None))]
    fn save(&self, path: Option<PathBuf>) -> PyResult<()> {
        let target = match path {
            Some(path) => path,
            None => self.source_path.clone().ok_or_else(|| {
                NoOrigin::new_err("this document has no origin: pass a path to save()")
            })?,
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
