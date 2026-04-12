//! Lazy ILP modeler built on top of [`collomatique_ilp`].
//!
//! This crate provides a [`Modeler`] that lets callers register
//! base variables, user constraints/objectives and *extras* —
//! named auxiliary variables whose definition (a list of
//! constraints, possibly using freshly minted helper variables)
//! is supplied as a closure and only evaluated lazily, at
//! [`Modeler::build`] time.
//!
//! This file implements the lazy modeler core
//! only. Reification helpers are intentionally out of scope.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use collomatique_ilp::{
    Constraint, LinExpr, Objective, ObjectiveSense, Problem, ProblemBuilder, UsableData, Variable,
};

pub mod bundle;
pub use bundle::{
    ConstraintBundle, EagerObjectifyError, EagerReifyError, ExtraEntry, IntConstraintBundle,
    ReifyError,
};

/// Boxed future returned by extra-definition closures.
///
/// Aliased to avoid pulling in `futures` as a dependency.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

// ---------------------------------------------------------------------------
// Variable references
// ---------------------------------------------------------------------------

/// Variable reference used in user-visible constraints and
/// objectives. Either an externally-declared base variable (`B`)
/// or a user-named extra (`E`). Helpers do not appear here: the
/// user never manipulates them directly.
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum Var<B, E> {
    Base(B),
    Extra(E),
}

/// Opaque helper id. The inner field is private — the only public
/// constructor is [`HelperFactory::new_helper`]. Combined with the
/// smuggling check in [`Modeler::build`] this gives us both
/// forgery and smuggling resistance.
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct HelperId(u64);

/// Variable reference available *inside* an extra-definition
/// closure. Adds a helper case to [`Var`].
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum ExtraVar<B, E> {
    Base(B),
    Extra(E),
    Helper(HelperId),
}

impl<B, E> From<Var<B, E>> for ExtraVar<B, E> {
    fn from(v: Var<B, E>) -> Self {
        match v {
            Var::Base(b) => ExtraVar::Base(b),
            Var::Extra(e) => ExtraVar::Extra(e),
        }
    }
}

/// The fully-qualified variable type fed into the underlying
/// [`Problem`]. Public so callers can map a solved assignment
/// back to their own base/extra names; in practice most users
/// only care about the [`InternalVar::Base`] case.
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum InternalVar<B, E> {
    Base(B),
    Extra(E),
    Helper { owner: E, id: HelperId },
}

