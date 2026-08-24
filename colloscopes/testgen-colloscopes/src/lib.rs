//! Deterministic op-sequence generation for colloscope property tests
//!
//! Extracted from `colloscopes/state-colloscopes/tests/property_ops` so that other
//! crates (e.g. `constraints-colloscopes`) can reuse the same fuzzy-walk
//! machinery. Consumers get the generator, the per-seed harness, and the
//! entity synthesizers, plus the `rand`/`rand_chacha` re-exports they need
//! to drive the RNG without adding those deps themselves.

pub mod generator;
pub mod harness;
pub mod synth;

pub use rand;
pub use rand_chacha::{self, ChaCha8Rng};
