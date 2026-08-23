use super::*;

use collomatique_strategies::StopReason;

/// One outcome, built from the four things [status_of] looks at
///
/// The solution is a `ConfigData` and never read — only its presence is —
/// so an empty one stands for « the engine came back with a colloscope ».
/// Written as `Default::default()` rather than named, since naming it would
/// pull `collomatique-ilp` into this crate for one test line.
fn outcome(
    status: RawSolveStatus,
    objective: Option<f64>,
    best_bound: Option<f64>,
    has_solution: bool,
) -> Outcome {
    Outcome {
        status,
        objective,
        best_bound,
        solution: has_solution.then(Default::default),
    }
}

/// A run that broke down says so, whatever it was holding
///
/// The one arm that ignores the solution: `ERROR` is a status precisely so
/// that a best-so-far colloscope survives the report, but it does not become
/// `FEASIBLE` on the strength of holding one.
#[test]
fn an_error_stays_an_error_even_with_a_solution() {
    let outcome = outcome(RawSolveStatus::Error, Some(12.0), Some(12.0), true);

    assert_eq!(status_of(&outcome), SolveStatus::Error);
}

/// « No colloscope exists » is its own answer
#[test]
fn an_infeasible_problem_is_infeasible() {
    let outcome = outcome(RawSolveStatus::Infeasible, None, None, false);

    assert_eq!(status_of(&outcome), SolveStatus::Infeasible);
}

/// An incumbent with the gap still open is `FEASIBLE`, not `OPTIMAL`
///
/// The whole point of the split: the engine reports `Optimal` here, meaning
/// only that it holds a colloscope. A bound below the objective is a solve
/// that had not finished proving anything.
#[test]
fn an_open_gap_is_only_feasible() {
    let outcome = outcome(RawSolveStatus::Optimal, Some(120.0), Some(98.0), true);

    assert_eq!(status_of(&outcome), SolveStatus::Feasible);
}

/// No bound is no proof either
///
/// The ordinary shape of a run whose bound never improved: there is a
/// colloscope, and nothing at all is known about how good it is.
#[test]
fn a_missing_bound_is_only_feasible() {
    let outcome = outcome(RawSolveStatus::Optimal, Some(120.0), None, true);

    assert_eq!(status_of(&outcome), SolveStatus::Feasible);
}

/// A closed gap is the proof `OPTIMAL` promises
///
/// Closed to within `OPTIMALITY_GAP_EPS`, the application's own rule before it
/// writes « Solution optimale trouvée » — so the two say « optimal » about
/// exactly the same runs.
#[test]
fn a_closed_gap_is_a_proof() {
    let outcome = outcome(
        RawSolveStatus::Optimal,
        Some(120.0),
        Some(120.0 - OPTIMALITY_GAP_EPS / 2.0),
        true,
    );

    assert_eq!(status_of(&outcome), SolveStatus::Optimal);
}

/// A run cut short with nothing in hand is `STOPPED`
#[test]
fn a_stop_without_a_solution_is_stopped() {
    let outcome = outcome(
        RawSolveStatus::Stopped(StopReason::Callback),
        None,
        None,
        false,
    );

    assert_eq!(status_of(&outcome), SolveStatus::Stopped);
}

/// A run cut short *with* a colloscope is `FEASIBLE`
///
/// The reason `STOPPED` is about emptiness and not about being interrupted: to
/// a script, a colloscope in hand is a colloscope in hand, and how the run
/// ended changes nothing it can do with it.
#[test]
fn a_stop_with_a_solution_is_feasible() {
    let outcome = outcome(
        RawSolveStatus::Stopped(StopReason::TimeLimit),
        Some(120.0),
        Some(98.0),
        true,
    );

    assert_eq!(status_of(&outcome), SolveStatus::Feasible);
}
