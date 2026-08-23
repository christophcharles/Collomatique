//! Running a solve, as a script drives it
//!
//! §13 of `docs/python/new_api_design.md` is the design. What lives here is
//! everything about *running* a solve rather than about the document a solve
//! recomputes: the two presets the strategy's classmethods answer, the
//! warnings a strategy is looked over for before it is handed to anything, and
//! the run itself — the handle on a live engine, what it reports while it
//! works, and what it produced when it is over.
//!
//! The strategy value itself is in `data.rs`, with the other value classes —
//! it is written in python, like all of them, and this module only says what
//! the application's own two look like. The door that starts a run is
//! `ColloscopeModel::solve`, in `model.rs`, because a solve is a thing done to
//! a model.
//!
//! A solve does not run here. The engine is its own process, spawned by
//! `subprocesses`, and everything in [SolveRun] is about talking to it from the
//! outside: the interpreter is released for every wait, and taken back for the
//! length of one callback at a time.

use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use collomatique_constraints_colloscopes::{
    ConfiguredColloscopeModel, ConfiguredExtra, InternalVar, Var, convert,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_strategies::{
    ConductorPayload, ConductorProgress, ConductorStrategy as RawConductorStrategy,
    ConductorWarning as RawConductorWarning, IncrementalPayload, SolveVerdict, StrategyOutcome,
};
use collomatique_subprocesses::{EngineExe, StrategySubprocess};

use crate::errors::SolveError;

/// The « Recherche simple » preset, as the application builds it
///
/// The door `ConductorStrategy.search()` goes through, private to the module
/// because a script has the classmethod. A preset written out in `data.py`
/// would be a copy of the application's, and a copy drifts; this hands back
/// the very structure the application's dialog opens with, converted.
#[pyfunction]
fn _conductor_search(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    crate::data::ConductorStrategy::to_py(py, &RawConductorStrategy::default())
}

/// The « Optimisation complète » preset, sized to this machine
///
/// The same door, for `ConductorStrategy.optimize()`. This one could not be
/// written out in python at all: its worker count is read from the cores the
/// machine reports, so the number is not known until it is asked for.
#[pyfunction]
fn _conductor_optimize(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    crate::data::ConductorStrategy::to_py(py, &RawConductorStrategy::with_parallelism_defaults())
}

/// A misconfiguration the conductor can see before running
///
/// The eight of them are class attributes — `clm.ConductorWarning.NO_SEED` —
/// the way [crate::values::Weekday]'s days are, and for the same reason: every
/// variant is payload-less, so there is nothing for a subclass per kind to
/// carry. `str()` is the French sentence the application's own solve dialog
/// shows, out of `ui-text`; the identifiers stay English, like every other
/// name a script writes.
///
/// A warning is a remark, never a refusal. A strategy that warns still runs —
/// `.warnings()` is there so a script may print what it would have shown the
/// user, and decide for itself.
// Handed out only, never taken back in — nothing here reads a warning out of a
// script — so the extraction the days opt into is skipped rather than declared
// and left unused.
#[pyclass(module = "collomatique", frozen, eq, hash, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConductorWarning {
    #[pyo3(name = "NO_STRATEGY_ENABLED")]
    NoStrategyEnabled,
    #[pyo3(name = "NO_OPTIMIZING")]
    NoOptimizing,
    #[pyo3(name = "NO_SEED")]
    NoSeed,
    #[pyo3(name = "STARVED_FUZZY")]
    StarvedFuzzy,
    #[pyo3(name = "WONT_FINISH")]
    WontFinish,
    #[pyo3(name = "COLD_FUZZY")]
    ColdFuzzy,
    #[pyo3(name = "REDUNDANT_WARM_START")]
    RedundantWarmStart,
    #[pyo3(name = "OVERWHELMED_CPU")]
    OverwhelmedCpu,
}

#[pymethods]
impl ConductorWarning {
    /// The French sentence the application shows for this warning
    ///
    /// The dialog's own words, not a second set written here: a script that
    /// prints a warning prints what the user would have read.
    fn __str__(&self) -> &'static str {
        collomatique_ui_text::solver::conductor_warning_text(self.to_model())
    }
}

