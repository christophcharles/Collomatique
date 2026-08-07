// Progress must keep flowing when CBC restarts its tree search.
//
// On some models CBC abandons the model it started branching on and restarts on
// a smaller one, logging:
//
//     Cbc0044I Reduced cost fixing - 758 rows, 398 columns - restarting search
//
// That restarted searcher is a `CbcModel` whose `parentModel()` is the top-level
// one, and the handler in cpp/collo_cbc.cpp used to drop every event from a model
// with a parent. So after the restart we went deaf: no bound, no node counts, no
// incumbents. Everything that enforces a limit runs inside the progress callback
// — the global and after-incumbent deadlines in ilp, the distance cutoff in
// strategies — so a solve we hear nothing from runs unbounded. That is the bug
// this file pins.
//
// The model is the real one that exhibited it: epoch 8 of a group-list generation
// run, dumped with `COLLO_CBC_DUMP_MODEL` (1107 columns x 971 rows, 57 KB of pure
// numbers — a dump carries no names by construction). No synthetic instance is
// known to reach `Cbc0044I`: the restart needs a large objective with a *tiny*
// relative gap, not merely a big tree — a market-split instance searching 1.5M
// nodes never restarts. Background and measurements:
// docs/todos/todo_subtree_incumbent_reconstruction.md.

use std::path::PathBuf;
use std::time::Instant;

use collo_cbc::{IncumbentEvent, Model, ProblemDesc, Status};

/// Wall-clock backstop, handed to CBC as its own `seconds` limit. It exists so a
/// build that delivers no search-model events still *ends* — otherwise the
/// callback would never see its stop condition and the test would hang instead of
/// failing. A healthy run stops itself after a couple of seconds and never gets
/// near this.
const BACKSTOP_SECONDS: &str = "30";

fn fixture() -> ProblemDesc {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/restarted_search.collomodel");
    ProblemDesc::read_from(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// What the callback saw, in the order it mattered.
#[derive(Default)]
struct Seen {
    events: usize,
    /// An incumbent CBC reported that we could not map back to original space.
    /// Only the restarted search produces these, so this is also our marker for
    /// "the restart has happened".
    failed_incumbent: bool,
    /// An event carrying a node count that arrived *after* that marker.
    node_progress_after_restart: bool,
    /// The largest node count any event reported, for the failure messages.
    max_nodes: i32,
    /// The objective of the last incumbent we did map back.
    last_objective: Option<f64>,
}

#[test]
fn a_restarted_search_still_reports_progress() {
    let desc = fixture();

    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_parameter("timeMode", "elapsed");
    model.set_parameter("seconds", BACKSTOP_SECONDS);

    let started = Instant::now();
    let mut seen = Seen::default();

    let result = model.solve_with_callback(|progress| {
        seen.events += 1;
        seen.max_nodes = seen.max_nodes.max(progress.node_count);
        // Read the flag *before* this event updates it, so the event that
        // carries the first failed incumbent does not count as progress after
        // it: we want to see the search go on reporting, not just announce
        // itself once.
        if seen.failed_incumbent && progress.node_count > 0 {
            seen.node_progress_after_restart = true;
        }
        match &progress.incumbent {
            IncumbentEvent::Reconstructed { objective, .. } => {
                seen.last_objective = Some(*objective)
            }
            IncumbentEvent::ReconstructionFailed => seen.failed_incumbent = true,
            IncumbentEvent::None => {}
        }
        // Stop as soon as both properties have been observed, rather than at a
        // fixed time: the search never terminates on its own here.
        !(seen.failed_incumbent && seen.node_progress_after_restart)
    });
    let elapsed = started.elapsed();

    // Incumbents found after the restart live in the restarted model's own
    // column space, which `cbcPreProcessPointer` does not describe — it is
    // published for the top-level preprocessing and is not republished for the
    // restart. So they must come back `ReconstructionFailed`, never
    // `Reconstructed`: reconstructing them anyway produced objectives off by
    // ~120000 while claiming success, and a confidently wrong incumbent is worse
    // than a missing one (the distance cutoff would measure the bound against
    // garbage and never fire).
    //
    // This also doubles as the assertion that the restart happened at all, in
    // terms the Rust side can observe — it never sees `CbcModel` pointers. A
    // reconstruction can only fail on a model whose shape differs from the
    // published one, i.e. only after a restart. If a future CBC stops
    // restarting, this goes red and tells us the premise changed, rather than
    // passing for the wrong reason. A bare "some event reported a node count"
    // would not: the top-level model reports node counts before the restart, so
    // that alone was already true with the events dropped.
    assert!(
        seen.failed_incumbent,
        "no incumbent came back ReconstructionFailed, so the search never \
         restarted (saw {} events, up to node {}, in {elapsed:?}); if CBC no \
         longer emits Cbc0044I on this model, this test needs a new fixture",
        seen.events, seen.max_nodes
    );

    // The restarted search runs for tens of thousands of nodes. If we hear
    // nothing from it, we are deaf to the tree for the rest of the solve —
    // which is exactly what lets an epoch run past its deadlines.
    assert!(
        seen.node_progress_after_restart,
        "no progress event carried a node count after the restart (saw {} \
         events, up to node {}, in {elapsed:?})",
        seen.events, seen.max_nodes
    );

    // Stopping from the callback must hand back a real solution, not the state
    // of whatever nested model CBC happened to be in. `bestSolution()` is read
    // after `CbcMain1` returns and postprocessed into original space, so it is
    // CBC's best at stop time: at least as good as the last incumbent we were
    // told about.
    assert_eq!(result.status, Status::Stopped);
    let solution = result.solution.as_ref().expect("a solution");
    assert_eq!(
        solution.len(),
        desc.num_cols as usize,
        "solution must be in original column space"
    );

    let last = seen
        .last_objective
        .expect("the root incumbent should have been reported");
    let objective: f64 = desc
        .obj_coeffs
        .iter()
        .zip(solution)
        .map(|(c, x)| c * x)
        .sum();
    // obj_sense 1: minimize, so "no worse" means "no larger".
    assert_eq!(desc.obj_sense, 1);
    assert!(
        objective <= last + 1e-6,
        "the returned solution ({objective}) is worse than the last incumbent \
         the callback saw ({last})"
    );
    assert!(
        (objective - result.obj_value).abs() < 1e-6,
        "reported objective {} does not match the returned solution ({objective})",
        result.obj_value
    );
}
