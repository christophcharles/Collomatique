# Step 5 — the solver: `model.solve`, the run handle, `SolveOutcome`, `colloscope.install`, engine location

The implementation plan for the last migration milestone of `docs/python/new_api_design.md`
(§13.5), worked out in discussion (August 2026). §10 of the design doc is the shape being
built; this document is the site-by-site plan, plus the decisions this step settled that
the design doc leaves open.

## Context

Step 4 ended with `doc.build_colloscope_model(config)` handing back an opaque
`ColloscopeModel` and `model.export_mps` (commits `a0330d84`, `c17507ed`, `67aaf9c3`).
Step 5 hangs `model.solve` on that same object:

```python
run = model.solve(strategy, on_progress=..., on_log=...)   # non-blocking
run.progress(); run.stop(); run.kill()
outcome = run.wait()                     # status, objective, bound, colloscope
doc.colloscope.install(outcome.colloscope)
```

plus the `ConductorStrategy` value family with the two GUI presets and `.warnings()`
(§10), the `colloscope.install` landing door gated out of the ops mirror for exactly
this moment, and the engine-location mechanism. Execution reuses
`StrategySubprocess::spawn` (`subprocesses/src/strategy_solver.rs:70`) — the GUI's own
battle-tested path — and the caller-less `EngineExe::Explicit` arm finally gets its
caller.

Wheel/cdylib packaging is explicitly **not** part of this step (§13, last paragraph).
Group-list generation stays out (§10).

**Verified facts the plan leans on** (read in the code, not paraphrased):

- `conductor_outcome` (`strategies/src/strategies/conductor.rs:693`) reports
  `SolveStatus::Optimal` whenever *any* incumbent exists — `Optimal` is not a proof.
  The GUI proves optimality itself with `|objective − bound| ≤ OPTIMALITY_GAP_EPS`
  (`gtk4/src/editor/run_solver.rs:919`); the constant is `pub` and re-exported from
  `strategies/src/lib.rs:6`.
- `StrategySubprocess { stop(), kill(self), last_progress(), spawn(engine, model,
  strategy, warm_start, payload, result_cb, progress_cb, log_cb) }` — callbacks are
  `Fn + Send + 'static` on plain `std::thread`s, no tokio in the parent; spawn is
  synchronous and takes ~1 s (the GUI runs it on a blocking pool,
  `run_solver.rs:604–673`). There is no `wait()`; completion arrives only via
  `result_callback`. When the engine dies, the callback `Arc`s drop, so an
  `mpsc::Sender` captured in them drops too and a receiver sees `Disconnected` — no
  hang.
- `ConductorStatus { best_solution: Option<Solution>, best_bound }`,
  `Solution { config, objective }`, `ConductorProgress::Conductor(status)`
  (`conductor.rs:28–43`); `ConductorStrategy`'s spawn progress type is
  `ConductorProgress<V>` (`conductor.rs:1088`).
- Solution → colloscope: `config.filter_transmute(InternalVar::Base)` then
  `constraints_colloscopes::convert::build_colloscope(&Parameters, &ConfigData<Var>)
  -> Option<Colloscope>` (`convert.rs:153`; GUI use
  `gtk4/src/editor/colloscope.rs:828–852`), then `ColloscopeContents::from(&Colloscope)`
  (`ops/src/colloscope.rs:29`). **`ColloscopeModel` does not store `Parameters` today**
  — it must snapshot it at build time. `Parameters` is
  `state_colloscopes::colloscope_params::Parameters`, the `InnerData.params` field.
- `build_incremental_epochs(model)` (`constraints-colloscopes/src/incremental.rs:16`)
  is one cheap pass; the payload is recomputed per solve, not stored (the GUI computes
  it at spawn time too, `loading_dialog.rs:45–55`).
- `python/` has no dep on `subprocesses` or `strategies` yet — both get added. Nothing
  from `ilp`/`ilp-modeler` needs naming directly: `InternalVar`, `Var`,
  `ConfiguredExtra`, `convert` are re-exported by `constraints-colloscopes`.
- `install` is explicitly reserved in `python/src/collections/colloscope.rs:41–44`;
  `ColloscopeData` (dataclass `data.py:960`, Rust `data.rs:2251`,
  `Model = ColloscopeContents`) already round-trips.
- pyo3 fieldless-enum pattern with `#[pyo3(name = "SCREAMING")]`: `values.rs:47–64`
  (`Weekday`). Callback pattern (GIL released for the work, re-attach per event, first
  exception stashed, callback silenced, error propagated at the end):
  `document.rs:1116–1147`. Runner-injected static: `host.rs` `set_host`, cleared after
  the run (`python-runner/src/lib.rs`).
- `data.py` methods that need the Rust side use a local `import collomatique` (the
  `_every_other_week` precedent, `data.py:105–116`); `__all__` (`data.py:67–92`) is the
  single registration sync point.
- `collomatique-gtk4` has **both a lib and a bin**; its `main.rs` already branches on
  `--rpc-engine` before any GTK initialization, and there is no `gtk4/tests/` yet.
  `CARGO_BIN_EXE_collomatique-gtk4` in a gtk4 integration test names the binary cargo
  just built, in the invocation's own profile.

## Settled decisions

