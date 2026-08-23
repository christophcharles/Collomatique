use super::*;

use crate::StopReason;

/// One outcome, built from the four things [verdict] looks at
///
/// The solution is a `ConfigData` and never read — only its presence is — so
/// an empty one stands for « the engine came back with a colloscope ». The
/// variable type is `u32` because nothing here ever names a variable: any
/// [UsableData] would do.
fn outcome(
    status: SolveStatus,
    objective: Option<f64>,
    best_bound: Option<f64>,
    has_solution: bool,
) -> StrategyOutcome<u32> {
    StrategyOutcome {
        status,
        objective,
        best_bound,
        solution: has_solution.then(Default::default),
    }
}

/// A run that broke down says so, whatever it was holding
///
/// The one arm that ignores the solution: `Error` is a verdict precisely so
/// that a best-so-far colloscope survives the report, but it does not become
/// `Feasible` on the strength of holding one.
#[test]
fn an_error_stays_an_error_even_with_a_solution() {
    let outcome = outcome(SolveStatus::Error, Some(12.0), Some(12.0), true);

    assert_eq!(verdict(&outcome), SolveVerdict::Error);
}

/// « No colloscope exists » is nothing in hand
#[test]
fn an_infeasible_problem_has_no_solution() {
    let outcome = outcome(SolveStatus::Infeasible, None, None, false);

    assert_eq!(verdict(&outcome), SolveVerdict::NoSolution);
}

/// A run cut short with nothing in hand has nothing in hand
///
/// The conductor's only way of coming back empty, and what the solve dialog
/// writes « Pas de solution ! » about.
#[test]
fn a_solution_less_stop_has_no_solution() {
    let outcome = outcome(
        SolveStatus::Stopped(StopReason::Callback),
        None,
        None,
        false,
    );

    assert_eq!(verdict(&outcome), SolveVerdict::NoSolution);
}

/// An incumbent with the gap still open is `Feasible`, not `Optimal`
///
/// The whole point of the split: the engine reports `Optimal` here, meaning
/// only that it holds a colloscope. A bound below the objective is a solve
/// that had not finished proving anything.
#[test]
fn an_open_gap_is_only_feasible() {
    let outcome = outcome(SolveStatus::Optimal, Some(120.0), Some(98.0), true);

    assert_eq!(verdict(&outcome), SolveVerdict::Feasible);
}

/// No bound is no proof either
///
/// The ordinary shape of a run whose bound never improved: there is a
/// colloscope, and nothing at all is known about how good it is.
#[test]
fn a_missing_bound_is_only_feasible() {
    let outcome = outcome(SolveStatus::Optimal, Some(120.0), None, true);

    assert_eq!(verdict(&outcome), SolveVerdict::Feasible);
}

/// A closed gap is the proof `Optimal` promises
///
/// Closed to within [OPTIMALITY_GAP_EPS], the rule the application has always
/// applied before it writes « Solution optimale trouvée ! ».
#[test]
fn a_closed_gap_is_a_proof() {
    let outcome = outcome(
        SolveStatus::Optimal,
        Some(120.0),
        Some(120.0 - OPTIMALITY_GAP_EPS / 2.0),
        true,
    );

    assert_eq!(verdict(&outcome), SolveVerdict::Optimal);
}

/// A run cut short *with* a colloscope is `Feasible`
///
/// The reason `NoSolution` is about emptiness and not about being interrupted:
/// a colloscope in hand is a colloscope in hand, and how the run ended changes
/// nothing anyone can do with it.
#[test]
fn a_stop_with_a_solution_is_feasible() {
    let outcome = outcome(
        SolveStatus::Stopped(StopReason::TimeLimit),
        Some(120.0),
        Some(98.0),
        true,
    );

    assert_eq!(verdict(&outcome), SolveVerdict::Feasible);
}
