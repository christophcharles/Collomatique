//! The built ILP model of a colloscope
//!
//! What `doc.build_colloscope_model(config)` hands back: the problem the
//! solver would attack, built once and then used — written out as an MPS file,
//! or solved. `docs/python/new_api_design.md` §10.2 is the design; this module
//! is the object it describes.
//!
//! It is **opaque**. A script can hold it, pass it back to a method and print
//! it, and that is all: no variables, no constraints, no accessors. Publishing
//! the problem would make the internal variable and constraint naming of
//! `constraints-colloscopes` public API, and a rename there would then break
//! scripts. The only thing that crosses is the file the export writes.
//!
//! It is **detached**, like the values of §2: a snapshot of the document as it
//! stood when the build ran. Editing the document afterwards neither changes
//! the model nor invalidates it, so there is no staleness question to ask of
//! it — it is not a handle.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use collomatique_constraints_colloscopes::{
    ConfiguredColloscopeModel, ConfiguredConstraintDesc, SolutionFromDataError, convert,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;
use collomatique_subprocesses::{IlpSolverConfig, SolverSubprocess};

use crate::blame::{ConstraintViolation, SeverityLevel};
use crate::errors::{ExportError, SolveError};

/// What a colloscope the model's own parameters cannot read is refused with
///
/// One sentence for the two moments it can be found out at — the conversion
/// here, and the wider variable check inside the modeler — because they are
/// one mistake seen from two distances: this colloscope and this model are
/// not about the same document.
const INCOMPATIBLE: &str = "this colloscope is not compatible with the model: it names \
                            something the model's parameters do not hold, or places a \
                            student the model does not place";

/// A colloscope problem, built and ready to be written out or solved
///
/// ```python
/// model = doc.build_colloscope_model(collomatique.ColloscopeSolveConfig())
/// print(model)   # <ColloscopeModel: 12345 variables, 6789 constraints>
/// ```
///
/// `doc.build_colloscope_model` is the only way to get one — the class is here
/// so that `isinstance` and `repr` say something useful, not so that a script
/// can build one.
///
/// The model is a snapshot: it holds no document and it is never stale. What
/// it holds cannot be looked at, only used.
#[pyclass(module = "collomatique", frozen)]
pub struct ColloscopeModel {
    /// The built problem, in the shape `SolveConfig::build_model` left it —
    /// which already carries both the real problem and the constraints-only
    /// checker one, so the choice between them belongs to the export and not
    /// to the build (§10.2).
    model: ConfiguredColloscopeModel,
    /// The parameters the model was built against
    ///
    /// Kept because a solution is a set of variable values and nothing more:
    /// turning one back into a colloscope is `convert::build_colloscope`, and
    /// that reads the periods, the weeks, the slots and the students out of
    /// these. Only the parameters, not the whole `InnerData` — the colloscope
    /// the document happens to hold is not part of the question.
    ///
    /// Part of the same snapshot as the model itself, taken in the same
    /// breath: so the two can never disagree, and neither goes stale when the
    /// document is edited afterwards.
    params: Parameters,
}

impl ColloscopeModel {
    /// The python object for one built model
    pub(crate) fn new(model: ConfiguredColloscopeModel, params: Parameters) -> Self {
        ColloscopeModel { model, params }
    }

    /// The problem itself, for the doors this crate opens on it
    ///
    /// Crate-private on purpose: `export_mps` and, later, `solve` are the only
    /// things that ever see it.
    pub(crate) fn inner(&self) -> &ConfiguredColloscopeModel {
        &self.model
    }

    /// What a solution of this model is read against
    ///
    /// Crate-private for the same reason as [ColloscopeModel::inner]: the
    /// parameters are the document's own structures, and publishing them would
    /// publish a second way of reading a document alongside the collections.
    pub(crate) fn params(&self) -> &Parameters {
        &self.params
    }
}

#[pymethods]
impl ColloscopeModel {
    /// How big the problem is, and nothing else
    ///
    /// The two counts the application's own advanced-tools panel shows, each
    /// summed over the three kinds the modeler distinguishes: the base
    /// variables plus the helper variables the constraints and the objective
    /// needed, and the constraints written down plus the ones those helpers
    /// are defined by. No names: what a variable *is* is not part of the API
    /// (§10.2).
    fn __repr__(&self) -> String {
        let stats = self.model.stats();

        let variables =
            stats.base_variable_count + stats.constraint_extra_count + stats.objective_extra_count;
        let constraints = stats.user_constraint_count
            + stats.constraint_defining_constraint_count
            + stats.objective_defining_constraint_count;

        format!("<ColloscopeModel: {variables} variables, {constraints} constraints>")
    }

