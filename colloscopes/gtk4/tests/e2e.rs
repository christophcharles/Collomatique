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

const COLLOMATIQUE: &str = env!("CARGO_BIN_EXE_collomatique-gtk4");

#[path = "e2e/solve.rs"]
mod solve;
