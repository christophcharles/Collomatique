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
use std::marker::PhantomData;
use std::sync::Mutex;

use collomatique_ilp::solvers::{Solver, SolverModel};
use collomatique_ilp::{
    Config, ConfigData, ConfigDataVarCheck, Constraint, DefaultRepr, FeasibleConfig, LinExpr,
    Objective, ObjectiveSense, Problem, ProblemBuilder, UsableData, Variable,
};

pub mod bundle;
pub mod model_desc;
pub use bundle::{
    ConstraintBundle, EagerObjectifyError, EagerReifyError, ExtraEntry, IntConstraintBundle,
    ReifyError,
};

mod describe_var;
pub use describe_var::DescribeVar;

mod enumerate;
pub use enumerate::{EnumerateAll, EnumerateFrom};

pub mod violation_implication;
pub use violation_implication::{MinimalBlame, ViolationImplication};

/// Re-export the derive macro so users can write
/// `#[derive(DescribeVar)]` after `use collomatique_ilp_modeler::DescribeVar`.
#[cfg(feature = "derive")]
pub use collomatique_ilp_modeler_derive::DescribeVar;

/// Fixer closure. Called lazily at build time for undeclared
/// base variables found in constraints/objectives. The first fixer
/// in the chain to return `Some(_)` wins.
pub type FixerFn<'m, B, Env> = dyn Fn(&B, &Env) -> Option<f64> + Send + Sync + 'm;

// ---------------------------------------------------------------------------
// HasBase (private trait for generic fix logic)
// ---------------------------------------------------------------------------

/// Extract a base-variable reference from a composite variable
/// type. Implemented for [`Var`] and [`ExtraVar`] so that the
/// fix methods on [`VarContext`] are generic over both.
trait HasBase<B> {
    fn as_base(&self) -> Option<&B>;
}

impl<B, E> HasBase<B> for Var<B, E> {
    fn as_base(&self) -> Option<&B> {
        match self {
            Var::Base(b) => Some(b),
            Var::Extra(_) => None,
        }
    }
}

impl<B, E> HasBase<B> for ExtraVar<B, E> {
    fn as_base(&self) -> Option<&B> {
        match self {
            ExtraVar::Base(b) => Some(b),
            _ => None,
        }
    }
}

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
    DefiningExtra {
        extra: E,
        index: usize,
        for_constraints: bool,
    },
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

/// A base variable required for reconstruction was not provided.
#[derive(Debug, Clone, thiserror::Error)]
#[error("base variable `{0:?}` is required for reconstruction but no value was provided")]
pub struct ReconstructionError<B: UsableData>(pub B);

/// Errors that can occur when reconstructing a [`Solution`] from base variable data.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SolutionFromDataError<B: UsableData, E: UsableData> {
    /// The provided config data has invalid variables (unknown, missing, or non-conforming).
    #[error("config data has invalid variables for reconstruction")]
    MissingVariables,
    /// The solver did not produce a feasible solution for the reconstruction problem.
    #[error("reconstruction solver did not produce a feasible solution")]
    NoSolutionFromSolver,
    /// The reconstructed complete configuration failed validation against the problem.
    #[error("reconstructed configuration failed validation: {0:?}")]
    BuildConfigError(ConfigDataVarCheck<InternalVar<B, E>>),
}

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
// Dependency graph
// ---------------------------------------------------------------------------

/// Transitive base-variable dependencies of the extras that were
/// expanded when a [`Model`] was built.
///
/// The graph is computed as a by-product of the build-time DFS
/// expansion (see [`Modeler::build`]), so querying it later is a pure,
/// reusable `&self` operation. Only extras that were actually expanded
/// appear — those referenced by a user constraint/objective, or
/// force-included via [`Modeler::build_forcing`]. Extras cut out at the
/// DFS stage are absent from the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph<B, E>
where
    B: UsableData,
    E: UsableData,
{
    /// extra name -> transitive set of base variables it depends on.
    per_extra: HashMap<E, HashSet<B>>,
    /// Empty set handed back for extras with no (or absent) footprint.
    empty: HashSet<B>,
}

impl<B, E> DependencyGraph<B, E>
where
    B: UsableData,
    E: UsableData,
{
    fn new(per_extra: HashMap<E, HashSet<B>>) -> Self {
        DependencyGraph {
            per_extra,
            empty: HashSet::new(),
        }
    }

    /// Build the graph from each extra's *direct* dependencies:
    /// `(direct base variables, direct extra references)`. The transitive
    /// base footprint is the closure of these edges. Used both by the
    /// build-time DFS and when reconstructing a [`Model`] from a
    /// serialized description.
    fn from_direct(direct: HashMap<E, (HashSet<B>, HashSet<E>)>) -> Self {
        let mut memo: HashMap<E, HashSet<B>> = HashMap::new();
        let mut on_stack: HashSet<E> = HashSet::new();
        for e in direct.keys() {
            Self::resolve_footprint(e, &direct, &mut memo, &mut on_stack);
        }
        DependencyGraph::new(memo)
    }

    /// Rebuild the graph from a model's extra-defining constraints. Each
    /// [`ConstraintSource::DefiningExtra`] constraint contributes its base
    /// variables and referenced extras as direct edges for its extra. Used
    /// when reconstructing a [`Model`] from a serialized description, where
    /// the graph was not persisted.
    pub(crate) fn from_defining_constraints<C: UsableData>(
        constraints: &[(Constraint<InternalVar<B, E>>, ConstraintSource<E, C>)],
    ) -> Self {
        let mut direct: HashMap<E, (HashSet<B>, HashSet<E>)> = HashMap::new();
        for (c, src) in constraints {
            let ConstraintSource::DefiningExtra { extra, .. } = src else {
                continue;
            };
            let entry = direct
                .entry(extra.clone())
                .or_insert_with(|| (HashSet::new(), HashSet::new()));
            for v in c.variable_refs() {
                match v {
                    InternalVar::Base(b) => {
                        entry.0.insert(b.clone());
                    }
                    InternalVar::Extra(ex) if ex != extra => {
                        entry.1.insert(ex.clone());
                    }
                    _ => {}
                }
            }
        }
        Self::from_direct(direct)
    }

    /// Memoized transitive-closure helper for [`Self::from_direct`]. The
    /// `on_stack` set defensively breaks cycles (a valid model is acyclic).
    fn resolve_footprint(
        e: &E,
        direct: &HashMap<E, (HashSet<B>, HashSet<E>)>,
        memo: &mut HashMap<E, HashSet<B>>,
        on_stack: &mut HashSet<E>,
    ) -> HashSet<B> {
        if let Some(f) = memo.get(e) {
            return f.clone();
        }
        if !on_stack.insert(e.clone()) {
            return HashSet::new();
        }
        let mut footprint = HashSet::new();
        if let Some((bases, edges)) = direct.get(e) {
            footprint.extend(bases.iter().cloned());
            for t in edges {
                let sub = Self::resolve_footprint(t, direct, memo, on_stack);
                footprint.extend(sub);
            }
        }
        on_stack.remove(e);
        memo.insert(e.clone(), footprint.clone());
        footprint
    }

    /// Transitive base footprint of a single extra. Empty when the extra
    /// has no base dependencies or was not expanded.
    pub fn base_footprint(&self, extra: &E) -> &HashSet<B> {
        self.per_extra.get(extra).unwrap_or(&self.empty)
    }

    /// Footprint of a single variable. Works for both the user-facing
    /// [`Var`] (as seen in [`Model::filter`] callbacks) and the flattened
    /// [`InternalVar`] (as seen in a [`Problem`]): `Base(b)` -> `{b}`,
    /// `Extra(e)` -> `base_footprint(e)`, `Helper { owner, .. }` ->
    /// `base_footprint(owner)`.
    pub fn var_footprint<V: FootprintKey<B, E>>(&self, v: &V) -> HashSet<B> {
        v.footprint(self)
    }

    /// Union of [`Self::var_footprint`] over every variable of a
    /// constraint. Works for `Constraint<Var<B, E>>` and
    /// `Constraint<InternalVar<B, E>>` alike.
    pub fn constraint_footprint<V>(&self, c: &Constraint<V>) -> HashSet<B>
    where
        V: UsableData + FootprintKey<B, E>,
    {
        let mut out = HashSet::new();
        for v in c.variable_refs() {
            out.extend(v.footprint(self));
        }
        out
    }
}

