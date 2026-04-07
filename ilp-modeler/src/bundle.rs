//! Composable model fragments.
//!
//! A [`ConstraintBundle`] is "the same arguments you'd pass to
//! [`Modeler::add_constraint`], [`Modeler::add_objective`] and
//! [`Modeler::declare_extra`], queued up to be applied to a
//! modeler later." It carries eager constraints, weighted
//! objectives, and lazy extra-variable definitions, and can be
//! composed with other bundles via [`ConstraintBundle::merge`]
//! before being handed to [`Modeler::apply_bundle`].
//!
//! [`IntConstraintBundle`] is the same shape but with
//! [`IntConstraint`] in place of [`Constraint`]. It exists to
//! support integer-specific transformations (notably reification,
//! added in a follow-up). Drop the int wrapping with
//! [`IntConstraintBundle::into_general`].

use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use collomatique_ilp::linexpr::EqSymbol;
use collomatique_ilp::{Constraint, IntConstraint, LinExpr, Objective, UsableData, Variable};

use crate::{DefineFn, ExtraVar, HelperFactory, Modeler, Var, VarKinds};

// ---------------------------------------------------------------------------
// ExtraEntry
// ---------------------------------------------------------------------------

/// One declare-extra worth of arguments, stored as a value so
/// that it can sit inside a bundle until the bundle is applied.
pub struct ExtraEntry<'m, B, E, Db, Err>
where
    B: UsableData,
    E: UsableData,
{
    pub name: E,
    pub kind: Variable,
    pub define: Box<DefineFn<'m, B, E, Db, Err>>,
}

impl<'m, B, E, Db, Err> ExtraEntry<'m, B, E, Db, Err>
where
    B: UsableData,
    E: UsableData,
{
    /// Build an entry from a closure of the same shape as
    /// [`Modeler::declare_extra`]. The closure is boxed under
    /// the proper HRTB so callers don't have to wrangle the
    /// `dyn` lifetimes themselves.
    pub fn new<F>(name: E, kind: Variable, define: F) -> Self
    where
        F: for<'a> FnOnce(
                &'a Db,
                &'a mut crate::HelperFactory<B, E>,
                &'a crate::VarKinds<'a, B, E>,
                E,
            ) -> crate::BoxFuture<
                'a,
                Result<Vec<Constraint<crate::ExtraVar<B, E>>>, Err>,
            > + 'm,
    {
        ExtraEntry {
            name,
            kind,
            define: Box::new(define),
        }
    }
}

// ---------------------------------------------------------------------------
// ConstraintBundle
// ---------------------------------------------------------------------------

/// A queued-up set of operations to perform on a [`Modeler`].
/// The three fields shadow [`Modeler::add_constraint`],
/// [`Modeler::add_objective`] and [`Modeler::declare_extra`]
/// one-to-one. Bundles compose by appending and are consumed by
/// [`Modeler::apply_bundle`].
pub struct ConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Eager user constraints.
    pub constraints: Vec<(Constraint<Var<B, E>>, C)>,
    /// Weighted objective contributions. Mirrors the modeler's
    /// internal `objectives` vec — merging two bundles is
    /// concat, no arithmetic, all weights preserved.
    pub objectives: Vec<(f64, Objective<Var<B, E>>)>,
    /// Lazy extra-variable definitions. Each entry is exactly
    /// what `declare_extra` would receive: a name, a kind, and
    /// a definition closure. Stored as closures because there is
    /// no way to flatten an extra back into top-level
    /// constraints.
    pub extras: Vec<ExtraEntry<'m, B, E, Db, Err>>,
    _phantom: PhantomData<Db>,
}

