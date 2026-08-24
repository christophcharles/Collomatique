//! Smoke test: every file under the repo-root `examples/` directory must load.
//!
//! Example files are canonical, current-format documents, so they must be
//! *pristine*: they decode without errors AND without any caveats. Caveats
//! signal forward-compatibility concerns (opening a file produced by a newer
//! Collomatique version); a shipped example should never trip one.
//!
//! Each example also goes through the in-memory invariant gate. The decoder
//! returns a raw document and diagnoses the file format's constraints on its
//! own, so passing the gate is expected — this is where whole-document
//! agreement between the two is checked on real files.
//!
//! The directory is walked at runtime, so adding a new `.collomatique` example
//! is covered automatically with no edit here.

use collomatique_storage::deserialize_data;
use std::path::PathBuf;

/// The repo-root `examples/` directory, relative to this crate's manifest.
fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
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
fn all_examples_load_pristine() {
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
        let (inner, caveats) =
            deserialize_data(&content).unwrap_or_else(|e| panic!("failed to load {name}: {e}"));
        assert!(
            caveats.is_empty(),
            "{name} is not pristine: loaded with caveats {caveats:?}"
        );
        collomatique_state_colloscopes::Data::from_inner_data(inner)
            .unwrap_or_else(|e| panic!("{name} does not pass the invariant gate: {e}"));
    }
}
