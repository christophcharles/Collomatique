//! The files this user opened last
//!
//! The welcome screen offers the last few colloscopes the user worked on, so
//! that coming back to one is a click rather than a trip through a file
//! dialog. What is remembered is a short ordered list, newest first, of the
//! files that were successfully opened or saved.
//!
//! An entry holds *two* paths, because under a sandbox they are not the same
//! one. When Collomatique runs as a Flatpak it never sees the user's
//! filesystem: the file chooser goes through the XDG document portal, which
//! exposes the chosen file at a path of its own,
//! `/run/user/<uid>/doc/<id>/<name>`, and grants access to it. That grant is
//! persistent — it survives the end of the session — so the portal path is
//! exactly what has to be stored to be able to reopen the file later. It is
//! also unspeakable to a human, so the path the user knows is kept beside it.
//! Outside a sandbox the two are simply equal.
//!
//! Entries are never dropped for having become unreachable. A file on an
//! unplugged USB key is not a mistake to be cleaned up; the caller is expected
//! to show such an entry as unavailable and leave it in place. An entry leaves
//! the list only by being pushed off the end of it.
//!
//! Like the rest of this crate, all of this is best effort: a history that
//! cannot be read is treated as empty, and one that cannot be written is
//! reported on stderr and otherwise dropped.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config_dir;

#[cfg(test)]
mod tests;

/// How many files are remembered
pub const HISTORY_LENGTH: usize = 5;

/// One remembered file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The path to open the file with, which under a sandbox is the document
    /// portal's own and not one the user would recognize.
    pub access: PathBuf,
    /// The path as the user knows it, which is what to show them. It is also
    /// what tells two entries apart: the same document reopened through the
    /// portal may well come back under a different `access` path.
    pub display: PathBuf,
}

/// Remembers `access` as the most recently used file
///
/// Best effort: a failure is reported on stderr and otherwise ignored. The
/// stored history is re-read first, so an entry another running instance added
/// meanwhile is not lost.
pub fn record(access: &Path) {
    if let Err(e) = try_record(access) {
        eprintln!("Impossible d'enregistrer les fichiers récents : {e}");
    }
}

/// The remembered files, newest first, or an empty list if they cannot be read
///
/// Whether each file is still reachable is not answered here: that is a
/// question about the world right now, which the caller asks when it draws the
/// list.
pub fn list() -> Vec<Entry> {
    let Some(path) = path() else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    parse(&contents)
}

/// Writes `access` to the front of the stored history
///
/// Several instances of Collomatique can run at once, and each of them writes
/// this file whenever a document is opened or saved, so two of them can meet
/// here. The whole read-modify-write is therefore done while holding an
/// exclusive lock, and the file itself is replaced by an atomic rename so that
/// a reader — which takes no lock at all — sees either the old history or the
/// new one and never half of either.
///
/// The lock is taken on a file of its own rather than on the history. A lock
/// belongs to the inode behind the file, and the rename below replaces that
/// inode, so a second instance locking the history by name would end up
/// holding a lock on a file that no longer exists and would let both writers
/// through.
fn try_record(access: &Path) -> Result<(), String> {
    let path = path().ok_or_else(|| String::from("aucun dossier de configuration disponible"))?;
    let dir = path
        .parent()
        .ok_or_else(|| String::from("le fichier d'historique n'a pas de dossier parent"))?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let lock = std::fs::File::create(dir.join("recent-files.json.lock"))
        .map_err(|e| format!("impossible d'ouvrir le verrou des fichiers récents : {e}"))?;
    fs4::fs_std::FileExt::lock_exclusive(&lock)
        .map_err(|e| format!("impossible de verrouiller les fichiers récents : {e}"))?;

    // Re-read from disk under the lock rather than trusting anything read
    // before it: what another instance wrote while we were waiting is exactly
    // what must not be lost.
    let history = match std::fs::read_to_string(&path) {
        Ok(contents) => parse(&contents),
        Err(_) => Vec::new(),
    };
    let history = merge(history, entry_for(access));

    // A fixed temporary name is safe: only the holder of the lock writes it,
    // and the write truncates whatever an interrupted earlier run left behind.
    let temp = dir.join("recent-files.json.tmp");
    std::fs::write(&temp, render(&history)?).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &path).map_err(|e| e.to_string())?;

    // The lock is released by dropping `lock` here.
    Ok(())
}

/// The file holding the history
///
/// Its own file rather than a field of a general settings file, for the same
/// reason as the development warning: the whole content is the list.
fn path() -> Option<PathBuf> {
    Some(config_dir()?.join("recent-files.json"))
}

/// The entry to remember for a file opened at `access`
fn entry_for(access: &Path) -> Entry {
    Entry {
        access: access.to_path_buf(),
        display: display_path(access),
    }
}

/// The path of `access` as the user knows it
///
/// The document portal records the real path of the file it exposes in an
/// extended attribute of the file it hands over, which is the supported way of
/// asking "what did the user actually pick?". Anything unexpected — no such
/// attribute, because the file was not handed over by a portal, or an empty
/// value — means the path we were given is already the user's own.
#[cfg(unix)]
fn display_path(access: &Path) -> PathBuf {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    match xattr::get(access, "user.document-portal.host-path") {
        Ok(Some(bytes)) if !bytes.is_empty() => PathBuf::from(OsString::from_vec(bytes)),
        _ => access.to_path_buf(),
    }
}

/// Outside unix there is no document portal, so nothing stands between the
/// application and the file the user picked.
#[cfg(not(unix))]
fn display_path(access: &Path) -> PathBuf {
    access.to_path_buf()
}

/// Puts `entry` at the front of `history`
///
/// The same document opened twice must appear once, and the deciding path is
/// the displayed one: a sandbox can hand the same file over under a new
/// `access` path each time, and two entries the user cannot tell apart would
/// be a bug to them whatever the paths say.
fn merge(mut history: Vec<Entry>, entry: Entry) -> Vec<Entry> {
    history.retain(|e| e.display != entry.display);
    history.insert(0, entry);
    history.truncate(HISTORY_LENGTH);
    history
}

/// Reads the file body, treating anything unexpected as an empty history
fn parse(contents: &str) -> Vec<Entry> {
    serde_json::from_str(contents).unwrap_or_default()
}

/// Writes the file body
///
/// Fallible, unlike its neighbours elsewhere in this crate: a path is not
/// always text. Unix lets a file name be any bytes at all, and serde refuses
/// to write one that is not valid UTF-8 rather than mangle it. That file
/// simply does not get remembered, which is worth an unwritten history entry
/// and not a crash.
fn render(history: &[Entry]) -> Result<String, String> {
    // A slice serializes as a JSON array, so the file is a bare list of
    // `{"access": ..., "display": ...}` objects, newest first.
    serde_json::to_string(history).map_err(|e| e.to_string())
}
