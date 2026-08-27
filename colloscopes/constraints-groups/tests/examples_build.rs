//! Smoke test: the generation plan must build — and the greedy must run — for
//! every file under the repo-root `examples/` directory, for the *maximal*
//! request (rebuild every assigned pair, keep every prefilled list).
//!
//! `build_generation_plan` panics — or returns `Err` — on internal
//! inconsistency, so getting through without one is the assertion. The greedy
//! needs no solver: it produces the lists itself, so this test also checks
//! that they are structurally sound. The directory is walked at runtime, so a
//! new `.collomatique` example is covered automatically.
//!
//! This test is about the *documents*: our shipped examples must be usable by
//! the generator, and a new one must be covered the day it lands. That is why
//! it reads `examples/` directly rather than a frozen copy — the property
//! walks make the opposite call, reading `tests/fixtures/`, because they are
//! about the code.

use collomatique_constraints_groups::{
    GenerationRequest, build_generation_plan, greedy_group_lists,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_storage::deserialize_data;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The repo-root `examples/` directory, relative to this crate's manifest.
fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// The maximal request against a document: rebuild every assigned pair whose
/// subject has interrogations, keep every prefilled list.
fn maximal_request(params: &Parameters) -> GenerationRequest {
    let mut rebuild = BTreeSet::new();
    for (period, subject, _students) in params.assignments.iter() {
        let has_interrogations = params
            .subjects
            .find_subject(subject)
            .is_some_and(|s| s.parameters.interrogation_parameters.is_some());
        if has_interrogations {
            rebuild.insert((period, subject));
        }
    }
    let kept_lists = params
        .group_lists
        .group_list_map
        .iter()
        .filter(|(_, list)| list.is_prefilled())
        .map(|(id, _)| id)
        .collect();

    GenerationRequest {
        rebuild,
        kept_lists,
    }
}

/// Sorted list of every `*.collomatique` file in `examples/`.
fn example_files() -> Vec<PathBuf> {
    let dir = examples_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read examples dir {}: {e}", dir.display()))
        .map(|entry| entry.expect("cannot read dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "collomatique"))
        .collect();
    files.sort();
    files
}

#[test]
fn all_examples_build() {
    let files = example_files();
    assert!(
        !files.is_empty(),
        "no *.collomatique files found in {} — did the examples dir move?",
        examples_dir().display()
    );

    for path in files {
        let name = path.display();
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {name}: {e}"));
        let (inner, _caveats) =
            deserialize_data(&content).unwrap_or_else(|e| panic!("failed to load {name}: {e}"));
        let params = &inner.params;

        let request = maximal_request(params);
        let plan = build_generation_plan(params, &request)
            .unwrap_or_else(|e| panic!("plan build failed for {name}: {e}"));
        // The greedy must produce structurally valid lists on every example.
        let names: Vec<String> = (0..plan.specs.len())
            .map(|i| format!("Liste {i}"))
            .collect();
        let lists = greedy_group_lists(&plan, &names);
        assert_eq!(
            lists.len(),
            plan.specs.len(),
            "one list per spec for {name}"
        );
        for ((list, _covered), (spec, _)) in lists.iter().zip(plan.specs.iter()) {
            assert_eq!(
                list.filling().iter_students().count(),
                spec.students().len(),
                "every student placed exactly once in {name}",
            );
        }
    }
}