/// Variable types whose base footprint can be looked up in a
/// [`DependencyGraph`]. Implemented for [`Var`] (the user-facing view,
/// e.g. inside [`Model::filter`] callbacks) and [`InternalVar`] (the
/// flattened view used inside a [`Problem`]). Not meant to be implemented
/// outside this crate.
pub trait FootprintKey<B, E>
where
    B: UsableData,
    E: UsableData,
{
    /// This variable's transitive base footprint in `graph`.
    fn footprint(&self, graph: &DependencyGraph<B, E>) -> HashSet<B>;
}

impl<B: UsableData, E: UsableData> FootprintKey<B, E> for Var<B, E> {
    fn footprint(&self, graph: &DependencyGraph<B, E>) -> HashSet<B> {
        match self {
            Var::Base(b) => std::iter::once(b.clone()).collect(),
            Var::Extra(e) => graph.base_footprint(e).clone(),
        }
    }
}

impl<B: UsableData, E: UsableData> FootprintKey<B, E> for InternalVar<B, E> {
    fn footprint(&self, graph: &DependencyGraph<B, E>) -> HashSet<B> {
        match self {
            InternalVar::Base(b) => std::iter::once(b.clone()).collect(),
            InternalVar::Extra(e) => graph.base_footprint(e).clone(),
            InternalVar::Helper { owner, .. } => graph.base_footprint(owner).clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ModelStats {
    pub base_variable_count: usize,
    pub user_constraint_count: usize,
    pub constraint_extra_count: usize,
    pub constraint_defining_constraint_count: usize,
    pub objective_extra_count: usize,
    pub objective_defining_constraint_count: usize,
}

/// The output of [`Modeler::build`]. Wraps the assembled
/// [`Problem`] and carries data needed to build a
/// reconstruction problem (for computing extra/helper values
/// from a known base-variable assignment).
#[derive(Debug, Clone, PartialEq, Eq)]
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
    checker_problem: Problem<InternalVar<B, E>, ConstraintSource<E, C>>,
    checker_reconstruction_constraints:
        Vec<(Constraint<InternalVar<B, E>>, ConstraintSource<E, C>)>,
    checker_reconstruction_variables: HashMap<InternalVar<B, E>, Variable>,
    checker_base_variable_set: HashSet<B>,
    reconstruction_objective: Objective<InternalVar<B, E>>,
    checker_reconstruction_objective: Objective<InternalVar<B, E>>,
    base_var_list: HashMap<B, Variable>,
    dependency_graph: DependencyGraph<B, E>,
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

    /// Borrow the dependency graph computed during expansion.
    ///
    /// Maps each expanded extra to the transitive set of base variables
    /// it depends on. Non-destructive and reusable — build it once, query
    /// it repeatedly (e.g. inside a [`Self::filter`] callback).
    pub fn dependency_graph(&self) -> &DependencyGraph<B, E> {
        &self.dependency_graph
    }

    /// Filter user constraints, base variables and objective terms, keeping
    /// every extra-defining constraint (and every extra/helper variable)
    /// verbatim.
    ///
    /// The callbacks work at the user-facing level:
    /// - `keep_constraint` sees each user constraint as a
    ///   `Constraint<Var<B, E>>` (helpers never appear in user constraints);
    /// - `keep_base_variable` sees each base variable `&B` — the caller
    ///   decides which base variables the slice keeps, referenced or not (this
    ///   mirrors the full `Model`, whose problem declares *every* base
    ///   variable). Extras and helpers are always kept, since their defining
    ///   constraints are kept.
    /// - `keep_obj_term` sees each objective variable as a `Var<B, E>`.
    ///
    /// Delegates to the dumb [`Problem::filter`] primitive, so consistency is
    /// verified rather than repaired: if a kept constraint or objective term
    /// references a base variable that `keep_base_variable` dropped, the
    /// underlying build returns an error. In the intended footprint-based
    /// usage (`keep_constraint` = footprint ⊆ blessed, `keep_base_variable` =
    /// `b ∈ blessed`, `keep_obj_term` = footprint ⊆ blessed) the result is
    /// always `Ok`.
    ///
    /// Dead extras — those whose only user references were filtered out — are
    /// *not* shed here. To drop them, round-trip the result through
    /// [`Modeler::from_model_problem`] followed by [`Modeler::build`], whose
    /// lazy re-expansion prunes whatever is no longer referenced.
    pub fn filter<FC, FV, FO>(
        &self,
        mut keep_constraint: FC,
        mut keep_base_variable: FV,
        mut keep_obj_term: FO,
    ) -> collomatique_ilp::BuildResult<
        Problem<InternalVar<B, E>, ConstraintSource<E, C>>,
        InternalVar<B, E>,
        ConstraintSource<E, C>,
    >
    where
        FC: FnMut(&Constraint<Var<B, E>>, &C) -> bool,
        FV: FnMut(&B) -> bool,
        FO: FnMut(&Var<B, E>) -> bool,
    {
        self.problem.filter(
            |c, src| match src {
                ConstraintSource::DefiningExtra { .. } => true,
                ConstraintSource::User(desc) => {
                    let var_c = c.transmute(internal_to_var);
                    keep_constraint(&var_c, desc)
                }
            },
            |v| match v {
                InternalVar::Base(b) => keep_base_variable(b),
                // Extras/helpers stay declared: their definitions are kept.
                InternalVar::Extra(_) | InternalVar::Helper { .. } => true,
            },
            |v| match v {
                // Helpers never appear in the objective; keep defensively.
                InternalVar::Helper { .. } => true,
                _ => keep_obj_term(&internal_to_var(v)),
            },
        )
    }

    /// Build a reconstruction problem: given base variable values,
    /// produce a [`Problem`] whose solution determines all
    /// extra and helper variable values.
    ///
    /// Every base variable that appears in extra-defining
    /// constraints must be present in `base_values`. Returns
    /// [`ReconstructionError`] if any is missing.
    ///
    /// The returned problem contains the defining constraints of
    /// extras (with base variables substituted out) and the full
    /// objective reduced by the given base variable values.
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
                .set_objective(self.reconstruction_objective.reduce(&fixes));
        Ok(builder
            .build()
            .expect("reconstruction problem should always be valid"))
    }

    /// Access the checker problem (user constraints +
    /// constraint-needed extras only, trivial objective).
    pub fn checker_problem(&self) -> &Problem<InternalVar<B, E>, ConstraintSource<E, C>> {
        &self.checker_problem
    }

    pub fn stats(&self) -> ModelStats {
        let base_variable_count = self.base_var_list.len();

        let user_constraint_count = self
            .problem
            .get_constraints()
            .iter()
            .filter(|(_, src)| matches!(src, ConstraintSource::User(_)))
            .count();

        let constraint_extra_count = self
            .checker_reconstruction_variables
            .keys()
            .filter(|v| !matches!(v, InternalVar::Base(_)))
            .count();

        let constraint_defining_constraint_count = self.checker_reconstruction_constraints.len();

        let all_extra_count = self
            .reconstruction_variables
            .keys()
            .filter(|v| !matches!(v, InternalVar::Base(_)))
            .count();

        let all_defining_constraint_count = self.reconstruction_constraints.len();

        ModelStats {
            base_variable_count,
            user_constraint_count,
            constraint_extra_count,
            constraint_defining_constraint_count,
            objective_extra_count: all_extra_count - constraint_extra_count,
            objective_defining_constraint_count: all_defining_constraint_count
                - constraint_defining_constraint_count,
        }
    }

    /// Build a checker reconstruction problem: given base variable
    /// values, produce a [`Problem`] whose solution determines only
    /// the extra/helper variable values needed for constraint
    /// checking.
    pub fn checker_reconstruction_problem(
        &self,
        base_values: &HashMap<B, f64>,
    ) -> Result<Problem<InternalVar<B, E>, ConstraintSource<E, C>>, ReconstructionError<B>> {
        for b in &self.checker_base_variable_set {
            if !base_values.contains_key(b) {
                return Err(ReconstructionError(b.clone()));
            }
        }

        let fixes: HashMap<InternalVar<B, E>, f64> = base_values
            .iter()
            .map(|(b, v)| (InternalVar::Base(b.clone()), *v))
            .collect();

        let reduced_constraints: Vec<_> = self
            .checker_reconstruction_constraints
            .iter()
            .map(|(c, src)| (c.reduce(&fixes), src.clone()))
            .filter(|(c, _)| !c.is_trivially_true())
            .collect();

        let recon_vars: HashMap<InternalVar<B, E>, Variable> = self
            .checker_reconstruction_variables
            .iter()
            .filter(|(v, _)| !matches!(v, InternalVar::Base(_)))
            .map(|(v, kind)| (v.clone(), kind.clone()))
            .collect();

        let builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
            ProblemBuilder::new()
                .set_variables(recon_vars)
                .add_constraints(reduced_constraints)
                .set_objective(self.checker_reconstruction_objective.reduce(&fixes));
        Ok(builder
            .build()
            .expect("checker reconstruction problem should always be valid"))
    }

    /// Solve the full optimization problem using a callback.
    ///
    /// The callback receives the problem and should return
    /// the solver's result. This allows customizing the solving
    /// strategy (e.g., time limits).
    pub fn solve_with<'a>(
        &'a self,
        f: impl FnOnce(
            &'a Problem<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
        ) -> Option<
            FeasibleConfig<
                'a,
                InternalVar<B, E>,
                ConstraintSource<E, C>,
                DefaultRepr<InternalVar<B, E>>,
            >,
        >,
    ) -> Option<FeasibleSolution<'a, B, E, C>> {
        f(&self.problem).map(|feasible_config| FeasibleSolution { feasible_config })
    }

    /// Solve the full optimization problem.
    pub fn solve<'a, S>(&'a self, solver: &S) -> Option<FeasibleSolution<'a, B, E, C>>
    where
        S: Solver<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
    {
        self.solve_with(|pb| solver.build_model(pb).solve())
    }

    /// Solve the checker problem (feasibility only) using a callback.
    pub fn solve_checker_with<'a>(
        &'a self,
        f: impl FnOnce(
            &'a Problem<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
        ) -> Option<
            FeasibleConfig<
                'a,
                InternalVar<B, E>,
                ConstraintSource<E, C>,
                DefaultRepr<InternalVar<B, E>>,
            >,
        >,
    ) -> Option<FeasibleSolution<'a, B, E, C>> {
        f(&self.checker_problem).map(|feasible_config| FeasibleSolution { feasible_config })
    }

    /// Solve the checker problem (feasibility only, no objective optimization).
    pub fn solve_checker<'a, S>(&'a self, solver: &S) -> Option<FeasibleSolution<'a, B, E, C>>
    where
        S: Solver<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
    {
        self.solve_checker_with(|pb| solver.build_model(pb).solve())
    }

    /// Build a [`Solution`] by reconstructing extra variable
    /// values from base variable values (full reconstruction),
    /// using a callback.
    pub fn solution_from_data_with(
        &self,
        config_data: &ConfigData<B>,
        f: impl for<'a> FnOnce(
            &'a Problem<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
        ) -> Option<
            FeasibleConfig<
                'a,
                InternalVar<B, E>,
                ConstraintSource<E, C>,
                DefaultRepr<InternalVar<B, E>>,
            >,
        >,
    ) -> Result<Solution<'_, B, E, C>, SolutionFromDataError<B, E>> {
        if !self.check_no_missing_variables(config_data) {
            return Err(SolutionFromDataError::MissingVariables);
        }

        let base_values: HashMap<B, f64> = config_data.get_values().into_iter().collect();
        let recon_problem = self
            .reconstruction_problem(&base_values)
            .map_err(|_| SolutionFromDataError::MissingVariables)?;
        let recon_sol = f(&recon_problem).ok_or(SolutionFromDataError::NoSolutionFromSolver)?;

        let mut complete_values: HashMap<InternalVar<B, E>, f64> = base_values
            .into_iter()
            .map(|(b, v)| (InternalVar::Base(b), v))
            .collect();
        complete_values.extend(recon_sol.get_values());
        let new_config_data = ConfigData::from(complete_values);

        let config = self
            .problem
            .build_config(new_config_data)
            .map_err(SolutionFromDataError::BuildConfigError)?;
        Ok(Solution { config })
    }

    /// Build a [`Solution`] by reconstructing extra variable
    /// values from base variable values (full reconstruction).
    pub fn solution_from_data<S>(
        &self,
        config_data: &ConfigData<B>,
        solver: &S,
    ) -> Result<Solution<'_, B, E, C>, SolutionFromDataError<B, E>>
    where
        S: Solver<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
    {
        self.solution_from_data_with(config_data, |pb| solver.build_model(pb).solve())
    }

    /// Build a [`Solution`] by reconstructing only the extra
    /// variable values needed for constraint checking,
    /// using a callback.
    pub fn checker_solution_from_data_with(
        &self,
        config_data: &ConfigData<B>,
        f: impl for<'a> FnOnce(
            &'a Problem<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
        ) -> Option<
            FeasibleConfig<
                'a,
                InternalVar<B, E>,
                ConstraintSource<E, C>,
                DefaultRepr<InternalVar<B, E>>,
            >,
        >,
    ) -> Result<Solution<'_, B, E, C>, SolutionFromDataError<B, E>> {
        if !self.check_no_missing_variables(config_data) {
            return Err(SolutionFromDataError::MissingVariables);
        }

        let base_values: HashMap<B, f64> = config_data.get_values().into_iter().collect();
        let recon_problem = self
            .checker_reconstruction_problem(&base_values)
            .map_err(|_| SolutionFromDataError::MissingVariables)?;
        let recon_sol = f(&recon_problem).ok_or(SolutionFromDataError::NoSolutionFromSolver)?;

        let mut complete_values: HashMap<InternalVar<B, E>, f64> = base_values
            .into_iter()
            .map(|(b, v)| (InternalVar::Base(b), v))
            .collect();
        complete_values.extend(recon_sol.get_values());
        let new_config_data = ConfigData::from(complete_values);

        let config = self
            .checker_problem
            .build_config(new_config_data)
            .map_err(SolutionFromDataError::BuildConfigError)?;
        Ok(Solution { config })
    }

    /// Build a [`Solution`] by reconstructing only the extra
    /// variable values needed for constraint checking.
    pub fn checker_solution_from_data<S>(
        &self,
        config_data: &ConfigData<B>,
        solver: &S,
    ) -> Result<Solution<'_, B, E, C>, SolutionFromDataError<B, E>>
    where
        S: Solver<InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
    {
        self.checker_solution_from_data_with(config_data, |pb| solver.build_model(pb).solve())
    }

    /// Build a [`Solution`] from a complete set of variable values
    /// (base + extra + helper).
    pub fn solution_from_complete_data(
        &self,
        config_data: ConfigData<InternalVar<B, E>>,
    ) -> Option<Solution<'_, B, E, C>> {
        Some(Solution {
            config: self.problem.build_config(config_data).ok()?,
        })
    }

    fn check_no_missing_variables(&self, config_data: &ConfigData<B>) -> bool {
        if !config_data
            .get_values()
            .keys()
            .all(|x| self.base_var_list.contains_key(x))
        {
            return false;
        }

        self.base_var_list
            .iter()
            .all(|(var, var_def)| match config_data.get(var.clone()) {
                Some(v) => var_def.checks_value(v),
                None => false,
            })
    }
}

