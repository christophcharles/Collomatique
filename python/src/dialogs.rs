//! The file choosers a script can put on the screen
//!
//! `docs/python/new_api_design.md` §9.3 is the design. The short version is that
//! file selection is the one dialog every script needs — an import script that
//! cannot ask for its csv is a script with a path written into it — and that
//! files and folders are *all* this buys. Message boxes are `zenity` under the
//! portal backend, and a run in a sandbox has no reason to hold an external
//! binary; they stay `tkinter`'s job, as text entry and list choice already do.
//!
//! `rfd` is the implementation, and it sits behind a trait for the reason
//! `host.rs` has one: nothing in a test suite can click a real button.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::errors::DialogUnavailable;

/// What a file dialog was asked for
///
/// One struct for all three dialogs: a folder picker ignores the filters and
/// the file name, and a shape per dialog would buy nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRequest {
    /// The window title, or the desktop's own default when there is none
    pub title: Option<String>,
    /// The filters to offer, as `(description, extensions)`, already bare
    pub filters: Vec<(String, Vec<String>)>,
    /// Where the chooser opens
    pub directory: Option<PathBuf>,
    /// The name to start from, which only a save dialog does anything with
    pub file_name: Option<String>,
}

impl FileRequest {
    /// Builds a request out of what a python caller passed
    ///
    /// The one place the arguments are checked, so the three functions below
    /// cannot disagree about what a filter is.
    fn new(
        title: Option<String>,
        filters: Option<Vec<(String, Vec<String>)>>,
        directory: Option<PathBuf>,
        file_name: Option<String>,
    ) -> PyResult<FileRequest> {
        let filters = filters
            .unwrap_or_default()
            .into_iter()
            .map(|(description, extensions)| {
                let extensions: PyResult<Vec<_>> =
                    extensions.iter().map(|e| extension(e)).collect();
                Ok((description, extensions?))
            })
            .collect::<PyResult<Vec<_>>>()?;

        Ok(FileRequest {
            title,
            filters,
            directory,
            file_name,
        })
    }
}

/// The extension `rfd` wants, from the one a script naturally writes
///
/// `rfd` filters on a bare extension, while `"*.csv"` — the shape the dialog
/// itself displays — is what gets written first, and `".csv"` second. All three
/// mean `"csv"` here. A bare `"*"`, the all-files filter, is left alone.
fn extension(written: &str) -> PyResult<String> {
    let bare = written
        .strip_prefix("*.")
        .or_else(|| written.strip_prefix('.'))
        .unwrap_or(written);

    if bare.is_empty() {
        return Err(PyValueError::new_err(format!(
            "{written:?} is not an extension a filter can be built from"
        )));
    }

    Ok(bare.to_owned())
}

/// Whatever actually puts a file chooser on the screen
///
/// `rfd` is the implementation; the trait exists so the tests can answer a
/// dialog without one. The `Err` is a sentence for the script to read, as
/// [Host::send](crate::Host::send)'s is — it becomes a `DialogUnavailable`,
/// because everything that can go wrong here is the machine being unable to
/// show a dialog at all.
pub trait Dialogs: Send + Sync {
    /// Asks for an existing file
    fn open_file(&self, request: &FileRequest) -> Result<Option<PathBuf>, String>;

    /// Asks where to write, which may be a file that is not there yet
    fn save_file(&self, request: &FileRequest) -> Result<Option<PathBuf>, String>;

    /// Asks for a directory
    fn pick_folder(&self, request: &FileRequest) -> Result<Option<PathBuf>, String>;
}

/// What the current run answers dialogs with, or `None` for the real thing
static DIALOGS: Mutex<Option<Arc<dyn Dialogs>>> = Mutex::new(None);

/// Answers the dialogs with something other than `rfd`, for the tests
///
/// The runner never calls this: a real run wants a real chooser, which is what
/// `None` — the state the module starts in — means.
pub fn set_dialogs(dialogs: Option<Arc<dyn Dialogs>>) {
    *DIALOGS.lock().unwrap() = dialogs;
}

/// The backend the next dialog goes to
fn backend() -> Arc<dyn Dialogs> {
    DIALOGS
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| Arc::new(RfdDialogs))
}

/// The real chooser, drawn by the desktop through `rfd`
struct RfdDialogs;

impl Dialogs for RfdDialogs {
    fn open_file(&self, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        show(|| build(request).pick_file())
    }

    fn save_file(&self, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        show(|| build(request).save_file())
    }

    fn pick_folder(&self, request: &FileRequest) -> Result<Option<PathBuf>, String> {
        show(|| build(request).pick_folder())
    }
}

/// The `rfd` dialog a request describes
///
/// An argument that was not given is not passed on: no title means no
/// `set_title` call, i.e. whatever the desktop would have written there.
fn build(request: &FileRequest) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new();

    if let Some(title) = &request.title {
        dialog = dialog.set_title(title.as_str());
    }

    for (description, extensions) in &request.filters {
        dialog = dialog.add_filter(description.as_str(), extensions);
    }

    if let Some(directory) = &request.directory {
        dialog = dialog.set_directory(directory);
    }

    if let Some(file_name) = &request.file_name {
        dialog = dialog.set_file_name(file_name.as_str());
    }

    dialog
}

/// Whether this machine can show a dialog at all
///
/// A library that can open windows can hang a cron job forever, and `rfd`
/// cannot be asked: a portal that is not there comes back as `None`, which is
/// exactly what a user pressing Cancel looks like. So the question is put
/// before the dialog instead, to the environment.
///
/// Permissive on purpose — any one of the three is enough. A desktop session
/// that autolaunches its bus sets no `DBUS_SESSION_BUS_ADDRESS`, and refusing
/// it would be worse than the cron job being guarded against, which has none of
/// the three.
#[cfg(all(unix, not(target_os = "macos")))]
fn session_available() -> bool {
    ["WAYLAND_DISPLAY", "DISPLAY", "DBUS_SESSION_BUS_ADDRESS"]
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
}

