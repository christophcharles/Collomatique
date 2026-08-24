// Incumbents found after CBC restarts its tree search must come back as real
// solutions in the original column space.
//
// On some models CBC abandons the model it started branching on and restarts on
// a smaller one, logging:
//
//     Cbc0044I Reduced cost fixing - 758 rows, 398 columns - restarting search
//
// Reduced-cost fixing proves that many columns cannot take their other value in
// any improving solution, fixes them, and restarts branch and bound on the
// resulting smaller model. It needs a large objective with a *tiny* relative gap
// — that is the regime where reduced costs dominate — so "a big tree" does not
// reproduce it: a synthetic market-split instance searching 1.5M nodes never
// restarted. The model here is the real one that exhibited it: epoch 8 of a
// group-list generation run, dumped with `COLLO_CBC_DUMP_MODEL` (1107 columns x
// 971 rows, 57 KB of pure numbers — a dump carries no names by construction).
//
// The restarted searcher is a `CbcModel` whose `parentModel()` is the top-level
// one. Two bugs lived here, and this file pins both:
//
//   1. The handler in cpp/collo_cbc.cpp used to drop every event from a model
//      with a parent, so after the restart we went deaf: no bound, no node
//      counts, no incumbents. Everything that enforces a limit runs inside the
//      progress callback — the global and after-incumbent deadlines in ilp, the
//      distance cutoff in strategies — so a solve we hear nothing from runs
//      unbounded.
//   2. Once the events flowed again, their incumbents were reconstructed through
//      `cbcPreProcessPointer`, which describes the *top-level* preprocessing and
//      is not republished for the restart. That read a 398-column vector into a
//      map expecting 1056 columns and still reported success: after a root
//      incumbent of -115984.91 came objectives of +15015, +12014, -3985, while
//      CBC's actual incumbents were -115984.93, -115984.96, -115985.07. A
//      confidently wrong incumbent is worse than a missing one, so those were
//      made to fail honestly first, and are now expanded up the parent chain
//      properly (see `expand_to_parent` in cpp/collo_cbc.cpp).
//
// Hence the assertions below: every incumbent must reconstruct, and each one
// must be a genuinely feasible original-space vector whose objective matches the
// one reported. The deafness pin survives implicitly — if parent-model events
// were dropped again, no post-restart incumbent would ever arrive and the test
// would hang until the backstop and then fail.

use std::path::PathBuf;
use std::time::Instant;

use collo_cbc::{IncumbentEvent, Model, ProblemDesc, Status};

/// Wall-clock backstop, handed to CBC as its own `seconds` limit. It exists so a
/// build that delivers no search-model events still *ends* — otherwise the
/// callback would never see its stop condition and the test would hang instead of
/// failing. A healthy run stops itself after a couple of seconds and never gets
/// near this.
const BACKSTOP_SECONDS: &str = "30";

/// Slack for every numeric comparison: integrality, row activities, objectives.
const TOL: f64 = 1e-6;

