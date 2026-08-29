//! Fuzz net over the greedy group-list generator
//!
//! Reuses the deterministic fuzzy-walk machinery from
//! `collomatique-testgen-colloscopes` (the same generator/harness that drives
//! the state property tests) to reach many arbitrary-but-valid documents, and
//! at each probe point draws a random *request* against that document, plans
//! it, and runs the greedy on the plan. The assertion is threefold: the
//! greedy **never panics** on a reachable valid state — it is riddled with
//! `expect`/`debug_assert` invariant claims, and it promises to always
//! succeed — it is **deterministic**, and it always emits **structurally
//! valid** lists (one per spec, minimal group count, balanced sizes inside
//! the spec's range, every student placed exactly once).
//!
//! The walk runs from five start points (`support/start_points.rs`), because
//! from the small bootstrap document alone every single spec it ever planned
//! was a one-group list, and a one-group list has no placement to decide.
//!
//! On failure the harness prints the seed and the full op log, so re-running
//! the binary reproduces the exact walk.

use std::cell::Cell;
use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{ChaCha8Rng, generator, harness};

use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::group_lists::{GroupList, GroupListFilling};
use collomatique_state_colloscopes::{InnerData, PeriodId, StudentId, SubjectId};

use collomatique_greedy_groups::{build_generation_plan, greedy_group_lists};

use harness::RunConfig;

// The request generator lives outside this file — see the module for why.
#[path = "support/generation_request.rs"]
mod generation_request;
use generation_request::gen_generation_request;

// The five documents the walk starts from. This walk is the reason they
// exist: from the bootstrap alone it planned 1331 specs and not one of them
// had a second group to place anybody in.
#[path = "support/start_points.rs"]
mod start_points;

/// The greedy on testgen-sized states costs microseconds, so this walk probes
/// after *every* successful op. Tune against the measured runtime.
///
/// `seeds` is the budget for the bootstrap start; the four big starts share
/// the same again between them (`start_points::seeds_for`).
const CONFIG: RunConfig = RunConfig {
    seeds: 15,
    ops_per_run: 200,
    invalid_fraction: 0.0,
};

/// One list's groups, as student sets in group order.
fn groups_of(list: &GroupList) -> Vec<BTreeSet<StudentId>> {
    match list.filling() {
        GroupListFilling::Prefilled { groups } => {
            groups.iter().map(|group| group.students.clone()).collect()
        }
        GroupListFilling::Automatic { .. } => panic!("the greedy only emits prefilled lists"),
    }
}

/// The comparable form of a whole output: `GroupList` also carries names,
/// which say nothing about the placement.
fn memberships(
    lists: &[(GroupList, BTreeSet<(PeriodId, SubjectId)>)],
) -> Vec<Vec<BTreeSet<StudentId>>> {
    lists
        .iter()
        .map(|(list, _covered)| groups_of(list))
        .collect()
}

/// What shape of spec the walk actually reached, across every (start, seed)
///
/// A green run is not by itself evidence here, and this walk is the one file
/// that already proved it: from the bootstrap start alone it planned 1331
/// specs, **every one a single group**, and stayed green under a mutation that
/// reverses the greedy's balanced targets. With one group there is nothing to
/// place and nothing to balance, so the assertions above are all satisfied
/// vacuously. The two guards below are what make the run mean something —
/// they count the shapes the greedy is actually about, and they are why the
/// start points exist.
///
/// `for_each_start_and_seed` takes a `Fn` closure, so these need interior
/// mutability.
#[derive(Default)]
struct Coverage {
    specs: Cell<usize>,       // specs planned in total
    multi_group: Cell<usize>, // specs the greedy split into two groups or more
    with_pair: Cell<usize>,   // specs where some group ended up holding two
}

impl Coverage {
    fn bump(cell: &Cell<usize>) {
        cell.set(cell.get() + 1);
    }

    fn assert_covered(&self) {
        assert!(
            self.multi_group.get() > 0,
            "every spec the walk planned was a single group — the greedy never had \
             a placement to decide, so a green run proves nothing about it",
        );
        assert!(
            self.with_pair.get() > 0,
            "no group the greedy emitted ever held two students — with every group \
             a singleton there is no balancing to get wrong",
        );
    }

    fn report(&self) {
        eprintln!(
            "greedy fuzz: {} specs planned, {} with several groups, {} with a group of two \
             or more",
            self.specs.get(),
            self.multi_group.get(),
            self.with_pair.get(),
        );
    }
}

