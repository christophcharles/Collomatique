//! Standing coverage net for [`InnerData::compact_ids`] over generated documents
//!
//! The unit tests in `src/compact.rs` pin the renumbering *exactly*, on a
//! hand-built fixture that was written against the inventory of id-bearing
//! fields — it is exhaustive today. This harness is the net for tomorrow: it
//! runs the same pass over documents the testgen walk builds, so a field added
//! to the model and populated by the generator, but forgotten by the
//! compaction walk, is caught even if nobody thinks to extend that fixture.
//!
//! Two properties, and they catch different mistakes:
//!
//! - **Compaction preserves validity.** A reference (or ordering-mirror key)
//!   the walk forgets keeps its old value while its target is renumbered, so it
//!   dangles — and the invariant gate says so.
//! - **The result is densely numbered.** A whole table of *defining* ids the
//!   walk forgets dangles nowhere (nothing references e.g. an incompatibility),
//!   so only counting the ids catches it.
//!
//! On failure the harness prints the seed and the full op log, so the op
//! sequence replays exactly.

use std::cell::Cell;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, InnerData, Op};

use harness::{OpLog, RunConfig};

/// Smaller than the cascade family harnesses: this property is about the shape
/// of a document, not about a trajectory through the op space, so a handful of
/// varied end states is what it needs.
const CONFIG: RunConfig = RunConfig {
    seeds: 10,
    ops_per_run: 200,
    invalid_fraction: 0.15,
};

/// Generates the next op and logs it.
fn next_op(
    rng: &mut ChaCha8Rng,
    data: &Data,
    snapshots: &[InnerData],
    log: &mut OpLog,
) -> (&'static str, Op) {
    let (category, op) = generator::gen_op(
        rng,
        data.get_inner_data(),
        snapshots,
        CONFIG.invalid_fraction,
    );
    log.push(category, &op);
    (category, op)
}

#[test]
fn compaction_of_a_generated_document_is_valid_and_dense() {
    let compacted_documents = Cell::new(0usize);
    let compacted_ids = Cell::new(0usize);

    harness::for_each_seed(
        "compaction_of_a_generated_document_is_valid_and_dense",
        &CONFIG,
        |rng, log, stats| {
            let (mut state, _) = harness::bootstrap(rng);
            let mut snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = next_op(rng, state.get_data(), &snapshots, log);
                let applied = state.apply(op, "generated op".to_string());
                stats.record(category, applied.is_ok());
                if applied.is_ok() && snapshots.len() < 8 && rng.random_bool(0.02) {
                    snapshots.push(state.get_data().get_inner_data().clone());
                }
            }

            let compacted = state.get_data().get_inner_data().clone().compact_ids();

            // Every occurrence of an id moved together, so the document is
            // still one the rest of the crate accepts.
            let checked = Data::from_inner_data(compacted)
                .expect("compaction should preserve the invariants of a valid document");

            // ...and what came out is numbered 0, 1, 2… — nothing was left
            // behind at its old value. `all_ids` enumerates the defining ids of
            // every table, which on a valid document is the whole id space
            // (every reference resolves to one of them).
            let mut ids: Vec<u64> = checked
                .get_inner_data()
                .params
                .all_ids()
                .map(|id| id.inner())
                .collect();
            ids.sort_unstable();
            assert_eq!(
                ids,
                (0..ids.len() as u64).collect::<Vec<u64>>(),
                "the ids of a compacted document must be exactly 0..n",
            );

            compacted_documents.set(compacted_documents.get() + 1);
            compacted_ids.set(compacted_ids.get() + ids.len());
        },
    );

    // Coverage guard: both assertions are vacuous on an empty document, so pin
    // that the walk really saw populated ones.
    assert!(
        compacted_ids.get() > 10 * compacted_documents.get(),
        "the generated documents were too small to exercise compaction \
         ({} ids over {} documents)",
        compacted_ids.get(),
        compacted_documents.get(),
    );

    eprintln!(
        "compaction fuzz: {} documents compacted, {} ids renumbered in total",
        compacted_documents.get(),
        compacted_ids.get(),
    );
}