    /// Writes the problem out as an MPS file
    ///
    /// ```python
    /// model = doc.build_colloscope_model(collomatique.ColloscopeSolveConfig())
    /// model.export_mps("problem.mps")
    /// model.export_mps("checker.mps", checker=True)
    /// ```
    ///
    /// MPS is the file format solvers read: every variable, every constraint
    /// and the objective, written down in full. It is a diagnostic file — it
    /// goes to a solver, or to somebody looking at why a colloscope will not
    /// come out. Nothing reads one back, and a solved MPS file is not a
    /// colloscope.
    ///
    /// The names it carries are this program's own names for its variables and
    /// its constraints, mangled into what MPS allows. They are a debugging aid
    /// and nothing more: they can change from one version to the next, so a
    /// script must not read them.
    ///
    /// With `checker=True` a second, smaller problem is written instead: the
    /// constraints alone, with no objective and without what only the objective
    /// needed. It answers « is this colloscope allowed », where the full file
    /// answers « what is the best colloscope ». One build carries both, so the
    /// choice is made here and not at `build_colloscope_model`.
    ///
    /// The model is not touched and the document is not touched either — an
    /// export writes a file, so it takes no undo slot. A path that cannot be
    /// written raises `ExportError`, and the message names the path.
    #[pyo3(signature = (path, *, checker=false))]
    fn export_mps(&self, py: Python<'_>, path: PathBuf, checker: bool) -> PyResult<()> {
        let model = self.inner();
        let problem = if checker {
            model.checker_problem()
        } else {
            model.problem()
        };

        // Released for the duration: a whole colloscope written out variable by
        // variable is long enough to be worth not blocking the interpreter over
        // (`document.rs`'s own `export_xlsx`).
        py.detach(|| {
            let names = collomatique_mps::generate_names(problem);
            let contents = collomatique_mps::generate_mps(problem, &names);
            std::fs::write(&path, contents)
        })
        .map_err(|e| ExportError::new_err(format!("{}: {e}", path.display())))
    }

    /// Launches the solver on this model
    ///
    /// ```python
    /// run = model.solve(collomatique.ConductorStrategy.optimize(), on_log=print)
    /// outcome = run.wait()
    /// if outcome.colloscope is not None:
    ///     doc.colloscope.install(outcome.colloscope)
    /// ```
    ///
    /// This does not block: the engine runs in its own process, and what comes
    /// back is a `SolveRun` — the handle that answers `progress()`, `stop()`,
    /// `kill()` and `wait()`. Dropping the run kills the engine, so a script
    /// holds it for as long as the solve should live.
    ///
    /// `strategy` is a `ConductorStrategy` — how hard to look, with how many
    /// workers, for how long. `ConductorStrategy.search()` and
    /// `.optimize()` are the application's own two presets.
    ///
    /// `engine=` names the collomatique executable the workers re-execute.
    /// Without it, the module uses the application it is running inside, then
    /// the `COLLOMATIQUE_ENGINE` environment variable, and raises `NoEngine`
    /// when neither says anything — a bare python interpreter is not an engine,
    /// and guessing one would spawn the wrong program.
    ///
    /// `on_log` is called with one line of the engine's log at a time,
    /// `on_progress` with a `SolveProgress` each time the engine improves on
    /// itself. Both run on another thread, and neither may call `wait()`. A
    /// callback that raises is not called again, and its exception comes out of
    /// `wait()` with no outcome — the rule `build_colloscope_model` follows for
    /// its own log.
    ///
    /// Nothing is written to the document, so a solve takes no undo slot: what
    /// it produces is a value, and `doc.colloscope.install(...)` is what lands
    /// it. Two solves on one model are fine and independent — each starts its
    /// own engine.
    ///
    /// An engine that cannot be started raises `SolveError`.
    #[pyo3(signature = (strategy, *, engine=None, on_progress=None, on_log=None))]
    fn solve(
        &self,
        py: Python<'_>,
        strategy: &Bound<'_, PyAny>,
        engine: Option<PathBuf>,
        on_progress: Option<Py<PyAny>>,
        on_log: Option<Py<PyAny>>,
    ) -> PyResult<crate::solve::SolveRun> {
        // The refusal order a script can reason about: what it wrote down
        // first, then what the machine was asked for, and only then the spawn.
        let strategy = crate::data::ConductorStrategy::from_py(strategy)?;
        let engine = crate::engine::resolve(engine)?;

        crate::solve::SolveRun::start(
            py,
            self.inner(),
            self.params(),
            &strategy,
            &engine,
            on_progress,
            on_log,
        )
    }