// ---------------------------------------------------------------------------
// Solution / FeasibleSolution
// ---------------------------------------------------------------------------

/// A solution (possibly infeasible) evaluated against a [`Model`].
#[derive(Debug, Clone)]
pub struct Solution<'a, B: UsableData, E: UsableData, C: UsableData> {
    config: Config<'a, InternalVar<B, E>, ConstraintSource<E, C>, DefaultRepr<InternalVar<B, E>>>,
}

impl<'a, B: UsableData, E: UsableData, C: UsableData> Solution<'a, B, E, C> {
    /// Extract base variable values only.
    pub fn get_data(&self) -> ConfigData<B> {
        ConfigData::from(self.config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                InternalVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    /// Get all variable values (base + extra + helper).
    pub fn get_complete_data(&self) -> ConfigData<InternalVar<B, E>> {
        ConfigData::from(self.config.get_values())
    }

    pub fn is_feasible(&self) -> bool {
        self.config.is_feasible()
    }

    pub fn into_feasible(self) -> Option<FeasibleSolution<'a, B, E, C>> {
        Some(FeasibleSolution {
            feasible_config: self.config.into_feasible()?,
        })
    }

    /// Iterate over unsatisfied constraints.
    pub fn blame<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<Item = &'b (Constraint<InternalVar<B, E>>, ConstraintSource<E, C>)>
    + use<'a, 'b, B, E, C> {
        self.config.blame()
    }

    /// Iterate over unsatisfied user constraints, filtered to remove
    /// redundant ones via violation implication.
    pub fn minimal_blame(&self) -> MinimalBlame<&C>
    where
        C: ViolationImplication,
    {
        self.blame()
            .filter_map(|(_constraint, desc)| match desc {
                ConstraintSource::User(desc) => Some(desc),
                ConstraintSource::DefiningExtra { .. } => None,
            })
            .collect()
    }

    /// Evaluate the objective function for this solution.
    pub fn eval(&self) -> f64 {
        self.config.eval()
    }
}

/// A feasible solution evaluated against a [`Model`].
#[derive(Debug, Clone)]
pub struct FeasibleSolution<'a, B: UsableData, E: UsableData, C: UsableData> {
    feasible_config: FeasibleConfig<
        'a,
        InternalVar<B, E>,
        ConstraintSource<E, C>,
        DefaultRepr<InternalVar<B, E>>,
    >,
}