impl ConductorWarning {
    /// The python warning for one model warning
    ///
    /// A match, like [crate::values::Weekday]'s conversions, so that a ninth
    /// warning over in `strategies` is a compile error here rather than a
    /// warning python silently never hears about.
    fn from_model(warning: RawConductorWarning) -> ConductorWarning {
        match warning {
            RawConductorWarning::NoStrategyEnabled => ConductorWarning::NoStrategyEnabled,
            RawConductorWarning::NoOptimizing => ConductorWarning::NoOptimizing,
            RawConductorWarning::NoSeed => ConductorWarning::NoSeed,
            RawConductorWarning::StarvedFuzzy => ConductorWarning::StarvedFuzzy,
            RawConductorWarning::WontFinish => ConductorWarning::WontFinish,
            RawConductorWarning::ColdFuzzy => ConductorWarning::ColdFuzzy,
            RawConductorWarning::RedundantWarmStart => ConductorWarning::RedundantWarmStart,
            RawConductorWarning::OverwhelmedCpu => ConductorWarning::OverwhelmedCpu,
        }
    }

    /// The model warning for one python warning
    ///
    /// The reverse of [ConductorWarning::from_model], and what `__str__` asks
    /// `ui-text` about — so the sentence is looked up by the model's own
    /// variant and no table is copied over here.
    fn to_model(self) -> RawConductorWarning {
        match self {
            ConductorWarning::NoStrategyEnabled => RawConductorWarning::NoStrategyEnabled,
            ConductorWarning::NoOptimizing => RawConductorWarning::NoOptimizing,
            ConductorWarning::NoSeed => RawConductorWarning::NoSeed,
            ConductorWarning::StarvedFuzzy => RawConductorWarning::StarvedFuzzy,
            ConductorWarning::WontFinish => RawConductorWarning::WontFinish,
            ConductorWarning::ColdFuzzy => RawConductorWarning::ColdFuzzy,
            ConductorWarning::RedundantWarmStart => RawConductorWarning::RedundantWarmStart,
            ConductorWarning::OverwhelmedCpu => RawConductorWarning::OverwhelmedCpu,
        }
    }
}

/// The preflight warnings of one strategy, in the variants' declaration order
///
/// The door `ConductorStrategy.warnings()` goes through. The strategy is
/// extracted first, so a malformed one is refused here with the very sentence
/// `solve` would refuse it with: asking what is wrong with a strategy that
/// cannot be read at all would answer about the wrong thing.
///
/// The order is the model's, which sorts by declaration — the same order the
/// dialog lists them in.
#[pyfunction]
fn _conductor_warnings<'py>(
    py: Python<'py>,
    strategy: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyTuple>> {
    let strategy = crate::data::ConductorStrategy::from_py(strategy)?;

    PyTuple::new(
        py,
        strategy
            .warnings()
            .into_iter()
            .map(ConductorWarning::from_model),
    )
}

/// A variable of the configured model, base or added by the configuration
///
/// The space a solution comes back in: `SolveConfig::build_model` extends the
/// colloscope's own variables with the penalty variables its objective needed,
/// so what the engine solved is wider than what a colloscope is read from.
type ConfiguredVar = InternalVar<Var, ConfiguredExtra>;

/// One finished solve, as the strategies crate reports it
type Outcome = StrategyOutcome<ConfiguredVar>;

/// What the run has reported so far, in the two numbers a script can act on
#[derive(Clone, Copy)]
struct Snapshot {
    /// The best incumbent's cost, or `None` while there is none
    objective: Option<f64>,
    /// The best proven bound on any colloscope's cost, or `None`
    bound: Option<f64>,
}

/// How a solve ended
///
/// The four the application's own solve dialog distinguishes, mirrored from
/// [SolveVerdict] so that a script and a user are told the same thing about the
/// same run. This is deliberately *not* what the engine reports: the engine
/// calls a run « optimal » as soon as it holds any colloscope at all, so a
/// script handed the raw word would read a promise nobody made.
///
/// `OPTIMAL` is always a *proof*: the solver closed the gap between the
/// colloscope it found and the best any colloscope could be. `FEASIBLE` is a
/// colloscope in hand with that question still open — which is what a run cut
/// short almost always gives.
///
/// `NO_SOLUTION` is about emptiness and not about how the run ended: a run
/// stopped before it found anything and a problem no colloscope satisfies are
/// the same answer to a script, and the engine cannot tell them apart either.
///
/// `ERROR` is a status and not an exception, because a run that broke down may
/// still carry the best colloscope it had found by then: raising would throw it
/// away.
// Handed out only, never taken back in — a script compares a status, it never
// passes one in — so the extraction is skipped, like [ConductorWarning]'s.
#[pyclass(module = "collomatique", frozen, eq, hash, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolveStatus {
    /// A colloscope, and the proof that none is better
    #[pyo3(name = "OPTIMAL")]
    Optimal,
    /// A colloscope, with the question of a better one left open
    #[pyo3(name = "FEASIBLE")]
    Feasible,
    /// Nothing in hand: stopped before finding one, or none exists
    #[pyo3(name = "NO_SOLUTION")]
    NoSolution,
    /// The run broke down
    #[pyo3(name = "ERROR")]
    Error,
}

