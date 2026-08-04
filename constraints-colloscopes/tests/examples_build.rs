//! Smoke test: the constraint model must build for every file under the
//! repo-root `examples/` directory.
//!
//! `build_model` panics on internal inconsistency, so calling it without a
//! panic is the assertion; no solver or solution is needed. The directory is
//! walked at runtime, so a new `.collomatique` example is covered automatically.

use collomatique_constraints_colloscopes::build_model;
use collomatique_storage::deserialize_data;
use std::path::PathBuf;

/// The repo-root `examples/` directory, relative to this crate's manifest.
fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

/// Sorted list of every `*.collomatique` file in `examples/`.
fn example_files() -> Vec<PathBuf> {
    let dir = examples_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read examples dir {}: {e}", dir.display()))
        .map(|entry| entry.expect("cannot read dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "collomatique"))
        .collect();
    files.sort();
    files
}

#[test]
fn all_examples_build() {
    let files = example_files();
    assert!(
        !files.is_empty(),
        "no *.collomatique files found in {} — did the examples dir move?",
        examples_dir().display()
    );

    for path in files {
        let name = path.display();
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {name}: {e}"));
        let (inner, _caveats) =
            deserialize_data(&content).unwrap_or_else(|e| panic!("failed to load {name}: {e}"));
        // Panics on internal inconsistency; building without a panic is the check.
        let _ = build_model(&inner.params);
    }
}