1. **Status vocabulary: `OPTIMAL` / `FEASIBLE` / `INFEASIBLE` / `STOPPED` / `ERROR`,
   with `OPTIMAL` always a proof.** The conductor's raw `Optimal` means only "has
   incumbent"; exposing it as-is would be a trap. Map: raw `Error` → `ERROR`, raw
   `Infeasible` → `INFEASIBLE`, raw `Optimal|Stopped` without solution → `STOPPED`,
   with solution → gap test (`OPTIMALITY_GAP_EPS`, the GUI's own rule) → `OPTIMAL` or
   `FEASIBLE`. `StopReason` is not exposed (§10 lists status/objective/bound/result and
   nothing more). `ERROR` stays a status rather than raising from `wait()`: an errored
   run may still carry a best-so-far solution, and raising would discard it.
2. **`TimeLimit` mirror: `int | None` whole seconds; `0` refused.** `None` says "no
   limit"; `TimeLimit` exists to kill the `Some(0)` footgun (`time/src/lib.rs:162–168`)
   and Python re-importing "0 = unlimited" would resurrect it.
3. **Presets delegate to Rust.** `ConductorStrategy.search()`/`.optimize()` call
   module-private `_conductor_search`/`_conductor_optimize`, which convert
   `RawConductorStrategy::default()` / `::with_parallelism_defaults()` via `to_py` —
   the presets are literally the application's structs, drift structurally impossible
   (`optimize` needs `available_parallelism` anyway).
4. **The French warning sentences go into `ui-text`** — the goto crate for the
   application's French text. New module `ui-text/src/solver.rs` with
   `pub fn conductor_warning_text(w: ConductorWarning) -> &'static str`, the
   `caveats::caveat_text` shape: a French-sentence function over another crate's enum.
   `ui-text` gains a `collomatique-strategies` dependency (no cycle — strategies
   depends only on ilp, ilp-modeler and time); gtk4 and python both already depend on
   ui-text. gtk4's local `warning_message` is deleted in favor of the shared function.
5. **`warnings()` returns a tuple** in the `BTreeSet`'s (declaration) order —
   deterministic, `in` works, immutable container per §2.
6. **Engine precedence: `engine=` param > runner-injected > `COLLOMATIQUE_ENGINE` >
   raise `NoEngine`.** `engine=` is keyword-only on `solve()`, per-call like every
   other argument in this API; nothing stored on model or document. The injection is
   the *caller's* choice: `run_python_script` grows an `engine: Option<EngineExe>`
   parameter — `rpc-engine` passes `Some(EngineExe::Current)` (a hosted process *is* a
   collomatique binary), the CLI of Part H passes the same unless told not to.
7. **New exceptions: `SolveError` under `Error`, `NoEngine` under `SolveError`** — a
   script that only cares that solving failed catches one thing (the
   `SaveError`/`IdCeilingExceeded` pattern, §6). `SolveError` covers spawn failure,
   engine death without an outcome, `wait()`/`stop()` on a killed run, and a solution
   that does not form a colloscope.
8. **`wait()` semantics.** A second `wait()` returns the *cached same* outcome object
   (a fact about a finished run; `is`-identity is the honest expression). `wait()`
   after `kill()` raises `SolveError` pointing at `stop()` — a killed run has no
   outcome, and a `KILLED` status would put a non-outcome in the outcome vocabulary;
   exception: a result that arrived before the kill is still drained and returned
   (mpsc delivers buffered messages before `Disconnected`). Callback exceptions: the
   first one is stashed in a single shared slot, both callbacks are silenced, the run
   is *not* auto-stopped (the `build_colloscope_model` rule: the work runs to its
   end), and the exception is re-raised from `wait()` instead of an outcome.
9. **`kill()` is idempotent** and a no-op after natural finish (teardown in a
   `finally:` must be safe). **`stop()` after kill raises** `SolveError` (a script
   stopping a run it discarded is mistaken about its own run — the `NothingToUndo`
   philosophy); stop after natural finish is inherently a harmless no-op (cooperative
   stop races the finish by design).
10. **`progress()`** returns `SolveProgress { objective, bound }` or `None` before the
    first report — the two numbers a script can act on, from `ConductorStatus`. The
    per-worker union stays out (it would pin the strategy kinds as public API). Never
    blocks, never raises, still answers after finish or kill.
11. **Strategy dataclass names take no `*Data` suffix** (§10.1's rule: call arguments,
    not entity values) and keep the Rust serde field names (the §10.1 precedent kept
    `l1_anchor_weight` etc.).
12. **The application runs Python from the command line** — `--python <CODE>` and
    `--python-file <PATH>` on the collomatique binary, no UI started, plus
    `--python-no-engine` to withhold the injected engine (Part H). This is worth
    having on its own, and it is what makes end-to-end testing clean.
13. **End-to-end tests live in `gtk4/tests/`, in the ordinary suite.** No new crate,
    no alias, no feature gate. The tests spawn `CARGO_BIN_EXE_collomatique-gtk4` —
    guaranteed fresh, profile-correct — with `--python-file`, and assert on exit
    status. No embedded interpreter in the test process, so full libtest parallelism,
    and each child owns its environment, which is what makes the four engine rungs
    (injected, explicit `engine=`, env var, `NoEngine`) each testable in isolation.
    A stable-cargo constraint worth recording: a normal dependency never builds
    another package's binaries, and the feature that would (`artifact = "bin"`,
    bindeps) is still nightly-only — the day it stabilizes, these tests could move to
    a standalone crate unchanged in spirit. If the suite ever grows too slow, the
    `required-features` target gate is the tool to reach for — not before.

---

## Part A — the conductor warnings' French sentences in `ui-text`

**New file `ui-text/src/solver.rs`** (module registered in `ui-text/src/lib.rs` beside
`caveats` and `rendering`, with a sentence added to the crate doc naming it):

```rust
//! The words the application speaks about a solve's configuration.
//!
//! One French sentence per [ConductorWarning] variant, phrased the way the
//! application's own solve dialog shows it. Exhaustive with no wildcard arm,
//! so a new warning over there is a compile error here — the
//! [crate::caveats::caveat_text] shape.

use collomatique_strategies::ConductorWarning;

/// The French sentence the application shows for one conductor warning.
pub fn conductor_warning_text(warning: ConductorWarning) -> &'static str {
    match warning {
        ConductorWarning::NoStrategyEnabled => {
            "Aucune stratégie n'est activée : rien ne sera exécuté."
        }
        // ... the remaining seven arms, moved VERBATIM from
        // gtk4/src/editor/run_solver/conductor_config.rs:1033–1070
    }
}
```

(Match the real `caveats.rs` header and doc style when writing it.)

**`ui-text/Cargo.toml`**: add `collomatique-strategies = { path = "../strategies" }`.
No cycle: strategies depends only on ilp, ilp-modeler and time.

**`gtk4/src/editor/run_solver/conductor_config.rs`** — delete `fn warning_message`
(lines 1033–1070); at its one call site (~line 992):

```rust
// old
guard.push_back(warning_message(warning).to_string());
// new
guard.push_back(collomatique_ui_text::solver::conductor_warning_text(warning).to_string());
```

