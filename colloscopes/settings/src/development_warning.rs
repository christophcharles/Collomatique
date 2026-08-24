//! Whether to warn that this build of Collomatique is a development version
//!
//! A development version says so in a box at startup. The person running it
//! can ask not to be warned again, but only for the version in front of them:
//! what is remembered is the set of versions they acknowledged, so the next
//! development version warns afresh, and so does an older one they come back
//! to.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::{Version, config_dir};

/// Tells whether `version` is a development version
///
/// A version is a development version exactly when it carries a semver
/// prerelease tag. Reaching 1.0.0 is *not* the criterion: a plain `0.1.0` or
/// `0.2.0` is a released version and is treated as stable, while
/// `0.1.0-alpha.0.99` and `0.2.0-beta.2` are development versions. Build
/// metadata (`+something`) is not a prerelease and does not count.
pub fn is_development(version: &Version) -> bool {
    !version.pre.is_empty()
}

/// Whether the development warning is due for `version`
///
/// True when `version` is a development version and the user has not already
/// asked not to be warned about it. Any failure to read the acknowledgements
/// answers true: showing the warning is the safe direction.
pub fn is_due(version: &Version) -> bool {
    is_due_in(&load(), version)
}

/// Records that the user asked not to be warned about `version` again
///
/// Best effort: a failure is reported on stderr and otherwise ignored. The
/// stored set is re-read first, so versions acknowledged in earlier runs
/// survive.
pub fn acknowledge(version: &Version) {
    let mut acknowledged = load();
    acknowledged.insert(version.to_string());

    if let Err(e) = store(&acknowledged) {
        eprintln!("Impossible d'enregistrer la préférence d'avertissement : {e}");
    }
}

/// The decision itself, over an already-read set of acknowledged versions
fn is_due_in(acknowledged: &BTreeSet<String>, version: &Version) -> bool {
    is_development(version) && !acknowledged.contains(&version.to_string())
}

/// The file holding the acknowledged versions
///
/// It is its own file rather than a field of a general settings file so that
/// it has no neighbours to be forward-compatible with: the whole content is
/// the list.
fn path() -> Option<PathBuf> {
    Some(config_dir()?.join("development-warning.json"))
}

/// The acknowledged versions, or an empty set if they cannot be read
fn load() -> BTreeSet<String> {
    let Some(path) = path() else {
        return BTreeSet::new();
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return BTreeSet::new();
    };

    parse(&contents)
}

fn store(acknowledged: &BTreeSet<String>) -> Result<(), String> {
    let path = path().ok_or_else(|| String::from("aucun dossier de configuration disponible"))?;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, render(acknowledged)).map_err(|e| e.to_string())
}

/// Reads the file body, treating anything unexpected as an empty set
fn parse(contents: &str) -> BTreeSet<String> {
    serde_json::from_str(contents).unwrap_or_default()
}

/// Writes the file body
fn render(acknowledged: &BTreeSet<String>) -> String {
    // A set serializes as a JSON array, so the file is a bare list of version
    // strings and nothing else.
    serde_json::to_string(acknowledged).expect("a set of strings always serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn acknowledged(versions: &[&str]) -> BTreeSet<String> {
        versions.iter().map(|v| String::from(*v)).collect()
    }

    #[test]
    fn plain_versions_are_not_development() {
        assert!(!is_development(&version("0.1.0")));
        assert!(!is_development(&version("0.2.0")));
        assert!(!is_development(&version("1.0.0")));
        assert!(!is_development(&version("0.3.7")));
    }

    #[test]
    fn prereleases_are_development() {
        assert!(is_development(&version("0.1.0-alpha.0.99")));
        assert!(is_development(&version("0.2.0-beta.2")));
        assert!(is_development(&version("1.0.0-rc.1")));
    }

    #[test]
    fn build_metadata_is_not_a_prerelease() {
        assert!(!is_development(&version("0.1.0+20260806")));
        assert!(is_development(&version("0.1.0-alpha.1+20260806")));
    }

    #[test]
    fn an_unacknowledged_development_version_is_due() {
        assert!(is_due_in(&acknowledged(&[]), &version("0.1.0-alpha.0.99")));
    }

    #[test]
    fn an_acknowledged_development_version_is_not_due() {
        let set = acknowledged(&["0.1.0-alpha.0.99"]);

        assert!(!is_due_in(&set, &version("0.1.0-alpha.0.99")));
    }

    #[test]
    fn acknowledging_one_version_does_not_cover_its_neighbours() {
        let set = acknowledged(&["0.1.0-beta.2"]);

        // The next development version warns afresh...
        assert!(is_due_in(&set, &version("0.2.0-alpha.2")));
        // ...and so does an earlier one the user comes back to.
        assert!(is_due_in(&set, &version("0.1.0-beta.1")));
    }

    #[test]
    fn a_released_version_is_never_due() {
        // Not in the set, and still silent: only a prerelease ever warns.
        assert!(!is_due_in(&acknowledged(&[]), &version("0.1.0")));
        assert!(!is_due_in(
            &acknowledged(&["0.9.9-rc.1"]),
            &version("1.0.0")
        ));
    }

    #[test]
    fn an_unreadable_body_acknowledges_nothing() {
        assert_eq!(parse(""), acknowledged(&[]));
        assert_eq!(parse("not json"), acknowledged(&[]));
        assert_eq!(parse("{\"versions\": []}"), acknowledged(&[]));
    }

    #[test]
    fn the_body_round_trips() {
        let set = acknowledged(&["0.1.0-alpha.0.99", "0.2.0-beta.2"]);

        assert_eq!(parse(&render(&set)), set);
    }

    #[test]
    fn the_body_is_a_bare_list_of_versions() {
        let set = acknowledged(&["0.1.0-alpha.0.99", "0.2.0-beta.2"]);

        assert_eq!(render(&set), "[\"0.1.0-alpha.0.99\",\"0.2.0-beta.2\"]");
    }
}
