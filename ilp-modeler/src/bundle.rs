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
use collomatique_ilp::{
    Constraint, IntConstraint, LinExpr, Objective, ObjectiveSense, UsableData, Variable,
};

use crate::{DefineFn, DuplicateExtra, ExtraVar, HelperFactory, Modeler, Var, VarContext};

// ---------------------------------------------------------------------------
// ExtraEntry
// ---------------------------------------------------------------------------

/// One declare-extra worth of arguments, stored as a value so
/// that it can sit inside a bundle until the bundle is applied.
/// The extra's name is not stored here — it serves as the key
/// in the bundle's `HashMap<E, ExtraEntry>`.
pub struct ExtraEntry<'m, B, E, Env, Err>
where
    B: UsableData,
    E: UsableData,
{
    kind: Variable,
    define: Box<DefineFn<'m, B, E, Env, Err>>,
}

impl<'m, B, E, Env, Err> ExtraEntry<'m, B, E, Env, Err>
where
    B: UsableData,
    E: UsableData,
{
    /// Build an entry from a closure of the same shape as
    /// [`Modeler::declare_extra`]. The closure is boxed under
    /// the proper HRTB so callers don't have to wrangle the
    /// `dyn` lifetimes themselves.
    pub fn new<F>(kind: Variable, define: F) -> Self
    where
        F: for<'a> FnOnce(
                &'a mut crate::HelperFactory<B, E>,
                &'a crate::VarContext<'a, B, E, Env>,
                E,
            ) -> Result<Vec<Constraint<crate::ExtraVar<B, E>>>, Err>
            + Send
            + 'm,
    {
        ExtraEntry {
            kind,
            define: Box::new(define),
        }
    }

    /// The extra's variable kind.
    pub fn kind(&self) -> &Variable {
        &self.kind
    }

    /// Consume the entry and return its parts.
    pub(crate) fn into_parts(self) -> (Variable, Box<DefineFn<'m, B, E, Env, Err>>) {
        (self.kind, self.define)
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
pub struct ConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    constraints: Vec<(Constraint<Var<B, E>>, C)>,
    objectives: Vec<(f64, Objective<Var<B, E>>)>,
    extras: HashMap<E, ExtraEntry<'m, B, E, Env, Err>>,
    _phantom: PhantomData<Env>,
}

impl<'m, B, E, C, Env, Err> Default for ConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    fn default() -> Self {
        ConstraintBundle {
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'m, B, E, C, Env, Err> ConstraintBundle<'m, B, E, C, Env, Err>
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

    /// Construct from a list of weighted objectives.
    pub fn from_objectives<I>(objectives: I) -> Self
    where
        I: IntoIterator<Item = (f64, Objective<Var<B, E>>)>,
    {
        ConstraintBundle {
            objectives: objectives.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Add a constraint with description.
    pub fn with_constraint(mut self, constraint: Constraint<Var<B, E>>, desc: C) -> Self {
        self.constraints.push((constraint, desc));
        self
    }

    /// Add a weighted objective.
    pub fn with_objective(mut self, coef: f64, objective: Objective<Var<B, E>>) -> Self {
        self.objectives.push((coef, objective));
        self
    }

    /// Add a weighted minimization objective.
    pub fn with_minimize(self, coef: f64, expr: LinExpr<Var<B, E>>) -> Self {
        self.with_objective(coef, Objective::new(expr, ObjectiveSense::Minimize))
    }

    /// Add a weighted maximization objective.
    pub fn with_maximize(self, coef: f64, expr: LinExpr<Var<B, E>>) -> Self {
        self.with_objective(coef, Objective::new(expr, ObjectiveSense::Maximize))
    }

    /// Add an extra-variable definition. Returns
    /// [`DuplicateExtra`] if an extra with the same name
    /// already exists in this bundle.
    pub fn with_extra(
        mut self,
        name: E,
        entry: ExtraEntry<'m, B, E, Env, Err>,
    ) -> Result<Self, DuplicateExtra<E>> {
        if self.extras.contains_key(&name) {
            return Err(DuplicateExtra(name));
        }
        self.extras.insert(name, entry);
        Ok(self)
    }

    /// Read-only access to the constraints.
    pub fn constraints(&self) -> &[(Constraint<Var<B, E>>, C)] {
        &self.constraints
    }

    /// Read-only access to the objectives.
    pub fn objectives(&self) -> &[(f64, Objective<Var<B, E>>)] {
        &self.objectives
    }

    /// Read-only access to the extra definitions.
    pub fn extras(&self) -> &HashMap<E, ExtraEntry<'m, B, E, Env, Err>> {
        &self.extras
    }

    /// Append all of `other`'s entries into `self`. Constraints,
    /// objectives, and extras are combined; no arithmetic.
    /// Returns [`DuplicateExtra`] if any extra in `other` has
    /// the same name as one already in `self`.
    pub fn merge(mut self, other: Self) -> Result<Self, DuplicateExtra<E>> {
        for key in other.extras.keys() {
            if self.extras.contains_key(key) {
                return Err(DuplicateExtra(key.clone()));
            }
        }
        self.constraints.extend(other.constraints);
        self.objectives.extend(other.objectives);
        self.extras.extend(other.extras);
        Ok(self)
    }
}

// ---------------------------------------------------------------------------
// IntConstraintBundle
// ---------------------------------------------------------------------------

/// Same shape as [`ConstraintBundle`] but the eager constraints
/// are [`IntConstraint`] rather than [`Constraint`]. The
/// objective stays as [`Objective`] (no `IntObjective`).
pub struct IntConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    constraints: Vec<(IntConstraint<Var<B, E>>, C)>,
    objectives: Vec<(f64, Objective<Var<B, E>>)>,
    extras: HashMap<E, ExtraEntry<'m, B, E, Env, Err>>,
    _phantom: PhantomData<Env>,
}

