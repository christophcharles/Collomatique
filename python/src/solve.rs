//! Running a solve, as a script drives it
//!
//! §13 of `docs/python/new_api_design.md` is the design. What lives here is
//! everything about *running* a solve rather than about the document a solve
//! recomputes: for now, the two presets the strategy's classmethods answer and
//! the warnings a strategy is looked over for before it is handed to anything.
//!
//! The strategy value itself is in `data.rs`, with the other value classes —
//! it is written in python, like all of them, and this module only says what
//! the application's own two look like.

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use collomatique_strategies::{
    ConductorStrategy as RawConductorStrategy, ConductorWarning as RawConductorWarning,
};

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

/// Puts what a script drives a solve with in the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ConductorWarning>()?;

    m.add_function(wrap_pyfunction!(_conductor_search, m)?)?;
    m.add_function(wrap_pyfunction!(_conductor_optimize, m)?)?;
    m.add_function(wrap_pyfunction!(_conductor_warnings, m)?)?;

    Ok(())
}
