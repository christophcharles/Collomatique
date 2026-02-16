use crate::ExprType;
use collomatique_ilp::UsableData;
use std::collections::HashMap;

/// Represents variables in an Integer Linear Programming problem.
///
/// This trait defines the interface for enumerating and working with ILP variables that are
/// parameterized by data from the environment. Variables can have parameters like object IDs,
/// integers, or booleans, and the trait provides methods for:
///
/// - Describing the schema of variable parameters
/// - Enumerating all valid variable instances
/// - Determining if a variable should be fixed to a specific value
///
/// # Associated Type
///
/// - `Env`: The environment type that provides data needed for variable enumeration and fixing.
///   Set via `#[env(EnvType)]` on the enum.
///
/// # Dynamic Features
///
/// Variables can have dynamic behavior based on the environment:
///
/// - **Dynamic ranges**: `#[range(0..env.max_week)]` - Range depends on environment state
/// - **Dynamic fix values**: `#[fix_with(if env.flag { 1.0 } else { 0.5 })]` - Fix value depends on environment
/// - **Field-aware fixes**: `#[fix_with(if hour >= 12 { 2.0 } else { 0.0 })]` - Fix value depends on field values
/// - **Complex logic**: `#[defer_fix(Self::check(env, params...))]` - Custom fix logic
///
/// # Implementation
///
/// This trait is typically implemented via the `#[derive(EvalVar)]` macro on an enum.
/// The `#[env(EnvType)]` attribute is required and sets the associated `Env` type.
///
/// ## Variables without objects
///
/// ```ignore
/// #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
/// #[env(MyEnv)]
/// enum Var {
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,
///     },
/// }
/// ```
///
/// The macro generates:
///
/// ```ignore
/// impl EvalVar for Var {
///     type Env = MyEnv;
///     fn field_schema() -> HashMap<String, Vec<ExprType>> { /* ... */ }
///     fn vars(env: &MyEnv) -> BTreeMap<Self, Variable> { /* ... */ }
///     fn fix(&self, env: &MyEnv) -> Option<f64> { /* ... */ }
/// }
/// ```
///
/// # Usage Example
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(MyEnv)]
/// enum Var {
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,
///     },
/// }
///
/// let env = MyEnv::new();
/// let vars = Var::vars(&env);
///
/// let var = Var::TimeSlot { day: 0, hour: 8 };
/// if let Some(value) = var.fix(&env) {
///     // This variable should be fixed to `value`
/// }
/// ```
///
/// # Thread Safety
///
/// Like the view pattern, `EvalVar` is designed for single-threaded use:
/// - `vars()` takes an immutable reference to the environment
/// - The generated variables are independent and can be used concurrently if needed
pub trait EvalVar: UsableData {
    /// The environment type used for variable enumeration and fix computation.
    type Env;

    /// Returns the schema describing all variable types and their parameters.
    fn field_schema() -> HashMap<String, Vec<ExprType>>;

    /// Generates all valid variable instances by enumerating parameter combinations.
    ///
    /// **Important**: Variables with `#[defer_fix(...)]` that return `Some(_)` are automatically
    /// excluded from the result.
    ///
    /// # Returns
    ///
    /// A map of all variable instances to their variable types.
    fn vars(env: &Self::Env) -> std::collections::BTreeMap<Self, collomatique_ilp::Variable>;

    /// Returns a fixed value for this variable instance, if it should be fixed.
    ///
    /// # Returns
    ///
    /// * `None` - This variable instance is free to take any value (it's a decision variable)
    /// * `Some(value)` - This variable instance should be fixed to `value`
    fn fix(&self, env: &Self::Env) -> Option<f64>;
}