/// The runtime every dialog in this process runs on
///
/// One for the process, and not one per dialog, because the bus outlives the
/// dialog: `ashpd` keeps the session connection it opens in a global
/// `OnceLock`, and that connection's socket task was spawned onto whatever
/// runtime was in context when it was made. Drop that runtime at the end of the
/// first dialog and the connection is still cached but no longer driven, so the
/// second dialog waits on it forever.
///
/// The failure is remembered along with the runtime: a runtime that could not be
/// built will not build a moment later either, and the sentence is the same one.
#[cfg(all(unix, not(target_os = "macos")))]
fn runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    static RUNTIME: std::sync::OnceLock<Result<tokio::runtime::Runtime, String>> =
        std::sync::OnceLock::new();

    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .map_err(|e| format!("the dialog has no runtime to reach the desktop over: {e}"))
        })
        .as_ref()
        .map_err(|e| e.clone())
}

/// Shows a dialog, with whatever the platform needs standing around it
///
/// `rfd`'s synchronous calls block on `zbus` under the portal, and `zbus`
/// spawns onto the runtime in context — a plain python interpreter has none, so
/// the dialog brings its own. Multi-thread and not current-thread: the blocking
/// happens on *this* thread, so the tasks it spawns need a worker of their own
/// or nothing ever drives them.
#[cfg(all(unix, not(target_os = "macos")))]
fn show<T>(dialog: impl FnOnce() -> T) -> Result<T, String> {
    if !session_available() {
        return Err(
            "there is no desktop session here to show a file chooser in: none of \
                    WAYLAND_DISPLAY, DISPLAY or DBUS_SESSION_BUS_ADDRESS is set"
                .to_owned(),
        );
    }

    let _entered = runtime()?.enter();

    Ok(dialog())
}

/// Everywhere else the platform draws the dialog itself, with nothing to set up
#[cfg(not(all(unix, not(target_os = "macos"))))]
fn show<T>(dialog: impl FnOnce() -> T) -> Result<T, String> {
    Ok(dialog())
}

/// Asks the user for a file to read, and hands back its path
///
/// Returns `None` when the dialog was cancelled — the ordinary way out, and not
/// an error. Raises `DialogUnavailable` on a machine that cannot show a dialog,
/// rather than waiting for a click that will never come.
///
/// Filters are `(description, extensions)` pairs, and an extension may be
/// written any of the three natural ways:
///
/// ```python
/// clm.dialogs.open_file(
///     title="Ouvrir la liste des élèves",
///     filters=[("Tableur", ["csv", "*.xlsx"]), ("Tous les fichiers", ["*"])],
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (*, title=None, filters=None, directory=None))]
pub fn open_file(
    py: Python<'_>,
    title: Option<String>,
    filters: Option<Vec<(String, Vec<String>)>>,
    directory: Option<PathBuf>,
) -> PyResult<Option<PathBuf>> {
    let request = FileRequest::new(title, filters, directory, None)?;
    let dialogs = backend();

    // The GIL goes back while the dialog is up: it is open for as long as the
    // user takes to answer, and holding it would freeze every other thread the
    // script started.
    py.detach(|| dialogs.open_file(&request))
        .map_err(DialogUnavailable::new_err)
}

/// Asks the user where to write, and hands back the path
///
/// The file need not exist. `file_name` is the name the dialog starts with, and
/// the user is free to replace it. Returns `None` on cancel.
#[pyfunction]
#[pyo3(signature = (*, title=None, filters=None, directory=None, file_name=None))]
pub fn save_file(
    py: Python<'_>,
    title: Option<String>,
    filters: Option<Vec<(String, Vec<String>)>>,
    directory: Option<PathBuf>,
    file_name: Option<String>,
) -> PyResult<Option<PathBuf>> {
    let request = FileRequest::new(title, filters, directory, file_name)?;
    let dialogs = backend();

    py.detach(|| dialogs.save_file(&request))
        .map_err(DialogUnavailable::new_err)
}

/// Asks the user for a directory, and hands back its path
///
/// Returns `None` on cancel. There is nothing to filter a folder by, so this
/// one takes no filters.
#[pyfunction]
#[pyo3(signature = (*, title=None, directory=None))]
pub fn pick_folder(
    py: Python<'_>,
    title: Option<String>,
    directory: Option<PathBuf>,
) -> PyResult<Option<PathBuf>> {
    let request = FileRequest::new(title, None, directory, None)?;
    let dialogs = backend();

    py.detach(|| dialogs.pick_folder(&request))
        .map_err(DialogUnavailable::new_err)
}

/// Adds the `collomatique.dialogs` submodule to the module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();

    // Named in full, so that `repr` on one of the functions says
    // `collomatique.dialogs.open_file` rather than `dialogs.open_file`.
    let m = PyModule::new(py, "collomatique.dialogs")?;
    m.add_function(wrap_pyfunction!(open_file, &m)?)?;
    m.add_function(wrap_pyfunction!(save_file, &m)?)?;
    m.add_function(wrap_pyfunction!(pick_folder, &m)?)?;

    // A submodule hung off its parent is not one python can `import`, and
    // `from collomatique.dialogs import open_file` is a reasonable thing for a
    // script to write; `sys.modules` is what makes it work.
    py.import("sys")?
        .getattr("modules")?
        .set_item("collomatique.dialogs", &m)?;

    parent.add("dialogs", m)
}
