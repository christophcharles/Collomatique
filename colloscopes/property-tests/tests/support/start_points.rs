//! Where a property walk begins.
//!
//! Every walk in this package used to begin at `harness::bootstrap`: a small
//! random document of a handful of students, with no assignment row worth
//! splitting into groups. Measured over 2810 probes, `property_greedy_groups`
//! planned 1331 group-list specs and **every one of them was a single group**,
//! so the greedy's placement search was never exercised at all.
//!
//! A longer walk does not fix that — `tests/fixture_starts.rs` explains why,
//! with the measurements. What fixes it is starting somewhere else. So a walk
//! runs from five start points: the small random document it has always used,
//! plus the four committed documents of `tests/fixtures/`. One seed × 200 ops
//! from a big start reaches more interesting specs than the whole current
//! fifteen-seed run at twenty times the length.
//!
//! **The seed budget.** A big document costs about four times as much per op
//! (it is cloned and invariant-checked at every step), so five starts at full
//! seed count would take the fuzz suite from 44 s to about twelve minutes.
//! Instead the four big starts *together* get the budget of the one small
//! start: each runs `seeds / 4`. That is sound because a seed no longer buys
//! what it buys from the bootstrap. There the seed picks the document as well
//! as the op stream; from a big start the document is fixed and the seed only
//! varies the ops — and there is far more to see per op.
//!
//! **Who opts in.** Six walks: `property_ops`, `property_update_ops`,
//! `property_apply_gate`, `property_cascade`, `property_content_ord` and
//! `property_greedy_groups`. The two model-build walks keep the bootstrap
//! start alone, each for a reason stated in its own file.
//!
//! Files under `tests/` subdirectories are not auto-discovered as test
//! targets, so this one is pulled in with `#[path]` and needs no Cargo stanza,
//! exactly like `generation_request.rs` beside it.

use std::path::PathBuf;

use collomatique_testgen_colloscopes::{ChaCha8Rng, harness};

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;
use collomatique_storage::deserialize_data;

use harness::RunConfig;

/// The four committed big start documents
///
/// The same list as `fixture_starts.rs`'s `FIXTURES`, which is where these
/// files come from and what guards them: a fifth fixture has to be named in
/// both places.
const DOCUMENTS: [&str; 4] = [
    "start_hogwarts.collomatique",
    "start_grown_0.collomatique",
    "start_grown_1.collomatique",
    "start_grown_hogwarts.collomatique",
];

/// The fraction of a walk's seeds each big start gets, as a divisor
///
/// Four starts at a quarter each, so the big starts cost about what the small
/// one costs — see the module doc for why fewer seeds lose nothing here.
const BIG_START_SEED_DIVISOR: u64 = 4;

/// Where a walk begins
///
/// [Start::Bootstrap] is the small random document the walks have always
/// used; [Start::Document] is one of the committed fixtures, carrying its file
/// name so a failure report says which one.
pub(crate) enum Start {
    Bootstrap,
    Document(&'static str, Data),
}

impl std::fmt::Display for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Start::Bootstrap => f.write_str("bootstrap"),
            Start::Document(name, _data) => f.write_str(name),
        }
    }
}

/// Loads one committed fixture through the invariant gate
fn load(name: &'static str) -> Start {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the walk-start fixture {}: {e} — regenerate it with \
             `cargo test -p collomatique-property-tests --features fuzz \
             --test fixture_starts -- --ignored`",
            path.display(),
        )
    });
    let (inner, caveats) =
        deserialize_data(&text).unwrap_or_else(|e| panic!("cannot load {name}: {e}"));
    assert!(
        caveats.is_empty(),
        "{name} must load without caveats, got {caveats:?}",
    );
    // A walk assumes a valid document at op zero, so a fixture that no longer
    // passes the gate must fail here rather than somewhere inside a property.
    let data = Data::from_inner_data(inner)
        .unwrap_or_else(|e| panic!("{name}: a start document must pass the invariant gate: {e}"));

    Start::Document(name, data)
}

/// The five start points, in a fixed order
///
/// Called once per walk, so the four documents are parsed once per walk rather
/// than once per seed.
pub(crate) fn all() -> Vec<Start> {
    let mut starts = vec![Start::Bootstrap];
    starts.extend(DOCUMENTS.map(load));

    starts
}

/// How many seeds this start deserves out of the walk's budget
pub(crate) fn seeds_for(start: &Start, cfg: &RunConfig) -> u64 {
    match start {
        Start::Bootstrap => cfg.seeds,
        // Rounded up, so a walk configured with fewer than four seeds still
        // runs each big start at least once.
        Start::Document(_name, _data) => cfg.seeds.div_ceil(BIG_START_SEED_DIVISOR),
    }
}

/// The state and initial snapshot pool for one start point
///
/// The snapshots are the walk's history positions. A bootstrap returns one per
/// bootstrap op; a loaded document has no history at all, so it returns the
/// single position it starts at — which is the shape the undo/redo walks of
/// `property_ops` need, and they were run this way before the plan was
/// written.
pub(crate) fn open(start: &Start, rng: &mut ChaCha8Rng) -> (AppState<Data, String>, Vec<Data>) {
    match start {
        Start::Bootstrap => harness::bootstrap(rng),
        Start::Document(_name, data) => {
            (AppState::<_, String>::new(data.clone()), vec![data.clone()])
        }
    }
}
