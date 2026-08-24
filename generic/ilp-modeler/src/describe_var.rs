use std::collections::HashMap;

use collomatique_ilp::{UsableData, Variable};

/// Describes variable enumeration and fix logic against a
/// pre-loaded environment. This is the derive-macro target.
///
/// The `Env` associated type holds all data needed for
/// enumeration and fixing (ranges, availability maps, etc.).
///
/// Use [`Modeler::from_described`] to build a modeler from
/// an environment reference.
///
/// [`Modeler::from_described`]: crate::Modeler::from_described
pub trait DescribeVar: UsableData {
    /// The environment type holding pre-loaded data for
    /// variable enumeration and fixing.
    type Env;

    /// Enumerate all base variable instances from the environment.
    ///
    /// Variables that should be fixed must NOT be included in the
    /// result — they are handled by [`check_fix`](Self::check_fix)
    /// through the fixer chain.
    fn enumerate(env: &Self::Env) -> HashMap<Self, Variable>;

    /// Return a fixed value for this variable, or `None` if free.
    fn check_fix(&self, env: &Self::Env) -> Option<f64>;
}
