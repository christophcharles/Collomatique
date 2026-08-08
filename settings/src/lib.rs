//! This installation of Collomatique
//!
//! This crate holds what is true of the installation rather than of any
//! colloscope: which build this is, and the settings that belong to the person
//! running it — where those are stored, and the decisions that depend on them.
//! The two belong together, since the one preference there is so far is about a
//! version. It deliberately depends on no other `collomatique-*` crate and on
//! no toolkit, so that a frontend other than the GTK one asks the same
//! questions and gets the same answers.
//!
//! Everything here is best effort. A preference that cannot be read is treated
//! as never having been set, and a preference that cannot be written is
//! reported on stderr and otherwise dropped: no setting is worth refusing to
//! start over.

use std::path::PathBuf;

pub use semver::Version;

pub mod development_warning;

/// Returns the version number of the compiled Collomatique package
///
/// Every crate in the workspace carries `version.workspace = true`, so this
/// crate's own `CARGO_PKG_VERSION` is the whole program's version. Reading it
/// in a single place is what makes that a fact rather than a coincidence.
pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION should be a valid semantic version")
}

/// The per-user configuration directory for Collomatique
///
/// This is `~/.config/collomatique` on Linux, `~/Library/Application
/// Support/collomatique` on macOS and `%APPDATA%\collomatique\config` on
/// Windows. A sandbox needs no special case: Flatpak and Snap redirect
/// `XDG_CONFIG_HOME`, which is what this reads on Linux.
///
/// Returns [None] when the platform has no home directory to speak of, which
/// is the signal that nothing can be persisted.
fn config_dir() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "collomatique")?;
    Some(dirs.config_dir().to_path_buf())
}
