use super::{EvalObject, FieldType};
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
/// # Design Philosophy
///
/// `EvalVar` is **generic over `T: EvalObject`**, allowing the same variable definitions to work
/// with different data sources. This enables:
///
/// - Testing with mock data while using production data in deployment
/// - Reusing variable definitions across different problem instances
/// - Clear separation between variable structure and data access
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
/// To enable environment field access, use `#[env(EnvType)]` on the enum. This constrains the
/// implementation to work with any `EvalObject` that has that environment type, while remaining
/// generic over the specific `EvalObject`.
///
/// # Type Parameter
///
/// - `T`: The `EvalObject` type that provides access to the underlying data via its environment
///
/// # Implementation
///
/// This trait is typically implemented via the `#[derive(EvalVar)]` macro on an enum:
///
/// ## Static Variables (Fully Generic)
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
/// enum Var {
///     StudentInGroup(StudentId, GroupId),
///
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
/// impl<T: EvalObject> EvalVar<T> for Var
/// where
///     StudentId: TryFrom<T, Error = TypeConversionError>,
///     GroupId: TryFrom<T, Error = TypeConversionError>,
/// {
///     fn field_schema() -> HashMap<String, Vec<FieldType>> { /* ... */ }
///     fn vars(env: &T::Env) -> Result<BTreeMap<Self, Variable>, TypeId> { /* ... */ }
///     fn fix(&self, env: &T::Env) -> Option<f64> { /* ... */ }
/// }
/// ```
///
/// ## Dynamic Variables (Environment-Specific)
///
/// ```ignore
/// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
/// #[env(DynamicEnv)]  // Required for env field access
/// #[fix_with(0.0)]
/// enum DynamicVar {
///     StudentInWeek {
///         student: StudentId,
///         #[range(0..env.max_week)]  // Dynamic range
///         week: i32,
///     },
///
///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]  // Dynamic fix value
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..env.last_hour)]  // Dynamic range
///         hour: i32,
///     },
/// }
/// ```
///
/// The macro generates:
///
/// ```ignore
/// impl<T> EvalVar<T> for DynamicVar
/// where
///     T: EvalObject<Env = DynamicEnv>,  // Constrained to specific Env!
///     StudentId: TryFrom<T, Error = TypeConversionError>,
/// {
///     fn field_schema() -> HashMap<String, Vec<FieldType>> { /* ... */ }
///     fn vars(env: &DynamicEnv) -> Result<BTreeMap<Self, Variable>, TypeId> {
///         // env is concrete &DynamicEnv - can access env.max_week!
///         // Variables with defer_fix returning Some are excluded
///     }
///     fn fix(&self, env: &DynamicEnv) -> Option<f64> {
///         // env is concrete &DynamicEnv - can evaluate dynamic expressions!
///     }
/// }
/// ```
///
/// # Usage Example
///
/// ## Static Variables
///
/// ```ignore
/// // Define variables with static ranges
/// #[derive(EvalVar)]
/// enum Var {
///     StudentInGroup(StudentId, GroupId),
/// }
///
/// // Create environment
/// let env = MyEnv::new();
///
/// // Generate all variables with a specific EvalObject type
/// let vars = <Var as EvalVar<ObjectId>>::vars(&env)?;
///
/// // Check if a specific variable should be fixed
/// let var = Var::StudentInGroup(StudentId(0), GroupId(1));
/// if let Some(value) = <Var as EvalVar<ObjectId>>::fix(&var, &env) {
///     // This variable should be fixed to `value`
/// }
///
/// // Get the schema
/// let schema = Var::field_schema();
/// ```
///
/// ## Dynamic Variables
///
/// ```ignore
/// // Define variables with environment-dependent behavior
/// #[derive(EvalVar)]
/// #[env(DynamicEnv)]
/// #[fix_with(0.0)]
/// enum DynamicVar {
///     #[range(0..env.max_week)]
///     StudentInWeek {
///         student: StudentId,
///         week: i32,
///     },
///
///     #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
///     TimeSlot {
///         #[range(0..7)]
///         day: i32,
///         #[range(8..env.last_hour)]
///         hour: i32,
///     },
/// }
///
/// // Create environment with dynamic configuration
/// let env = DynamicEnv { max_week: 4, last_hour: 17, /* ... */ };
///
/// // Works with any EvalObject that has Env = DynamicEnv
/// let vars = <DynamicVar as EvalVar<ObjectId>>::var(&env)?;
/// let vars = <DynamicVar as EvalVar<MockObjectId>>::var(&env)?;  // Also works
///
/// // Fix values depend on environment and field values
/// let var = DynamicVar::TimeSlot { day: 3, hour: 13 };
/// // Returns Some(2.0) because hour is in lunch range (12-14)
/// let fix = <DynamicVar as EvalVar<ObjectId>>::fix(&var, &env);
/// ```
///
/// # Variable Enumeration Strategy
///
/// The `vars()` method generates all valid combinations by taking the cartesian product of:
///
/// - **i32 parameters**: All values in the specified range (e.g., `0..10` -> 10 values)
///   - Ranges can be static: `#[range(0..10)]`
///   - Or dynamic from environment: `#[range(0..env.max_week)]`
/// - **bool parameters**: Both `true` and `false` (2 values)
/// - **Object parameters**: All objects of that type from `T::objects_with_typ(env, type_name)`
///
/// **Filtering with defer_fix**: Variables where `#[defer_fix(...)]` returns `Some(_)` are
/// automatically excluded from `vars()` for performance.
///
/// For example:
/// ```ignore
/// // Static ranges
/// TimeSlot {
///     #[range(0..7)]
///     day: i32,     // range 0..7  -> 7 values
///     #[range(8..18)]
///     hour: i32,    // range 8..18 -> 10 values
/// }
/// // Total: 7 × 10 = 70 variables
///
/// // Dynamic ranges
/// #[env(DynamicEnv)]
/// StudentInWeek {
///     student: StudentId,
///     #[range(0..env.max_week)]  // Range depends on env!
///     week: i32,
/// }
/// // With 3 students and env.max_week = 4: 3 × 4 = 12 variables
/// // With env.max_week = 2: 3 × 2 = 6 variables
/// ```
///
/// # Fix Values
///
/// The `fix()` method allows specifying that certain variable instances should be fixed to
/// specific values in the ILP solver. This is useful for:
///
/// - Enforcing constraints (e.g., "this combination is impossible")
/// - Handling boundary cases
/// - Pre-solving parts of the problem
/// - Dynamic business rules (e.g., "no scheduling during lunch if mandatory")
///
/// Return `None` if the variable is free to take any value, or `Some(value)` to fix it.
///
/// ## Fix Mechanisms
///
/// There are two ways to specify fix values:
///
/// ### 1. `#[fix_with(expr)]` - Returns `f64`
///
/// Evaluated when any field is out of its range. The expression can reference:
/// - `env` - The environment (if `#[env(EnvType)]` is specified)
/// - Field names (for named fields) or `v0`, `v1`, etc. (for unnamed fields)
///
/// ```ignore
/// #[env(DynamicEnv)]
/// enum Var {
///     // Static value
///     #[fix_with(1.0)]
///     SimpleSlot { #[range(0..10)] slot: i32 },
///
///     // Based on environment
///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
///     TimeSlot { #[range(0..7)] day: i32, #[range(8..18)] hour: i32 },
///
///     // Based on field values (named)
///     #[fix_with(if *hour >= 12 && *hour < 14 { 2.0 } else { 0.0 })]
///     WorkSlot { #[range(0..5)] day: i32, #[range(8..18)] hour: i32 },
///
///     // Based on field values (unnamed)
///     #[fix_with(if *v1 >= 12 { 2.0 } else { 0.0 })]
///     Slot(#[range(0..7)] i32, #[range(8..18)] i32),
/// }
/// ```
///
/// ### 2. `#[defer_fix(expr)]` - Returns `Option<f64>`
///
/// Completely replaces range checking. Useful for complex logic that can't be expressed
/// with simple range checks. Variables where this returns `Some(_)` are excluded from `vars()`.
///
/// ```ignore
/// #[env(DynamicEnv)]
/// enum Var {
///     // Call custom function
///     #[defer_fix(Self::check_availability(env, student, week))]
///     StudentAvailable {
///         student: StudentId,
///         #[range(0..env.max_week)]
///         week: i32,
///     },
///
///     // Inline expression
///     #[defer_fix({ if *week >= 3 { Some(5.0) } else { None } })]
///     LateWeek { #[range(0..10)] week: i32 },
/// }
///
/// impl Var {
///     fn check_availability<T: EvalObject>(
///         env: &T::Env,
///         student: &StudentId,
///         week: &i32,
///     ) -> Option<f64>
///     where
///         T::Env: AsRef<DynamicEnv>,
///     {
///         let env = env.as_ref();
///         if env.is_student_absent(student, *week) {
///             Some(10.0)  // Penalty for unavailable
///         } else {
///             None
///         }
///     }
/// }
/// ```
///
/// # Integration with ColloML
///
/// Variables defined with `EvalVar` can be referenced in ColloML constraint scripts using
/// the `$` syntax:
///
/// ```ignore
/// // Rust
/// #[derive(EvalVar)]
/// enum Var {
///     #[name("SiG")]
///     StudentInGroup(StudentId, GroupId),
/// }
///
/// // ColloML
/// pub let one_group_per_student() -> [Constraint] = [
///     sum g in @[Group] { $SiG(s, g) } === 1
///     for s in @[Student]
/// ];
/// ```
///
/// # Thread Safety
///
/// Like the view pattern, `EvalVar` is designed for single-threaded use:
/// - `vars()` takes an immutable reference to the environment
/// - The generated variables are independent and can be used concurrently if needed
/// - Type `T` and `Self` must be `Send` if variables need to cross thread boundaries
pub trait EvalVar<T: EvalObject>: UsableData {
    /// Returns the schema describing all variable types and their parameters.
    ///
    /// This method provides type information for each variable variant, mapping the DSL name
    /// to a list of parameter types. This schema is used for semantic analysis and validation
    /// before code generation.
    ///
    /// # Returns
    ///
    /// A map where:
    /// - Keys are DSL variable names (affected by `#[name("...")]` attributes)
    /// - Values are vectors of parameter types in declaration order
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// enum Var {
    ///     #[name("SiG")]
    ///     StudentInGroup(StudentId, GroupId),
    ///
    ///     TimeSlot {
    ///         #[range(0..7)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let schema = Var::field_schema();
    /// // Returns:
    /// // {
    /// //     "SiG": [FieldType::Object(TypeId::of::<StudentId>()),
    /// //             FieldType::Object(TypeId::of::<GroupId>())],
    /// //     "TimeSlot": [FieldType::Int, FieldType::Int],
    /// // }
    /// ```
    fn field_schema() -> HashMap<String, Vec<FieldType>>;
    /// Generates all valid variable instances by enumerating parameter combinations.
    ///
    /// This method creates the cartesian product of all possible parameter values, using the
    /// environment to enumerate object instances. Each generated variable is associated with
    /// its `Variable` type (binary, integer, continuous, etc.).
    ///
    /// **Important**: Variables with `#[defer_fix(...)]` that return `Some(_)` are automatically
    /// excluded from the result.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment containing the data needed to enumerate objects
    ///   - For generic variables (no `#[env]`): type is `&T::Env`
    ///   - For environment-specific variables (with `#[env(EnvType)]`): type is `&EnvType`
    ///
    /// # Returns
    ///
    /// * `Ok(BTreeMap<Self, Variable>)` - Map of all variable instances to their variable types
    /// * `Err(TypeId)` - If an object type cannot be resolved via `T::type_id_to_name()`
    ///
    /// The error case typically indicates a mismatch between the variable definition and the
    /// `EvalObject` implementation.
    ///
    /// # Enumeration Strategy
    ///
    /// For each parameter type:
    ///
    /// - **i32**: Iterates through the range specified in `#[range(expr)]`
    ///   - Can be static: `#[range(0..10)]`
    ///   - Or dynamic: `#[range(0..env.max_week)]`
    /// - **bool**: Enumerates `[false, true]`
    /// - **Objects**: Calls `T::objects_with_typ(env, type_name)` and converts to the ID type
    ///
    /// # Example
    ///
    /// ## Static Variables
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// enum Var {
    ///     StudentInGroup(StudentId, GroupId),
    ///     WeekUsed(#[range(0..3)] i32),
    /// }
    ///
    /// let env = MyEnv { /* ... */ };
    /// let vars = <Var as EvalVar<ObjectId>>::vars(&env)?;
    ///
    /// // If there are 5 students and 3 groups:
    /// // - StudentInGroup generates: 5 × 3 = 15 variables
    /// // - WeekUsed generates: 3 variables
    /// // Total: 18 variables
    /// ```
    ///
    /// ## Dynamic Variables
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum DynamicVar {
    ///     StudentInWeek {
    ///         student: StudentId,
    ///         #[range(0..env.max_week)]  // Dynamic range
    ///         week: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv { max_week: 4, /* ... */ };
    /// let vars = <DynamicVar as EvalVar<ObjectId>>::vars(&env)?;
    ///
    /// // With 3 students and max_week = 4:
    /// // - StudentInWeek generates: 3 × 4 = 12 variables
    ///
    /// // Change environment
    /// env.max_week = 2;
    /// let vars2 = <DynamicVar as EvalVar<ObjectId>>::vars(&env)?;
    /// // Now generates: 3 × 2 = 6 variables
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// The cartesian product can grow quickly with multiple parameters:
    /// - 10 students × 5 groups × 7 days = 350 variables
    /// - 100 tasks × 20 slots × 2 bools = 4000 variables
    ///
    /// Be mindful of the total number of combinations when designing your variables.
    fn vars(
        env: &T::Env,
    ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>;
    /// Returns a fixed value for this variable instance, if it should be fixed.
    ///
    /// This method allows specifying that certain variable instances should be fixed to
    /// specific values in the ILP solver, rather than being decision variables. This is
    /// useful for handling boundary cases or invalid states. The goal is mainly to catch
    /// variables that are invalid but that the type system of ColloML cannot forbid.
    ///
    /// # Arguments
    ///
    /// * `env` - Reference to the environment
    ///   - For generic variables (no `#[env]`): type is `&T::Env`
    ///   - For environment-specific variables (with `#[env(EnvType)]`): type is `&EnvType`
    ///
    /// # Returns
    ///
    /// * `None` - This variable instance is free to take any value (it's a decision variable)
    /// * `Some(value)` - This variable instance should be fixed to `value`
    ///
    /// # Generated Behavior
    ///
    /// The macro generates a `fix()` implementation based on the attributes:
    ///
    /// ## With `#[fix_with(expr)]`
    ///
    /// Returns `Some(expr)` if any `i32` parameter is outside its specified range, and `None`
    /// otherwise. The expression can reference:
    /// - `env` - The environment (if `#[env(EnvType)]` is used)
    /// - Field names (for named fields) or `v0`, `v1`, etc. (for unnamed fields)
    ///
    /// Default fix value is:
    /// - `0.0` by default
    /// - Can be overridden with `#[fix_with(value)]` on the enum
    /// - Can be overridden with `#[fix_with(value)]` on the variant
    ///
    /// ## With `#[defer_fix(expr)]`
    ///
    /// Returns the result of evaluating `expr` directly, completely replacing range checking.
    /// The expression must return `Option<f64>`.
    ///
    /// # Examples
    ///
    /// ## Basic Range Checking
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[fix_with(0.0)]
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
    ///
    /// let valid_var = Var::TimeSlot { day: 3, hour: 10 };
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&valid_var, &env), None);  // Free variable
    ///
    /// let invalid_var = Var::TimeSlot { day: 10, hour: 10 };  // day out of range
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&invalid_var, &env), Some(0.0));  // Fixed
    /// ```
    ///
    /// ## Dynamic Fix Based on Environment
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum Var {
    ///     #[fix_with(if env.lunch_mandatory { 1.0 } else { 0.5 })]
    ///     TimeSlot {
    ///         #[range(0..7)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv { lunch_mandatory: true, /* ... */ };
    /// let var = Var::TimeSlot { day: 10, hour: 10 };  // Out of range
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&var, &env), Some(1.0));  // Uses env.lunch_mandatory
    /// ```
    ///
    /// ## Dynamic Fix Based on Field Values
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum Var {
    ///     #[fix_with(if hour >= 12 && hour < 14 { 2.0 } else { 0.0 })]
    ///     WorkSlot {
    ///         #[range(0..5)]
    ///         day: i32,
    ///         #[range(8..18)]
    ///         hour: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv::new();
    /// let var = Var::WorkSlot { day: 10, hour: 13 };  // day out of range, hour in lunch
    /// assert_eq!(<Var as EvalVar<ObjectId>>::fix(&var, &env), Some(2.0));  // Lunch hour penalty
    /// ```
    ///
    /// ## Complex Logic with defer_fix
    ///
    /// ```ignore
    /// #[derive(EvalVar)]
    /// #[env(DynamicEnv)]
    /// enum Var {
    ///     #[defer_fix(Self::check_availability(env, student, week))]
    ///     StudentAvailable {
    ///         student: StudentId,
    ///         #[range(0..env.max_week)]
    ///         week: i32,
    ///     },
    /// }
    ///
    /// let env = DynamicEnv::new();
    /// let var = Var::StudentAvailable {
    ///     student: StudentId(0),
    ///     week: 1,
    /// };
    ///
    /// // Returns Some(10.0) if student is absent that week, None otherwise
    /// let fix_value = <Var as EvalVar<ObjectId>>::fix(&var, &env);
    /// ```
    ///
    fn fix(&self, env: &T::Env) -> Option<f64>;
}