(gtk4 already depends on ui-text; import per the file's existing style.)

## Part B — `doc.colloscope.install(ColloscopeData)`

**`python/src/collections/colloscope.rs`** — a fifth pymethod after
`erase_group_lists` (~line 344). Extraction runs before the borrow, per §5 (`write`
takes `borrow_mut`):

```rust
/// Replaces the whole colloscope
///
/// ```python
/// outcome = run.wait()
/// doc.colloscope.install(outcome.colloscope)
/// ```
///
/// Afterwards the document holds exactly the value's rows and no others —
/// one operation, one undo slot, however much changed. The op *carries* a
/// whole colloscope but *lands* as a diff, so a row the document already
/// holds costs nothing. The refusals are the model's, each a
/// `ColloscopeError` naming the offending row; the colloscope is pointed at
/// by nothing, so `warnings` is empty.
fn install(&self, py: Python<'_>, colloscope: &Bound<'_, PyAny>) -> PyResult<OpResult> {
    // Extracted before the borrow below and never inside it (§5).
    let contents = ColloscopeData::from_py(&self.doc, colloscope)?;
    self.write(
        py,
        UpdateOp::Colloscope(ColloscopeUpdateOp::InstallColloscope(contents)),
    )
}
```

Two doc comments in the same file go stale and change with it: the module header's
"The family's fifth op, `install`, … is not published here" (lines 41–44) becomes a
sentence saying `install` is the whole-colloscope door and the solver's landing door;
`to_data`'s "There is no write that takes one back whole" (~line 211) becomes
"`install` takes one back whole; the row-by-row doors remain for a single cell." The
matching note near `ColloscopeData` in `data.py` (~line 964) is updated the same way.

`ColloscopeData::from_py` already validates every id against the document, and
`InstallColloscopeError` reaches scripts as `ColloscopeError` through the structural
payload walk (`errors.rs:139–189`) — nothing to add there.

## Part C — the strategy value family

### C.1 `python/src/data.py`

Append after `ColloscopeSolveConfig`; add the five names to `__all__`;
`ConductorWarning` joins the `TYPE_CHECKING` import block (used by `warnings()`'
annotation — that import lands with Part D's commit). Defaults are pinned against the
Rust `Default` impls (`conductor.rs:435–443, 315–324, 348–360, 365–372, 380–388`) by
the round-trip test. Docstrings in the file's house voice (English prose, examples).

```python
@dataclass
class DefaultConfig:
    time_limit: int | None = None
    incumbent_time_limit: int | None = None

@dataclass
class WarmStartConfig:
    time_limit: int | None = None

@dataclass
class IncrementalConfig:
    l1_weight: float = 1000.0
    distance_tolerance: float = 10.0
    epoch_time_limit: int | None = None
    epoch_incumbent_time_limit: int | None = 60

@dataclass
class FuzzyConfig:
    fuzzy_sigma: float = 0.2
    find_closest_tolerance: float = 10.0
    time_limit: int | None = None
    incumbent_time_limit: int | None = None

@dataclass
class ConductorStrategy:
    """How a solve is run: which substrategies, on how many worker slots.

    Each `*_config` field both enables its substrategy and tunes it: `None`
    disables it, an object enables it. `ConductorStrategy()` is the
    application's « Recherche simple » — one worker, warm-start only — and
    the two classmethods are the application's own presets.
    """
    worker_count: int = 1
    default_config: DefaultConfig | None = None
    warm_start_config: WarmStartConfig | None = field(default_factory=WarmStartConfig)
    incremental_config: IncrementalConfig | None = None
    fuzzy_config: FuzzyConfig | None = None

    @classmethod
    def search(cls) -> ConductorStrategy:
        """The « Recherche simple » preset: find a feasible colloscope fast."""
        import collomatique
        return collomatique._conductor_search()

    @classmethod
    def optimize(cls) -> ConductorStrategy:
        """The « Optimisation complète » preset, sized to this machine's cores."""
        import collomatique
        return collomatique._conductor_optimize()

    def warnings(self) -> tuple[ConductorWarning, ...]:
        """Misconfigurations detectable before running, in a fixed order."""
        import collomatique
        return collomatique._conductor_warnings(self)
```

Document on the classmethods that they answer plain `ConductorStrategy` instances
built on the Rust side (which is what makes drift against the application's presets
impossible).

### C.2 `python/src/data.rs` — extraction and construction

Imports: `use collomatique_strategies::{ConductorStrategy as RawConductorStrategy,
DefaultConfig as RawDefaultConfig, FuzzyConfig as RawFuzzyConfig, IncrementalConfig as
RawIncrementalConfig, WarmStartConfig as RawWarmStartConfig};`

Two new field helpers beside `weight` (:380):

```rust
/// A field the solver counts in whole seconds, or `None` for no limit
///
/// Zero is refused rather than read as "no limit": `None` is how no limit is
/// said, and the model's own `TimeLimit` has no room for a zero on purpose.
fn time_limit(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>)
    -> PyResult<collomatique_time::TimeLimit>
{
    let value = field(site, name, obj)?;
    if value.is_none() {
        return Ok(collomatique_time::TimeLimit::none());
    }
    let seconds: u32 = value.extract().map_err(|_| {
        PyTypeError::new_err(format!(
            "{} is a number of seconds or None, and {} is neither",
            site.field(name), shown(&value, "that value"),
        ))
    })?;
    NonZeroU32::new(seconds)
        .map(collomatique_time::TimeLimit::seconds)
        .ok_or_else(|| PyValueError::new_err(format!(
            "{} is at least one second; None is how no limit is said",
            site.field(name),
        )))
}

/// A field that measures without a sign — a sigma, a tolerance
///
/// Zero is allowed; negative and non-finite are not.
fn nonnegative_number(site: Site<'_>, name: &str, obj: &Bound<'_, PyAny>) -> PyResult<f64> {
    // extract f64 with an "is a non-negative number" TypeError, then refuse
    // !is_finite and < 0.0 with PyValueError — mirror checked_weight's shape
}
```

(Check `collomatique_time::TimeLimit`'s real constructor/accessor names before
writing; the sketch assumes `none()`, `seconds()` and a seconds getter.)

The family does **not** implement the `Value` trait — `Value::from_py` takes
`&Py<Document>` for entity resolution and a strategy names no entity (its callers
hold no document). A marker struct with inherent methods keeps the reading shape:

```rust
/// The strategy a solve runs — the one value family with no document behind it
pub struct ConductorStrategy;

impl ConductorStrategy {
    pub(crate) const CLASS: &'static str = "ConductorStrategy";

    pub(crate) fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<RawConductorStrategy> {
        let site = Site::whole(Self::CLASS);
        Ok(RawConductorStrategy {
            // `non_zero_count`'s sentence — "is a number of slots" — is
            // exactly what a worker count is, so the helper is reused.
            worker_count: non_zero_count(site, "worker_count", obj)?,
            default_config: sub_config(site, "default_config", obj, default_config)?,
            warm_start_config: sub_config(site, "warm_start_config", obj, warm_start_config)?,
            incremental_config: sub_config(site, "incremental_config", obj, incremental_config)?,
            fuzzy_config: sub_config(site, "fuzzy_config", obj, fuzzy_config)?,
        })
    }

    pub(crate) fn to_py<'py>(py: Python<'py>, strategy: &RawConductorStrategy)
        -> PyResult<Bound<'py, PyAny>>
    {
        let kwargs = PyDict::new(py);
        kwargs.set_item("worker_count", strategy.worker_count.get())?;
        kwargs.set_item("default_config",
            strategy.default_config.as_ref().map(|c| default_config_to_py(py, c)).transpose()?)?;
        // ... warm_start / incremental / fuzzy alike ...
        class(py, Self::CLASS)?.call((), Some(&kwargs))
    }
}

/// One optional sub-config: `None` disables the substrategy, an object tunes it
fn sub_config<T>(
    site: Site<'_>, name: &'static str, obj: &Bound<'_, PyAny>,
    read: impl Fn(Site<'_>, &Bound<'_, PyAny>) -> PyResult<T>,
) -> PyResult<Option<T>> {
    let value = field(site, name, obj)?;
    if value.is_none() { return Ok(None); }
    read(site.inside(name), &value).map(Some)
}