impl<'a, B: UsableData, E: UsableData, C: UsableData> FeasibleSolution<'a, B, E, C> {
    pub fn into_solution(self) -> Solution<'a, B, E, C> {
        Solution {
            config: self.feasible_config.into_inner(),
        }
    }

    /// Extract base variable values only.
    pub fn get_data(&self) -> ConfigData<B> {
        ConfigData::from(self.feasible_config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                InternalVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    /// Get all variable values (base + extra + helper).
    pub fn get_complete_data(&self) -> ConfigData<InternalVar<B, E>> {
        ConfigData::from(self.feasible_config.get_values())
    }
}

// ---------------------------------------------------------------------------
// Modeler
// ---------------------------------------------------------------------------

pub type DefineFn<'m, B, E, Env, Err> = dyn for<'a> FnOnce(
        &'a mut HelperFactory<B, E>,
        &'a VarContext<'a, B, E, Env>,
        E,
    ) -> Result<Vec<Constraint<ExtraVar<B, E>>>, Err>
    + Send
    + 'm;

// ---------------------------------------------------------------------------
// VarContext
// ---------------------------------------------------------------------------

/// Build-time context for variable kinds and cached fix resolution,
/// available inside extra-definition closures.
///
/// Constructed once at the start of [`Modeler::build`] and shared
/// (via reconstruction) across all extra expansion calls. The fix
/// cache persists for the duration of the build, so each undeclared
/// base variable is resolved at most once through the fixer chain.
///
/// Helper kinds are *not* in this view: they are owned by the
/// [`HelperFactory`] the same closure already has access to,
/// and can be looked up there via [`HelperFactory::kind_of`].
/// This split avoids a borrow conflict between the factory
/// (which mutates its declared set) and the context view.
///
/// Fix methods use interior mutability ([`Mutex`]) for the cache,
/// allowing `VarContext` to be passed as a shared reference through
/// the HRTB-based closure signature while still caching results.
pub struct VarContext<'a, B, E, Env>
where
    B: UsableData,
    E: UsableData,
{
    base: &'a HashMap<B, Variable>,
    extras: &'a HashMap<E, Variable>,
    fixers: &'a [Box<FixerFn<'a, B, Env>>],
    env: &'a Env,
    cache: &'a Mutex<HashMap<B, Option<f64>>>,
}

