//! The command line's script mode with a document, on the real binary
//!
//! `[FILE]` (or `--new`) hosts the script with a document, `--out` writes what
//! the process holds when the script ends, and a send that no `--out` will keep
//! is warned about. No solve runs here: every script only reads and edits, so
//! this family is cheap next to its neighbours.
//!
//! The scripts get their parameters on the command line itself — the fixture as
//! the positional `[FILE]`, the destination as `--out` — rather than through the
//! environment like `open_and_solve`'s, because the feature under test is
//! precisely that the command line carries them.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{COLLOMATIQUE, succeeds};

/// Something in this crate, named from the crate root
///
/// Both the scripts and the fixture live beside this file, and a test is not
/// run from any particular directory — so neither can be named relatively.
fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// A directory of this test's own, emptied first
///
/// `open_and_solve`'s, plus a per-test suffix: two tests of this family write
/// files, they run in threads of one process, and emptying the directory first
/// is what would otherwise put them in each other's way.
fn workspace(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "collomatique-gtk4-e2e-{}-{test}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary directory should be creatable");
    dir
}

/// The frozen fixture, which is only ever opened
fn fixture() -> PathBuf {
    manifest_path("tests/e2e/fixtures/hogwarts.collomatique")
}

/// Reads a colloscope file the way the application does
///
/// `collomatique_storage::deserialize_data` and then the invariant gate, the
/// pair `colloscopes/gtk4/src/loading/file_loader.rs` runs: the point is that
/// what `--out` wrote is a document, and not merely bytes.
fn reload(path: &Path) -> collomatique_state_colloscopes::Data {
    let content = std::fs::read_to_string(path).expect("the run saved this file");
    let (inner_data, caveats) =
        collomatique_storage::deserialize_data(&content).expect("the saved file should decode");
    assert!(
        caveats.is_empty(),
        "the saved file should read whole, got {caveats:?}"
    );
    collomatique_state_colloscopes::Data::from_inner_data(inner_data)
        .expect("the saved document should satisfy the in-memory invariants")
}

/// An opened document is edited, sent, and `--out` keeps it
///
/// The whole road of the feature: the positional file becomes the script's
/// `current_document()`, the script sends an edited one back, and what the run
/// leaves at `--out` is that one — read back here through the invariant gate,
/// with the added student in it.
///
/// `--out` comes before the positional because `gtk_options` is a trailing
/// var-arg that takes hyphenated values: after the file, the option would be
/// swallowed as a GTK argument.
#[test]
fn an_opened_document_is_modified_sent_and_written_out() {
    let dir = workspace("out");
    let saved = dir.join("out.collomatique");

    let mut command = Command::new(COLLOMATIQUE);
    command
        .arg("--python-file")
        .arg(manifest_path("tests/e2e/scripts/script_document_e2e.py"))
        .arg("--out")
        .arg(&saved)
        .arg(fixture())
        .env_remove("COLLOMATIQUE_ENGINE");

    succeeds(command);

    let data = reload(&saved);
    let students = &data.get_inner_data().params.students.student_map;
    assert_eq!(
        students.len(),
        25,
        "the fixture's 24 students plus the one the script added"
    );
    assert!(
        students
            .iter()
            .any(|(_, s)| s.desc.firstname == "Nymphadora" && s.desc.surname == "Tonks"),
        "the added student is in the saved file"
    );

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// `--new` and `--out`: a document built from nothing is written out
///
/// The other end of the same road — a script that starts from an empty document
/// rather than from a file, which is what makes `--new` and a script no longer
/// exclude each other.
#[test]
fn a_new_document_is_built_and_written_out() {
    let dir = workspace("new");
    let saved = dir.join("new.collomatique");

    let mut command = Command::new(COLLOMATIQUE);
    command
        .arg("--python-file")
        .arg(manifest_path(
            "tests/e2e/scripts/script_document_new_e2e.py",
        ))
        .arg("--new")
        .arg("--out")
        .arg(&saved)
        .env_remove("COLLOMATIQUE_ENGINE");

    succeeds(command);

    let data = reload(&saved);
    let students = &data.get_inner_data().params.students.student_map;
    assert_eq!(students.len(), 1, "the one student the script added");
    let (_, student) = students.iter().next().expect("one student");
    assert_eq!(student.desc.firstname, "Harry");
    assert_eq!(student.desc.surname, "Potter");

    std::fs::remove_dir_all(&dir).expect("the temporary directory should be removable");
}

/// A send with no `--out` succeeds, and says the work is dropped
///
/// The run ends well: sending with nowhere to send to is not the script's
/// mistake, so the only trace is the sentence — which is asked of
/// `collomatique_ui_text` rather than spelled out here, so that rewording it
/// stays one edit.
#[test]
fn dropped_modifications_are_warned_about() {
    let mut command = Command::new(COLLOMATIQUE);
    command
        .arg("--python-file")
        .arg(manifest_path(
            "tests/e2e/scripts/script_document_warn_e2e.py",
        ))
        .arg(fixture())
        .env_remove("COLLOMATIQUE_ENGINE");

    let output = succeeds(command);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(collomatique_ui_text::script::lost_modifications_text()),
        "the warning is on stderr: {stderr}"
    );
}

/// A script that never sends is not warned at
///
/// The same command line as the test above, minus the send: what the pair says
/// together is that the warning follows the send, and is not just what a run
/// without `--out` always prints.
#[test]
fn a_script_that_only_reads_is_not_warned_at() {
    let mut command = Command::new(COLLOMATIQUE);
    command
        .arg("--python-file")
        .arg(manifest_path(
            "tests/e2e/scripts/script_document_read_e2e.py",
        ))
        .arg(fixture())
        .env_remove("COLLOMATIQUE_ENGINE");

    let output = succeeds(command);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains(collomatique_ui_text::script::lost_modifications_text()),
        "no warning for a document that was never sent: {stderr}"
    );
}