    /// The constraints one colloscope violates, worst first
    ///
    /// ```python
    /// for violation in model.blame(doc.colloscope.to_data()):
    ///     print(violation.severity, "-", violation)
    /// ```
    ///
    /// The question the application answers under « Vérification du
    /// colloscope »: not « what is the best colloscope » but « what is wrong
    /// with this one ». An empty list is a colloscope the model has nothing
    /// against.
    ///
    /// `colloscope` is a `ColloscopeData` — `doc.colloscope.to_data()`, or the
    /// `outcome.colloscope` of a solve. It is read against the model's own
    /// snapshot of the document and not against a live one, so its keys are
    /// ids: a handle names an entity of a document, and a detached model has
    /// none to check it against.
    ///
    /// What comes back is the *minimal* blame: a violation another reported one
    /// already implies is left out, so what remains is the shortest honest
    /// account. It is sorted worst severity first, the way the application
    /// lists it, and every constraint of the model is hard — `PREFERENCE` is
    /// the last thing a relaxation would give up, not something the colloscope
    /// is allowed to break.
    ///
    /// **This blocks**, unlike `solve()`. Filling in what the colloscope does
    /// not say — the helper variables every constraint is really written
    /// against — takes a solver, so an engine runs in its own process here too;
    /// it is a quick feasibility problem with nothing to optimize. Ctrl-C
    /// interrupts the wait and kills that process. `engine=` and `on_log=` mean
    /// what they mean for `solve()`, and `NoEngine` is raised the same way.
    ///
    /// A colloscope this model cannot read at all — a slot or a week that is
    /// not in it, a group number past the list's last group, a student it does
    /// not place — raises `ValueError`. An engine that cannot verify it raises
    /// `SolveError`. Nothing is written to the document, which this model is
    /// not attached to anyway.
    #[pyo3(signature = (colloscope, *, engine=None, on_log=None))]
    fn blame(
        &self,
        py: Python<'_>,
        colloscope: &Bound<'_, PyAny>,
        engine: Option<PathBuf>,
        on_log: Option<Py<PyAny>>,
    ) -> PyResult<Vec<ConstraintViolation>> {
        // The refusal order `solve` follows, most local first: what the script
        // wrote down, then whether it fits this model at all, and only then the
        // machine it would take to check it. So a script can be told its
        // colloscope is the wrong one without an engine anywhere in sight.
        let contents = crate::data::detached_colloscope(colloscope)?;

        // Both setters are plain upserts — they validate nothing, and nothing
        // here wants them to: `build_complete_config` below is the one skeptic,
        // and it reads the colloscope against the parameters that will judge it.
        let mut rebuilt = Colloscope::default();
        for ((slot, week), groups) in contents.interrogations {
            rebuilt.set_interrogation(slot, week, groups);
        }
        for (group_list, placements) in contents.group_lists {
            rebuilt.set_group_list(group_list, placements);
        }

        let config_data = convert::build_complete_config(self.params(), &rebuilt)
            .map_err(|_| PyValueError::new_err(INCOMPATIBLE))?;

        let engine = crate::engine::resolve(engine)?;

        // The first exception a callback raised, and what the interrupted wait
        // leaves behind. Either one wins over whatever the run made of it:
        // `wait()`'s rule, and `build_colloscope_model`'s before it.
        let failure: Arc<Mutex<Option<PyErr>>> = Arc::new(Mutex::new(None));

        let quiet = on_log.is_none();
        let log_failure = Arc::clone(&failure);
        let log_callback = move |line: &str| {
            let Some(callback) = on_log.as_ref() else {
                return;
            };
            if log_failure.lock().unwrap().is_some() {
                return;
            }

            // A worker thread with no interpreter of its own, so each line
            // takes it back for the length of one call and gives it up again.
            Python::attach(|py| {
                if let Err(error) = callback.call1(py, (line,)) {
                    let mut slot = log_failure.lock().unwrap();
                    if slot.is_none() {
                        *slot = Some(error);
                    }
                }
            });
        };

        let spawn_failure = Arc::clone(&failure);
        let wait_failure = Arc::clone(&failure);

        // Released for the whole verification: an engine is started, a problem
        // is solved and an answer comes back, and none of it is the
        // interpreter's business — which is also what lets the log callback
        // above take the interpreter when it has a line.
        let outcome = py.detach(|| {
            self.inner()
                .checker_solution_from_data_with(&config_data, |pb| {
                    let (problem_desc, var_order) = pb.get_desc();
                    let (tx, rx) = mpsc::channel();

                    let subprocess = match SolverSubprocess::spawn(
                        &engine,
                        IlpSolverConfig {
                            problem_desc,
                            warm_start: None,
                            // A feasibility problem with a trivial objective:
                            // CBC finishes quickly, and Ctrl-C covers the
                            // pathological case (the application's own
                            // verification runs it with no limit either).
                            time_limit: collomatique_time::TimeLimit::none(),
                            incumbent_time_limit: collomatique_time::TimeLimit::none(),
                            // A script that asked for no lines gets a quiet
                            // engine rather than a log read and thrown away.
                            disable_logging: quiet,
                        },
                        move |result| {
                            // The receiver is right here and outlives this, but
                            // a send that finds nobody is not worth a panic.
                            let _ = tx.send(result);
                        },
                        // No progress: a reconstruction has an answer, not a
                        // proportion. `SolverSubprocess` answers the
                        // cooperative stop protocol on its own.
                        |_| {},
                        log_callback,
                    ) {
                        Ok(subprocess) => subprocess,
                        Err(e) => {
                            let mut slot = spawn_failure.lock().unwrap();
                            if slot.is_none() {
                                *slot = Some(SolveError::new_err(format!(
                                    "the engine could not be started: {e}"
                                )));
                            }
                            return None;
                        }
                    };

                    let result = loop {
                        match rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(result) => break result,
                            // Woken every 100ms so Ctrl-C reaches the script.
                            // Returning here drops the handle, and dropping it
                            // kills the engine — the wait and the work end
                            // together, which is what a blocking call owes.
                            Err(RecvTimeoutError::Timeout) => {
                                if let Err(error) = Python::attach(|py| py.check_signals()) {
                                    let mut slot = wait_failure.lock().unwrap();
                                    if slot.is_none() {
                                        *slot = Some(error);
                                    }
                                    return None;
                                }
                            }
                            // The engine died without answering; the outcome
                            // says so as `NoSolutionFromSolver`.
                            Err(RecvTimeoutError::Disconnected) => return None,
                        }
                    };
                    // Killing is idempotent, and the child has nothing left to
                    // say once its result is in hand.
                    drop(subprocess);

                    let solution = result.solution?;
                    let data = collomatique_ilp::solution_to_config_data(&solution, &var_order);
                    pb.build_config(data).ok()?.into_feasible()
                })
        });

