//! Solving a colloscope from a script, on the real binary
//!
//! One fixture script, `scripts/solve_e2e.py`, run through `--python-file` once
//! per test. It carries every assertion about what the api does; what a test
//! here carries is the *surroundings* — the flag, the environment variable, the
//! engine path — and the exit status is how the two meet. That split is the
//! reason these are subprocesses at all: the three rungs of engine resolution
//! are a property of the process a script runs in, and cannot be told apart
//! from inside one interpreter.

use std::path::PathBuf;
use std::process::Command;

use crate::{COLLOMATIQUE, succeeds};

/// The script every test here runs
fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/e2e/scripts/solve_e2e.py")
}

/// A collomatique about to run the fixture script in `mode`
///
/// `COLLOMATIQUE_ENGINE` is removed rather than left alone: a developer with
/// one set in their shell would otherwise be running different tests from the
/// ones on a build machine. The tests it is about put it back themselves.
fn child(mode: &str) -> Command {
    let mut command = Command::new(COLLOMATIQUE);

    command
        .arg("--python-file")
        .arg(script())
        .env("E2E_MODE", mode)
        .env_remove("COLLOMATIQUE_ENGINE")
        .env_remove("E2E_ENGINE");

    command
}

/// The whole road, on the engine the runner injected
///
/// No flag and no variable: the child is a collomatique, so it offers itself as
/// the engine and the script never names one. This is the ordinary case, and
/// the only test that goes as far as installing the colloscope it obtained.
#[test]
fn a_solve_runs_on_the_injected_engine() {
    succeeds(child("full"));
}

/// The injected engine outranks the environment
///
/// The variable names a path with nothing at it, and the solve succeeds anyway:
/// it was never consulted.
#[test]
fn the_injected_engine_beats_the_environment() {
    let mut command = child("engine_rung");
    command.env("COLLOMATIQUE_ENGINE", "/nonexistent/collomatique");

    succeeds(command);
}

/// The environment names the engine when nothing else does
///
/// `--python-no-engine` is what withholds the child from itself, which leaves
/// the variable as the only rung with anything to say.
#[test]
fn the_environment_names_the_engine() {
    let mut command = child("engine_rung");
    command
        .arg("--python-no-engine")
        .env("COLLOMATIQUE_ENGINE", COLLOMATIQUE);

    succeeds(command);
}

/// An `engine=` outranks the environment
///
/// The two swap roles from the test above: the variable is the dead path this
/// time, and the working one reaches `engine=` through a variable of the test's
/// own, since the script has no argv to read it from.
#[test]
fn an_explicit_engine_beats_the_environment() {
    let mut command = child("engine_rung");
    command
        .arg("--python-no-engine")
        .env("COLLOMATIQUE_ENGINE", "/nonexistent/collomatique")
        .env("E2E_ENGINE", COLLOMATIQUE);

    succeeds(command);
}

/// No rung says anything, and the refusal is loud
#[test]
fn no_engine_is_a_loud_refusal() {
    let mut command = child("no_engine");
    command.arg("--python-no-engine");

    succeeds(command);
}

/// An engine was named, and there is nothing at that path
#[test]
fn a_dead_engine_fails_the_solve() {
    let mut command = child("dead_engine");
    command
        .arg("--python-no-engine")
        .env("COLLOMATIQUE_ENGINE", "/nonexistent/collomatique");

    succeeds(command);
}
