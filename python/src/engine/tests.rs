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

/// Runs one body with nothing above the baked rung naming an engine
///
/// The injected rung is emptied here. The environment rung cannot be — it is
/// the process's own, shared with every other test in this binary — so it is
/// checked instead: a machine that exports `COLLOMATIQUE_ENGINE` would make
/// the rung below it look broken, and saying which it is beats a bare
/// inequality.
fn with_only_the_baked_rung<T>(body: impl FnOnce() -> T) -> T {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());

    assert!(
        std::env::var_os("COLLOMATIQUE_ENGINE").is_none_or(|path| path.is_empty()),
        "this test reads the rung below COLLOMATIQUE_ENGINE: the test process \
         must not name an engine of its own"
    );

    set_engine(None);
    body()
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

/// What the runner injected outranks what the build baked in
///
/// The same most-local-wins order as the rungs above: a runner naming the
/// binary is talking about this one run, and a baked default is about every
/// run the build will ever serve.
#[test]
fn the_injected_engine_beats_the_baked_default() {
    let answer = with_injected(EngineExe::Current, || {
        resolve_with(None, Some("/built/against/collomatique")).ok()
    });

    assert_eq!(answer, Some(EngineExe::Current));
}

/// With call, runner and environment all silent, the build answers
///
/// The rung a standalone python library rides: the nix wheel derivation bakes
/// the store path of the collomatique it was built against, so an installed
/// module solves without anyone naming a binary.
#[test]
fn the_baked_default_answers_when_nothing_else_does() {
    let baked = "/built/against/collomatique";

    let answer = with_only_the_baked_rung(|| resolve_with(None, Some(baked)).ok());

    assert_eq!(answer, Some(EngineExe::Explicit(PathBuf::from(baked))));
}

/// A build that baked an empty name baked no engine
///
/// The rule the environment rung already follows one line above: the name is
/// there and says nothing, which is not the same as naming the empty path.
#[test]
fn an_empty_baked_default_is_not_an_engine() {
    let refused = with_only_the_baked_rung(|| resolve_with(None, Some("")).is_err());

    assert!(refused);
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