fn fixture() -> ProblemDesc {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/restarted_search.collomodel");
    ProblemDesc::read_from(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// What the callback saw, in the order it mattered.
#[derive(Default)]
struct Seen {
    events: usize,
    /// The largest node count any event reported, for the failure messages.
    max_nodes: i32,
    /// An incumbent CBC reported that we could not map back to original space.
    /// Must never happen on this model anymore.
    failed_incumbent: bool,
    /// `(node_count, reported objective, solution)` per reconstructed incumbent,
    /// in report order.
    incumbents: Vec<(i32, f64, Vec<f64>)>,
}

/// Row activities of `solution` under the problem's constraint matrix, which is
/// stored column-major: `mat_start` is indexed by column, `mat_index` holds row
/// indices.
fn row_activities(desc: &ProblemDesc, solution: &[f64]) -> Vec<f64> {
    let mut activities = vec![0.0; desc.num_rows as usize];
    for col in 0..desc.num_cols as usize {
        let x = solution[col];
        if x == 0.0 {
            continue;
        }
        for k in desc.mat_start[col] as usize..desc.mat_start[col + 1] as usize {
            activities[desc.mat_index[k] as usize] += desc.mat_value[k] * x;
        }
    }
    activities
}

#[test]
fn a_restarted_search_reports_reconstructed_incumbents() {
    let desc = fixture();
    // obj_sense 1: minimize. Several assertions below depend on the direction.
    assert_eq!(desc.obj_sense, 1);

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
        match &progress.incumbent {
            IncumbentEvent::Reconstructed {
                objective,
                solution,
            } => seen
                .incumbents
                .push((progress.node_count, *objective, solution.clone())),
            IncumbentEvent::ReconstructionFailed => seen.failed_incumbent = true,
            IncumbentEvent::None => {}
        }
        // Stop as soon as the question is settled either way — on the first
        // failed incumbent, or on the first incumbent from the restarted search
        // — rather than at a fixed time: the search never terminates on its own
        // here.
        !(seen.failed_incumbent || seen.incumbents.iter().any(|(nodes, _, _)| *nodes > 0))
    });
    let elapsed = started.elapsed();

    assert!(
        !seen.failed_incumbent,
        "an incumbent came back ReconstructionFailed (saw {} events, up to node \
         {}, in {elapsed:?})",
        seen.events, seen.max_nodes
    );

    // On this model the top-level search only ever reports the root incumbent, at
    // node 0; everything at node > 0 comes from the restarted search. So this
    // doubles as the assertion that the restart happened at all, in terms the
    // Rust side can observe — it never sees `CbcModel` pointers. If a future CBC
    // stops restarting on this model, this goes red and tells us the premise
    // changed, rather than passing for the wrong reason.
    assert!(
        seen.incumbents.iter().any(|(nodes, _, _)| *nodes > 0),
        "no incumbent arrived after node 0, so the search never restarted (saw \
         {} events, {} incumbents, up to node {}, in {elapsed:?}); if CBC no \
         longer emits Cbc0044I on this model, this test needs a new fixture",
        seen.events,
        seen.incumbents.len(),
        seen.max_nodes
    );

    // Each incumbent must be a genuine solution of the *original* problem. The
    // feasibility check is the one that catches a subtle expansion bug: wrong
    // values for the columns the restarted model dropped would still produce a
    // plausible-looking objective, but not a feasible vector.
    for (index, (nodes, objective, solution)) in seen.incumbents.iter().enumerate() {
        let at = format!("incumbent {index} (node {nodes})");

        assert_eq!(
            solution.len(),
            desc.num_cols as usize,
            "{at}: solution must be in original column space"
        );

        for (col, x) in solution.iter().enumerate() {
            if desc.is_integer[col] != 0 {
                assert!(
                    (x - x.round()).abs() < TOL,
                    "{at}: integer column {col} holds {x}"
                );
            }
        }

        for (row, activity) in row_activities(&desc, solution).iter().enumerate() {
            assert!(
                *activity >= desc.row_lb[row] - TOL && *activity <= desc.row_ub[row] + TOL,
                "{at}: row {row} activity {activity} outside [{}, {}]",
                desc.row_lb[row],
                desc.row_ub[row]
            );
        }

        let recomputed: f64 = desc
            .obj_coeffs
            .iter()
            .zip(solution)
            .map(|(c, x)| c * x)
            .sum();
        assert!(
            (recomputed - objective).abs() < TOL,
            "{at}: reported objective {objective} does not match the solution \
             ({recomputed})"
        );
    }

    // CBC only ever announces an improving incumbent, so the reported objectives
    // must fall. The old bug's sequence (-115984.91, +15015, +12014, -3985)
    // breaks this at the first post-restart incumbent.
    for pair in seen.incumbents.windows(2) {
        let (_, previous, _) = &pair[0];
        let (nodes, objective, _) = &pair[1];
        assert!(
            objective <= &(previous + TOL),
            "incumbent at node {nodes} ({objective}) is worse than the one before \
             it ({previous})"
        );
    }

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

    let (_, last, _) = seen
        .incumbents
        .last()
        .expect("the root incumbent should have been reported");
    let objective: f64 = desc
        .obj_coeffs
        .iter()
        .zip(solution)
        .map(|(c, x)| c * x)
        .sum();
    assert!(
        objective <= last + TOL,
        "the returned solution ({objective}) is worse than the last incumbent \
         the callback saw ({last})"
    );
    assert!(
        (objective - result.obj_value).abs() < TOL,
        "reported objective {} does not match the returned solution ({objective})",
        result.obj_value
    );
}