#[pymethods]
impl SolveStatus {
    /// The French sentence the application shows for this status
    ///
    /// The solve dialog's own words, not a second set written here: a script
    /// that prints a status prints what the user would have read.
    fn __str__(&self) -> &'static str {
        collomatique_ui_text::solver::solve_verdict_text(self.to_model())
    }
}

impl SolveStatus {
    /// The name python knows this status by, for the reprs that spell one out
    ///
    /// A match rather than a formatting of the rust variant, so the two names
    /// cannot drift apart: they are written next to each other above.
    fn name(self) -> &'static str {
        match self {
            SolveStatus::Optimal => "OPTIMAL",
            SolveStatus::Feasible => "FEASIBLE",
            SolveStatus::NoSolution => "NO_SOLUTION",
            SolveStatus::Error => "ERROR",
        }
    }

    /// The python status for one verdict
    ///
    /// A match, like [ConductorWarning::from_model], so that a fifth verdict
    /// over in `strategies` is a compile error here rather than an outcome
    /// python silently cannot describe.
    fn from_model(verdict: SolveVerdict) -> SolveStatus {
        match verdict {
            SolveVerdict::Optimal => SolveStatus::Optimal,
            SolveVerdict::Feasible => SolveStatus::Feasible,
            SolveVerdict::NoSolution => SolveStatus::NoSolution,
            SolveVerdict::Error => SolveStatus::Error,
        }
    }

    /// The verdict for one python status, for the sentence `ui-text` keys on it
    fn to_model(self) -> SolveVerdict {
        match self {
            SolveStatus::Optimal => SolveVerdict::Optimal,
            SolveStatus::Feasible => SolveVerdict::Feasible,
            SolveStatus::NoSolution => SolveVerdict::NoSolution,
            SolveStatus::Error => SolveVerdict::Error,
        }
    }
}

/// One optional number, spelled the way python spells it
///
/// `{:?}` and not `{}`, so a whole cost reads `123.0` and not `123` — python's
/// own repr of a float, which is what these reprs are imitating.
fn number(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:?}"),
        None => "None".to_owned(),
    }
}

/// The best the run has found and proven so far
///
/// What `run.progress()` answers, and what `on_progress` is called with. Two
/// numbers and nothing else: the objective of the best colloscope found, and
/// the bound below which no colloscope can go. They meet when the solve is
/// over and proven.
#[pyclass(module = "collomatique", frozen)]
pub struct SolveProgress {
    /// The best colloscope's cost so far, or `None` while there is none
    #[pyo3(get)]
    objective: Option<f64>,
    /// The best proven bound on any colloscope's cost, or `None`
    #[pyo3(get)]
    bound: Option<f64>,
}

#[pymethods]
impl SolveProgress {
    fn __repr__(&self) -> String {
        format!(
            "SolveProgress(objective={}, bound={})",
            number(self.objective),
            number(self.bound)
        )
    }
}

/// What one finished solve produced
///
/// ```python
/// outcome = run.wait()
/// if outcome.colloscope is not None:
///     doc.colloscope.install(outcome.colloscope)
/// ```
///
/// The status, the two numbers, and the colloscope itself when there is one.
/// The colloscope is a `ColloscopeData` — a detached value like every other,
/// so it is `install`ed rather than written to, and editing it changes nothing
/// on its own.
#[pyclass(module = "collomatique", frozen)]
pub struct SolveOutcome {
    /// How the solve ended
    status: SolveStatus,
    /// The cost of the colloscope below, or `None` without one
    objective: Option<f64>,
    /// The best proven bound on any colloscope's cost, or `None`
    bound: Option<f64>,
    /// The solved colloscope as a `ColloscopeData`, or `None` without a
    /// solution
    ///
    /// Built once, when the outcome was, and handed out by reference
    /// afterwards: a value the script mutates is its own business, and the
    /// next `.colloscope` answers the same object it did before.
    colloscope: Option<Py<PyAny>>,
}

