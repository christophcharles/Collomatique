//! Which version of Collomatique this is

use collomatique_storage::{Version, current_version};

/// Tells whether `version` is a development version
///
/// A version is a development version exactly when it carries a semver
/// prerelease tag. Reaching 1.0.0 is *not* the criterion: a plain `0.1.0` or
/// `0.2.0` is a released version and is treated as stable, while
/// `0.1.0-alpha.0.99` and `0.2.0-beta.2` are development versions. Build
/// metadata (`+something`) is not a prerelease and does not count.
fn is_development(version: &Version) -> bool {
    !version.pre.is_empty()
}

/// The version of the running Collomatique, when it is a development version
///
/// Returns [None] for a released version, which is the signal that no
/// development warning is due.
pub fn development_build() -> Option<Version> {
    let version = current_version();
    is_development(&version).then_some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
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
}
