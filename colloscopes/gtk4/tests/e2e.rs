//! End-to-end tests against the built collomatique binary
//!
//! Everything here spawns `CARGO_BIN_EXE_collomatique-gtk4` — the binary cargo
//! just built, in this invocation's own profile — and asserts on exit status
//! and output. No embedded interpreter lives in this process: each child owns
//! its environment, which is what makes the engine rungs testable one by one.
//!
//! One module per family, and their files live under `tests/e2e/` beside the
//! fixture scripts they need. This file is the crate root of its own target, so
//! its modules would otherwise be looked for beside *it*, in `tests/` — hence
//! the paths, one per `mod`.
//!
//! What every family shares lives here rather than in one of them: the binary
//! they all spawn, and the assertion they all end on.
//!
//! Real solves in a debug build cost about two minutes, so this target is not
//! part of the day-to-day `cargo test`: it is behind the package's `e2e`
//! feature, and `cargo e2e` (or `cargo full-test`) is what turns it on.

use std::process::{Command, Output};

const COLLOMATIQUE: &str = env!("CARGO_BIN_EXE_collomatique-gtk4");

#[path = "e2e/open_and_solve.rs"]
mod open_and_solve;
#[path = "e2e/solve.rs"]
mod solve;

/// Runs `command` and insists it ended well
///
/// The script's own output is what says *why* when it did not, so both streams
/// come back out here: an assertion failing inside python is otherwise a bare
/// exit code.
#[track_caller]
fn succeeds(mut command: Command) -> Output {
    let output = command
        .output()
        .expect("the collomatique binary should start");

    assert!(
        output.status.success(),
        "the script failed ({}):\n--- stdout ---\n{}--- stderr ---\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    output
}
