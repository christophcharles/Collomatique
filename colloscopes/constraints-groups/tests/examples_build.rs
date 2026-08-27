//! Smoke test: the generation model must build — and the greedy must run —
//! for every file under the repo-root `examples/` directory, for the
//! *maximal* request (rebuild every assigned pair, keep every prefilled
//! list).
//!
//! `build_generation_plan` and `build_model` panic — or, for the former,
//! return `Err` — on internal inconsistency, so getting through without one
//! is the assertion; no solver or solution is needed. The greedy needs no
//! solution either: it produces the lists itself, so this test also checks
//! that they are structurally sound. The directory is walked at runtime, so
//! a new `.collomatique` example is covered automatically.
//!
//! Beside it, `the_model_scores_the_greedy_placement_at_the_greedy_score`
//! reads a **frozen copy** of `examples/hogwarts.collomatique` under
//! `tests/fixtures/` instead. The two are different kinds of test. The walk
//! above is about the *documents*: our shipped examples must be usable by the
//! generator, and a new one must be covered the day it lands. The equality
//! below is about the *code* — it says nothing about hogwarts, only that the
//! model and the greedy agree — and a big real document is merely the context
//! that makes it bite. `examples/` is free to evolve, so a test of the
//! objective must not move with it. The property walks make the same split:
//! they read `tests/fixtures/`, and only the explicitly-run
//! `fixture_starts.rs` reads `examples/`.

use collomatique_constraints_groups::{
    FrozenPlacements, GenerationRequest, build_generation_plan, build_model, greedy_group_lists,
    group_lists_to_warm_start, placement_objective,
};
use collomatique_ilp::f64_equals;
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
        // Panics on internal inconsistency; building without a panic is the check.
        let _ = build_model(&plan, &FrozenPlacements::default());

        // The greedy must produce structurally valid lists on every example.
        let names: Vec<String> = (0..plan.specs.len())
            .map(|i| format!("Liste {i}"))
            .collect();
        let lists = greedy_group_lists(&plan, &names).lists;
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

/// A copy of `examples/hogwarts.collomatique`, frozen when the test below was
/// written. Own copy on purpose — see the module doc: the example is free to
/// evolve, this test is about the objective.
const FIXTURE: &str = include_str!("fixtures/hogwarts.collomatique");

#[test]
fn the_model_scores_the_greedy_placement_at_the_greedy_score() {
    // The anti-drift net of `objective::tests::objective_matches_the_greedy_-
    // ground_truth`, at a scale the hand-written plans cannot reach. It
    // catches two things they miss: a warm start that names a variable the
    // lazily-built model does not have — or misses one it does, since
    // `solution_from_complete_data` refuses either wholesale — and any drift
    // between the two objectives on the tier shapes, multiplicities and kept
    // lists that only a real document produces. No solver: reading an
    // objective at a known configuration is arithmetic.
    let (inner, caveats) =
        deserialize_data(FIXTURE).unwrap_or_else(|e| panic!("failed to load the fixture: {e}"));
    // It was pristine when it was copied and nothing writes it, so a caveat
    // here means a format migration passed the fixture by.
    assert!(
        caveats.is_empty(),
        "the fixture must load pristine, got {caveats:?}",
    );

    let params = &inner.params;
    let request = maximal_request(params);
    let plan = build_generation_plan(params, &request)
        .unwrap_or_else(|e| panic!("plan build failed for the fixture: {e}"));
    let model = build_model(&plan, &FrozenPlacements::default());

    let names: Vec<String> = (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect();
    let lists = greedy_group_lists(&plan, &names).lists;
    let expected = placement_objective(&plan, &lists);

    let solution = model
        .solution_from_complete_data(group_lists_to_warm_start(&plan, &lists))
        .expect("the warm start must value exactly the model's variables");
    assert!(
        solution.is_feasible(),
        "the greedy placement breaks {} constraint(s)",
        solution.blame().len(),
    );
    assert!(
        f64_equals(solution.eval(), expected),
        "the model scores {} where the greedy scores {expected}",
        solution.eval(),
    );
}
