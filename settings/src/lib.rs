//! Per-user preferences for Collomatique
//!
//! This crate holds the settings that belong to the person running
//! Collomatique rather than to any colloscope: where they are stored, and the
//! decisions that depend on them. It deliberately depends on no other
//! `collomatique-*` crate and on no toolkit, so that a frontend other than the
//! GTK one asks the same questions and gets the same answers.
//!
//! Everything here is best effort. A preference that cannot be read is treated
//! as never having been set, and a preference that cannot be written is
//! reported on stderr and otherwise dropped: no setting is worth refusing to
//! start over.

use std::path::PathBuf;

pub use semver::Version;

pub mod development_warning;

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
