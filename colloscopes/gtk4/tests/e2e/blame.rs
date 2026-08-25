//! Blaming a real colloscope, on the real binary
//!
//! One script, `scripts/blame_e2e.py`, and two frozen documents: the hogwarts
//! fixture the solve tests open, and the same document with the colloscope a
//! solve produced. The second file is the first plus its `Colloscope` block and
//! nothing else, so the two blames are one model asked about two colloscopes.
//!
//! This is the only place `model.blame` runs end to end. Its refusals are
//! covered in `colloscopes/python/tests/module.rs` without an engine; what
//! needs a real one is the answer itself, since filling in the variables a
//! colloscope does not carry takes a solver.

use std::path::PathBuf;
use std::process::Command;

use crate::{COLLOMATIQUE, succeeds};

/// Something in this crate, named from the crate root
///
/// Both the script and the fixtures live beside this file, and a test is not
/// run from any particular directory — so none of them can be named relatively.
fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// An unsolved colloscope is blamed, and a solved one is not
///
/// The two halves say different things. The empty colloscope says a blame is
/// *reached* and is about this document — there is plenty wrong with a
/// colloscope holding nothing, and the script checks that none of it is worse
/// than `STRUCTURAL`: no pin was asked for, and a document a solve answers is
/// not an infeasible one. The solved colloscope says the blame is *right* —
/// what the solver produced breaks nothing, which is the one answer that cannot
/// be had by getting the question wrong.
///
/// Everything else is the script's own, and it prints what it saw on the way.
#[test]
fn a_colloscope_is_blamed_and_a_solved_one_is_not() {
    let mut command = Command::new(COLLOMATIQUE);
    command
        .arg("--python-file")
        .arg(manifest_path("tests/e2e/scripts/blame_e2e.py"))
        .env(
            "E2E_FIXTURE",
            manifest_path("tests/e2e/fixtures/hogwarts.collomatique"),
        )
        .env(
            "E2E_SOLVED",
            manifest_path("tests/e2e/fixtures/hogwarts_solved.collomatique"),
        )
        .env_remove("COLLOMATIQUE_ENGINE");

    succeeds(command);
}
