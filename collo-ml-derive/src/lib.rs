//! Procedural macros for the collo-ml DSL.
//!
//! This crate provides derive macros for the ColloML evaluation system.
//!
//! # Available Macros
//!
//! - [`EvalVar`] - Derives the `EvalVar` trait for ILP variable enums

use proc_macro::TokenStream;

mod eval_var;

/// Derives the `EvalVar` trait for an enum of ILP variables.
///
/// This macro generates the complete `EvalVar` trait implementation, including field schema,
/// variable enumeration, and fix value computation. The enum represents all possible variable
/// types in your Integer Linear Programming problem.
///
/// # Key Design
///
/// The trait uses an **associated type `Env`** to specify the environment. The `#[env(EnvType)]`
/// attribute is required to set this associated type.
///
/// # Required Attributes
///
/// ## On the enum:
///
/// - `#[env(EnvType)]` - Specifies the environment type (sets `type Env = EnvType`)
///
/// # Optional Attributes
///
/// ## On the enum:
///
/// - `#[fix_with(value)]` - Sets the default fix value for out-of-range variables (default: 0.0)
///
/// ## On variants:
///
/// - `#[name("VarName")]` - Overrides the DSL variable name (default is variant name)
/// - `#[var(expr)]` - Sets the variable type (default: `Variable::binary()`)
/// - `#[fix_with(expr)]` - Expression returning `f64`, evaluated when any field is out of range
/// - `#[defer_fix(expr)]` - Expression returning `Option<f64>`, replaces all range checking
///
/// ## On fields:
///
/// - `#[range(expr)]` - **Required for `i32` fields**. Expression returning a range of i32
///
/// # Supported Field Types
///
/// - `i32` - **Must** have `#[range(...)]` attribute
/// - `bool` - Automatically enumerates `true` and `false`
///
/// # Requirements
///
/// - Must be applied to an enum
/// - All `i32` fields must have `#[range(...)]` attribute
/// - The enum must implement standard derives: `Clone, PartialEq, Eq, PartialOrd, Ord`
///
/// # Examples
///
/// ## Basic usage
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
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
/// ## With custom DSL names
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(MyEnv)]
/// enum Var {
///     #[name("TS")]  // Called "TS" in DSL scripts
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,
///     },
/// }
/// ```
///
/// ## With dynamic ranges from environment
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// #[fix_with(0.0)]
/// enum Var {
///     SlotInWeek {
///         #[range(0..env.max_slots)]
///         slot: i32,
///         #[range(0..env.max_week)]  // Range depends on environment!
///         week: i32,
///     },
/// }
/// ```
///
/// ## With dynamic fix values based on environment
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..env.last_hour)]  // Dynamic range
///         hour: i32,
///     },
/// }
/// ```
///
/// ## With fix values based on field values (named fields)
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
///     WorkSlot {
///         #[range(0..5)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,  // Can reference 'hour' in fix_with
///     },
/// }
/// ```
///
/// ## With fix values based on field values (unnamed fields)
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     // Use v0, v1, etc. for unnamed fields
///     #[fix_with(if *v1 >= 12 && *v1 < 14 { 2.0 } else { 0.0 })]
///     TimeSlot(
///         #[range(0..7)] i32,
///         #[range(8..18)] i32,
///     ),
/// }
/// ```
///
/// ## With defer_fix for complex logic
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[defer_fix(Self::check_availability(env, day, week))]
///     SlotAvailable {
///         #[range(0..7)]
///         day: i32,
///         #[range(0..env.max_week)]
///         week: i32,
///     },
/// }
///
/// impl Var {
///     fn check_availability(
///         env: &DynamicEnv,
///         day: &i32,
///         week: &i32,
///     ) -> Option<f64> {
///         if env.is_day_off(*day, *week) {
///             Some(10.0)  // High penalty for unavailable slots
///         } else {
///             None
///         }
///     }
/// }
/// ```
///
/// ## With inline defer_fix expression
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum Var {
///     #[defer_fix({
///         if *week >= 3 {
///             Some(5.0)
///         } else {
///             None
///         }
///     })]
///     LateWeekSlot {
///         #[range(0..env.max_slots)]
///         slot: i32,
///         #[range(0..env.max_week)]
///         week: i32,
///     },
/// }
/// ```
///
/// ## With custom variable types
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(MyEnv)]
/// enum Var {
///     #[var(Variable::binary())]
///     IsSelected(#[range(0..env.num_tasks)] i32),
///
///     #[var(Variable::integer())]
///     Count(#[range(0..100)] i32),
///
///     #[var(Variable::continuous().min(0.0).max(1.0))]
///     Proportion(#[range(0..env.num_items)] i32),
/// }
/// ```
///
/// ## With boolean fields
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(MyEnv)]
/// enum Var {
///     TaskEnabled {
///         #[range(0..env.num_tasks)]
///         task: i32,
///         is_enabled: bool,  // No range needed - automatically [false, true]
///     },
/// }
/// ```
///
/// # Generated Code
///
/// ## EvalVar Implementation
///
/// ```ignore
/// impl EvalVar for Var {
///     type Env = DynamicEnv;
///
///     fn field_schema() -> HashMap<String, Vec<ExprType>> {
///         // Maps DSL names to parameter types
///     }
///
///     fn vars(env: &DynamicEnv) -> BTreeMap<Self, Variable> {
///         // Generates cartesian product of all parameter combinations
///         // Variables with defer_fix returning Some are excluded
///     }
///
///     fn fix(&self, env: &DynamicEnv) -> Option<f64> {
///         // Evaluates fix_with or defer_fix expressions
///         // Returns Some(fix_value) if any i32 is out of range
///     }
/// }
/// ```
///
/// ## TryFrom Implementation
///
/// ```ignore
/// // Also generates generic TryFrom for converting from DSL representation
/// impl<__D: DatabaseConnection> TryFrom<&ExternVar<__D>> for Var {
///     type Error = VarConversionError;
///
///     fn try_from(value: &ExternVar<__D>) -> Result<Self, Self::Error> {
///         // Matches DSL name and converts parameters
///     }
/// }
/// ```
///
/// # Usage
///
/// ```ignore
/// let vars = <Var as EvalVar>::vars(&env);
/// let fix = <Var as EvalVar>::fix(&var, &env);
/// ```
///
/// # Variable Enumeration
///
/// The `vars()` method generates all valid variable combinations:
///
/// - **i32**: Iterates through the specified range `start..end` (exclusive end)
///   - Ranges can be static: `#[range(0..10)]`
///   - Or dynamic from environment: `#[range(0..env.max_week)]`
/// - **bool**: Iterates through `[false, true]`
///
/// **Filtering with defer_fix**: Variables where `defer_fix` returns `Some(_)` are automatically
/// excluded from `vars()`.
///
/// For example:
/// ```ignore
/// // Static ranges
/// TimeSlot { day: i32, hour: i32 }  // #[range(0..7)] and #[range(8..18)]
/// // 7 days × 10 hours -> generates 70 variables
///
/// // Dynamic ranges
/// SlotInWeek { slot: i32, week: i32 }  // #[range(0..env.max_slots)] and #[range(0..env.max_week)]
/// // With env.max_slots = 3 and env.max_week = 4 -> generates 12 variables
/// ```
///
/// # Fix Values
///
/// The `fix()` method returns a value for variables that should be fixed in the ILP solver:
///
/// ## With fix_with
///
/// - Returns `None` if all parameters are within their valid ranges
/// - Returns `Some(fix_value)` if any `i32` parameter is out of range
/// - The `fix_value` can be:
///   - Static: `#[fix_with(1.0)]`
///   - Based on environment: `#[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]`
///   - Based on field values (named): `#[fix_with(if *hour >= 12 { 2.0 } else { 0.0 })]`
///   - Based on field values (unnamed): `#[fix_with(if *v0 > 10 { 1.0 } else { 0.0 })]`
///
/// ## With defer_fix
///
/// - Completely replaces range checking
/// - Returns `Option<f64>` directly from the expression
/// - Can call custom functions: `#[defer_fix(Self::check(env, student, week))]`
/// - Can use inline logic: `#[defer_fix({ if *week >= 3 { Some(5.0) } else { None } })]`
/// - Variables where `defer_fix` returns `Some(_)` are excluded from `vars()`
///
/// ## Priority
///
/// When a field is out of range:
/// 1. If `defer_fix` is present -> return `defer_fix` result
/// 2. Otherwise -> return `fix_with` value (variant-level or enum-level default)
///
/// Useful for handling edge cases or enforcing constraints.
///
/// # Error Handling
///
/// ## Compile-time Errors
///
/// The macro panics at compile time if:
/// - Applied to non-enum types
/// - An `i32` field lacks `#[range(...)]` attribute
/// - A `bool` field has `#[range(...)]` attribute
/// - A `Vec` type is used (lists cannot be enumerated)
/// - Both `#[fix_with(...)]` and `#[defer_fix(...)]` are present on the same variant
/// - `#[env(...)]` is missing
///
/// ## Runtime Errors
///
/// `TryFrom` returns `VarConversionError` if:
/// - The DSL variable name is unknown
/// - Parameter count doesn't match
/// - A parameter has the wrong type
///
/// ## Range Expression Errors
///
/// Range expressions (e.g., `#[range(0..env.max_week)]`) are evaluated at runtime:
/// - If the expression panics, `vars()` will panic
/// - If the expression produces an invalid range (start > end), behavior is defined by Rust's Range
/// - Empty ranges (start == end) generate zero variables
///
/// # Integration with ColloML
///
/// Variables work seamlessly with ColloML constraint scripts:
///
/// ```ignore
/// // Rust definition
/// #[derive(EvalVar)]
/// #[env(MyEnv)]
/// enum Var {
///     #[name("TS")]
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..18)]
///         hour: i32,
///     },
/// }
///
/// // ColloML script
/// pub let constraints() -> [Constraint] = [
///     sum h in 8..18 { $TS(d, h) } <= 8
///     for d in 0..7
/// ];
/// ```
///
/// # The #[env(...)] Attribute
///
/// The `#[env(EnvType)]` attribute is **required** and sets the associated type `type Env = EnvType`.
/// This allows the generated code to use `env` as a concrete typed reference.
///
/// ```ignore
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// enum DynamicVar {
///     #[range(0..env.max_week)]  // Can access env.max_week
///     Slot { slot: i32 },
/// }
/// // Generated: impl EvalVar for DynamicVar { type Env = DynamicEnv; ... }
/// ```
///
/// # Variable Type Expressions
///
/// The `#[var(...)]` attribute accepts any expression that evaluates to `Variable`:
///
/// ```ignore
/// #[var(Variable::binary())]                    // Binary variable (0 or 1)
/// #[var(Variable::integer())]                   // Integer variable
/// #[var(Variable::continuous())]                // Continuous variable
/// #[var(Variable::integer().min(0).max(100))]   // With bounds
/// ```
#[proc_macro_derive(EvalVar, attributes(name, var, range, fix_with, defer_fix, env))]
pub fn derive_eval_var(input: TokenStream) -> TokenStream {
    eval_var::derive(input)
}
