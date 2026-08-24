//! Opening a real colloscope file and solving it, on the real binary
//!
//! One document, `fixtures/hogwarts.collomatique`, opened from disk rather than
//! built in memory: a frozen copy of the example, decoupled from the living
//! `examples/` file the way `colloscopes/ops/tests/fixtures/` is, so the
//! example can evolve without changing what this asserts.
//!
//! The script does the road — load, build, solve, install, save — and the two
//! ends of it are rust's: the file handed in, and the file that came back out,
//! read here the way the application reads one.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{COLLOMATIQUE, succeeds};

/// Something in this crate, named from the crate root
///
/// Both the script and the fixture live beside this file, and a test is not run
/// from any particular directory — so neither can be named relatively.
fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// A directory of this test's own, emptied first
///
/// The script saves, so it needs somewhere to save that is not the repository.
/// A per-process name keeps two runs of the suite out of each other's way
/// without a `tempfile` dependency — `colloscopes/python/tests/module.rs`'s
/// `workspace` does the same, for the same reason.
fn workspace() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("collomatique-gtk4-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary directory should be creatable");
    dir
}

/// Reads a colloscope file the way the application does
///
/// `collomatique_storage::deserialize_data` and then the invariant gate, which
/// is the pair `colloscopes/gtk4/src/loading/file_loader.rs` runs: the point is
/// that what the solve wrote is a document, and not merely bytes.
fn reload(path: &Path) -> collomatique_state_colloscopes::Data {
    let content = std::fs::read_to_string(path).expect("the script saved this file");
    let (inner_data, caveats) =
        collomatique_storage::deserialize_data(&content).expect("the saved file should decode");
    assert!(
        caveats.is_empty(),
        "the saved file should read whole, got {caveats:?}"
    );
    collomatique_state_colloscopes::Data::from_inner_data(inner_data)
        .expect("the saved document should satisfy the in-memory invariants")
}

/// A file is opened, solved, installed, saved, and reads back as a document
///
/// The whole road on a real document. What the script leaves behind is the
/// proof it ran: a file with a colloscope in it, which this reads back through
/// the invariant gate — a solved colloscope that will not load is not a solved
/// colloscope.
#[test]
fn a_file_is_opened_solved_and_saved() {
    let dir = workspace();
    let saved = dir.join("solved.collomatique");

    let mut command = Command::new(COLLOMATIQUE);
    command
        .arg("--python-file")
        .arg(manifest_path("tests/e2e/scripts/open_and_solve_e2e.py"))
        .env(
            "E2E_FIXTURE",
            manifest_path("tests/e2e/fixtures/hogwarts.collomatique"),
        )
        .env("E2E_SAVE", &saved)
        .env_remove("COLLOMATIQUE_ENGINE");

    succeeds(command);

    let data = reload(&saved);
    assert!(
        !data.get_inner_data().colloscope.are_interrogations_empty(),
        "the saved document holds the colloscope the solve produced",
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}