impl<'m, B, E, C, Env, Err> Default for IntConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    fn default() -> Self {
        IntConstraintBundle {
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'m, B, E, C, Env, Err> IntConstraintBundle<'m, B, E, C, Env, Err>
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

    /// Construct from a list of weighted objectives.
    pub fn from_objectives<I>(objectives: I) -> Self
    where
        I: IntoIterator<Item = (f64, Objective<Var<B, E>>)>,
    {
        IntConstraintBundle {
            objectives: objectives.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Create a bundle containing a single infeasible constraint.
    pub fn infeasible(desc: C) -> Self {
        Self::new().with_infeasible(desc)
    }

    /// Add a constraint with description.
    pub fn with_constraint(mut self, constraint: IntConstraint<Var<B, E>>, desc: C) -> Self {
        self.constraints.push((constraint, desc));
        self
    }

    /// Add an always-false constraint with description.
    pub fn with_infeasible(self, desc: C) -> Self {
        self.with_constraint(IntConstraint::infeasible(), desc)
    }

    /// Add a weighted objective.
    pub fn with_objective(mut self, coef: f64, objective: Objective<Var<B, E>>) -> Self {
        self.objectives.push((coef, objective));
        self
    }

    /// Add a weighted minimization objective.
    pub fn with_minimize(self, coef: f64, expr: LinExpr<Var<B, E>>) -> Self {
        self.with_objective(coef, Objective::new(expr, ObjectiveSense::Minimize))
    }

    /// Add a weighted maximization objective.
    pub fn with_maximize(self, coef: f64, expr: LinExpr<Var<B, E>>) -> Self {
        self.with_objective(coef, Objective::new(expr, ObjectiveSense::Maximize))
    }

    /// Add an extra-variable definition. Returns
    /// [`DuplicateExtra`] if an extra with the same name
    /// already exists in this bundle.
    pub fn with_extra(
        mut self,
        name: E,
        entry: ExtraEntry<'m, B, E, Env, Err>,
    ) -> Result<Self, DuplicateExtra<E>> {
        if self.extras.contains_key(&name) {
            return Err(DuplicateExtra(name));
        }
        self.extras.insert(name, entry);
        Ok(self)
    }

    /// Read-only access to the constraints.
    pub fn constraints(&self) -> &[(IntConstraint<Var<B, E>>, C)] {
        &self.constraints
    }

    /// Read-only access to the objectives.
    pub fn objectives(&self) -> &[(f64, Objective<Var<B, E>>)] {
        &self.objectives
    }

    /// Read-only access to the extra definitions.
    pub fn extras(&self) -> &HashMap<E, ExtraEntry<'m, B, E, Env, Err>> {
        &self.extras
    }

    /// Append all of `other`'s entries into `self`. Constraints,
    /// objectives, and extras are combined; no arithmetic.
    /// Returns [`DuplicateExtra`] if any extra in `other` has
    /// the same name as one already in `self`.
    pub fn merge(mut self, other: Self) -> Result<Self, DuplicateExtra<E>> {
        for key in other.extras.keys() {
            if self.extras.contains_key(key) {
                return Err(DuplicateExtra(key.clone()));
            }
        }
        self.constraints.extend(other.constraints);
        self.objectives.extend(other.objectives);
        self.extras.extend(other.extras);
        Ok(self)
    }

    /// Drop the int wrapping. Each [`IntConstraint`] is unwrapped
    /// into its underlying [`Constraint`]; objectives and extras
    /// pass through unchanged.
    pub fn into_general(self) -> ConstraintBundle<'m, B, E, C, Env, Err> {
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

impl<'m, B, E, C, Env, Err> Modeler<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + Send + 'static,
{
    /// Apply a bundle to the modeler: push every constraint,
    /// push every weighted objective, and declare every extra.
    /// Equivalent to repeated `add_constraint` / `add_objective`
    /// / `declare_extra` calls in field-then-vec order.
    pub fn apply_bundle(
        &mut self,
        bundle: ConstraintBundle<'m, B, E, C, Env, Err>,
    ) -> Result<(), DuplicateExtra<E>> {
        for (c, desc) in bundle.constraints {
            self.add_constraint(c, desc);
        }
        for (coef, obj) in bundle.objectives {
            self.add_objective(coef, obj);
        }
        for (name, entry) in bundle.extras {
            let (kind, define) = entry.into_parts();
            self.declare_extra_boxed(name, kind, define)?;
        }
        Ok(())
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
    /// is not declared as integer in the current `VarContext`.
    /// Reification with epsilon-slack only soundly works on
    /// discrete (integer) variables.
    #[error("variable {0:?} is not discrete (integer); cannot reify")]
    NonDiscreteVariable(ExtraVar<B, E>),
    /// A variable referenced in the constraints being reified
    /// has no kind known to the current `VarContext` /
    /// `HelperFactory`.
    #[error("variable {0:?} has no declared kind; cannot reify")]
    UndeclaredVariable(ExtraVar<B, E>),
    /// The LHS of a constraint being reified has an unbounded
    /// (infinite) range. Big-M linearization needs finite bounds.
    #[error("constraint LHS has an unbounded range; cannot reify")]
    InfiniteLhsRange,
    /// A fixed variable substituted into a reified constraint has
    /// a non-integer value. The big-M epsilon encoding requires
    /// all values to be integers.
    #[error("fixed variable {variable:?} has non-integer value {value}; cannot reify")]
    NonIntegerFixValue {
        variable: ExtraVar<B, E>,
        value: f64,
    },
}

/// Errors detectable eagerly at the [`IntConstraintBundle::reify`]
/// or [`IntConstraintBundle::reify_with_epsilon`] call site,
/// as opposed to [`ReifyError`] which surfaces lazily at build
/// time.
#[derive(Debug, thiserror::Error)]
pub enum EagerReifyError<E: UsableData> {
    /// The epsilon value is out of the valid range `(0, 1)`.
    #[error("epsilon {0} is out of range (must be 0 < epsilon < 1)")]
    InvalidEpsilon(f64),
    /// The reification variable name conflicts with an existing
    /// extra already queued in the bundle.
    #[error("reification variable `{0:?}` conflicts with an existing extra in the bundle")]
    DuplicateVariable(E),
}

/// Look up the kind of any [`ExtraVar`] reference reachable
/// from inside an extra-definition closure. Helpers are routed
/// through the factory; base/extra refs through `kinds`.
fn lookup_kind<'a, B, E, Env>(
    var: &ExtraVar<B, E>,
    kinds: &'a VarContext<'a, B, E, Env>,
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
pub(crate) fn reify_and_inner<B, E, Env>(
    constraints: &[Constraint<ExtraVar<B, E>>],
    indicator: ExtraVar<B, E>,
    factory: &mut HelperFactory<B, E>,
    kinds: &VarContext<'_, B, E, Env>,
    epsilon: f64,
) -> Result<Vec<Constraint<ExtraVar<B, E>>>, ReifyError<B, E>>
where
    B: UsableData,
    E: UsableData,
{
    debug_assert!(epsilon > 0.0 && epsilon < 1.0);

    // Empty list: the AND is trivially true; pin the indicator
    // to 1.
    if constraints.is_empty() {
        let v = LinExpr::var(indicator);
        return Ok(vec![v.eq(&LinExpr::constant(1.0))]);
    }
    // Single constraint: skip the AND-combine encoding.
    if constraints.len() == 1 {
        return reify_single(&constraints[0], indicator, factory, kinds, epsilon);
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
        output.extend(reify_single(c, h, factory, kinds, epsilon)?);
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
///
/// `epsilon` must satisfy `0 < epsilon < 1`. It is used as slack
/// in the big-M linearization. Correctness relies on all
/// referenced variables being integer: with integrality, an
/// integer expression `<= epsilon` is equivalent to `<= 0`.
pub(crate) fn reify_single<B, E, Env>(
    constraint: &Constraint<ExtraVar<B, E>>,
    indicator: ExtraVar<B, E>,
    factory: &mut HelperFactory<B, E>,
    kinds: &VarContext<'_, B, E, Env>,
    epsilon: f64,
) -> Result<Vec<Constraint<ExtraVar<B, E>>>, ReifyError<B, E>>
where
    B: UsableData,
    E: UsableData,
{
    debug_assert!(epsilon > 0.0 && epsilon < 1.0);

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
            let eps = LinExpr::constant(epsilon);
            let v = LinExpr::var(indicator);
            Ok(vec![
                lin_expr.leq(&(max * (&one - &v) + &eps)),
                lin_expr.geq(&((min - 1.0) * &v + &one - &eps)),
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
            let mut out = reify_single(&c1, v1.clone(), factory, kinds, epsilon)?;
            out.extend(reify_single(&c2, v2.clone(), factory, kinds, epsilon)?);
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

impl<'m, B, E, C, Env, Err> IntConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData + 'm,
    E: UsableData + 'm,
    C: UsableData + 'm,
    Env: Sync + 'm,
    Err: Debug + Send + 'static + From<ReifyError<B, E>>,
{
    /// Add a lazy reified extra to this bundle.
    ///
    /// `build_constraints` is only called during
    /// [`Modeler::build`] if the extra is actually referenced.
    /// At that point, the returned `IntConstraint`s are
    /// transmuted, fixed, and linearized via big-M reification.
    ///
    /// `epsilon` must satisfy `0 < epsilon < 1`. Returns `Err`
    /// eagerly if `epsilon` is out of range or if `var` conflicts
    /// with an extra already in the bundle.
    pub fn and_reified_with_epsilon<F>(
        mut self,
        var: E,
        build_constraints: F,
        epsilon: f64,
    ) -> Result<Self, EagerReifyError<E>>
    where
        F: FnOnce() -> Vec<IntConstraint<Var<B, E>>> + Send + 'm,
    {
        if !(epsilon > 0.0 && epsilon < 1.0) {
            return Err(EagerReifyError::InvalidEpsilon(epsilon));
        }
        if self.extras.contains_key(&var) {
            return Err(EagerReifyError::DuplicateVariable(var));
        }

        let entry = ExtraEntry::new(
            Variable::binary(),
            move |factory: &mut HelperFactory<B, E>, ctx, e| {
                let int_constraints = build_constraints();
                let constraints: Vec<Constraint<ExtraVar<B, E>>> = int_constraints
                    .into_iter()
                    .map(|c| c.into_constraint().transmute(|v| ExtraVar::from(v.clone())))
                    .collect();
                let (reduced, fixes) = ctx.fix_constraints(constraints);

                for (b, &val) in &fixes {
                    if val != val.round() {
                        return Err(Err::from(ReifyError::NonIntegerFixValue {
                            variable: ExtraVar::Base(b.clone()),
                            value: val,
                        }));
                    }
                }

                reify_and_inner(&reduced, ExtraVar::Extra(e), factory, ctx, epsilon)
                    .map_err(Err::from)
            },
        );

        self.extras.insert(var, entry);
        Ok(self)
    }

    /// Add a lazy reified extra with the default epsilon of 0.1.
    pub fn and_reified<F>(self, var: E, build_constraints: F) -> Result<Self, EagerReifyError<E>>
    where
        F: FnOnce() -> Vec<IntConstraint<Var<B, E>>> + Send + 'm,
    {
        self.and_reified_with_epsilon(var, build_constraints, 0.1)
    }

    /// Construct a bundle containing a single lazy reified extra
    /// with a custom epsilon.
    pub fn with_reified_with_epsilon<F>(
        var: E,
        build_constraints: F,
        epsilon: f64,
    ) -> Result<Self, EagerReifyError<E>>
    where
        F: FnOnce() -> Vec<IntConstraint<Var<B, E>>> + Send + 'm,
    {
        Self::new().and_reified_with_epsilon(var, build_constraints, epsilon)
    }

    /// Construct a bundle containing a single lazy reified extra
    /// with the default epsilon of 0.1.
    pub fn with_reified<F>(var: E, build_constraints: F) -> Result<Self, EagerReifyError<E>>
    where
        F: FnOnce() -> Vec<IntConstraint<Var<B, E>>> + Send + 'm,
    {
        Self::new().and_reified(var, build_constraints)
    }

    /// Reify the bundle's eager `constraints` into a binary
    /// indicator named `var` with a custom epsilon.
    ///
    /// Returns a new bundle whose `constraints` is empty (the
    /// linearization lives inside the new extra's closure),
    /// `objectives` is a pass-through, and `extras` is
    /// `self.extras` plus one new binary extra for `var`.
    pub fn reify_with_epsilon(
        self,
        var: E,
        epsilon: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerReifyError<E>> {
        let constraints = self.constraints;
        IntConstraintBundle {
            constraints: Vec::new(),
            objectives: self.objectives,
            extras: self.extras,
            _phantom: PhantomData,
        }
        .and_reified_with_epsilon(
            var,
            move || constraints.into_iter().map(|(c, _desc)| c).collect(),
            epsilon,
        )
    }

    /// Reify the bundle's eager `constraints` into a binary
    /// indicator named `var` using the default epsilon of 0.1.
    pub fn reify(
        self,
        var: E,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerReifyError<E>> {
        self.reify_with_epsilon(var, 0.1)
    }
}

// ---------------------------------------------------------------------------
// Objectification
// ---------------------------------------------------------------------------

/// Errors detectable eagerly at the
/// [`ConstraintBundle::objectify`] or
/// [`ConstraintBundle::objectify_with_balance`] call site.
#[derive(Debug, thiserror::Error)]
pub enum EagerObjectifyError<E: UsableData> {
    /// The penalty variable name conflicts with an existing extra
    /// already queued in the bundle.
    #[error("objectify variable `{0:?}` conflicts with an existing extra in the bundle")]
    DuplicateVariable(E),
    /// The bundle has no constraints to objectify.
    #[error("cannot objectify an empty constraint set")]
    EmptyConstraints,
    /// The balance parameter is out of the valid range `[0, 1]`.
    #[error("balance {0} is out of range (must be 0 <= alpha <= 1)")]
    InvalidBalance(f64),
}

/// Link one constraint's violation to a target variable.
///
/// For `lhs <= 0`: returns `[lhs <= target]`.
/// For `lhs == 0`: returns `[lhs <= target, lhs >= -target]`.
///
/// In both cases, minimizing `target` drives it toward the
/// violation magnitude.
fn objectify_single<B, E>(
    constraint: &Constraint<ExtraVar<B, E>>,
    target: ExtraVar<B, E>,
) -> Vec<Constraint<ExtraVar<B, E>>>
where
    B: UsableData,
    E: UsableData,
{
    let lhs = constraint.get_lhs().clone();
    let target_expr = LinExpr::var(target);
    match constraint.get_symbol() {
        EqSymbol::LessThan => {
            vec![lhs.leq(&target_expr)]
        }
        EqSymbol::Equals => {
            vec![lhs.leq(&target_expr), lhs.geq(&(-&target_expr))]
        }
    }
}

/// Build the full objectification for a set of constraints.
///
/// `alpha` in `[0, 1]` controls the balance between L1 (sum,
/// alpha=0) and L∞ (minimax, alpha=1). For a single constraint
/// alpha has no effect.
fn objectify_inner<B, E>(
    constraints: &[Constraint<ExtraVar<B, E>>],
    penalty: ExtraVar<B, E>,
    factory: &mut HelperFactory<B, E>,
    alpha: f64,
) -> Vec<Constraint<ExtraVar<B, E>>>
where
    B: UsableData,
    E: UsableData,
{
    debug_assert!(alpha >= 0.0 && alpha <= 1.0);
    debug_assert!(!constraints.is_empty());

    // Single constraint: penalty IS the lambda, alpha irrelevant.
    if constraints.len() == 1 {
        return objectify_single(&constraints[0], penalty);
    }

    let n = constraints.len() as f64;
    let mut output = Vec::new();

    if alpha == 1.0 {
        // Pure L∞: penalty IS the global bound. Link each
        // constraint's violation directly to penalty, no
        // per-constraint helpers needed.
        for c in constraints {
            output.extend(objectify_single(c, penalty.clone()));
        }
        return output;
    }

    // Create per-constraint lambdas (needed for alpha < 1).
    let mut lambdas: Vec<ExtraVar<B, E>> = Vec::with_capacity(constraints.len());
    for c in constraints {
        let lambda = factory.new_helper(Variable::non_negative());
        lambdas.push(lambda.clone());
        output.extend(objectify_single(c, lambda));
    }

    if alpha == 0.0 {
        // Pure L1: no global Lambda needed.
        // penalty = (1/n) * sum(lambda_i)
        let mut sum_expr: LinExpr<ExtraVar<B, E>> = LinExpr::constant(0.0);
        for l in &lambdas {
            sum_expr = sum_expr + LinExpr::var(l.clone());
        }
        output.push(LinExpr::var(penalty).eq(&((1.0 / n) * sum_expr)));
        return output;
    }

    // General case (0 < alpha < 1): both lambdas and global bound.
    let lambda_global = factory.new_helper(Variable::non_negative());
    let lambda_global_expr = LinExpr::var(lambda_global);
    for l in &lambdas {
        output.push(LinExpr::var(l.clone()).leq(&lambda_global_expr));
    }

    // penalty = alpha * Lambda_global + (1-alpha)/n * sum(lambda_i)
    let mut penalty_expr: LinExpr<ExtraVar<B, E>> = alpha * &lambda_global_expr;
    let per_lambda_weight = (1.0 - alpha) / n;
    for l in &lambdas {
        penalty_expr = penalty_expr + per_lambda_weight * LinExpr::var(l.clone());
    }

    output.push(LinExpr::var(penalty).eq(&penalty_expr));
    output
}

// ---------------------------------------------------------------------------
// ConstraintBundle::objectify
// ---------------------------------------------------------------------------

impl<'m, B, E, C, Env, Err> ConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData + 'm,
    E: UsableData + 'm,
    C: UsableData,
    Env: Sync + 'm,
    Err: Debug + Send + 'static,
{
    /// Convert the bundle's constraints into a penalty variable
    /// that captures how much those constraints are violated.
    ///
    /// `alpha` in `[0, 1]` controls the balance between L1 (sum
    /// of violations, alpha=0) and L∞ (minimax / worst violation,
    /// alpha=1). `coef` scales the minimize objective added for
    /// the penalty variable.
    ///
    /// Consumes `self`. Returns a new [`ConstraintBundle`] whose:
    /// - `constraints` is empty (absorbed into the extra's closure),
    /// - `extras` is `self.extras` plus one new extra named `var`
    ///   with kind `non_negative`,
    /// - `objectives` is `self.objectives` plus a minimize term
    ///   for `var` weighted by `coef`.
    pub fn objectify_with_balance_and_coef(
        self,
        var: E,
        alpha: f64,
        coef: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        if self.constraints.is_empty() {
            return Err(EagerObjectifyError::EmptyConstraints);
        }
        if !(alpha >= 0.0 && alpha <= 1.0) {
            return Err(EagerObjectifyError::InvalidBalance(alpha));
        }
        if self.extras.contains_key(&var) {
            return Err(EagerObjectifyError::DuplicateVariable(var));
        }

        let constraints: Vec<Constraint<ExtraVar<B, E>>> = self
            .constraints
            .into_iter()
            .map(|(c, _desc)| c.transmute(|v| ExtraVar::from(v.clone())))
            .collect();

        let entry = ExtraEntry::new(
            Variable::non_negative(),
            move |factory: &mut HelperFactory<B, E>, ctx, e| {
                let (reduced, _fixes) = ctx.fix_constraints(constraints);
                Ok(objectify_inner(
                    &reduced,
                    ExtraVar::Extra(e),
                    factory,
                    alpha,
                ))
            },
        );

        let mut extras = self.extras;
        extras.insert(var.clone(), entry);

        let mut objectives = self.objectives;
        objectives.push((
            coef,
            Objective::new(LinExpr::var(Var::Extra(var)), ObjectiveSense::Minimize),
        ));

        Ok(IntConstraintBundle {
            constraints: Vec::new(),
            objectives,
            extras,
            _phantom: PhantomData,
        })
    }

    /// Objectify with a custom balance and default coefficient 1.0.
    pub fn objectify_with_balance(
        self,
        var: E,
        alpha: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.objectify_with_balance_and_coef(var, alpha, 1.0)
    }

    /// Objectify with a custom coefficient and default balance 0.5.
    pub fn objectify_with_coef(
        self,
        var: E,
        coef: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.objectify_with_balance_and_coef(var, 0.5, coef)
    }

    /// Objectify with default balance 0.5 and coefficient 1.0.
    pub fn objectify(
        self,
        var: E,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.objectify_with_balance_and_coef(var, 0.5, 1.0)
    }
}

// ---------------------------------------------------------------------------
// IntConstraintBundle::objectify
// ---------------------------------------------------------------------------

impl<'m, B, E, C, Env, Err> IntConstraintBundle<'m, B, E, C, Env, Err>
where
    B: UsableData + 'm,
    E: UsableData + 'm,
    C: UsableData,
    Env: Sync + 'm,
    Err: Debug + Send + 'static,
{
    /// Convenience wrapper: drops the int wrapping and delegates
    /// to [`ConstraintBundle::objectify_with_balance_and_coef`].
    pub fn objectify_with_balance_and_coef(
        self,
        var: E,
        alpha: f64,
        coef: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.into_general()
            .objectify_with_balance_and_coef(var, alpha, coef)
    }

    /// Convenience wrapper: drops the int wrapping and delegates
    /// to [`ConstraintBundle::objectify_with_balance`].
    pub fn objectify_with_balance(
        self,
        var: E,
        alpha: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.objectify_with_balance_and_coef(var, alpha, 1.0)
    }

    /// Convenience wrapper: drops the int wrapping and delegates
    /// to [`ConstraintBundle::objectify_with_coef`].
    pub fn objectify_with_coef(
        self,
        var: E,
        coef: f64,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.objectify_with_balance_and_coef(var, 0.5, coef)
    }

    /// Convenience wrapper: drops the int wrapping and delegates
    /// to [`ConstraintBundle::objectify`].
    pub fn objectify(
        self,
        var: E,
    ) -> Result<IntConstraintBundle<'m, B, E, C, Env, Err>, EagerObjectifyError<E>> {
        self.objectify_with_balance_and_coef(var, 0.5, 1.0)
    }
}
