mod describe_var;

/// Derive macro for [`collomatique_ilp_modeler::DescribeVar`].
///
/// Generates `enumerate()` and `check_fix()` implementations from
/// enum variants annotated with `#[env]`, `#[range]`, `#[var]`,
/// `#[fix_with]`, and `#[defer_fix]` attributes.
///
/// # Example
///
/// ```ignore
/// #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, DescribeVar)]
/// #[env(MyEnv)]
/// enum Var {
///     #[var(Variable::binary())]
///     StudentSlot {
///         #[range(0..env.num_students)]
///         student: i32,
///         #[range(0..env.num_slots)]
///         slot: i32,
///     },
/// }
/// ```
#[proc_macro_derive(DescribeVar, attributes(env, var, range, fix_with, defer_fix))]
pub fn derive_describe_var(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    describe_var::derive(input)
}