impl<'a, B, E, Env> VarContext<'a, B, E, Env>
where
    B: UsableData,
    E: UsableData,
{
    /// Access the environment.
    pub fn env(&self) -> &Env {
        self.env
    }

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

    /// Resolve fix for a single base variable. Checks cache first,
    /// then calls fixers in order. Caches both `Some` and `None`
    /// results so a variable is never queried twice.
    fn resolve_fix(&self, b: &B) -> Option<f64> {
        if self.base.contains_key(b) {
            return None;
        }
        if let Some(&cached) = self.cache.lock().unwrap().get(b) {
            return cached;
        }
        let mut result = None;
        for fixer in self.fixers {
            if let Some(value) = fixer(b, self.env) {
                result = Some(value);
                break;
            }
        }
        self.cache.lock().unwrap().insert(b.clone(), result);
        result
    }

    /// Collect fix values for a set of base variable references.
    /// Deduplicates and uses the cache.
    fn resolve_fixes<'b>(&self, base_refs: impl Iterator<Item = &'b B>) -> HashMap<B, f64>
    where
        B: 'b,
    {
        let mut fixes = HashMap::new();
        for b in base_refs {
            if !fixes.contains_key(b) {
                if let Some(val) = self.resolve_fix(b) {
                    fixes.insert(b.clone(), val);
                }
            }
        }
        fixes
    }

    /// Fix a single constraint by resolving undeclared base
    /// variables through the fixer chain.
    pub fn fix_constraint(
        &self,
        constraint: Constraint<ExtraVar<B, E>>,
    ) -> (Constraint<ExtraVar<B, E>>, HashMap<B, f64>) {
        let base_refs = constraint.variable_refs().filter_map(|v| v.as_base());
        let fixes = self.resolve_fixes(base_refs);
        if fixes.is_empty() {
            return (constraint, fixes);
        }
        let fixes_v: HashMap<ExtraVar<B, E>, f64> = fixes
            .iter()
            .map(|(b, &val)| (ExtraVar::Base(b.clone()), val))
            .collect();
        (constraint.reduce(&fixes_v), fixes)
    }

    /// Fix a single linear expression by resolving undeclared base
    /// variables through the fixer chain.
    pub fn fix_expr(
        &self,
        expr: LinExpr<ExtraVar<B, E>>,
    ) -> (LinExpr<ExtraVar<B, E>>, HashMap<B, f64>) {
        let base_refs = expr.variable_refs().filter_map(|v| v.as_base());
        let fixes = self.resolve_fixes(base_refs);
        if fixes.is_empty() {
            return (expr, fixes);
        }
        let fixes_v: HashMap<ExtraVar<B, E>, f64> = fixes
            .iter()
            .map(|(b, &val)| (ExtraVar::Base(b.clone()), val))
            .collect();
        (expr.reduce(&fixes_v), fixes)
    }

    /// Fix a batch of constraints. Resolves all undeclared base
    /// variables across the batch, then reduces each constraint.
    /// Trivially-true constraints are filtered out.
    pub fn fix_constraints(
        &self,
        constraints: Vec<Constraint<ExtraVar<B, E>>>,
    ) -> (Vec<Constraint<ExtraVar<B, E>>>, HashMap<B, f64>) {
        let base_refs = constraints
            .iter()
            .flat_map(|c| c.variable_refs())
            .filter_map(|v| v.as_base());
        let fixes = self.resolve_fixes(base_refs);
        if fixes.is_empty() {
            return (constraints, fixes);
        }
        let fixes_extra: HashMap<ExtraVar<B, E>, f64> = fixes
            .iter()
            .map(|(b, &val)| (ExtraVar::Base(b.clone()), val))
            .collect();
        let reduced = constraints
            .into_iter()
            .map(|c| c.reduce(&fixes_extra))
            .filter(|c| !c.is_trivially_true())
            .collect();
        (reduced, fixes)
    }
}

struct ExtraDef<'m, B, E, Env, Err>
where
    B: UsableData,
    E: UsableData,
{
    kind: Variable,
    define: Box<DefineFn<'m, B, E, Env, Err>>,
}

/// Lazy ILP modeler.
///
/// `'m` is the lifetime everything captured by extra-definition
/// closures must outlive (use `'static` if you don't need
/// borrowing). `B` is the base-variable name type, `E` is the
/// extra-variable name type, `C` is the user constraint
/// description type, `Env` is the shared context handed to
/// each extra-definition closure and fixer, and `Err` is the
/// user-defined error type returned by fallible extra-definition
/// closures.
pub struct Modeler<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    base_vars: HashMap<B, Variable>,
    constraints: Vec<(Constraint<Var<B, E>>, C)>,
    objectives: Vec<(f64, Objective<Var<B, E>>)>,
    extras: HashMap<E, ExtraDef<'m, B, E, Env, Err>>,
    fixers: Vec<Box<FixerFn<'m, B, Env>>>,
}

