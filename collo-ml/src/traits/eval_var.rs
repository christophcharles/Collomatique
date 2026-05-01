use crate::ExprType;
use std::collections::HashMap;

/// Extends [`DescribeVar`](collomatique_ilp_modeler::DescribeVar) with
/// ColloML-specific schema information.
///
/// `EvalVar` is a supertrait of `DescribeVar`: any type implementing
/// `EvalVar` automatically has `enumerate()` and `check_fix()` from
/// `DescribeVar`, plus `field_schema()` for the ColloML DSL.
///
/// # Implementation
///
/// Typically derived via `#[derive(EvalVar)]`, which generates both
/// `impl DescribeVar` (for `enumerate`/`check_fix`) and `impl EvalVar`
/// (for `field_schema`), plus `TryFrom<&ExternVar<D>>`.
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
pub trait EvalVar: collomatique_ilp_modeler::DescribeVar {
    /// Returns the schema describing all variable types and their parameters.
    fn field_schema() -> HashMap<String, Vec<ExprType>>;
}