        // A callback that raised, or an interrupted wait: either is the answer,
        // and the run's own outcome is not looked at.
        if let Some(error) = failure.lock().unwrap().take() {
            return Err(error);
        }

        let solution = outcome.map_err(|e| match e {
            // The two deeper mismatches the conversion cannot see on its own —
            // a student the model does not place, a row for a list it fills
            // itself — surfacing as a variable set that does not match. One
            // mistake, so one sentence.
            SolutionFromDataError::MissingVariables
            | SolutionFromDataError::BuildConfigError(_) => PyValueError::new_err(INCOMPATIBLE),
            SolutionFromDataError::NoSolutionFromSolver => {
                SolveError::new_err("the engine could not verify this colloscope")
            }
        })?;

        let mut violations: Vec<ConstraintViolation> = solution
            .minimal_blame()
            .iter()
            .map(|desc| {
                let message = match desc {
                    ConfiguredConstraintDesc::Inner(inner) => inner.user_readable(self.params()),
                    ConfiguredConstraintDesc::Fixed { var, value } => {
                        collomatique_ui_text::solver::fixed_pin_violation_text(
                            var,
                            value.into_inner(),
                            self.params(),
                        )
                    }
                };
                ConstraintViolation::new(SeverityLevel::from_desc(desc), message)
            })
            .collect();

        // Worst first, the way the application lists a blame; and the sentence
        // as the tie-break, because a blame comes out of hash sets and two runs
        // over one colloscope must read the same.
        violations.sort_by(|a, b| {
            a.severity()
                .cmp(&b.severity())
                .then_with(|| a.message_text().cmp(b.message_text()))
        });

        Ok(violations)
    }
}

/// Adds the model class to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ColloscopeModel>()
}