fn default_config(site: Site<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RawDefaultConfig> {
    Ok(RawDefaultConfig {
        time_limit: time_limit(site, "time_limit", obj)?,
        incumbent_time_limit: time_limit(site, "incumbent_time_limit", obj)?,
    })
}
// warm_start_config: one time_limit.
// incremental_config: l1_weight via weight(), distance_tolerance via
//   nonnegative_number(), the two epoch limits via time_limit().
// fuzzy_config: fuzzy_sigma and find_closest_tolerance via
//   nonnegative_number(), the two limits via time_limit().
// Matching *_to_py builders through `class(py, ...)`, writing seconds as ints.
```

`Site::inside` gives the refusal path (« a ConductorStrategy's
`incremental_config.epoch_time_limit` is at least one second… » — English, exception
messages are for the script author). `fuzzy_sigma` refuses negatives because
`rand_distr`'s `Normal` errors on a negative sigma inside the engine — the boundary is
the last place the field can be named.

### C.3 Presets — new file `python/src/solve.rs`

```rust
/// The « Recherche simple » preset, as the application builds it
///
/// A module-private door for `ConductorStrategy.search()`: the preset is the
/// model's own `Default`, converted — so the classmethod cannot drift from
/// the application.
#[pyfunction]
fn _conductor_search(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    crate::data::ConductorStrategy::to_py(py, &RawConductorStrategy::default())
}

/// The « Optimisation complète » preset, sized to this machine
#[pyfunction]
fn _conductor_optimize(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    crate::data::ConductorStrategy::to_py(py, &RawConductorStrategy::with_parallelism_defaults())
}
```

`lib.rs`: `pub mod solve;` and `solve::register(m)?;` in the `#[pymodule]`.
`solve::register` adds the functions (and, from later parts, the classes).

**`python/Cargo.toml`**: add `collomatique-strategies = { path = "../strategies" }`
with a one-line why-comment in the file's style.

## Part D — `ConductorWarning` and `.warnings()`

In `solve.rs`, the `Weekday` shape (fieldless pyclass enum — all eight variants are
payload-less, so the `Caveat` subclass-per-kind shape buys nothing):

```rust
/// A misconfiguration the conductor can see before running
///
/// `str()` is the French sentence the application's own dialog shows;
/// the identifiers stay English (§3).
#[pyclass(module = "collomatique", frozen, eq, hash)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConductorWarning {
    #[pyo3(name = "NO_STRATEGY_ENABLED")]  NoStrategyEnabled,
    #[pyo3(name = "NO_OPTIMIZING")]        NoOptimizing,
    #[pyo3(name = "NO_SEED")]              NoSeed,
    #[pyo3(name = "STARVED_FUZZY")]        StarvedFuzzy,
    #[pyo3(name = "WONT_FINISH")]          WontFinish,
    #[pyo3(name = "COLD_FUZZY")]           ColdFuzzy,
    #[pyo3(name = "REDUNDANT_WARM_START")] RedundantWarmStart,
    #[pyo3(name = "OVERWHELMED_CPU")]      OverwhelmedCpu,
}

#[pymethods]
impl ConductorWarning {
    fn __str__(&self) -> &'static str {
        collomatique_ui_text::solver::conductor_warning_text(self.to_model())
    }
}

impl ConductorWarning {
    /// Match-based both ways, so a new variant over there is a compile error here
    fn from_model(w: collomatique_strategies::ConductorWarning) -> ConductorWarning { /* match */ }
    fn to_model(self) -> collomatique_strategies::ConductorWarning { /* match */ }
}

/// The preflight warnings of one strategy, in the variants' declaration order
///
/// Extraction validates the strategy, so `warnings()` on a malformed one
/// raises the same refusal `solve` would.
#[pyfunction]
fn _conductor_warnings<'py>(py: Python<'py>, strategy: &Bound<'py, PyAny>)
    -> PyResult<Bound<'py, PyTuple>>
{
    let strategy = crate::data::ConductorStrategy::from_py(strategy)?;
    PyTuple::new(py, strategy.warnings().into_iter().map(ConductorWarning::from_model))
}
```

The `warnings()` method and the `TYPE_CHECKING` import land in `data.py` with this
commit.

## Part E — engine location

**New file `python/src/engine.rs`**, mirroring `host.rs`'s runner-injected static:

```rust
//! The executable a solve re-executes as its engine
//!
//! `docs/python/new_api_design.md` §10 is the design. Explicit beats
//! injected beats the environment; nothing found is a loud `NoEngine`.

static ENGINE: Mutex<Option<EngineExe>> = Mutex::new(None);

/// Installs, or clears, the engine for the coming run
///
/// The runner calls this on both sides of a script, like [crate::set_host],
/// with whatever its own caller chose (`run_python_script`'s `engine`
/// parameter). A hosted process *is* a collomatique binary, so rpc-engine
/// passes [EngineExe::Current] and scripts never think about it.
pub fn set_engine(engine: Option<EngineExe>) {
    *ENGINE.lock().unwrap() = engine;
}

/// The engine one solve will re-execute
pub(crate) fn resolve(explicit: Option<PathBuf>) -> PyResult<EngineExe> {
    if let Some(path) = explicit {
        return Ok(EngineExe::Explicit(path));
    }
    if let Some(engine) = ENGINE.lock().unwrap().clone() {
        return Ok(engine);
    }
    // An empty variable is an unset one: `COLLOMATIQUE_ENGINE= script.py`
    // means "not here", not "the empty path".
    if let Some(path) = std::env::var_os("COLLOMATIQUE_ENGINE").filter(|p| !p.is_empty()) {
        return Ok(EngineExe::Explicit(PathBuf::from(path)));
    }
    Err(NoEngine::new_err(
        "no engine to run the solve: pass engine= with the path of a collomatique \
         executable, set the COLLOMATIQUE_ENGINE environment variable, or run the \
         script inside collomatique",
    ))
}
```

**`python/src/lib.rs`**: `pub mod engine;`, `pub use engine::set_engine;`, and
`pub use collomatique_subprocesses::EngineExe;` (the `Host` re-export pattern —
python-runner then needs this crate and nothing else).

**`python/src/errors.rs`** — two additions plus their `m.add` lines in `register`:

```rust
create_exception!(collomatique, SolveError, Error,
    "A solve could not be run to an outcome.");
create_exception!(collomatique, NoEngine, SolveError,
    "No collomatique engine was found to run a solve.");
```

Exception messages are English; a wrapped `WorkerSpawnError` sentence they embed is
the Rust error's own (French), exactly as `ModelBuildError` embeds the builder's
sentence.

**`python-runner/src/lib.rs`** — `run_python_script` grows the caller-decided engine
parameter (re-export `EngineExe` from this crate too, so its callers name one type):

```rust
// old
pub fn run_python_script(
    script: String,
    file_state: Option<SharedFileState>,
    host: Option<Arc<dyn Host>>,
) -> anyhow::Result<()> {
    collomatique_python_old::set_current_file_state(file_state);
    collomatique_python::set_host(host);
    ...
    collomatique_python_old::set_current_file_state(None);
    collomatique_python::set_host(None);
// new
pub fn run_python_script(
    script: String,
    file_state: Option<SharedFileState>,
    host: Option<Arc<dyn Host>>,
    engine: Option<EngineExe>,
) -> anyhow::Result<()> {
    collomatique_python_old::set_current_file_state(file_state);
    collomatique_python::set_host(host);
    collomatique_python::set_engine(engine);
    ...
    collomatique_python_old::set_current_file_state(None);
    collomatique_python::set_host(None);
    collomatique_python::set_engine(None);
```

**`rpc-engine/src/lib.rs`** (the one existing call site, line ~407):

```rust
// old
            collomatique_python_runner::run_python_script(
                script,
                Some(shared.clone()),
                Some(host),
            )?;
// new
            // A hosted process is a collomatique binary, so the running
            // executable is an engine a script's solve may re-execute —
            // hosted or not, since a script may solve a document it loaded
            // itself.
            collomatique_python_runner::run_python_script(
                script,
                Some(shared.clone()),
                Some(host),
                Some(collomatique_python_runner::EngineExe::Current),
            )?;
```

**`python/Cargo.toml`**: add `collomatique-subprocesses = { path = "../subprocesses" }`.

## Part F — the model keeps what a solve needs

**`python/src/model.rs`**:

```rust
// old
pub struct ColloscopeModel {
    model: ConfiguredColloscopeModel,
}
impl ColloscopeModel {
    pub(crate) fn new(model: ConfiguredColloscopeModel) -> Self { ColloscopeModel { model } }
// new
pub struct ColloscopeModel {
    model: ConfiguredColloscopeModel,
    /// The parameters the model was built against, kept so a solution can be
    /// read back as a colloscope (`convert::build_colloscope` takes them).
    /// Part of the same snapshot as the model itself: never stale, never a view.
    params: Parameters,
}
impl ColloscopeModel {
    pub(crate) fn new(model: ConfiguredColloscopeModel, params: Parameters) -> Self {
        ColloscopeModel { model, params }
    }
    pub(crate) fn params(&self) -> &Parameters { &self.params }
```

(`use collomatique_state_colloscopes::colloscope_params::Parameters;`.) Only
`Parameters`, not the whole `InnerData` — `build_colloscope(env: &Parameters, …)` is
the sole consumer (the GUI reads it off `ilp_problem.env` the same way).

**`python/src/document.rs`** (`build_colloscope_model` tail, ~:1148):

```rust
// old
        built
            .map(ColloscopeModel::new)
            .map_err(ModelBuildError::new_err)
// new
        built
            .map(|model| ColloscopeModel::new(model, inner.params))
            .map_err(ModelBuildError::new_err)
```

(`inner` is the function's own `InnerData` clone; the detach closure's borrow has
ended, so moving `inner.params` out is free.)

## Part G — `model.solve`, `SolveRun`, `SolveOutcome` (all in `solve.rs`)

### G.1 Types

```rust
use collomatique_constraints_colloscopes::{
    ConfiguredColloscopeModel, ConfiguredExtra, InternalVar, Var, convert,
};

type ConfiguredVar = InternalVar<Var, ConfiguredExtra>;
type Outcome = collomatique_strategies::StrategyOutcome<ConfiguredVar>;

/// What the run has reported so far, in the two numbers a script can act on
#[derive(Clone, Copy)]
struct Snapshot {
    objective: Option<f64>,
    bound: Option<f64>,
}
```

### G.2 `SolveStatus`

```rust
/// How a solve ended
#[pyclass(module = "collomatique", frozen, eq, hash)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolveStatus {
    #[pyo3(name = "OPTIMAL")]    Optimal,
    #[pyo3(name = "FEASIBLE")]   Feasible,
    #[pyo3(name = "INFEASIBLE")] Infeasible,
    #[pyo3(name = "STOPPED")]    Stopped,
    #[pyo3(name = "ERROR")]      Error,
}
```

French `__str__` reusing the application's words (`run_solver.rs:277,343,350`):
« Solution optimale trouvée » / « Solution trouvée » / « Pas de solution possible » /
« Arrêté sans solution » / « Erreur pendant l'exécution ».

```rust
/// The status one raw outcome earns
///
/// The conductor reports `Optimal` whenever any incumbent exists
/// (`conductor_outcome`); a proof is the closed gap, the same
/// `OPTIMALITY_GAP_EPS` test the application applies before it writes
/// « Solution optimale trouvée ». So `OPTIMAL` here is always a proof, and an
/// unproven incumbent is `FEASIBLE` — whether the run ended on its own or was
/// stopped, since to a script the two are the same colloscope-in-hand.
fn status_of(outcome: &Outcome) -> SolveStatus {
    match outcome.status {
        RawSolveStatus::Error => SolveStatus::Error,
        RawSolveStatus::Infeasible => SolveStatus::Infeasible,
        RawSolveStatus::Optimal | RawSolveStatus::Stopped(_) => {
            if outcome.solution.is_none() {
                return SolveStatus::Stopped;
            }
            let proven = match (outcome.objective, outcome.best_bound) {
                (Some(objective), Some(bound)) =>
                    (objective - bound).abs() <= collomatique_strategies::OPTIMALITY_GAP_EPS,
                _ => false,
            };
            if proven { SolveStatus::Optimal } else { SolveStatus::Feasible }
        }
    }
}
```

**Superseded by commits 9 and 10.** Running a real solve through this (commit 8)
showed that three of the five states cannot occur. `model.solve` always runs a
conductor, and `conductor_outcome` reports only `Optimal` (any incumbent) or
`Stopped` (none), so `INFEASIBLE` and `ERROR` are unreachable and `STOPPED` means
precisely « no solution ». What landed is four-way, and it is not this crate's to
decide: `collomatique_strategies::verdict` computes it, the solve dialog reads the
same one, its four French sentences live in `ui-text` beside the warnings', and
`STOPPED` is spelled `NO_SOLUTION`. `status_of` is gone. The settled shape is
`new_api_design.md` §10.3.

### G.3 `SolveProgress`

```rust
/// The best the run has found and proven so far
#[pyclass(module = "collomatique", frozen)]
pub struct SolveProgress {
    /// The best incumbent's cost, or `None` while there is none
    #[pyo3(get)]
    objective: Option<f64>,
    /// The best proven bound on any colloscope's cost, or `None`
    #[pyo3(get)]
    bound: Option<f64>,
}
// __repr__: "SolveProgress(objective=123.0, bound=98.0)"
```

### G.4 `SolveOutcome`

```rust
/// What one finished solve produced
#[pyclass(module = "collomatique", frozen)]
pub struct SolveOutcome {
    status: SolveStatus,
    objective: Option<f64>,
    bound: Option<f64>,
    /// The solved colloscope as a `ColloscopeData`, or `None` without a solution
    colloscope: Option<Py<PyAny>>,
}
```

Getters for all four (`colloscope` answers `clone_ref` — the same detached value every
time; mutating it mutates nothing, like every value). `__repr__`:
`<SolveOutcome: SolveStatus.FEASIBLE, objective=123.0, bound=98.0>`.

Built in `wait()`, on the Python thread:

```rust
fn build_outcome(&self, py: Python<'_>, outcome: Outcome) -> PyResult<SolveOutcome> {
    let colloscope = match &outcome.solution {
        None => None,
        Some(solution) => {
            // The solved config is over the *configured* model's variables;
            // strip to base directly (the gtk4 editor's own shortcut).
            let base = solution.filter_transmute(|var| match var {
                InternalVar::Base(b) => Some(b.clone()),
                _ => None,
            });
            let colloscope = convert::build_colloscope(&self.params, &base).ok_or_else(|| {
                // `build_colloscope` only answers None for a malformed
                // config, which is the engine's bug and not the script's:
                // said out loud rather than shrugged into `colloscope=None`.
                SolveError::new_err("the engine returned a solution that does not form a colloscope")
            })?;
            let contents = collomatique_ops::ColloscopeContents::from(&colloscope);
            Some(crate::data::ColloscopeData::to_py(py, &contents)?.unbind())
        }
    };
    Ok(SolveOutcome { status: status_of(&outcome), objective: outcome.objective,
                      bound: outcome.best_bound, colloscope })
}
```

### G.5 `SolveRun`

```rust
/// One running (or finished) solve
///
/// `model.solve(...)` is the only way to get one. Dropping the last reference
/// kills the subprocess (the handle's own RAII), so a script that wants the
/// solve to keep going holds on to the run.
#[pyclass(module = "collomatique", frozen)]
pub struct SolveRun {
    /// The live subprocess. `kill()` takes it out and drops it; every other
    /// door leaves it in place.
    subprocess: Mutex<Option<StrategySubprocess>>,
    /// Where the engine's one result arrives. `wait()` is the only reader,
    /// and holds this lock for the length of its wait — a second concurrent
    /// `wait()` queues behind the first and then answers from the cache.
    receiver: Mutex<mpsc::Receiver<Outcome>>,
    /// The outcome `wait()` built, so a second wait answers the first —
    /// the same object, not a rebuild.
    finished: Mutex<Option<Py<SolveOutcome>>>,
    /// The best objective and bound reported so far, mirrored by the
    /// progress callback; what `progress()` answers without waiting.
    progress: Arc<Mutex<Option<Snapshot>>>,
    /// The first exception a callback raised. Set once; both callbacks go
    /// quiet afterwards, and `wait()` re-raises it instead of an outcome.
    failure: Arc<Mutex<Option<PyErr>>>,
    /// The model's parameters, to read the solution back as a colloscope.
    params: Parameters,
}
```

Locking story: five independent locks; only `wait()` ever holds two, and never
`finished` and `receiver` in the opposite order anywhere, so `kill()` during a
`wait()` cannot deadlock — it drops the `Worker` under the `subprocess` lock, the
child dies, the reader threads drop the callback `Arc`s, the `Sender` drops, and the
waiter's `recv_timeout` answers `Disconnected`.

`start` (called by `model.solve`):

```rust
impl SolveRun {
    pub(crate) fn start(
        py: Python<'_>,
        model: &ConfiguredColloscopeModel,
        params: &Parameters,
        strategy: &RawConductorStrategy,
        engine: &EngineExe,
        on_progress: Option<Py<PyAny>>,
        on_log: Option<Py<PyAny>>,
    ) -> PyResult<SolveRun> {
        let payload = ConductorPayload {
            incremental: IncrementalPayload {
                epochs: collomatique_constraints_colloscopes::build_incremental_epochs(model),
            },
        };

        let (tx, rx) = mpsc::channel();
        let progress: Arc<Mutex<Option<Snapshot>>> = Arc::new(Mutex::new(None));
        let failure: Arc<Mutex<Option<PyErr>>> = Arc::new(Mutex::new(None));

        let result_callback = move |outcome: Outcome| {
            // The receiver may already be gone (run dropped mid-flight);
            // there is then nobody to tell, and nothing to do about it.
            let _ = tx.send(outcome);
        };

        let mirror = Arc::clone(&progress);
        let progress_failure = Arc::clone(&failure);
        let progress_callback = move |update: Result<ConductorProgress<ConfiguredVar>, String>| {
            // Only the conductor-level status is the API's progress; the
            // per-worker union and an undecodable line are the application's
            // panel vocabulary, not a script's.
            let Ok(ConductorProgress::Conductor(status)) = update else { return };
            let snapshot = Snapshot {
                objective: status.best_solution.as_ref().map(|s| s.objective),
                bound: status.best_bound,
            };
            *mirror.lock().unwrap() = Some(snapshot);

            let Some(callback) = on_progress.as_ref() else { return };
            if progress_failure.lock().unwrap().is_some() { return; }
            // Each event takes the interpreter back for one call
            // (`build_colloscope_model`'s own pattern, on a worker thread).
            Python::attach(|py| {
                let called = Py::new(py, SolveProgress { objective: snapshot.objective,
                                                         bound: snapshot.bound })
                    .and_then(|p| callback.call1(py, (p,)));
                if let Err(error) = called {
                    let mut slot = progress_failure.lock().unwrap();
                    if slot.is_none() { *slot = Some(error); }
                }
            });
        };

        let log_failure = Arc::clone(&failure);
        let log_callback = move |line: &str| {
            let Some(callback) = on_log.as_ref() else { return };
            if log_failure.lock().unwrap().is_some() { return; }
            Python::attach(|py| { /* call1((line,)), same first-error stash */ });
        };

        // Released for the duration: `spawn` serializes the whole model and
        // starts the process — over a second of blocking work (the
        // application runs it on a worker pool for the same reason).
        let subprocess = py
            .detach(|| StrategySubprocess::spawn(engine, model, strategy, None, payload,
                                                 result_callback, progress_callback, log_callback))
            .map_err(|e| SolveError::new_err(format!("the engine could not be started: {e}")))?;

        Ok(SolveRun {
            subprocess: Mutex::new(Some(subprocess)),
            receiver: Mutex::new(rx),
            finished: Mutex::new(None),
            progress,
            failure,
            params: params.clone(),
        })
    }
}
```

(`warm_start` is `None`, like the GUI: "start from what the document holds" is the
config's `use_current_values` anchoring, already inside the model.)

The pymethods:

- **`progress()`** → `Option<SolveProgress>`: the mirror's last snapshot, a fresh
  small object each call, `None` before the first report. Never raises, never blocks,
  still answers after finish or kill.
- **`stop()`**: `subprocess` lock; `Some` → `subprocess.stop()` (cooperative — takes
  effect on the child's next progress round-trip; the run then finishes with
  best-so-far and `wait()` collects it). `None` (killed) →
  `SolveError("this run was killed; there is nothing left to stop")`.
- **`kill()`**: `self.subprocess.lock().unwrap().take()` and drop (the `Worker`'s
  `Drop` kills). Idempotent; no-op after natural finish.
- **`wait()`**:

```rust
fn wait(&self, py: Python<'_>) -> PyResult<Py<SolveOutcome>> {
    // A second wait answers the first: the same outcome object (§10).
    if let Some(outcome) = self.finished.lock().unwrap().as_ref() {
        return Ok(outcome.clone_ref(py));
    }
    // A callback's exception is the run's terminal answer, every time asked.
    if let Some(error) = self.failure.lock().unwrap().as_ref() {
        return Err(error.clone_ref(py));
    }

    let receiver = self.receiver.lock().unwrap();
    let outcome = loop {
        // Released around each slice of waiting; woken every 100ms so
        // Ctrl-C still reaches the script (the run itself keeps going —
        // interrupting the wait is not stopping the solve).
        match py.detach(|| receiver.recv_timeout(Duration::from_millis(100))) {
            Ok(outcome) => break outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => py.check_signals()?,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Every sender is gone: the engine process is dead. A result
                // that raced ahead of the kill was drained by the Ok arm
                // above, so reaching here really means there is no outcome.
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
    };
    drop(receiver);

    // The callbacks' exception wins over the outcome, as it wins over the
    // model in build_colloscope_model: the script asked for the lines and
    // one was refused, so no outcome is handed back.
    if let Some(error) = self.failure.lock().unwrap().as_ref() {
        return Err(error.clone_ref(py));
    }

    let outcome = Py::new(py, self.build_outcome(py, outcome)?)?;
    *self.finished.lock().unwrap() = Some(outcome.clone_ref(py));
    Ok(outcome)
}
```

- **`__repr__`**: `<SolveRun: finished>` when `finished` is `Some`,
  `<SolveRun: killed>` when the subprocess is gone with no cached outcome,
  `<SolveRun: running>` otherwise (best-effort — a finished-but-unwaited run still
  says running; the docstring says so).
- The class docstring also documents: `wait()` must not be called from inside a
  callback (the callback thread is the one the child's stop round-trip is waiting
  on); a callback that raises is not called again and the exception comes out of
  `wait()`; the run is not auto-stopped on a callback failure (stopping would need
  the subprocess's private stop flag from inside its own callbacks — a
  `subprocesses` API change this step does not need; a script has
  `stop()`/`kill()`).

`solve::register` adds: `SolveRun`, `SolveOutcome`, `SolveStatus`, `SolveProgress`,
`ConductorWarning`, and the three `_conductor_*` functions.

### G.6 The door — `python/src/model.rs`

```rust
/// Launches the solver on this model
///
/// ```python
/// run = model.solve(clm.ConductorStrategy.optimize(), on_log=print)
/// outcome = run.wait()
/// if outcome.colloscope is not None:
///     doc.colloscope.install(outcome.colloscope)
/// ```
///
/// Non-blocking: the engine runs in its own process, and what comes back is
/// the run handle — `progress()`, `stop()`, `kill()`, `wait()`. `engine=`
/// names the collomatique executable to re-execute; without it, the module
/// uses the application it runs inside, then the COLLOMATIQUE_ENGINE
/// environment variable, and raises `NoEngine` with nothing found. `on_log`
/// takes one line at a time, `on_progress` a `SolveProgress`; a callback
/// that raises is not called again, and the exception comes out of `wait()`
/// with no outcome (`build_colloscope_model`'s rule). Dropping the run kills
/// the engine, so hold it for as long as the solve should live.
#[pyo3(signature = (strategy, *, engine=None, on_progress=None, on_log=None))]
fn solve(
    &self,
    py: Python<'_>,
    strategy: &Bound<'_, PyAny>,
    engine: Option<PathBuf>,
    on_progress: Option<Py<PyAny>>,
    on_log: Option<Py<PyAny>>,
) -> PyResult<crate::solve::SolveRun> {
    // The refusal order a script can reason about: its own strategy first,
    // then the machine's engine, then the spawn.
    let strategy = crate::data::ConductorStrategy::from_py(strategy)?;
    let engine = crate::engine::resolve(engine)?;

    crate::solve::SolveRun::start(py, self.inner(), self.params(), &strategy, &engine,
                                  on_progress, on_log)
}
```

Two solves on one model are fine and independent (each recomputes the payload, each
spawns its own engine).

## Part H — running Python from the command line

**`gtk4/src/main.rs`** — two new arguments beside `--rpc-engine`, and the flag that
withholds the engine. `--python`/`--python-file` are mutually exclusive with each
other and with `--rpc-engine`/`--debug`/`--new`; `--python-no-engine` requires one of
them (a clap `ArgGroup` over the two carries the `requires`):

```rust
    /// Run the given python code with the collomatique module, no UI
    #[arg(long)]
    python: Option<String>,

    /// Run the given python script with the collomatique module, no UI
    #[arg(long)]
    python_file: Option<PathBuf>,

    /// With --python/--python-file: do not offer this executable as the
    /// solve engine (the script then needs engine= or COLLOMATIQUE_ENGINE)
    #[arg(long, default_value_t = false)]
    python_no_engine: bool,
```

The branch sits right after the `--rpc-engine` one, before any GTK initialization —
no UI ever starts:

```rust
    if args.rpc_engine {
        return collomatique_rpc_engine::run_rpc_engine();
    }

    if let Some(code) = python_code(&args)? {
        // The same door the GUI's script runner uses, minus the host: no
        // hosted document (current_document() is None), no file state. The
        // running executable is collomatique itself, so it is the engine a
        // solve re-executes — unless --python-no-engine withholds it, which
        // is what lets a script (or a test) exercise the other rungs.
        let engine = (!args.python_no_engine)
            .then_some(collomatique_python_runner::EngineExe::Current);
        collomatique_python_runner::initialize();
        return collomatique_python_runner::run_python_script(code, None, None, engine);
    }
```

`python_code` reads `--python` verbatim or the `--python-file` contents
(`fs::read_to_string`, an unreadable file is an ordinary error). A script exception
comes back as the `anyhow` error `run_python_script` already produces: printed, exit
code nonzero — the contract the e2e tests assert on. (A full traceback on stderr is a
possible later refinement; the exception display is enough to start.)

Linux-first: on Windows this binary discards console output (`windows_stdio`), so
`--python` joins `--debug` as a unix-terminal feature. Noted, not designed around.

## Part I — tests

### In `python/tests` (the module.rs conventions: one `.py` per feature in
`tests/scripts/`, Rust-side assertions via globals, `ONE_SCRIPT_AT_A_TIME`)

1. `tests/scripts/strategy.py` + `the_conductor_strategy_crosses_the_boundary`: field
   orders and `__module__` of the five classes; `ConductorStrategy()` extracted `==`
   `RawConductorStrategy::default()`; each bare sub-config extracted against its Rust
   `Default` (pins `epoch_incumbent_time_limit=60` etc.); `search()` extracted `==`
   `default()`; `optimize()` extracted `==` `with_parallelism_defaults()` (same
   process, same `available_parallelism` — deterministic); a fully spelled-out
   strategy round-trips; refusals with message checks: `worker_count=0`,
   `worker_count="x"`, `time_limit=0` (message names "at least one second" and None),
   `time_limit=-5`, `time_limit="x"`, `l1_weight=-1.0`, `fuzzy_sigma=float("inf")`,
   `default_config=3`, a sub-config object missing a field (path
   `default_config.time_limit` in the sentence).
2. `tests/scripts/strategy_warnings.py` + `the_conductor_warnings_are_preflight`: the
   all-`None` strategy warns `NO_STRATEGY_ENABLED`;
   `ConductorStrategy.search().warnings() == (ConductorWarning.NO_OPTIMIZING,)` (the
   GUI's own search preset triggers it — faithfully reported); `optimize().warnings()`
   has no configuration warning; declaration order; `==`/`hash`/`in`; `str(w)` equals
   `conductor_warning_text` (asserted Rust-side against ui-text's own function); a
   malformed strategy raises the extraction refusal.
3. `tests/scripts/colloscope_install.py` + `a_colloscope_lands_whole_through_install`:
   on the two-filling fixture — `to_data()`, edit one cell and one placement,
   `install` → `OpResult` with empty warnings; the document holds exactly the
   payload's rows (a dropped row is gone); one `undo()` restores everything (one
   slot); a value naming a dead slot raises `StaleHandleError` via extraction; a
   model-level refusal (group number past the bound) arrives as `ColloscopeError`.
4. Rust unit tests, each in its own file per house rule: `python/src/solve/tests.rs`
   (the `status_of` table — `Error` even with a solution → `ERROR`; `Infeasible` →
   `INFEASIBLE`; `Optimal` with open gap or missing bound → `FEASIBLE`; closed gap →
   `OPTIMAL`; `Stopped` without solution → `STOPPED`; `Stopped(TimeLimit)` with
   solution → `FEASIBLE`) and `python/src/engine/tests.rs` (resolve precedence
   without the env arm: explicit beats injected; injected answers when no explicit;
   `set_engine(None)` restored after each).

   The first of those two moved with the logic it covers: commit 9 took the table
   down to `strategies/src/verdict/tests.rs`, where it is written against
   `SolveVerdict` and `STOPPED` reads `NoSolution` (G.2 above). Nothing is left in
   `python/src/solve/` to test — a 1:1 mirror match needs no test of its own.

### In `gtk4/tests` (end-to-end, new)

**`gtk4/tests/e2e.rs`** — the crate root of the one e2e target, thin:

```rust
//! End-to-end tests against the built collomatique binary
//!
//! Everything here spawns `CARGO_BIN_EXE_collomatique-gtk4` — the binary
//! cargo just built, in this invocation's own profile — and asserts on exit
//! status and output. No embedded interpreter lives in this process: each
//! child owns its environment, which is what makes the engine rungs
//! testable one by one.

const COLLOMATIQUE: &str = env!("CARGO_BIN_EXE_collomatique-gtk4");

mod solve;
```

**`gtk4/tests/e2e/solve.rs`** — the solve module. One fixture script,
`gtk4/tests/e2e/scripts/solve_e2e.py`, run via `--python-file` (path built from
`env!("CARGO_MANIFEST_DIR")`); the script has no argv and no `__file__`, so the test
selects a scenario through an `E2E_MODE` environment variable set per child. The
script builds a small feasible document through the API (the §14 import-style shape —
one period of two weeks, one subject, a teacher, a handful of students, two slots, a
*prefilled* group list with its association, assignments), so the model stays tiny and
each solve is fast. Every spawn controls `COLLOMATIQUE_ENGINE` explicitly
(`env_remove`/`env`), which is the point of the subprocess shape. The tests:

- `a_solve_runs_on_the_injected_engine`: env var removed, no extra flag — the full
  scenario: preset `warnings()` printed, model built with `on_log`, solve with no
  `engine=` (the injected rung — the child *is* collomatique), `wait()`, status in
  `{OPTIMAL, FEASIBLE}`, `colloscope is not None`, `install`, cells asserted; then a
  second run exercising `stop()` at the first progress event, asserting only that the
  status is in the vocabulary and the handle does not wedge (the race with a natural
  finish is inherent). Exit 0.
- `the_injected_engine_beats_the_environment`: `COLLOMATIQUE_ENGINE=/nonexistent`, no
  flag — the tiny solve still succeeds, proving the injected rung outranks the env.
- `the_environment_names_the_engine`: `--python-no-engine`,
  `COLLOMATIQUE_ENGINE=<COLLOMATIQUE>` — the env rung works.
- `an_explicit_engine_beats_the_environment`: `--python-no-engine`,
  `COLLOMATIQUE_ENGINE=/nonexistent`, the real path passed in a test-owned variable
  (`E2E_ENGINE`) that the script hands to `engine=` — the explicit rung outranks the
  env.
- `no_engine_is_a_loud_refusal`: `--python-no-engine`, env removed — the script
  catches `NoEngine`, asserts `isinstance` of `SolveError` and `Error`, and that the
  message names `engine=` and `COLLOMATIQUE_ENGINE`.
- `a_dead_engine_fails_the_solve`: `--python-no-engine`,
  `COLLOMATIQUE_ENGINE=/nonexistent/collomatique` — the failure is `SolveError`
  whether it surfaces from `solve()` (spawn refused) or from `wait()` ("the engine
  exited without an outcome"); the script accepts either point.

These run in the ordinary suite (`cargo test --workspace`) and parallelize freely —
each is its own process tree. They are also the CLI's own e2e (exit codes, both
flags), and the seed of the broader e2e module family (CLI-argument testing joins as
`gtk4/tests/e2e/cli.rs` later, one `mod` line each).

## Part J — commit sequence

Following the series' granularity (one door or one family per commit):

1. `ui-text: the conductor warnings' french sentences` — Part A (ui-text + gtk4, new
   `strategies` dep on ui-text).
2. `python: publish the whole-colloscope landing door, install` — Part B + test I.3.
3. `python: the conductor strategy values` — Part C (dataclasses, extraction,
   presets, new `strategies` dep) + test I.1.
4. `python: the conductor's preflight warnings` — Part D + test I.2.
5. `python: the engine a solve re-executes` — Part E (engine.rs, `SolveError`/
   `NoEngine`, the re-exports, the `run_python_script` engine parameter with its
   rpc-engine call site, new `subprocesses` dep) + the engine unit tests. The
   behavioral rung tests land with commit 11, the first door that reaches them —
   said in the message.
6. `python: the model keeps the parameters a solution needs` — Part F.
7. `python: solve on the model, the run handle and its outcome` — Part G + the
   `status_of` unit tests.
8. `gtk4: run python straight from the command line` — Part H.

Commits 9 and 10 were not in this plan. Commit 8 was the first that could run a real
solve, and it showed the five-way `SolveStatus` of G.2 to be three states too wide
and, worse, a second private copy of a verdict the solve dialog was already
computing for itself. The two commits move that verdict into `strategies`, where
both front ends read it:

9. `strategies: one verdict for a finished solve` — `SolveVerdict` and `verdict()` in
   a new `strategies/src/verdict.rs`, with G.2's table moved down to its tests
   (I.4), and the four French sentences in `ui-text` beside the warnings'.
10. `gtk4, python: read the shared verdict` — the solve dialog's three labels become
    one, its private `is_provably_optimal` goes, and python's `SolveStatus` becomes
    a 1:1 mirror with `STOPPED` renamed `NO_SOLUTION`.

11. `gtk4: end-to-end solve tests against the built binary` — Part I's e2e target,
    its solve module and fixture script.
12. `docs: record the solver landing` — mark §13.5 done in `new_api_design.md` and
    fold in the refinements this step settled (the four-way verdict and its home in
    `strategies`, the injected-engine rung in the precedence and its caller-decided
    parameter, the warnings' tuple shape and their sentences' home in ui-text,
    wait/kill semantics, the CLI python doors, the e2e target).

## Verification

- After each commit: `cargo build` of the touched crates; the full suite
  (`cargo test --workspace`, foreground, 10-minute timeout, captured once to a
  scratchpad file and grepped) at the natural checkpoints — after commits 2, 4, 7
  and 11. From commit 11 on, the suite includes the real end-to-end solve.
- **User-side nix step**: commits 1, 3 and 5 add path dependencies, so `Cargo.lock`
  moves and `cargoHash` in `collomatique.nix` needs refreshing once the series lands
  (not predicting the diffs — just flagging the step exists; check `git status`
  before each commit per the standing rule).
