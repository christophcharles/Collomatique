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

use pyo3::prelude::*;

use collomatique_constraints_colloscopes::ConfiguredColloscopeModel;
use collomatique_state_colloscopes::colloscope_params::Parameters;

use crate::errors::ExportError;

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
}

/// Adds the model class to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ColloscopeModel>()
}