#[pymethods]
impl SolveOutcome {
    /// How the solve ended — a `SolveStatus`
    #[getter]
    fn status(&self) -> SolveStatus {
        self.status
    }

    /// The cost of the colloscope found, or `None` without one
    #[getter]
    fn objective(&self) -> Option<f64> {
        self.objective
    }

    /// The best proven bound on any colloscope's cost, or `None`
    ///
    /// A bound equal to the objective is the proof behind `OPTIMAL`.
    #[getter]
    fn bound(&self) -> Option<f64> {
        self.bound
    }

    /// The solved colloscope as a `ColloscopeData`, or `None`
    #[getter]
    fn colloscope(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.colloscope
            .as_ref()
            .map(|colloscope| colloscope.clone_ref(py))
    }

    fn __repr__(&self) -> String {
        format!(
            "<SolveOutcome: SolveStatus.{}, objective={}, bound={}>",
            self.status.name(),
            number(self.objective),
            number(self.bound)
        )
    }
}

/// One running (or finished) solve
///
/// ```python
/// run = model.solve(collomatique.ConductorStrategy.optimize())
/// outcome = run.wait()
/// ```
///
/// `model.solve(...)` is the only way to get one. The engine runs in its own
/// process, so this object is a handle on something really happening
/// elsewhere: `progress()` asks what it has found, `stop()` asks it to settle
/// for the best so far, `kill()` ends it outright, and `wait()` blocks until it
/// is over and hands back the [SolveOutcome].
///
/// **Dropping the run kills the engine.** The subprocess belongs to this
/// object, so a script that means the solve to keep going holds on to it —
/// `model.solve(...)` on a line of its own starts an engine and kills it in the
/// same breath.
///
/// `wait()` must not be called from inside `on_progress` or `on_log`. The
/// callback runs on the thread that also answers the engine's progress
/// round-trips, so waiting there waits for a message that thread is the one
/// meant to deliver.
///
/// A callback that raises stops being called, and its exception comes out of
/// `wait()` in place of an outcome — the rule `build_colloscope_model` already
/// follows. The solve itself is left running: stopping it is the script's call,
/// through `stop()` or `kill()`.
#[pyclass(module = "collomatique", frozen)]
pub struct SolveRun {
    /// The live subprocess. `kill()` takes it out and drops it; every other
    /// door leaves it in place, and `None` is how they all recognise a run
    /// that was killed.
    subprocess: Mutex<Option<StrategySubprocess>>,
    /// Where the engine's one result arrives
    ///
    /// Read only by `wait()`, one 100ms slice at a time and never with the
    /// interpreter held — so a second thread's `wait()` polls the same channel
    /// rather than queueing behind the first.
    receiver: Mutex<mpsc::Receiver<Outcome>>,
    /// The raw outcome, off the channel and waiting to be built
    ///
    /// The step between the two: whoever takes the result out of the channel
    /// does so without the interpreter, and parks it here until a thread that
    /// holds the interpreter can turn it into a [SolveOutcome]. That is also
    /// what makes two concurrent `wait()`s agree — the second finds either
    /// this slot or [SolveRun::finished] filled, never both empty.
    arrived: Mutex<Option<Outcome>>,
    /// The outcome `wait()` built, so a second wait answers the first —
    /// the same object, not a rebuild.
    finished: Mutex<Option<Py<SolveOutcome>>>,
    /// The best objective and bound reported so far, mirrored by the
    /// progress callback; what `progress()` answers without waiting.
    progress: Arc<Mutex<Option<Snapshot>>>,
    /// The first exception a callback raised. Set once; both callbacks go
    /// quiet afterwards, and `wait()` re-raises it instead of an outcome.
    failure: Arc<Mutex<Option<PyErr>>>,
    /// The model's parameters, to read the solution back as a colloscope
    ///
    /// The model's own copy, handed over at `start`: the run outlives the call
    /// that made it, and a solution means nothing without the periods, weeks,
    /// slots and students it was written against.
    params: Parameters,
}