impl<'m, B, E, C, Env, Err> Modeler<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + Send + 'static,
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
            fixers: Vec::new(),
        }
    }

    /// Read-only access to the declared base variables.
    pub fn base_vars(&self) -> &HashMap<B, Variable> {
        &self.base_vars
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

    /// Remove all accumulated objectives.
    pub fn clear_objectives(&mut self) {
        self.objectives.clear();
    }

    /// Register a fixer closure. At build time, for every
    /// undeclared base variable found in constraints/objectives,
    /// fixers are called in registration order; the first to return
    /// `Some(value)` wins and that variable is substituted out.
    pub fn add_fixer<F>(&mut self, fixer: F)
    where
        F: Fn(&B, &Env) -> Option<f64> + Send + Sync + 'm,
    {
        self.fixers.push(Box::new(fixer));
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
                &'a mut HelperFactory<B, E>,
                &'a VarContext<'a, B, E, Env>,
                E,
            ) -> Result<Vec<Constraint<ExtraVar<B, E>>>, Err>
            + Send
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
        define: Box<DefineFn<'m, B, E, Env, Err>>,
    ) -> Result<(), DuplicateExtra<E>> {
        if self.extras.contains_key(&name) {
            return Err(DuplicateExtra(name));
        }
        self.extras.insert(name, ExtraDef { kind, define });
        Ok(())
    }

    /// Register multiple extras at once. Checks for duplicates
    /// both within the batch and against already-declared extras.
    pub fn declare_extras(
        &mut self,
        entries: impl IntoIterator<Item = (E, ExtraEntry<'m, B, E, Env, Err>)>,
    ) -> Result<(), DuplicateExtra<E>> {
        let entries: Vec<_> = entries.into_iter().collect();
        let mut seen: HashSet<E> = HashSet::new();
        for (name, _) in &entries {
            if self.extras.contains_key(name) || !seen.insert(name.clone()) {
                return Err(DuplicateExtra(name.clone()));
            }
        }
        for (name, entry) in entries {
            let (kind, define) = entry.into_parts();
            self.extras.insert(name, ExtraDef { kind, define });
        }
        Ok(())
    }

    /// Rebuild a [`Modeler`] from an already-assembled
    /// [`Problem`], reversing [`Modeler::build`]'s flattening.
    ///
    /// The [`ConstraintSource`] tag on each constraint is what
    /// makes this possible: [`ConstraintSource::User`] constraints
    /// become user constraints again, while
    /// [`ConstraintSource::DefiningExtra`] constraints are grouped
    /// per extra and replayed by a synthesized definition closure.
    /// Extra and helper variable kinds are recovered from the
    /// problem's variable set.
    ///
    /// Passed the full [`Model::problem`], the reconstruction is
    /// faithful: the user constraints and the objective reference
    /// the same extras, so [`Modeler::build`] rediscovers the exact
    /// same constraint/objective roots and recomputes each extra's
    /// `for_constraints` flag identically — an objective-only extra
    /// stays out of the checker problem, a constraint extra stays
    /// in. The only differences are constraint ordering and helper
    /// variable renumbering (helpers are re-minted with fresh ids),
    /// neither of which changes the problem's meaning. The
    /// synthesized closures ignore `Env` and never fail, so both
    /// type parameters are free at the call site.
    ///
    /// The folded objective is re-added as a single weighted
    /// objective; callers that want to optimize a different
    /// surrogate can follow with [`Modeler::clear_objectives`].
    pub fn from_model_problem(problem: &Problem<InternalVar<B, E>, ConstraintSource<E, C>>) -> Self
    where
        B: 'm,
        E: 'm,
    {
        // Recover variable kinds, split by role.
        let mut base_vars: HashMap<B, Variable> = HashMap::new();
        let mut extra_kinds: HashMap<E, Variable> = HashMap::new();
        let mut helper_kinds: HashMap<E, HashMap<HelperId, Variable>> = HashMap::new();
        for (var, kind) in problem.get_variables() {
            match var {
                InternalVar::Base(b) => {
                    base_vars.insert(b.clone(), kind.clone());
                }
                InternalVar::Extra(e) => {
                    extra_kinds.insert(e.clone(), kind.clone());
                }
                InternalVar::Helper { owner, id } => {
                    helper_kinds
                        .entry(owner.clone())
                        .or_default()
                        .insert(id.clone(), kind.clone());
                }
            }
        }

        let mut modeler = Self::new(base_vars);

        // Partition constraints: user constraints go back verbatim,
        // defining-extra constraints are grouped per extra (later
        // ordered by their original index).
        let mut per_extra: HashMap<E, Vec<(usize, Constraint<InternalVar<B, E>>)>> = HashMap::new();
        for (constraint, source) in problem.get_constraints() {
            match source {
                ConstraintSource::User(desc) => {
                    modeler.add_constraint(constraint.transmute(internal_to_var), desc.clone());
                }
                ConstraintSource::DefiningExtra { extra, index, .. } => {
                    per_extra
                        .entry(extra.clone())
                        .or_default()
                        .push((*index, constraint.clone()));
                }
            }
        }

        // Synthesize a definition closure for every extra. We cover
        // every extra name that has a kind, even one with no defining
        // constraints (its closure simply returns nothing).
        for (extra, kind) in extra_kinds {
            let mut defs = per_extra.remove(&extra).unwrap_or_default();
            defs.sort_by_key(|(index, _)| *index);
            let constraints: Vec<Constraint<InternalVar<B, E>>> =
                defs.into_iter().map(|(_, c)| c).collect();
            let owned_helper_kinds = helper_kinds.remove(&extra).unwrap_or_default();

            modeler
                .declare_extra(extra, kind, move |factory, _ctx, _name| {
                    // Re-mint this extra's helpers in first-seen
                    // order, mapping each original id to a fresh one.
                    let mut remap: HashMap<HelperId, ExtraVar<B, E>> = HashMap::new();
                    for c in &constraints {
                        for v in c.variable_refs() {
                            if let InternalVar::Helper { id, .. } = v
                                && !remap.contains_key(id)
                            {
                                let helper_kind = owned_helper_kinds
                                    .get(id)
                                    .cloned()
                                    .expect("helper kind recorded for defining constraint");
                                remap.insert(id.clone(), factory.new_helper(helper_kind));
                            }
                        }
                    }
                    let translated = constraints
                        .iter()
                        .map(|c| {
                            c.transmute(|v| match v {
                                InternalVar::Base(b) => ExtraVar::Base(b.clone()),
                                InternalVar::Extra(e) => ExtraVar::Extra(e.clone()),
                                InternalVar::Helper { id, .. } => remap
                                    .get(id)
                                    .cloned()
                                    .expect("helper remapped before translation"),
                            })
                        })
                        .collect();
                    Ok(translated)
                })
                .expect("extra names are unique in a built problem");
        }

        // Re-add the folded objective (weight 1.0). No helpers here.
        modeler.add_objective(1.0, problem.get_objective().transmute(internal_to_var));

        modeler
    }
}

/// Translate a flattened [`InternalVar`] back to the user-facing
/// [`Var`] used in constraints and objectives. Helpers never appear
/// in user constraints or the objective, so encountering one means
/// the problem was not produced by [`Modeler::build`].
fn internal_to_var<B, E>(var: &InternalVar<B, E>) -> Var<B, E>
where
    B: UsableData,
    E: UsableData,
{
    match var {
        InternalVar::Base(b) => Var::Base(b.clone()),
        InternalVar::Extra(e) => Var::Extra(e.clone()),
        InternalVar::Helper { .. } => {
            panic!(
                "expected a Modeler-built problem: helper variable found in a user constraint or the objective"
            )
        }
    }
}

// ---------------------------------------------------------------------------
// DescribeVar integration
// ---------------------------------------------------------------------------