impl<B, E> From<Var<B, E>> for InternalVar<B, E> {
    fn from(v: Var<B, E>) -> Self {
        match v {
            Var::Base(b) => InternalVar::Base(b),
            Var::Extra(e) => InternalVar::Extra(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Constraint provenance
// ---------------------------------------------------------------------------

/// How each constraint reaching the underlying [`Problem`] is
/// attributed.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ConstraintSource<E, C> {
    /// A constraint added directly by the user.
    User(C),
    /// A constraint produced by the definition closure of
    /// `extra`. `index` is the constraint's position within the
    /// vec returned by that closure (deterministic only if the
    /// closure itself is).
    DefiningExtra { extra: E, index: usize },
}

// ---------------------------------------------------------------------------
// Helper factory
// ---------------------------------------------------------------------------

/// Sole source of [`HelperId`]s. Each extra-definition closure
/// receives a fresh `HelperFactory` bound to its own expansion.
pub struct HelperFactory<B, E> {
    next: u64,
    declared: HashMap<HelperId, Variable>,
    _phantom: PhantomData<(B, E)>,
}

impl<B, E> Default for HelperFactory<B, E> {
    fn default() -> Self {
        HelperFactory {
            next: 0,
            declared: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<B, E> HelperFactory<B, E> {
    /// Mint a fresh helper of the given kind and return its
    /// [`ExtraVar::Helper`]. The returned id is the only way a
    /// closure can name the new helper.
    pub fn new_helper(&mut self, kind: Variable) -> ExtraVar<B, E> {
        let id = HelperId(self.next);
        self.next += 1;
        self.declared.insert(id.clone(), kind);
        ExtraVar::Helper(id)
    }

    /// Look up the kind of a helper that was minted by *this*
    /// factory. Returns `None` for unknown ids (e.g. ones cloned
    /// in from another factory's closure — the same condition
    /// the smuggling check in `build` catches).
    pub fn kind_of(&self, id: &HelperId) -> Option<&Variable> {
        self.declared.get(id)
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Attempted to declare two extras with the same name.
#[derive(Debug, Clone, thiserror::Error)]
#[error("extra `{0:?}` declared more than once")]
pub struct DuplicateExtra<E: UsableData>(pub E);

/// Errors from [`Modeler::fix_variables`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum FixError<B: UsableData> {
    /// The variable is a declared base variable. Fixed variables
    /// must be undeclared — they are named constants, not
    /// decision variables.
    #[error("variable `{0:?}` is a declared base variable and cannot be fixed")]
    DeclaredVariable(B),
    /// The variable has already been fixed (possibly with a
    /// different value).
    #[error("variable `{0:?}` has already been fixed")]
    AlreadyFixed(B),
}

/// A base variable required for reconstruction was not provided.
#[derive(Debug, Clone, thiserror::Error)]
#[error("base variable `{0:?}` is required for reconstruction but no value was provided")]
pub struct ReconstructionError<B: UsableData>(pub B);

/// Errors surfaced by [`Modeler::build`].
#[derive(Debug, thiserror::Error)]
pub enum BuildError<B, E, C, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// A user constraint/objective or another extra's definition
    /// referenced this extra, but it was never declared.
    #[error("extra `{0:?}` was referenced but never declared")]
    UndeclaredExtra(E),
    /// An extra's definition closure returned `Err`.
    #[error("definition of extra `{0:?}` failed: {1:?}")]
    ExtraError(E, Err),
    /// Extras transitively define each other.
    #[error("cyclic extras: {cycle:?}")]
    CyclicExtra { cycle: Vec<E> },
    /// A `HelperId` was used in `used_in`'s constraints but was
    /// never minted by `used_in`'s `HelperFactory` — i.e. a helper
    /// was smuggled in from another closure.
    #[error("helper {id:?} used in `{used_in:?}` was not minted there")]
    HelperLeak { used_in: E, id: HelperId },
    /// The underlying [`ProblemBuilder`] rejected the assembled
    /// problem.
    #[error("underlying ilp problem builder rejected the model: {0}")]
    Ilp(collomatique_ilp::BuildError<InternalVar<B, E>, ConstraintSource<E, C>>),
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// The output of [`Modeler::build`]. Wraps the assembled
/// [`Problem`] and carries data needed to build a
/// reconstruction problem (for computing extra/helper values
/// from a known base-variable assignment).
#[derive(Debug)]
pub struct Model<B, E, C>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    problem: Problem<InternalVar<B, E>, ConstraintSource<E, C>>,
    reconstruction_constraints: Vec<(Constraint<InternalVar<B, E>>, ConstraintSource<E, C>)>,
    reconstruction_variables: HashMap<InternalVar<B, E>, Variable>,
    base_variable_set: HashSet<B>,
}

impl<B, E, C> Model<B, E, C>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Access the assembled problem for solving.
    pub fn problem(&self) -> &Problem<InternalVar<B, E>, ConstraintSource<E, C>> {
        &self.problem
    }

    /// Consume the model and return the assembled problem.
    pub fn into_problem(self) -> Problem<InternalVar<B, E>, ConstraintSource<E, C>> {
        self.problem
    }

    /// Build a reconstruction problem: given base variable values,
    /// produce a [`Problem`] whose solution determines all
    /// extra and helper variable values.
    ///
    /// Every base variable that appears in extra-defining
    /// constraints must be present in `base_values`. Returns
    /// [`ReconstructionError`] if any is missing.
    ///
    /// The returned problem has a trivial objective (minimize 0)
    /// and only contains the defining constraints of extras, with
    /// base variables substituted out.
    pub fn reconstruction_problem(
        &self,
        base_values: &HashMap<B, f64>,
    ) -> Result<Problem<InternalVar<B, E>, ConstraintSource<E, C>>, ReconstructionError<B>> {
        // Completeness check.
        for b in &self.base_variable_set {
            if !base_values.contains_key(b) {
                return Err(ReconstructionError(b.clone()));
            }
        }

        // Convert base values to InternalVar keys.
        let fixes: HashMap<InternalVar<B, E>, f64> = base_values
            .iter()
            .map(|(b, v)| (InternalVar::Base(b.clone()), *v))
            .collect();

        // Reduce reconstruction constraints with base values.
        let reduced_constraints: Vec<_> = self
            .reconstruction_constraints
            .iter()
            .map(|(c, src)| (c.reduce(&fixes), src.clone()))
            .filter(|(c, _)| !c.is_trivially_true())
            .collect();

        // Collect only non-base variables for the reconstruction
        // problem (extras + helpers).
        let recon_vars: HashMap<InternalVar<B, E>, Variable> = self
            .reconstruction_variables
            .iter()
            .filter(|(v, _)| !matches!(v, InternalVar::Base(_)))
            .map(|(v, kind)| (v.clone(), kind.clone()))
            .collect();

        let builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
            ProblemBuilder::new()
                .set_variables(recon_vars)
                .add_constraints(reduced_constraints)
                .set_objective(Objective::new(
                    LinExpr::constant(0.0),
                    ObjectiveSense::Minimize,
                ));
        Ok(builder
            .build()
            .expect("reconstruction problem should always be valid"))
    }
}

// ---------------------------------------------------------------------------
// Modeler
// ---------------------------------------------------------------------------

pub type DefineFn<'m, B, E, Db, Err> = dyn for<'a> FnOnce(
        &'a Db,
        &'a mut HelperFactory<B, E>,
        &'a VarKinds<'a, B, E>,
        E,
    ) -> BoxFuture<'a, Result<Vec<Constraint<ExtraVar<B, E>>>, Err>>
    + 'm;

// ---------------------------------------------------------------------------
// VarKinds
// ---------------------------------------------------------------------------

/// Read-only view of base + extra variable kinds, available
/// inside an extra-definition closure.
///
/// Built fresh by [`Modeler::build`] before each closure call.
/// Reflects every declared base variable and every declared
/// extra (including ones not yet expanded).
///
/// Helper kinds are *not* in this view: they are owned by the
/// [`HelperFactory`] the same closure already has access to,
/// and can be looked up there via [`HelperFactory::kind_of`].
/// This split avoids a borrow conflict between the factory
/// (which mutates its declared set) and the kinds view.
pub struct VarKinds<'a, B, E>
where
    B: UsableData,
    E: UsableData,
{
    base: &'a HashMap<B, Variable>,
    extras: &'a HashMap<E, Variable>,
    fixes: &'a HashMap<B, f64>,
}

impl<'a, B, E> VarKinds<'a, B, E>
where
    B: UsableData,
    E: UsableData,
{
    /// Look up the kind of a base variable.
    pub fn base(&self, b: &B) -> Option<&Variable> {
        self.base.get(b)
    }

    /// Look up the kind of a declared extra variable.
    pub fn extra(&self, e: &E) -> Option<&Variable> {
        self.extras.get(e)
    }

    /// Look up the kind of a base or extra reference. Helper
    /// references go through [`HelperFactory::kind_of`] instead.
    pub fn get(&self, var: &Var<B, E>) -> Option<&Variable> {
        match var {
            Var::Base(b) => self.base.get(b),
            Var::Extra(e) => self.extras.get(e),
        }
    }

    /// Fixed (undeclared) variable values. Closures that inspect
    /// their captured constraints (e.g. reification) should reduce
    /// them with these values before processing.
    pub fn fixes(&self) -> &HashMap<B, f64> {
        self.fixes
    }
}

struct ExtraDef<'m, B, E, Db, Err>
where
    B: UsableData,
    E: UsableData,
{
    kind: Variable,
    define: Box<DefineFn<'m, B, E, Db, Err>>,
}

/// Lazy ILP modeler.
///
/// `'m` is the lifetime everything captured by extra-definition
/// closures must outlive (use `'static` if you don't need
/// borrowing). `B` is the base-variable name type, `E` is the
/// extra-variable name type, `C` is the user constraint
/// description type, `Db` is the shared async context handed to
/// each extra-definition closure, and `Err` is the user-defined
/// error type returned by fallible extra-definition closures.
pub struct Modeler<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    base_vars: HashMap<B, Variable>,
    constraints: Vec<(Constraint<Var<B, E>>, C)>,
    objectives: Vec<(f64, Objective<Var<B, E>>)>,
    extras: HashMap<E, ExtraDef<'m, B, E, Db, Err>>,
    fixed_variables: HashMap<B, f64>,
}

impl<'m, B, E, C, Db, Err> Modeler<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + 'static,
{
    /// The full set of base variables is fixed upfront. No
    /// incremental `declare_base` — if you don't know your base
    /// variables yet, you're not ready to model.
    pub fn new(base_vars: HashMap<B, Variable>) -> Self {
        Modeler {
            base_vars,
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: HashMap::new(),
            fixed_variables: HashMap::new(),
        }
    }

    /// Add a user constraint with a description.
    pub fn add_constraint(&mut self, constraint: Constraint<Var<B, E>>, desc: C) {
        self.constraints.push((constraint, desc));
    }

    /// Add a (weighted) user objective. Multiple calls accumulate;
    /// at build time they are folded into a single underlying
    /// objective via the [`Objective`] arithmetic from
    /// `collomatique_ilp`.
    pub fn add_objective(&mut self, coef: f64, objective: Objective<Var<B, E>>) {
        self.objectives.push((coef, objective));
    }

    /// Add a weighted minimization objective.
    pub fn minimize(&mut self, coef: f64, expr: LinExpr<Var<B, E>>) {
        self.add_objective(coef, Objective::new(expr, ObjectiveSense::Minimize));
    }

    /// Add a weighted maximization objective.
    pub fn maximize(&mut self, coef: f64, expr: LinExpr<Var<B, E>>) {
        self.add_objective(coef, Objective::new(expr, ObjectiveSense::Maximize));
    }

    /// Fix undeclared variables to known values. These must NOT
    /// be declared base variables — they are named constants that
    /// appear in constraint expressions and will be substituted
    /// out during [`Modeler::build`].
    pub fn fix_variables(&mut self, fixes: HashMap<B, f64>) -> Result<(), FixError<B>> {
        for key in fixes.keys() {
            if self.base_vars.contains_key(key) {
                return Err(FixError::DeclaredVariable(key.clone()));
            }
            if self.fixed_variables.contains_key(key) {
                return Err(FixError::AlreadyFixed(key.clone()));
            }
        }
        self.fixed_variables.extend(fixes);
        Ok(())
    }

    /// Register an extra. The definition closure is only invoked
    /// during [`Modeler::build`], and only if the extra is
    /// actually referenced (directly or transitively) by a user
    /// constraint/objective or by another expanded extra's
    /// definition.
    pub fn declare_extra<F>(
        &mut self,
        name: E,
        kind: Variable,
        define: F,
    ) -> Result<(), DuplicateExtra<E>>
    where
        F: for<'a> FnOnce(
                &'a Db,
                &'a mut HelperFactory<B, E>,
                &'a VarKinds<'a, B, E>,
                E,
            )
                -> BoxFuture<'a, Result<Vec<Constraint<ExtraVar<B, E>>>, Err>>
            + 'm,
    {
        if self.extras.contains_key(&name) {
            return Err(DuplicateExtra(name));
        }
        self.extras.insert(
            name,
            ExtraDef {
                kind,
                define: Box::new(define),
            },
        );
        Ok(())
    }

    /// Insert an already-boxed `DefineFn`. Used by
    /// `Modeler::apply_bundle` to drop bundle entries directly
    /// into the extras map without re-wrapping the closure.
    pub(crate) fn declare_extra_boxed(
        &mut self,
        name: E,
        kind: Variable,
        define: Box<DefineFn<'m, B, E, Db, Err>>,
    ) -> Result<(), DuplicateExtra<E>> {
        if self.extras.contains_key(&name) {
            return Err(DuplicateExtra(name));
        }
        self.extras.insert(name, ExtraDef { kind, define });
        Ok(())
    }

    /// Synchronous convenience wrapper around [`declare_extra`].
    /// Most callers' extras don't actually need async; this avoids
    /// forcing them to wrap their definition in
    /// `Box::pin(async move { ... })`.
    pub fn declare_extra_sync<F>(
        &mut self,
        name: E,
        kind: Variable,
        define: F,
    ) -> Result<(), DuplicateExtra<E>>
    where
        F: for<'a> FnOnce(
                &'a mut HelperFactory<B, E>,
                &'a VarKinds<'a, B, E>,
                E,
            ) -> Result<Vec<Constraint<ExtraVar<B, E>>>, Err>
            + 'm,
    {
        // Smuggle the FnOnce into a closure shape that matches
        // declare_extra. We need an Option dance because the inner
        // closure must be FnOnce-callable through a `for<'a>` HRTB.
        let mut slot: Option<F> = Some(define);
        self.declare_extra(name, kind, move |_db, factory, kinds, e| {
            let f = slot.take().expect("define called more than once");
            let result = f(factory, kinds, e);
            Box::pin(async move { result })
        })
    }
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

struct BuildState<B, E, C>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    out_vars: HashMap<InternalVar<B, E>, Variable>,
    out_constraints: Vec<(Constraint<InternalVar<B, E>>, ConstraintSource<E, C>)>,
    expanded: HashSet<E>,
    in_progress: HashSet<E>,
    /// Ordered version of `in_progress`, used to report cycles
    /// with their full chain.
    path: Vec<E>,
}

impl<'m, B, E, C, Db, Err> Modeler<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + 'static,
{
    /// Run the lazy expansion fixpoint and return the assembled
    /// [`Model`]. `db` is passed to every extra-definition
    /// closure that runs.
    pub async fn build(mut self, db: &Db) -> Result<Model<B, E, C>, BuildError<B, E, C, Err>> {
        // Step 0: reduce user constraints/objectives with fixed
        // variable values. Fixed variables are undeclared named
        // constants — substituting them out ensures they don't
        // reach ProblemBuilder (which would reject them).
        let fixes_var: HashMap<Var<B, E>, f64> = self
            .fixed_variables
            .iter()
            .map(|(b, v)| (Var::Base(b.clone()), *v))
            .collect();
        if !fixes_var.is_empty() {
            for (c, _) in &mut self.constraints {
                *c = c.reduce(&fixes_var);
            }
            self.constraints.retain(|(c, _)| !c.is_trivially_true());
            for (_, obj) in &mut self.objectives {
                *obj = obj.reduce(&fixes_var);
            }
        }

        // Step 1: transmute user constraints/objectives to InternalVar.
        let user_constraints: Vec<(Constraint<InternalVar<B, E>>, ConstraintSource<E, C>)> = self
            .constraints
            .iter()
            .map(|(c, desc)| {
                (
                    c.transmute(|v| InternalVar::from(v.clone())),
                    ConstraintSource::User(desc.clone()),
                )
            })
            .collect();

        // Fold weighted objectives into one.
        let folded_obj_var: Objective<Var<B, E>> = if self.objectives.is_empty() {
            Objective::new(LinExpr::constant(0.0), ObjectiveSense::Minimize)
        } else {
            let mut iter = self.objectives.drain(..);
            let (c0, o0) = iter.next().unwrap();
            let mut acc = c0 * o0;
            for (c, o) in iter {
                acc = acc + (c * o);
            }
            acc
        };
        let folded_obj: Objective<InternalVar<B, E>> =
            folded_obj_var.transmute(|v| InternalVar::from(v.clone()));

        // Step 2: collect initial roots.
        let mut roots: Vec<E> = Vec::new();
        let mut seen_root: HashSet<E> = HashSet::new();
        for (c, _) in &user_constraints {
            for v in c.variable_refs() {
                if let InternalVar::Extra(e) = v
                    && seen_root.insert(e.clone())
                {
                    roots.push(e.clone());
                }
            }
        }
        for v in folded_obj.get_function().variable_refs() {
            if let InternalVar::Extra(e) = v
                && seen_root.insert(e.clone())
            {
                roots.push(e.clone());
            }
        }

        // Step 3: DFS expansion.
        let mut state: BuildState<B, E, C> = BuildState {
            out_vars: HashMap::new(),
            out_constraints: user_constraints,
            expanded: HashSet::new(),
            in_progress: HashSet::new(),
            path: Vec::new(),
        };

        // Move extras and base_vars out of self. extras is drained
        // as extras are expanded; base_vars and extras_kinds are
        // borrowed read-only during expansion via VarKinds.
        let mut extras = std::mem::take(&mut self.extras);
        let base_vars = std::mem::take(&mut self.base_vars);
        let fixed_variables = std::mem::take(&mut self.fixed_variables);
        let extras_kinds: HashMap<E, Variable> = extras
            .iter()
            .map(|(name, def)| (name.clone(), def.kind.clone()))
            .collect();

        for root in roots {
            expand(
                &mut state,
                &mut extras,
                &base_vars,
                &extras_kinds,
                &fixed_variables,
                db,
                root,
            )
            .await?;
        }

        // Step 4: partition constraints for reconstruction.
        let mut all_vars: HashMap<InternalVar<B, E>, Variable> = HashMap::new();
        for (b, kind) in &base_vars {
            all_vars.insert(InternalVar::Base(b.clone()), kind.clone());
        }
        for (v, kind) in state.out_vars {
            all_vars.insert(v, kind);
        }

        let reconstruction_constraints: Vec<_> = state
            .out_constraints
            .iter()
            .filter(|(_, src)| matches!(src, ConstraintSource::DefiningExtra { .. }))
            .cloned()
            .collect();

        let mut reconstruction_variables: HashMap<InternalVar<B, E>, Variable> = HashMap::new();
        for (c, _) in &reconstruction_constraints {
            for v in c.variable_refs() {
                if let Some(kind) = all_vars.get(v) {
                    reconstruction_variables.insert(v.clone(), kind.clone());
                }
            }
        }

        let base_variable_set: HashSet<B> = reconstruction_variables
            .keys()
            .filter_map(|v| match v {
                InternalVar::Base(b) => Some(b.clone()),
                _ => None,
            })
            .collect();

        // Step 5: feed everything into ProblemBuilder.
        let builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
            ProblemBuilder::new()
                .set_variables(all_vars)
                .add_constraints(state.out_constraints)
                .set_objective(folded_obj);
        let problem = builder.build().map_err(BuildError::Ilp)?;

        Ok(Model {
            problem,
            reconstruction_constraints,
            reconstruction_variables,
            base_variable_set,
        })
    }
}

fn expand<'s, 'm, B, E, C, Db, Err>(
    state: &'s mut BuildState<B, E, C>,
    extras: &'s mut HashMap<E, ExtraDef<'m, B, E, Db, Err>>,
    base_vars: &'s HashMap<B, Variable>,
    extras_kinds: &'s HashMap<E, Variable>,
    fixed_variables: &'s HashMap<B, f64>,
    db: &'s Db,
    e: E,
) -> BoxFuture<'s, Result<(), BuildError<B, E, C, Err>>>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + 'static,
    'm: 's,
{
    Box::pin(async move {
        if state.expanded.contains(&e) {
            return Ok(());
        }
        if state.in_progress.contains(&e) {
            // Build the cycle chain: from the first occurrence of
            // `e` in `path` to the end, plus `e` itself.
            let start = state.path.iter().position(|x| *x == e).unwrap_or(0);
            let mut cycle: Vec<E> = state.path[start..].to_vec();
            cycle.push(e);
            return Err(BuildError::CyclicExtra { cycle });
        }
        let def = extras
            .remove(&e)
            .ok_or_else(|| BuildError::UndeclaredExtra(e.clone()))?;

        state.in_progress.insert(e.clone());
        state.path.push(e.clone());

        let mut factory: HelperFactory<B, E> = HelperFactory::default();
        let constraints = {
            let kinds = VarKinds {
                base: base_vars,
                extras: extras_kinds,
                fixes: fixed_variables,
            };
            (def.define)(db, &mut factory, &kinds, e.clone())
                .await
                .map_err(|err| BuildError::ExtraError(e.clone(), err))?
        };

        // Smuggling check.
        for c in &constraints {
            for v in c.variable_refs() {
                if let ExtraVar::Helper(hid) = v
                    && !factory.declared.contains_key(hid)
                {
                    return Err(BuildError::HelperLeak {
                        used_in: e.clone(),
                        id: hid.clone(),
                    });
                }
            }
        }

        // Reduce closure output with fixed variable values
        // (safety net for user-defined closures that don't
        // reduce internally).
        let fixes_extra: HashMap<ExtraVar<B, E>, f64> = fixed_variables
            .iter()
            .map(|(b, v)| (ExtraVar::Base(b.clone()), *v))
            .collect();
        let constraints: Vec<_> = if fixes_extra.is_empty() {
            constraints
        } else {
            constraints
                .into_iter()
                .map(|c| c.reduce(&fixes_extra))
                .filter(|c| !c.is_trivially_true())
                .collect()
        };

        // Register the extra itself and its helpers as variables.
        state
            .out_vars
            .insert(InternalVar::Extra(e.clone()), def.kind);
        for (hid, kind) in factory.declared {
            state.out_vars.insert(
                InternalVar::Helper {
                    owner: e.clone(),
                    id: hid,
                },
                kind,
            );
        }

        // Transmute constraints and append, collecting deps.
        let mut deps: Vec<E> = Vec::new();
        let mut seen_dep: HashSet<E> = HashSet::new();
        for (i, c) in constraints.into_iter().enumerate() {
            let owner = e.clone();
            let tc: Constraint<InternalVar<B, E>> = c.transmute(|v| match v {
                ExtraVar::Base(b) => InternalVar::Base(b.clone()),
                ExtraVar::Extra(ex) => InternalVar::Extra(ex.clone()),
                ExtraVar::Helper(h) => InternalVar::Helper {
                    owner: owner.clone(),
                    id: h.clone(),
                },
            });
            for v in tc.variable_refs() {
                if let InternalVar::Extra(ex) = v
                    && *ex != e
                    && !state.expanded.contains(ex)
                    && seen_dep.insert(ex.clone())
                {
                    deps.push(ex.clone());
                }
            }
            state.out_constraints.push((
                tc,
                ConstraintSource::DefiningExtra {
                    extra: e.clone(),
                    index: i,
                },
            ));
        }

        // Recurse into deps.
        for d in deps {
            expand(
                state,
                extras,
                base_vars,
                extras_kinds,
                fixed_variables,
                db,
                d,
            )
            .await?;
        }

        state.in_progress.remove(&e);
        let popped = state.path.pop();
        debug_assert_eq!(popped.as_ref(), Some(&e));
        state.expanded.insert(e);
        Ok(())
    })
}