/// What one slice of waiting came back with
///
/// The three answers of a `recv_timeout`, named so `wait()`'s loop reads as
/// what it is: the outcome itself never travels through here, because taking
/// it out of the channel and building it are two different moments.
enum Waited {
    /// The result is off the channel and parked in [SolveRun::arrived]
    Arrived,
    /// Nothing yet — the engine is still working
    Nothing,
    /// Every sender is gone: the engine process is dead
    Gone,
}

impl SolveRun {
    /// Spawns the engine and hands back the handle on it
    ///
    /// Called by `ColloscopeModel::solve`, which is where the refusal order
    /// lives: the strategy is read first, then the engine is resolved, and only
    /// then is anything started.
    pub(crate) fn start(
        py: Python<'_>,
        model: &ConfiguredColloscopeModel,
        params: &Parameters,
        strategy: &RawConductorStrategy,
        engine: &EngineExe,
        on_progress: Option<Py<PyAny>>,
        on_log: Option<Py<PyAny>>,
    ) -> PyResult<SolveRun> {
        // The staggered order the application solves in: the group assignment
        // first, then one week at a time on top of it. Built from the model
        // rather than asked for, because it is a fact about this problem and
        // not a choice a script makes.
        let payload = ConductorPayload {
            incremental: IncrementalPayload {
                epochs: collomatique_constraints_colloscopes::build_incremental_epochs(model),
            },
        };

        let (sender, receiver) = mpsc::channel();
        let progress: Arc<Mutex<Option<Snapshot>>> = Arc::new(Mutex::new(None));
        let failure: Arc<Mutex<Option<PyErr>>> = Arc::new(Mutex::new(None));

        let result_callback = move |outcome: Outcome| {
            // The receiver may already be gone — the run was dropped in
            // mid-flight. There is then nobody to tell, and nothing to do
            // about it.
            let _ = sender.send(outcome);
        };

        let mirror = Arc::clone(&progress);
        let progress_failure = Arc::clone(&failure);
        let progress_callback = move |update: Result<ConductorProgress<ConfiguredVar>, String>| {
            // Only the conductor's own status is this API's progress. The
            // per-worker union and an undecodable line are the application
            // panel's vocabulary, and publishing them would make the strategy
            // kinds public API.
            let Ok(ConductorProgress::Conductor(status)) = update else {
                return;
            };
            let snapshot = Snapshot {
                objective: status
                    .best_solution
                    .as_ref()
                    .map(|solution| solution.objective),
                bound: status.best_bound,
            };
            *mirror.lock().unwrap() = Some(snapshot);

            let Some(callback) = on_progress.as_ref() else {
                return;
            };
            if progress_failure.lock().unwrap().is_some() {
                return;
            }

            // This is a worker thread with no interpreter of its own, so each
            // event takes it back for the length of one call and gives it up
            // again (`build_colloscope_model`'s own pattern).
            Python::attach(|py| {
                let called = Py::new(
                    py,
                    SolveProgress {
                        objective: snapshot.objective,
                        bound: snapshot.bound,
                    },
                )
                .and_then(|progress| callback.call1(py, (progress,)));

                if let Err(error) = called {
                    let mut slot = progress_failure.lock().unwrap();
                    if slot.is_none() {
                        *slot = Some(error);
                    }
                }
            });
        };

        let log_failure = Arc::clone(&failure);
        let log_callback = move |line: &str| {
            let Some(callback) = on_log.as_ref() else {
                return;
            };
            if log_failure.lock().unwrap().is_some() {
                return;
            }

            Python::attach(|py| {
                if let Err(error) = callback.call1(py, (line,)) {
                    let mut slot = log_failure.lock().unwrap();
                    if slot.is_none() {
                        *slot = Some(error);
                    }
                }
            });
        };

        // Released for the duration: `spawn` serializes the whole model and
        // starts a process, over a second of blocking work — the application
        // runs it on a worker pool for the same reason.
        //
        // No warm start, like the application's own solve: « start from what
        // the document holds » is the configuration's anchoring, and that is
        // already inside the model.
        let subprocess = py
            .detach(|| {
                StrategySubprocess::spawn(
                    engine,
                    model,
                    strategy,
                    None,
                    payload,
                    result_callback,
                    progress_callback,
                    log_callback,
                )
            })
            .map_err(|e| SolveError::new_err(format!("the engine could not be started: {e}")))?;

        Ok(SolveRun {
            subprocess: Mutex::new(Some(subprocess)),
            receiver: Mutex::new(receiver),
            arrived: Mutex::new(None),
            finished: Mutex::new(None),
            progress,
            failure,
            params: params.clone(),
        })
    }

