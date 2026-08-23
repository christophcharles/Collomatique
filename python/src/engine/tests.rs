use super::*;

/// One test at a time, since the rung they are about is a process-wide static
///
/// They all install an engine and clear it again, so letting them overlap
/// would have each one reading the other's.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Runs one body with the injected rung set, and cleared again after
///
/// A panicking body poisons the lock; that is recovered from rather than
/// propagated, since the failure worth reporting is the assertion's and not
/// this guard's.
fn with_injected<T>(engine: EngineExe, body: impl FnOnce() -> T) -> T {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());

    set_engine(Some(engine));
    let answer = body();
    set_engine(None);

    answer
}

/// The `engine=` of the call outranks whatever the runner injected
///
/// The local thing wins: a script that names an engine for one solve means
/// that solve, whether or not it happens to be running inside collomatique.
#[test]
fn an_explicit_engine_beats_the_injected_one() {
    let named = PathBuf::from("/somewhere/collomatique");

    // The answer is read as an `Option` rather than unwrapped: printing a
    // `PyErr` needs an interpreter this test never starts, so an unwrap on the
    // day the rungs get their order wrong would abort instead of saying so.
    let answer = with_injected(EngineExe::Current, || resolve(Some(named.clone())).ok());

    assert_eq!(answer, Some(EngineExe::Explicit(named)));
}

/// With nothing named on the call, the injected engine is the answer
///
/// The rung the application itself rides: rpc-engine injects
/// [EngineExe::Current], so a hosted script's `solve()` needs no `engine=`.
///
/// The environment rung below it is deliberately left untested here — it is
/// the process's own, shared with every other test in this binary, and the
/// end-to-end tests own it one subprocess at a time instead.
#[test]
fn the_injected_engine_answers_when_the_call_names_none() {
    let answer = with_injected(EngineExe::Current, || resolve(None).ok());

    assert_eq!(answer, Some(EngineExe::Current));
}

/// Clearing the injected engine really clears it
///
/// The other half of what [set_engine] promises, and what makes the two tests
/// above independent of each other: the runner clears on the way out, so what
/// the first script ran on is not what the second one inherits.
#[test]
fn clearing_the_injected_engine_empties_the_rung() {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());

    set_engine(Some(EngineExe::Explicit(PathBuf::from(
        "/somewhere/collomatique",
    ))));
    set_engine(None);

    assert!(ENGINE.lock().unwrap().is_none());
}