impl<'m, B, E, C, Env, Err> Modeler<'m, B, E, C, Env, Err>
where
    B: DescribeVar<Env = Env> + UsableData + 'm,
    Env: Sync + 'm,
    E: UsableData,
    C: UsableData,
    Err: Debug + Send + 'static,
{
    /// Create a modeler from a [`DescribeVar`] type, using the
    /// environment for variable enumeration and fix resolution.
    pub fn from_described(env: &Env) -> Self {
        let base_vars = B::enumerate(env);
        let mut modeler = Self::new(base_vars);
        modeler.add_fixer(|b: &B, env: &Env| b.check_fix(env));
        modeler
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
    /// Direct dependencies of each expanded extra, collected during the
    /// DFS: `(direct base variables, direct extra references)`. Closed
    /// into a [`DependencyGraph`] once expansion finishes.
    dep_direct: HashMap<E, (HashSet<B>, HashSet<E>)>,
}

impl<'m, B, E, C, Env, Err> Modeler<'m, B, E, C, Env, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Env: Sync,
    Err: Debug + Send + 'static,
{
    /// Run the lazy expansion fixpoint and return the assembled
    /// [`Model`]. `env` is passed to every extra-definition
    /// closure and to the fixer chain.
    ///
    /// Only extras referenced (transitively) by a user constraint or the
    /// objective are expanded; declared-but-unreferenced extras are
    /// dropped. Use [`Self::build_forcing`] or [`Self::build_full`] to
    /// keep some or all of them.
    pub fn build(self, env: &Env) -> Result<Model<B, E, C>, BuildError<B, E, C, Err>> {
        self.build_with_log(env, &mut |_: &str| {})
    }

    /// Like [`Self::build`], but additionally force-expands every declared
    /// extra for which `force(&name)` returns `true`, even if nothing
    /// references it. The forced extras' definitions end up in the
    /// resulting [`Model`] (and its [`DependencyGraph`]).
    ///
    /// `build(env)` is `build_forcing(env, |_| false)`; a specific set is
    /// just `build_forcing(env, |e| wanted.contains(e))`.
    pub fn build_forcing(
        self,
        env: &Env,
        force: impl Fn(&E) -> bool,
    ) -> Result<Model<B, E, C>, BuildError<B, E, C, Err>> {
        self.build_with_log_forcing(env, &mut |_: &str| {}, force)
    }

    /// Like [`Self::build`], but force-expands *every* declared extra,
    /// so no extra definition is ever dropped.
    pub fn build_full(self, env: &Env) -> Result<Model<B, E, C>, BuildError<B, E, C, Err>> {
        self.build_forcing(env, |_: &E| true)
    }

    /// Like [`Self::build`], but calls `log` with progress
    /// messages as each step completes.
    pub fn build_with_log(
        self,
        env: &Env,
        log: &mut dyn FnMut(&str),
    ) -> Result<Model<B, E, C>, BuildError<B, E, C, Err>> {
        self.build_with_log_forcing(env, log, |_: &E| false)
    }

    /// Like [`Self::build_with_log`], but additionally force-expands every
    /// declared extra selected by `force` (see [`Self::build_forcing`]).
    pub fn build_with_log_forcing(
        mut self,
        env: &Env,
        log: &mut dyn FnMut(&str),
        force: impl Fn(&E) -> bool,
    ) -> Result<Model<B, E, C>, BuildError<B, E, C, Err>> {
        use std::time::Instant;
        let t_total = Instant::now();

        // Move data out of self for the build scope.
        let mut extras = std::mem::take(&mut self.extras);
        let base_vars = std::mem::take(&mut self.base_vars);
        let fixers = std::mem::take(&mut self.fixers);
        let extras_kinds: HashMap<E, Variable> = extras
            .iter()
            .map(|(name, def)| (name.clone(), def.kind.clone()))
            .collect();

        // Shared fix cache for the entire build.
        let fix_cache: Mutex<HashMap<B, Option<f64>>> = Mutex::new(HashMap::new());

        let ctx = VarContext {
            base: &base_vars,
            extras: &extras_kinds,
            fixers: &fixers,
            env,
            cache: &fix_cache,
        };

        // Step 0: lazily resolve fixes for user constraints and
        // objectives via the VarContext cache.
        let t_step = Instant::now();
        {
            let mut all_undeclared: HashSet<B> = HashSet::new();
            for (c, _) in &self.constraints {
                for v in c.variable_refs() {
                    if let Var::Base(b) = v {
                        if !base_vars.contains_key(b) {
                            all_undeclared.insert(b.clone());
                        }
                    }
                }
            }
            for (_, obj) in &self.objectives {
                for v in obj.get_function().variable_refs() {
                    if let Var::Base(b) = v {
                        if !base_vars.contains_key(b) {
                            all_undeclared.insert(b.clone());
                        }
                    }
                }
            }
            let mut fixes_var: HashMap<Var<B, E>, f64> = HashMap::new();
            for b in all_undeclared {
                if let Some(val) = ctx.resolve_fix(&b) {
                    fixes_var.insert(Var::Base(b), val);
                }
            }
            if !fixes_var.is_empty() {
                for (c, _) in &mut self.constraints {
                    *c = c.reduce(&fixes_var);
                }
                self.constraints.retain(|(c, _)| !c.is_trivially_true());
                for (_, obj) in &mut self.objectives {
                    *obj = obj.reduce(&fixes_var);
                }
            }
        }

        log(&format!(
            "[Modeler::build] Step 0: Fix resolution ({:.2?})",
            t_step.elapsed()
        ));

        // Step 1: transmute user constraints/objectives to InternalVar.
        let t_step = Instant::now();
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

        log(&format!(
            "[Modeler::build] Step 1: Transmutation ({:.2?})",
            t_step.elapsed()
        ));

        // Fast path: no extras means no DFS, no reconstruction.
        if extras.is_empty() {
            log("[Modeler::build] Steps 2-3: Skipped (no extras)");

            for (c, _) in &user_constraints {
                for v in c.variable_refs() {
                    if let InternalVar::Extra(e) = v {
                        return Err(BuildError::UndeclaredExtra(e.clone()));
                    }
                }
            }
            for v in folded_obj.get_function().variable_refs() {
                if let InternalVar::Extra(e) = v {
                    return Err(BuildError::UndeclaredExtra(e.clone()));
                }
            }

            let t_step = Instant::now();
            let mut all_vars: HashMap<InternalVar<B, E>, Variable> = HashMap::new();
            for (b, kind) in &base_vars {
                all_vars.insert(InternalVar::Base(b.clone()), kind.clone());
            }

            let checker_constraints = user_constraints.clone();
            let checker_vars = all_vars.clone();

            log(&format!(
                "[Modeler::build] Step 4: Constraint cloning ({:.2?})",
                t_step.elapsed()
            ));

            let t_step = Instant::now();
            let builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
                ProblemBuilder::new()
                    .set_variables(all_vars)
                    .add_constraints(user_constraints)
                    .set_objective(folded_obj);
            let problem = builder.build().map_err(BuildError::Ilp)?;
            log(&format!(
                "[Modeler::build] Step 5a: Main problem ({:.2?})",
                t_step.elapsed()
            ));

            let t_step = Instant::now();
            let checker_builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
                ProblemBuilder::new()
                    .set_variables(checker_vars)
                    .add_constraints(checker_constraints)
                    .set_objective(Objective::new(
                        LinExpr::constant(0.0),
                        ObjectiveSense::Minimize,
                    ));
            let checker_problem = checker_builder.build().map_err(BuildError::Ilp)?;
            log(&format!(
                "[Modeler::build] Step 5b: Checker problem ({:.2?})",
                t_step.elapsed()
            ));

            log(&format!(
                "[Modeler::build] Total ({:.2?})",
                t_total.elapsed()
            ));

            return Ok(Model {
                problem,
                reconstruction_constraints: Vec::new(),
                reconstruction_variables: HashMap::new(),
                base_variable_set: HashSet::new(),
                checker_problem,
                checker_reconstruction_constraints: Vec::new(),
                checker_reconstruction_variables: HashMap::new(),
                checker_base_variable_set: HashSet::new(),
                reconstruction_objective: Objective::default(),
                checker_reconstruction_objective: Objective::default(),
                base_var_list: base_vars,
                dependency_graph: DependencyGraph::new(HashMap::new()),
            });
        }

        // Step 2: collect initial roots (two passes).
        let t_step = Instant::now();
        let mut constraint_roots: Vec<E> = Vec::new();
        let mut seen_root: HashSet<E> = HashSet::new();
        for (c, _) in &user_constraints {
            for v in c.variable_refs() {
                if let InternalVar::Extra(e) = v
                    && seen_root.insert(e.clone())
                {
                    constraint_roots.push(e.clone());
                }
            }
        }
        log(&format!(
            "[Modeler::build] Step 2a: Constraint root collection ({:.2?})",
            t_step.elapsed()
        ));
        let t_step = Instant::now();
        let mut objective_roots: Vec<E> = Vec::new();
        for v in folded_obj.get_function().variable_refs() {
            if let InternalVar::Extra(e) = v
                && seen_root.insert(e.clone())
            {
                objective_roots.push(e.clone());
            }
        }

        log(&format!(
            "[Modeler::build] Step 2b: Objective root collection ({:.2?})",
            t_step.elapsed()
        ));

        // Step 2c: force-included roots — declared extras selected by
        // `force`, even if unreferenced. Seeded as objective-style roots
        // (for_constraints = false) so they never pollute the checker
        // problem.
        let mut forced_roots: Vec<E> = Vec::new();
        for name in extras_kinds.keys() {
            if force(name) && seen_root.insert(name.clone()) {
                forced_roots.push(name.clone());
            }
        }

        // Step 3: DFS expansion — constraint roots first, then
        // objective-only roots, then forced roots.
        let t_step = Instant::now();
        let mut state: BuildState<B, E, C> = BuildState {
            out_vars: HashMap::new(),
            out_constraints: user_constraints,
            expanded: HashSet::new(),
            in_progress: HashSet::new(),
            path: Vec::new(),
            dep_direct: HashMap::new(),
        };

        for root in constraint_roots {
            expand(
                &mut state,
                &mut extras,
                &base_vars,
                &extras_kinds,
                &fixers,
                env,
                &fix_cache,
                root,
                true,
            )?;
        }
        for root in objective_roots {
            expand(
                &mut state,
                &mut extras,
                &base_vars,
                &extras_kinds,
                &fixers,
                env,
                &fix_cache,
                root,
                false,
            )?;
        }
        for root in forced_roots {
            expand(
                &mut state,
                &mut extras,
                &base_vars,
                &extras_kinds,
                &fixers,
                env,
                &fix_cache,
                root,
                false,
            )?;
        }

        log(&format!(
            "[Modeler::build] Step 3: DFS expansion ({:.2?})",
            t_step.elapsed()
        ));

        // Close the collected direct dependency edges into the graph.
        let dependency_graph = DependencyGraph::from_direct(std::mem::take(&mut state.dep_direct));

        // Step 4: partition constraints for reconstruction.
        let t_step = Instant::now();
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

        // Step 4b: checker-specific partitioning.
        let checker_reconstruction_constraints: Vec<_> = state
            .out_constraints
            .iter()
            .filter(|(_, src)| {
                matches!(
                    src,
                    ConstraintSource::DefiningExtra {
                        for_constraints: true,
                        ..
                    }
                )
            })
            .cloned()
            .collect();

        let mut checker_reconstruction_variables: HashMap<InternalVar<B, E>, Variable> =
            HashMap::new();
        for (c, _) in &checker_reconstruction_constraints {
            for v in c.variable_refs() {
                if let Some(kind) = all_vars.get(v) {
                    checker_reconstruction_variables.insert(v.clone(), kind.clone());
                }
            }
        }

        let checker_base_variable_set: HashSet<B> = checker_reconstruction_variables
            .keys()
            .filter_map(|v| match v {
                InternalVar::Base(b) => Some(b.clone()),
                _ => None,
            })
            .collect();

        let checker_constraints: Vec<_> = state
            .out_constraints
            .iter()
            .filter(|(_, src)| {
                matches!(
                    src,
                    ConstraintSource::User(_)
                        | ConstraintSource::DefiningExtra {
                            for_constraints: true,
                            ..
                        }
                )
            })
            .cloned()
            .collect();

        let mut checker_vars: HashMap<InternalVar<B, E>, Variable> = HashMap::new();
        for (b, kind) in &base_vars {
            checker_vars.insert(InternalVar::Base(b.clone()), kind.clone());
        }
        for (c, _) in &checker_constraints {
            for v in c.variable_refs() {
                if !matches!(v, InternalVar::Base(_)) {
                    if let Some(kind) = all_vars.get(v) {
                        checker_vars.insert(v.clone(), kind.clone());
                    }
                }
            }
        }

        log(&format!(
            "[Modeler::build] Step 4: Constraint partitioning ({:.2?})",
            t_step.elapsed()
        ));

        // Precompute reconstruction objectives before step 5 consumes folded_obj.
        let reconstruction_objective = folded_obj.clone();
        let checker_reconstruction_objective = folded_obj.retained(|v| {
            matches!(v, InternalVar::Base(_)) || checker_reconstruction_variables.contains_key(v)
        });

        // Step 5: feed everything into ProblemBuilder.
        let t_step = Instant::now();
        let builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
            ProblemBuilder::new()
                .set_variables(all_vars)
                .add_constraints(state.out_constraints)
                .set_objective(folded_obj);
        let problem = builder.build().map_err(BuildError::Ilp)?;
        log(&format!(
            "[Modeler::build] Step 5a: Main problem ({:.2?})",
            t_step.elapsed()
        ));

        let t_step = Instant::now();
        let checker_builder: ProblemBuilder<InternalVar<B, E>, ConstraintSource<E, C>> =
            ProblemBuilder::new()
                .set_variables(checker_vars)
                .add_constraints(checker_constraints)
                .set_objective(Objective::new(
                    LinExpr::constant(0.0),
                    ObjectiveSense::Minimize,
                ));
        let checker_problem = checker_builder.build().map_err(BuildError::Ilp)?;
        log(&format!(
            "[Modeler::build] Step 5b: Checker problem ({:.2?})",
            t_step.elapsed()
        ));
        log(&format!(
            "[Modeler::build] Total ({:.2?})",
            t_total.elapsed()
        ));

        Ok(Model {
            problem,
            reconstruction_constraints,
            reconstruction_variables,
            base_variable_set,
            checker_problem,
            checker_reconstruction_constraints,
            checker_reconstruction_variables,
            checker_base_variable_set,
            reconstruction_objective,
            checker_reconstruction_objective,
            base_var_list: base_vars,
            dependency_graph,
        })
    }
}

fn expand<'s, 'm, B, E, C, Env, Err>(
    state: &'s mut BuildState<B, E, C>,
    extras: &'s mut HashMap<E, ExtraDef<'m, B, E, Env, Err>>,
    base_vars: &'s HashMap<B, Variable>,
    extras_kinds: &'s HashMap<E, Variable>,
    fixers: &'s [Box<FixerFn<'m, B, Env>>],
    env: &'s Env,
    fix_cache: &'s Mutex<HashMap<B, Option<f64>>>,
    e: E,
    for_constraints: bool,
) -> Result<(), BuildError<B, E, C, Err>>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Env: Sync,
    Err: Debug + Send + 'static,
    'm: 's,
{
    if state.expanded.contains(&e) {
        return Ok(());
    }
    if state.in_progress.contains(&e) {
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
        let ctx = VarContext {
            base: base_vars,
            extras: extras_kinds,
            fixers,
            env,
            cache: fix_cache,
        };
        (def.define)(&mut factory, &ctx, e.clone())
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

    // Safety-net fix: resolve undeclared base variables in
    // closure output via the VarContext cache.
    let ctx = VarContext {
        base: base_vars,
        extras: extras_kinds,
        fixers,
        env,
        cache: fix_cache,
    };
    let (constraints, _fixes) = ctx.fix_constraints(constraints);

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

    // Transmute constraints and append, collecting deps and the extra's
    // direct dependency edges (base variables + referenced extras).
    let mut deps: Vec<E> = Vec::new();
    let mut seen_dep: HashSet<E> = HashSet::new();
    let mut direct_bases: HashSet<B> = HashSet::new();
    let mut edge_targets: HashSet<E> = HashSet::new();
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
            match v {
                InternalVar::Base(b) => {
                    direct_bases.insert(b.clone());
                }
                InternalVar::Extra(ex) if *ex != e => {
                    edge_targets.insert(ex.clone());
                    if !state.expanded.contains(ex) && seen_dep.insert(ex.clone()) {
                        deps.push(ex.clone());
                    }
                }
                _ => {}
            }
        }
        state.out_constraints.push((
            tc,
            ConstraintSource::DefiningExtra {
                extra: e.clone(),
                index: i,
                for_constraints,
            },
        ));
    }

    // Record this extra's direct dependency edges for the dependency graph.
    state
        .dep_direct
        .insert(e.clone(), (direct_bases, edge_targets));

    // Recurse into deps.
    for d in deps {
        expand(
            state,
            extras,
            base_vars,
            extras_kinds,
            fixers,
            env,
            fix_cache,
            d,
            for_constraints,
        )?;
    }

    state.in_progress.remove(&e);
    let popped = state.path.pop();
    debug_assert_eq!(popped.as_ref(), Some(&e));
    state.expanded.insert(e);
    Ok(())
}
