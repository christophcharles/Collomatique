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

use pyo3::prelude::*;

use collomatique_constraints_colloscopes::ConfiguredColloscopeModel;

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
}

impl ColloscopeModel {
    /// The python object for one built model
    pub(crate) fn new(model: ConfiguredColloscopeModel) -> Self {
        ColloscopeModel { model }
    }

    /// The problem itself, for the doors this crate opens on it
    ///
    /// Crate-private on purpose: `export_mps` and, later, `solve` are the only
    /// things that ever see it.
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &ConfiguredColloscopeModel {
        &self.model
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
}

/// Adds the model class to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ColloscopeModel>()
}
