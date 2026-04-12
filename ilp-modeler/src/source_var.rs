use std::collections::HashMap;
use std::future::Future;

use collomatique_ilp::{UsableData, Variable};

/// Trait for types that can enumerate ILP base variables from a data source.
///
/// Both methods are async: variable enumeration and fixing may require
/// database access or other I/O.
///
/// # Type Parameters
///
/// - `Db`: the data source (e.g. a database connection pool) passed to
///   both methods.
///
/// # Contract
///
/// [`vars`](SourceVar::vars) must **not** include variables that should be
/// fixed. Those are handled separately by the fixer mechanism
/// (see [`Modeler::add_fixer`](crate::Modeler::add_fixer)).
pub trait SourceVar<Db>: UsableData {
    /// Enumerate all base variable instances with their variable kinds.
    ///
    /// Variables that should be fixed must NOT be included in the result.
    fn vars(db: &Db) -> impl Future<Output = HashMap<Self, Variable>>;
}