    /// One slice of waiting, with the interpreter released
    ///
    /// Everything that blocks happens in here, and nothing that needs the
    /// interpreter does: the result is parked in [SolveRun::arrived] rather
    /// than built, so the thread that finally builds it is one that holds the
    /// interpreter already.
    fn wait_a_slice(&self, py: Python<'_>) -> Waited {
        py.detach(|| {
            let receiver = self.receiver.lock().unwrap();
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(outcome) => {
                    *self.arrived.lock().unwrap() = Some(outcome);
                    Waited::Arrived
                }
                Err(mpsc::RecvTimeoutError::Timeout) => Waited::Nothing,
                Err(mpsc::RecvTimeoutError::Disconnected) => Waited::Gone,
            }
        })
    }

    /// The solved colloscope, and the two numbers that describe it
    ///
    /// Runs on the python thread, because what it builds is a python value.
    fn build_outcome(&self, py: Python<'_>, outcome: Outcome) -> PyResult<SolveOutcome> {
        use crate::data::Value as _;

        let colloscope = match &outcome.solution {
            None => None,
            Some(solution) => {
                // The solved configuration is over the *configured* model's
                // variables; the conversion wants the base ones, so they are
                // stripped straight out rather than rebuilt through a full
                // `Solution` (the application's own shortcut).
                let base = solution.filter_transmute(|var| match var {
                    InternalVar::Base(base) => Some(base.clone()),
                    _ => None,
                });

                let colloscope =
                    convert::build_colloscope(&self.params, &base).ok_or_else(|| {
                        // `build_colloscope` only answers `None` for a
                        // malformed configuration, which is the engine's bug
                        // and not the script's: said out loud rather than
                        // shrugged into `colloscope=None`.
                        SolveError::new_err(
                            "the engine returned a solution that does not form a colloscope",
                        )
                    })?;

                let contents = collomatique_ops::ColloscopeContents::from(&colloscope);
                Some(crate::data::ColloscopeData::to_py(py, &contents)?.unbind())
            }
        };

        Ok(SolveOutcome {
            status: SolveStatus::from_model(collomatique_strategies::verdict(&outcome)),
            objective: outcome.objective,
            bound: outcome.best_bound,
            colloscope,
        })
    }
}

#[pymethods]
impl SolveRun {
    /// The best the run has found so far, or `None` before its first report
    ///
    /// ```python
    /// while (progress := run.progress()) is None or progress.objective is None:
    ///     time.sleep(1)
    /// ```
    ///
    /// Never blocks and never raises: it reads what the engine last said, and
    /// says it. A run that has finished, or been killed, still answers with the
    /// last thing it reported.
    fn progress(&self) -> Option<SolveProgress> {
        (*self.progress.lock().unwrap()).map(|snapshot| SolveProgress {
            objective: snapshot.objective,
            bound: snapshot.bound,
        })
    }

    /// Asks the engine to settle for the best colloscope it has
    ///
    /// Cooperative, and therefore not immediate: the engine notices at its next
    /// progress round-trip, finishes what it is doing and reports the best it
    /// found. `wait()` then collects that as an ordinary outcome — which is the
    /// whole difference from `kill()`.
    ///
    /// Stopping a run that has already finished on its own does nothing, which
    /// is the honest answer: the two race by design. Stopping a run that was
    /// killed raises `SolveError` — there is no longer an engine to ask.
    fn stop(&self) -> PyResult<()> {
        match self.subprocess.lock().unwrap().as_ref() {
            Some(subprocess) => {
                subprocess.stop();
                Ok(())
            }
            None => Err(SolveError::new_err(
                "this run was killed; there is nothing left to stop",
            )),
        }
    }

