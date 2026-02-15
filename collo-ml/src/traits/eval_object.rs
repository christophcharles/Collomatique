use crate::semantics::ExprType;
use collomatique_ilp::UsableData;
use std::collections::HashMap;

/// The main trait for objects in the DSL evaluation system.
///
/// This trait provides type schema information for semantic analysis and links
/// to an environment type used by `EvalVar` during problem building.
///
/// # Associated Types
///
/// - `Env`: The environment type that holds the actual data, used by `EvalVar`
///   for variable enumeration and fixing.
///
/// # Implementation
///
/// For simple cases without objects, use `NoObject` with `NoObjectEnv`.
/// For application-specific environments, implement this trait on an empty enum.
pub trait EvalObject: UsableData {
    /// The environment type that provides access to the underlying data.
    type Env;

    /// Returns the schema for all object types in the DSL.
    ///
    /// This provides type information for semantic analysis and validation before execution.
    ///
    /// # Returns
    ///
    /// A nested map where:
    /// - Outer keys are DSL type names (e.g., "Student")
    /// - Inner maps associate field names with their types
    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>>;
}
