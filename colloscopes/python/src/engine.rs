//! The executable a solve re-executes as its engine
//!
//! A solve does not run in the interpreter's own process: the model is handed
//! to worker subprocesses, each of them a collomatique binary re-executed as
//! `<exe> --rpc-engine`. So a script that solves has to say *which* binary.
//!
//! Four rungs, in order: what the call was given, what the runner injected,
//! what the environment names, what the build baked in. Nothing found is a
//! loud [NoEngine] rather than a guess — the running executable is only an
//! engine when whoever started the interpreter says it is, and a bare `python`
//! is not one.
//!
//! The second rung mirrors [crate::host]: a static the runner fills on both
//! sides of a script, so this crate never has to know who is running it.

use std::path::PathBuf;
use std::sync::Mutex;

use pyo3::prelude::*;

/// Which collomatique binary the workers re-execute
///
/// `subprocesses`' own type, passed through rather than mirrored: whoever
/// installs an engine here is naming the same thing the solver will spawn, and
/// re-exporting it means a caller of [set_engine] needs this crate and nothing
/// else — the arrangement [crate::Host] already has.
pub use collomatique_subprocesses::EngineExe;

use crate::errors::NoEngine;

#[cfg(test)]
mod tests;

/// The engine the current run was handed, if it was handed one
static ENGINE: Mutex<Option<EngineExe>> = Mutex::new(None);

/// The engine the build baked in, if the build named one
///
/// `COLLOMATIQUE_DEFAULT_ENGINE` at *compile* time, which is what a standalone
/// python library has instead of a runner: the nix wheel derivation sets it to
/// the store path of the collomatique it was built against, so an installed
/// module solves without anyone naming a binary. Unset — the embedded build,
/// and any plain `cargo build` — leaves the rung simply absent.
const BAKED: Option<&str> = option_env!("COLLOMATIQUE_DEFAULT_ENGINE");

/// Installs, or clears, the engine for the coming run
///
/// The runner calls this on both sides of a script, like [crate::set_host],
/// with whatever its own caller chose (`run_python_script`'s `engine`
/// parameter). A hosted process *is* a collomatique binary, so rpc-engine
/// passes [EngineExe::Current] and scripts never think about it.
///
/// Clearing afterwards is what keeps a second run in the same process from
/// inheriting the first one's engine.
pub fn set_engine(engine: Option<EngineExe>) {
    *ENGINE.lock().unwrap() = engine;
}

/// The engine one solve will re-execute
///
/// `explicit` is the `engine=` of the call being served. It wins, because it
/// is the most local thing said about this particular solve; the injected
/// engine wins over the environment for the same reason, being about this run
/// rather than about the machine. The baked default is last, being about the
/// build — the least local thing there is.
pub(crate) fn resolve(explicit: Option<PathBuf>) -> PyResult<EngineExe> {
    resolve_with(explicit, BAKED)
}

/// [resolve], with the baked rung passed in rather than compiled in
///
/// The split is for the tests: a compile-time constant is not something a unit
/// test can vary, so the one thing that reads it is [resolve] itself.
fn resolve_with(explicit: Option<PathBuf>, baked: Option<&str>) -> PyResult<EngineExe> {
    if let Some(path) = explicit {
        return Ok(EngineExe::Explicit(path));
    }

    if let Some(engine) = ENGINE.lock().unwrap().clone() {
        return Ok(engine);
    }

    // An empty variable is an unset one: `COLLOMATIQUE_ENGINE= script.py`
    // means "not here", not "the empty path".
    if let Some(path) = std::env::var_os("COLLOMATIQUE_ENGINE").filter(|p| !p.is_empty()) {
        return Ok(EngineExe::Explicit(PathBuf::from(path)));
    }

    // Same rule as the variable above, for the same reason: a build that set
    // the name to nothing named nothing.
    if let Some(path) = baked.filter(|p| !p.is_empty()) {
        return Ok(EngineExe::Explicit(PathBuf::from(path)));
    }

    // The message says nothing of the baked rung: when this fires, the build
    // did not name an engine either, so there is nothing there to point at.
    Err(NoEngine::new_err(
        "no engine to run the solve: pass engine= with the path of a collomatique \
         executable, set the COLLOMATIQUE_ENGINE environment variable, or run the \
         script inside collomatique",
    ))
}