impl<'m, B, E, C, Db, Err> Default for ConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    fn default() -> Self {
        ConstraintBundle {
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'m, B, E, C, Db, Err> ConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Empty bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a list of constraints with descriptions.
    pub fn from_constraints<I>(constraints: I) -> Self
    where
        I: IntoIterator<Item = (Constraint<Var<B, E>>, C)>,
    {
        ConstraintBundle {
            constraints: constraints.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Append all of `other`'s entries into `self`. Constraints,
    /// objectives, and extras concat in order; no arithmetic.
    pub fn merge(&mut self, other: Self) {
        self.constraints.extend(other.constraints);
        self.objectives.extend(other.objectives);
        self.extras.extend(other.extras);
    }
}

// ---------------------------------------------------------------------------
// IntConstraintBundle
// ---------------------------------------------------------------------------

/// Same shape as [`ConstraintBundle`] but the eager constraints
/// are [`IntConstraint`] rather than [`Constraint`]. The
/// objective stays as [`Objective`] (no `IntObjective`).
pub struct IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    pub constraints: Vec<(IntConstraint<Var<B, E>>, C)>,
    pub objectives: Vec<(f64, Objective<Var<B, E>>)>,
    pub extras: Vec<ExtraEntry<'m, B, E, Db, Err>>,
    _phantom: PhantomData<Db>,
}

impl<'m, B, E, C, Db, Err> Default for IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    fn default() -> Self {
        IntConstraintBundle {
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'m, B, E, C, Db, Err> IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Empty bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a list of int constraints with descriptions.
    pub fn from_constraints<I>(constraints: I) -> Self
    where
        I: IntoIterator<Item = (IntConstraint<Var<B, E>>, C)>,
    {
        IntConstraintBundle {
            constraints: constraints.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Append all of `other`'s entries into `self`. Constraints,
    /// objectives, and extras concat in order; no arithmetic.
    pub fn merge(&mut self, other: Self) {
        self.constraints.extend(other.constraints);
        self.objectives.extend(other.objectives);
        self.extras.extend(other.extras);
    }

    /// Drop the int wrapping. Each [`IntConstraint`] is unwrapped
    /// into its underlying [`Constraint`]; objectives and extras
    /// pass through unchanged.
    pub fn into_general(self) -> ConstraintBundle<'m, B, E, C, Db, Err> {
        ConstraintBundle {
            constraints: self
                .constraints
                .into_iter()
                .map(|(c, desc)| (c.into_constraint(), desc))
                .collect(),
            objectives: self.objectives,
            extras: self.extras,
            _phantom: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Modeler::apply_bundle
// ---------------------------------------------------------------------------

impl<'m, B, E, C, Db, Err> Modeler<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + 'static,
{
    /// Apply a bundle to the modeler: push every constraint,
    /// push every weighted objective, and declare every extra.
    /// Equivalent to repeated `add_constraint` / `add_objective`
    /// / `declare_extra` calls in field-then-vec order.
    pub fn apply_bundle(&mut self, bundle: ConstraintBundle<'m, B, E, C, Db, Err>) {
        for (c, desc) in bundle.constraints {
            self.add_constraint(c, desc);
        }
        for (coef, obj) in bundle.objectives {
            self.add_objective(coef, obj);
        }
        for entry in bundle.extras {
            // The boxed `define` already matches DefineFn —
            // declare_extra_boxed (in lib.rs) inserts it
            // directly without re-wrapping.
            self.declare_extra_boxed(entry.name, entry.kind, entry.define);
        }
    }
}

// ---------------------------------------------------------------------------
// Reification
// ---------------------------------------------------------------------------

/// Errors raised by [`IntConstraintBundle::reify`] when its
/// linearization closure runs.
///
/// These errors surface late: `reify` itself is total at the
/// call site, but the closure it stacks on top of the bundle
/// can fail at expansion time and the modeler propagates the
/// failure as `BuildError::ExtraError`.
#[derive(Debug, thiserror::Error)]
pub enum ReifyError<B, E>
where
    B: UsableData,
    E: UsableData,
{
    /// A variable referenced in the constraints being reified
    /// is not declared as integer in the current `VarKinds`.
    /// Reification with epsilon-slack only soundly works on
    /// discrete (integer) variables.
    #[error("variable {0:?} is not discrete (integer); cannot reify")]
    NonDiscreteVariable(ExtraVar<B, E>),
    /// A variable referenced in the constraints being reified
    /// has no kind known to the current `VarKinds` /
    /// `HelperFactory`.
    #[error("variable {0:?} has no declared kind; cannot reify")]
    UndeclaredVariable(ExtraVar<B, E>),
    /// The LHS of a constraint being reified has an unbounded
    /// (infinite) range. Big-M linearization needs finite bounds.
    #[error("constraint LHS has an unbounded range; cannot reify")]
    InfiniteLhsRange,
}

/// Look up the kind of any [`ExtraVar`] reference reachable
/// from inside an extra-definition closure. Helpers are routed
/// through the factory; base/extra refs through `kinds`.
fn lookup_kind<'a, B, E>(
    var: &ExtraVar<B, E>,
    kinds: &'a VarKinds<'a, B, E>,
    factory: &'a HelperFactory<B, E>,
) -> Option<&'a Variable>
where
    B: UsableData,
    E: UsableData,
{
    match var {
        ExtraVar::Base(b) => kinds.base(b),
        ExtraVar::Extra(e) => kinds.extra(e),
        ExtraVar::Helper(h) => factory.kind_of(h),
    }
}

/// AND-reify a list of constraints into a single binary
/// indicator. Direct port of
/// `collo-ml/src/problem/builder.rs:582` (`reify_constraint`)
/// generalised to operate on `ExtraVar<B, E>`.
fn reify_and_inner<B, E>(
    constraints: &[Constraint<ExtraVar<B, E>>],
    indicator: ExtraVar<B, E>,
    factory: &mut HelperFactory<B, E>,
    kinds: &VarKinds<B, E>,
) -> Result<Vec<Constraint<ExtraVar<B, E>>>, ReifyError<B, E>>
where
    B: UsableData,
    E: UsableData,
{
    // Empty list: the AND is trivially true; pin the indicator
    // to 1.
    if constraints.is_empty() {
        let v = LinExpr::var(indicator);
        return Ok(vec![v.eq(&LinExpr::constant(1.0))]);
    }
    // Single constraint: skip the AND-combine encoding.
    if constraints.len() == 1 {
        return reify_single(&constraints[0], indicator, factory, kinds);
    }

    // General case: reify each sub-constraint into its own
    // binary helper, then enforce the AND via:
    //   indicator <= h_i  for each i
    //   sum(h_i) <= indicator + (n - 1)
    let mut output = Vec::new();
    let mut helpers: Vec<ExtraVar<B, E>> = Vec::with_capacity(constraints.len());
    for c in constraints {
        let h = factory.new_helper(Variable::binary());
        helpers.push(h.clone());
        output.extend(reify_single(c, h, factory, kinds)?);
    }

    let var_expr = LinExpr::var(indicator.clone());
    for h in &helpers {
        output.push(var_expr.leq(&LinExpr::var(h.clone())));
    }
    let rhs = &var_expr + LinExpr::constant((helpers.len() - 1) as f64);
    let mut lhs: LinExpr<ExtraVar<B, E>> = LinExpr::constant(0.0);
    for h in helpers {
        lhs = lhs + LinExpr::var(h);
    }
    output.push(lhs.leq(&rhs));
    Ok(output)
}

/// Reify a single constraint into the given binary indicator.
/// Direct port of `collo-ml/src/problem/builder.rs:461`
/// (`reify_single_constraint`).
fn reify_single<B, E>(
    constraint: &Constraint<ExtraVar<B, E>>,
    indicator: ExtraVar<B, E>,
    factory: &mut HelperFactory<B, E>,
    kinds: &VarKinds<B, E>,
) -> Result<Vec<Constraint<ExtraVar<B, E>>>, ReifyError<B, E>>
where
    B: UsableData,
    E: UsableData,
{
    use std::collections::HashSet;

    let vars: HashSet<ExtraVar<B, E>> = constraint.variables();

    // ----- 0/1-variable special cases (port of collo-ml lines
    // 472-518) -----
    match vars.len() {
        0 => {
            let v = LinExpr::var(indicator);
            let c = if constraint.is_trivially_true() {
                v.eq(&LinExpr::constant(1.0))
            } else {
                v.eq(&LinExpr::constant(0.0))
            };
            return Ok(vec![c]);
        }
        1 => {
            let single = vars.into_iter().next().unwrap();
            let single_kind = lookup_kind(&single, kinds, factory)
                .cloned()
                .ok_or_else(|| ReifyError::UndeclaredVariable(single.clone()))?;
            if single_kind == Variable::binary() {
                let eval = |val: bool| {
                    let mut subs = HashMap::new();
                    subs.insert(single.clone(), if val { 1.0 } else { 0.0 });
                    constraint
                        .reduce(&subs)
                        .trivially_eval()
                        .expect("Reduced constraint should be trivial")
                };
                let one = LinExpr::constant(1.0);
                let zero = LinExpr::constant(0.0);
                let v = LinExpr::var(indicator);
                let (b_true, b_false) = (eval(true), eval(false));
                let orig = LinExpr::var(single);
                let c = match (b_true, b_false) {
                    (true, true) => v.eq(&one),
                    (false, false) => v.eq(&zero),
                    (true, false) => v.eq(&orig),
                    (false, true) => v.eq(&(&one - &orig)),
                };
                return Ok(vec![c]);
            }
            // Non-binary single variable falls through to the
            // generic encoding.
        }
        _ => {}
    }

    // ----- Generic encoding (port of collo-ml lines 521-577) -----
    match constraint.get_symbol() {
        EqSymbol::LessThan => {
            // Discreteness check on every referenced variable.
            for v in constraint.variable_refs() {
                let kind = lookup_kind(v, kinds, factory)
                    .ok_or_else(|| ReifyError::UndeclaredVariable(v.clone()))?;
                if !kind.is_integer() {
                    return Err(ReifyError::NonDiscreteVariable(v.clone()));
                }
            }
            let lin_expr = constraint.get_lhs().clone();
            // Compute the LHS range using the same kind resolver.
            let range = lin_expr
                .compute_range_with(|v: &ExtraVar<B, E>| lookup_kind(v, kinds, factory).cloned());
            let min = *range.start();
            let max = *range.end();
            if !min.is_finite() || !max.is_finite() {
                return Err(ReifyError::InfiniteLhsRange);
            }
            let one = LinExpr::constant(1.0);
            let epsilon = LinExpr::constant(0.1);
            let v = LinExpr::var(indicator);
            Ok(vec![
                lin_expr.leq(&(max * (&one - &v) + &epsilon)),
                lin_expr.geq(&((min - 1.0) * &v + &one - &epsilon)),
            ])
        }
        EqSymbol::Equals => {
            // For equality, split lin_expr === 0 into
            //   lin_expr <== 0  AND  lin_expr >== 0
            // and AND-combine the two reified halves.
            let v1 = factory.new_helper(Variable::binary());
            let v2 = factory.new_helper(Variable::binary());
            let lin_expr = constraint.get_lhs().clone();
            let zero = LinExpr::constant(0.0);
            let c1 = lin_expr.leq(&zero);
            let c2 = lin_expr.geq(&zero);
            let mut out = reify_single(&c1, v1.clone(), factory, kinds)?;
            out.extend(reify_single(&c2, v2.clone(), factory, kinds)?);
            let v1e = LinExpr::var(v1);
            let v2e = LinExpr::var(v2);
            let v = LinExpr::var(indicator);
            out.push(v.leq(&v1e));
            out.push(v.leq(&v2e));
            out.push((&v1e + &v2e).leq(&(&v + &LinExpr::constant(1.0))));
            Ok(out)
        }
    }
}

// ---------------------------------------------------------------------------
// IntConstraintBundle::reify
// ---------------------------------------------------------------------------

impl<'m, B, E, C, Db, Err> IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData + 'm,
    E: UsableData + 'm,
    C: UsableData,
    Db: 'm,
    Err: Debug + 'static + From<ReifyError<B, E>>,
{
    /// Reify the bundle's `constraints` field into a binary
    /// indicator named `var`. Returns a new
    /// [`ConstraintBundle`] whose:
    ///
    /// - `constraints` is empty (the linearization lives inside
    ///   the new extra's body, not at top level),
    /// - `objectives` is a pass-through copy of self's,
    /// - `extras` is `self.extras` unchanged plus one new
    ///   `ExtraEntry` for `var` with kind binary.
    ///
    /// The reification closure cannot fail at the call site;
    /// any [`ReifyError`] surfaces lazily as
    /// `BuildError::ExtraError(var, _)` when the modeler
    /// expands the new extra. Hence the `Err: From<ReifyError>`
    /// bound — only callers that actually invoke `reify` need
    /// to satisfy it.
    pub fn reify(self, var: E) -> ConstraintBundle<'m, B, E, C, Db, Err> {
        // Capture inputs by move into the closure.
        let int_constraints: Vec<Constraint<ExtraVar<B, E>>> = self
            .constraints
            .into_iter()
            .map(|(c, _desc)| {
                // Drop the int wrapping and lift Var → ExtraVar.
                c.into_constraint().transmute(|v| ExtraVar::from(v.clone()))
            })
            .collect();

        let entry = ExtraEntry::new(
            var.clone(),
            Variable::binary(),
            move |_db, factory, kinds, e| {
                let int_constraints = int_constraints; // move
                Box::pin(async move {
                    let result =
                        reify_and_inner(&int_constraints, ExtraVar::Extra(e), factory, kinds);
                    result.map_err(Err::from)
                })
            },
        );

        let mut extras = self.extras;
        extras.push(entry);

        ConstraintBundle {
            constraints: Vec::new(),
            objectives: self.objectives,
            extras,
            _phantom: PhantomData,
        }
    }
}