    /// Ends the engine outright, keeping nothing
    ///
    /// The process is killed where it stands, so whatever it had found is lost
    /// and `wait()` afterwards raises rather than inventing an outcome.
    /// `stop()` is the door that keeps the best so far.
    ///
    /// Safe to call twice, and safe on a run that already finished — a
    /// `finally:` that tidies up does not have to ask first.
    fn kill(&self) {
        // Taken out of the lock and dropped after it: the `Worker`'s own `Drop`
        // is what kills the child, and there is no reason to hold the mutex
        // while it does.
        let killed = self.subprocess.lock().unwrap().take();
        drop(killed);
    }

    /// Waits for the solve to end, and hands back what it produced
    ///
    /// ```python
    /// outcome = run.wait()
    /// if outcome.colloscope is not None:
    ///     doc.colloscope.install(outcome.colloscope)
    /// ```
    ///
    /// Blocks until the engine reports, Ctrl-C still interrupting the wait —
    /// which interrupts the *waiting* and not the solve, since the engine is
    /// its own process and keeps going.
    ///
    /// Waiting twice answers the very same `SolveOutcome` object, because a
    /// finished run has one outcome and not two. A run that was killed has
    /// none at all, and raises `SolveError`; so does an engine that exits
    /// without reporting. An exception raised by `on_progress` or `on_log`
    /// comes out here in place of the outcome, every time this is asked.
    fn wait(&self, py: Python<'_>) -> PyResult<Py<SolveOutcome>> {
        loop {
            // A second wait answers the first: the same object, not a rebuild.
            if let Some(outcome) = self.finished.lock().unwrap().as_ref() {
                return Ok(outcome.clone_ref(py));
            }

            // The callbacks' exception wins over the outcome, as it wins over
            // the model in `build_colloscope_model`: the script asked for the
            // lines and one of them was refused, so nothing is handed back.
            if let Some(error) = self.failure.lock().unwrap().as_ref() {
                return Err(error.clone_ref(py));
            }

            // Whoever holds the interpreter builds what the channel gave up.
            // Both halves of this happen without letting go of it, which is
            // what stops a concurrent `wait()` from seeing neither.
            if let Some(outcome) = self.arrived.lock().unwrap().take() {
                let outcome = Py::new(py, self.build_outcome(py, outcome)?)?;
                *self.finished.lock().unwrap() = Some(outcome.clone_ref(py));
                return Ok(outcome);
            }

            match self.wait_a_slice(py) {
                // Parked, and built by the top of the next turn.
                Waited::Arrived => {}
                // Woken every 100ms so Ctrl-C reaches the script.
                Waited::Nothing => py.check_signals()?,
                Waited::Gone => {
                    // A result that raced ahead of the death was parked by the
                    // `Ok` arm, here or in another thread's wait, so one more
                    // turn answers it. Reaching this twice means there really
                    // is nothing.
                    if self.arrived.lock().unwrap().is_some()
                        || self.finished.lock().unwrap().is_some()
                    {
                        continue;
                    }

                    return Err(if self.subprocess.lock().unwrap().is_none() {
                        SolveError::new_err(
                            "this run was killed, so it has no outcome; \
                             stop() is the door that keeps the best so far",
                        )
                    } else {
                        SolveError::new_err("the engine exited without an outcome")
                    });
                }
            }
        }
    }

    /// What the run looks like from outside
    ///
    /// Best-effort, and honest about it: a run that has finished but has not
    /// been waited on still says « running », because nothing has collected its
    /// outcome yet.
    fn __repr__(&self) -> &'static str {
        if self.finished.lock().unwrap().is_some() {
            "<SolveRun: finished>"
        } else if self.subprocess.lock().unwrap().is_none() {
            "<SolveRun: killed>"
        } else {
            "<SolveRun: running>"
        }
    }
}

/// Puts what a script drives a solve with in the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ConductorWarning>()?;
    // Registered so `isinstance` and `repr` say something useful, like the
    // model class is; `model.solve(...)` is the only way to get a run, and a
    // run is the only way to get the other three.
    m.add_class::<SolveOutcome>()?;
    m.add_class::<SolveProgress>()?;
    m.add_class::<SolveRun>()?;
    m.add_class::<SolveStatus>()?;

    m.add_function(wrap_pyfunction!(_conductor_search, m)?)?;
    m.add_function(wrap_pyfunction!(_conductor_optimize, m)?)?;
    m.add_function(wrap_pyfunction!(_conductor_warnings, m)?)?;

    Ok(())
}