/// One probe: synthesize a request, plan it, and run the greedy twice.
fn greedy_check(rng: &mut ChaCha8Rng, inner: &InnerData, coverage: &Coverage) {
    let request = gen_generation_request(rng, &inner.params);
    let plan = build_generation_plan(&inner.params, &request)
        .expect("a request drawn from valid state must produce a plan");

    let names: Vec<String> = (0..plan.specs.len())
        .map(|i| format!("Liste {i}"))
        .collect();
    let lists = greedy_group_lists(&plan, &names);

    // Deterministic: a second run on the same plan yields the same groups.
    assert_eq!(
        memberships(&lists),
        memberships(&greedy_group_lists(&plan, &names)),
        "greedy placement must be deterministic",
    );

    assert_eq!(lists.len(), plan.specs.len(), "one list per spec");
    for ((group_list, covered), (spec, spec_covered)) in lists.iter().zip(plan.specs.iter()) {
        assert_eq!(
            covered, spec_covered,
            "the covered pairs are carried through"
        );

        let groups = groups_of(group_list);
        Coverage::bump(&coverage.specs);
        if groups.len() >= 2 {
            Coverage::bump(&coverage.multi_group);
        }
        if groups.iter().any(|group| group.len() >= 2) {
            Coverage::bump(&coverage.with_pair);
        }

        let n = spec.students().len();
        let range = spec.students_per_group();
        let (min, max) = (range.start().get() as usize, range.end().get() as usize);

        assert_eq!(
            group_list.params().group_names.len(),
            groups.len(),
            "one name slot per group",
        );
        assert!(
            group_list.params().group_names.iter().all(Option::is_none),
            "the greedy names no group",
        );
        assert_eq!(
            groups.len(),
            n.div_ceil(max),
            "the greedy imposes the minimal group count",
        );

        // Every student of the spec exactly once, and nobody else.
        assert_eq!(
            group_list.filling().iter_students().count(),
            n,
            "every student placed exactly once by the greedy",
        );
        let placed: BTreeSet<StudentId> = groups.iter().flatten().copied().collect();
        assert_eq!(&placed, spec.students(), "exactly the spec's students");

        // The balanced targets, characterized rather than recomputed:
        // minimal count, descending, spread at most one, inside the range.
        // Together with the exact cover above, that pins the sizes uniquely,
        // without this test restating the production formula.
        let sizes: Vec<usize> = groups.iter().map(BTreeSet::len).collect();
        assert!(
            sizes.windows(2).all(|w| w[0] >= w[1]),
            "group sizes are descending",
        );
        let biggest = sizes[0];
        let smallest = *sizes
            .last()
            .expect("a spec always has students, hence a group");
        assert!(biggest - smallest <= 1, "group sizes differ by at most one");
        assert!(
            smallest >= min && biggest <= max,
            "every group size is inside the spec's range",
        );
    }
}

/// Along random valid-op walks, the greedy must neither panic nor produce an
/// invalid or non-reproducible placement for any reachable state.
#[test]
fn greedy_group_lists_never_panic_along_random_walks() {
    let coverage = Coverage::default();

    harness::for_each_start_and_seed(
        "greedy_group_lists_never_panic_along_random_walks",
        &CONFIG,
        &start_points::all(),
        |start| start_points::seeds_for(start, &CONFIG),
        |start, rng, log, stats| {
            let (mut state, _) = start_points::open(start, rng);
            let mut snapshots: Vec<InnerData> = vec![state.get_data().get_inner_data().clone()];

            // Probe the starting state before any random op.
            greedy_check(rng, state.get_data().get_inner_data(), &coverage);

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = generator::gen_op(
                    rng,
                    state.get_data().get_inner_data(),
                    &snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);

                match state.apply(op, category.to_string()) {
                    Ok(_) => {
                        stats.record(category, true);
                        let inner = state.get_data().get_inner_data();
                        // Re-check the state invariants so a corrupt-state
                        // failure is attributed to the state layer, not
                        // blamed on the greedy. Trivially cheap next to it.
                        assert_eq!(
                            inner.broken_invariants(),
                            Ok(BTreeSet::new()),
                            "invariants must hold after a successful op",
                        );
                        if snapshots.len() < 8 && rng.random_bool(0.02) {
                            snapshots.push(inner.clone());
                        }
                        greedy_check(rng, inner, &coverage);
                    }
                    Err(_) => {
                        stats.record(category, false);
                    }
                }
            }

            // Probe the final state.
            greedy_check(rng, state.get_data().get_inner_data(), &coverage);
        },
    );

    // Reported before the guards are checked, so a run that fails one still
    // prints the numbers that explain it.
    coverage.report();
    coverage.assert_covered();
}
