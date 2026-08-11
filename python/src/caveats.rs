//! What could not be read from the file a document came from
//!
//! A caveat means the file held something this build does not understand, and
//! that it was dropped. `collomatique_storage::Caveat` says which; this module
//! is its python face — one class per variant, under a common base so a script
//! can write `isinstance(c, clm.Caveat)` without listing them.
//!
//! These are rust classes rather than the `.py` dataclasses of
//! `docs/python/new_api_design.md` §2. That section's reason for dataclasses is
//! that a pyo3 getter clones the struct it hands back, which brings the
//! temporary trap back for nested data — it does not apply to a flat immutable
//! value that only ever travels *out* of rust.

use pyo3::prelude::*;

use collomatique_settings::Version;

/// Something in the file could not be read
///
/// The base class of every caveat, so `isinstance(caveat, Caveat)` catches all
/// of them. It has no constructor of its own: every caveat is one of the
/// subclasses, and `collomatique.Caveat()` raises `TypeError`.
#[pyclass(module = "collomatique", subclass, frozen)]
pub struct Caveat;

/// The file was written by a newer Collomatique than this one
///
/// On its own this is not a loss — a newer version may well write a file this
/// one reads whole. It is a warning that the file may hold more than was read,
/// and it usually travels with an [UnknownEntry] when it does.
#[pyclass(module = "collomatique", extends = Caveat, frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct CreatedWithNewerVersion {
    /// The version stays a `semver::Version` inside so that rendering through
    /// the storage `Display` cannot fail; python sees the string, as it does
    /// for `collomatique.__version__`.
    version: Version,
}

/// A block of the file was skipped, and is lost if the file is written back
///
/// The block asks for a file-format spec version this build does not support
/// and does not declare itself needed, so the rest of the file was decoded
/// without it (the forward-compatibility rules of
/// `docs/file_format/file_format.md` §5).
#[pyclass(module = "collomatique", extends = Caveat, frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct UnknownEntry {
    #[pyo3(get)]
    block_name: String,
    #[pyo3(get)]
    minimum_spec_version: u32,
}

#[pymethods]
impl CreatedWithNewerVersion {
    /// The subclasses are constructible so that a script can name the caveat it
    /// expects — `clm.CreatedWithNewerVersion("9.0.0") in doc.caveats` — rather
    /// than picking it apart field by field.
    #[new]
    fn new(version: &str) -> PyResult<PyClassInitializer<Self>> {
        let version = Version::parse(version).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("{version:?} is not a version: {e}"))
        })?;
        Ok(CreatedWithNewerVersion { version }.init())
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("version",);

    /// The version that wrote the file, as a string
    #[getter]
    fn version(&self) -> String {
        self.version.to_string()
    }

    fn __repr__(&self) -> String {
        format!("CreatedWithNewerVersion(version={:?})", self.version())
    }

    /// The french sentence the application writes for this caveat
    fn __str__(&self) -> String {
        collomatique_ui_text::caveats::caveat_text(&self.to_storage())
    }
}

impl CreatedWithNewerVersion {
    /// Pairs the value with its base class, which is how a subclass instance
    /// is built — the tuple form is deprecated in pyo3.
    fn init(self) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Caveat).add_subclass(self)
    }

    fn to_storage(&self) -> collomatique_storage::Caveat {
        collomatique_storage::Caveat::CreatedWithNewerVersion {
            version: self.version.clone(),
        }
    }
}

#[pymethods]
impl UnknownEntry {
    #[new]
    fn new(block_name: String, minimum_spec_version: u32) -> PyClassInitializer<Self> {
        UnknownEntry {
            block_name,
            minimum_spec_version,
        }
        .init()
    }

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("block_name", "minimum_spec_version");

    fn __repr__(&self) -> String {
        format!(
            "UnknownEntry(block_name={:?}, minimum_spec_version={})",
            self.block_name, self.minimum_spec_version
        )
    }

    /// The french sentence the application writes for this caveat
    fn __str__(&self) -> String {
        collomatique_ui_text::caveats::caveat_text(&self.to_storage())
    }
}

impl UnknownEntry {
    fn init(self) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Caveat).add_subclass(self)
    }

    fn to_storage(&self) -> collomatique_storage::Caveat {
        collomatique_storage::Caveat::UnknownEntry {
            block_name: self.block_name.clone(),
            minimum_spec_version: self.minimum_spec_version,
        }
    }
}

/// Builds the python caveat for one storage caveat
///
/// Written as a match on the storage enum rather than as a `From` impl per
/// variant, so that a new variant over there is a compile error here.
pub fn to_python(py: Python<'_>, caveat: &collomatique_storage::Caveat) -> PyResult<Py<PyAny>> {
    use collomatique_storage::Caveat as Storage;
    Ok(match caveat {
        Storage::CreatedWithNewerVersion { version } => Py::new(
            py,
            CreatedWithNewerVersion {
                version: version.clone(),
            }
            .init(),
        )?
        .into_any(),
        Storage::UnknownEntry {
            block_name,
            minimum_spec_version,
        } => Py::new(
            py,
            UnknownEntry {
                block_name: block_name.clone(),
                minimum_spec_version: *minimum_spec_version,
            }
            .init(),
        )?
        .into_any(),
    })
}

/// Adds the caveat classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Caveat>()?;
    m.add_class::<CreatedWithNewerVersion>()?;
    m.add_class::<UnknownEntry>()?;
    Ok(())
}
