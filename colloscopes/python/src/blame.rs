//! What a colloscope breaks, as a script reads it
//!
//! `model.blame(colloscope)` (in `model.rs`, because a blame is a thing asked
//! of a model) hands back a list of these: one constraint the colloscope
//! violates, said in the sentence the application would have shown, with how
//! bad it is.
//!
//! A violation is **not** a structured mirror of the model's own constraint
//! descriptions. Publishing those would make the internal vocabulary of
//! `constraints-colloscopes` public API — the same reason §10.2 of
//! `docs/python/new_api_design.md` keeps the model itself opaque — and a
//! rename over there would then break scripts. What crosses is a severity a
//! script can compare and sort on, and a sentence it can print.
//!
//! Both classes are rust classes rather than the `.py` dataclasses of §2, for
//! [crate::caveats]'s reason: they are flat, immutable, and only ever travel
//! *out* of rust.

use pyo3::prelude::*;

use collomatique_constraints_colloscopes::ConfiguredConstraintDesc;

/// How bad a violated constraint is — the worst compares smallest
///
/// ```python
/// worst = min(v.severity for v in model.blame(colloscope))
/// ```
///
/// The five tiers the model itself distinguishes, plus `FIXED` on top of them.
/// `FIXED` is not one of the model's own: it marks a broken *pin* of the solve
/// configuration — a variable the configuration said not to recompute, which
/// the colloscope contradicts — and that outranks anything the model says,
/// because it is the one thing the person driving the solve asked for by hand.
///
/// The order is the declaration order, worst first, and it is what `sorted()`
/// uses: `FIXED < INFEASIBILITY < STRUCTURAL < QUALITY < PROGRESSIVE <
/// PREFERENCE`.
///
/// Every constraint of the model is hard — the tiers are about what a
/// relaxation would give up first, not about which violations are allowed. A
/// `PREFERENCE` violation is a real violation.
// Handed out only, never taken back in — nothing here reads a severity out of
// a script — so the extraction the id classes opt into is skipped, like
// [crate::solve::SolveStatus]'s.
#[pyclass(module = "collomatique", frozen, eq, ord, hash, skip_from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeverityLevel {
    /// A pin of the solve configuration the colloscope contradicts
    #[pyo3(name = "FIXED")]
    Fixed,
    /// The colloscope cannot be a colloscope at all with this in it
    #[pyo3(name = "INFEASIBILITY")]
    Infeasibility,
    /// A rule about the structure of the colloscope
    #[pyo3(name = "STRUCTURAL")]
    Structural,
    /// A rule about the quality of the result
    #[pyo3(name = "QUALITY")]
    Quality,
    /// A rule the solver tightens as it goes
    #[pyo3(name = "PROGRESSIVE")]
    Progressive,
    /// A preference between two results that are both allowed
    #[pyo3(name = "PREFERENCE")]
    Preference,
}

impl SeverityLevel {
    /// The name python knows this level by, for the repr that spells one out
    ///
    /// A match rather than a formatting of the rust variant, so the two names
    /// cannot drift apart: they are written next to each other above
    /// ([crate::solve::SolveStatus]'s own arrangement).
    fn name(self) -> &'static str {
        match self {
            SeverityLevel::Fixed => "FIXED",
            SeverityLevel::Infeasibility => "INFEASIBILITY",
            SeverityLevel::Structural => "STRUCTURAL",
            SeverityLevel::Quality => "QUALITY",
            SeverityLevel::Progressive => "PROGRESSIVE",
            SeverityLevel::Preference => "PREFERENCE",
        }
    }

    /// The severity of one violated constraint of a configured model
    ///
    /// A match, like every other conversion in this crate, so that a sixth
    /// tier over in `constraints-colloscopes` is a compile error here rather
    /// than a severity python silently mislabels.
    pub(crate) fn from_desc(desc: &ConfiguredConstraintDesc) -> SeverityLevel {
        use collomatique_constraints_colloscopes::SeverityLevel as Model;

        match desc {
            ConfiguredConstraintDesc::Fixed { .. } => SeverityLevel::Fixed,
            ConfiguredConstraintDesc::Inner(inner) => match inner.severity_level() {
                Model::Infeasibility => SeverityLevel::Infeasibility,
                Model::Structural => SeverityLevel::Structural,
                Model::Quality => SeverityLevel::Quality,
                Model::Progressive => SeverityLevel::Progressive,
                Model::Preference => SeverityLevel::Preference,
            },
        }
    }
}

/// One constraint the checked colloscope violates
///
/// ```python
/// for violation in model.blame(colloscope):
///     print(violation.severity, "-", violation)
/// ```
///
/// The severity, and the French sentence the application would have shown for
/// it. `str()` is that sentence, so a script prints a violation the way the
/// solve dialog writes one.
// Handed out only, never taken back in — nothing here reads a violation out of
// a script — so the extraction is skipped, like [SeverityLevel]'s above.
#[pyclass(module = "collomatique", frozen, eq, hash, skip_from_py_object)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstraintViolation {
    /// How bad this one is — a [SeverityLevel]
    #[pyo3(get)]
    severity: SeverityLevel,
    /// The sentence, built once when the blame was read
    message: String,
}

impl ConstraintViolation {
    /// One violation, as `model.blame` builds it
    ///
    /// Crate-private: a violation is something a blame hands back, not
    /// something a script writes down.
    pub(crate) fn new(severity: SeverityLevel, message: String) -> ConstraintViolation {
        ConstraintViolation { severity, message }
    }

    /// How bad this one is, for the sort a blame comes back in
    pub(crate) fn severity(&self) -> SeverityLevel {
        self.severity
    }

    /// What this violation sorts on, after its severity
    ///
    /// A blame comes out of hash sets, so the sentence is what makes the order
    /// the same from one run to the next.
    pub(crate) fn message_text(&self) -> &str {
        &self.message
    }
}

#[pymethods]
impl ConstraintViolation {
    /// The French sentence the application shows for this violation
    #[getter]
    fn message(&self) -> &str {
        &self.message
    }

    /// The same sentence — printing a violation prints what the user reads
    fn __str__(&self) -> &str {
        &self.message
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "<ConstraintViolation: SeverityLevel.{}, {}>",
            self.severity.name(),
            crate::handles::quoted(py, &self.message),
        )
    }
}

/// Puts what a blame hands back in the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Registered so `isinstance` and `repr` say something useful, like the
    // model class is; `model.blame(...)` is the only way to get a violation.
    m.add_class::<ConstraintViolation>()?;
    m.add_class::<SeverityLevel>()
}
