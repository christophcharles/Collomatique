//! The random `GenerationRequest` generator, shared by the walks that need
//! one.
//!
//! It lives here rather than in either walk so that `property_build_groups`
//! (plan → model → conversion) and `property_greedy_groups` (plan → greedy)
//! cannot drift apart in what they consider a reachable request. Files under
//! `tests/` subdirectories are not auto-discovered as test targets, so this
//! one is pulled in with `#[path]` and needs no Cargo stanza.

use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::ChaCha8Rng;
use collomatique_testgen_colloscopes::rand::Rng;

use collomatique_constraints_groups::{GenerationRequest, GroupListSpec};
use collomatique_state_colloscopes::colloscope_params::Parameters;

/// Random valid request drawn from the current state: any assigned
/// (period, subject) pair whose subject has interrogations *and* whose group
/// sizes can be satisfied may be rebuilt, any prefilled list may be kept.
///
/// The feasibility filter mirrors the config dialog, which gates on the very
/// same constructor before offering a subject: a valid document may perfectly
/// well ask for groups of 5 to 6 students out of a class of 4, and neither
/// the dialog nor this generator may hand such a pair to the planner.
pub(crate) fn gen_generation_request(
    rng: &mut ChaCha8Rng,
    params: &Parameters,
) -> GenerationRequest {
    let mut rebuild = BTreeSet::new();
    // Every size range seen along the way, so the canonical-range override
    // can be drawn from a plausible one.
    let mut ranges = Vec::new();
    for (period, subject, students) in params.assignments.iter() {
        let Some(interrogations) = params
            .subjects
            .find_subject(subject)
            .and_then(|s| s.parameters.interrogation_parameters.clone())
        else {
            continue;
        };
        // An empty student set is legitimately `skipped` by the planner, so
        // it stays in the request; only unsatisfiable sizes are filtered.
        let usable = students.is_empty()
            || GroupListSpec::new(students.clone(), interrogations.students_per_group.clone())
                .is_ok();
        if usable && rng.random_bool(0.5) {
            rebuild.insert((period, subject));
            ranges.push(interrogations.students_per_group);
        }
    }

    let mut kept_lists = BTreeSet::new();
    for (id, list) in params.group_lists.group_list_map.iter() {
        if list.is_prefilled() && rng.random_bool(0.5) {
            kept_lists.insert(id);
        }
    }

    // Mostly automatic, as the dialog leaves it: the manual path is rarer
    // but must survive the same round trip. The override is not required to
    // be a range any spec uses, but drawing it from one keeps the walk
    // exercising realistic elections.
    let canonical_range = if ranges.is_empty() || !rng.random_bool(0.1) {
        None
    } else {
        Some(ranges[rng.random_range(0..ranges.len())].clone())
    };

    GenerationRequest {
        rebuild,
        kept_lists,
        canonical_range,
    }
}
