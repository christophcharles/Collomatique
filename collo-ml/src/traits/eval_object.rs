use collomatique_ilp::UsableData;

/// The main trait for objects in the DSL evaluation system.
///
/// This trait links to an environment type used by `EvalVar` during problem building.
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
}
