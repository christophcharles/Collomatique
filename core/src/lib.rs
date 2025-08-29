//! Collomatique-core
//! ---
//!
//! This crate contains the main logic of Collomatique for solving colloscopes related problems.
//! The goal for this crate is to fulfill the role of a translator. It takes a description
//! of a colloscope (or at least the various constraints of a colloscope) and returns
//! an ILP problem as described by the crate [collomatique-ilp].
//!
//! Similarly, it can translate a solution of an ILP problem into the description of
//! an actual colloscope and conversly, it can take the description of a colloscope
//! and turn it into an ILP configuration. This is useful to check in real time if
//! a colloscope satisfies all the constraints and helps when constructing a colloscope
//! incrementally.
//!
//! This crate however does not expose a user-friendly interface. The reason is, to
//! make the translation algorithm as thin as possible, and its verification as easy as
//! possible, I strive to make the colloscopes constraints and the actual colloscopes
//! representation the least redundant possible.
//!
//! Also to keep this part lean, a lot of information is not represented as it is not
//! needed to build the constraint system. For instance, the name of the students or
//! the name of the teachers are not stored in the structures of this modules. Students
//! and teachers are represented with numbers and that's it. It is the job of other crates
//! from collomatique to provide necessary utilities to make working the algorithm
//! somewhat pleasant.
//!
//! The main struct is [ProblemBuilder] and you should start from there to see how this crate
//! works.

pub mod colloscopes;
pub mod time;

use collomatique_ilp::{ConfigData, Constraint, LinExpr, ObjectiveSense, UsableData, Variable};

pub trait BaseConstraints: Send + Sync + std::fmt::Debug + PartialEq + Eq {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;
    type Solution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq;

    fn variables(&self) -> Vec<(Self::VariableName, Variable)>;
    fn constraints(&self) -> Vec<(Constraint<Self::VariableName>, Self::ConstraintDesc)>;

    fn objective_func(&self) -> LinExpr<Self::VariableName>;
    fn objective_sense(&self) -> ObjectiveSense {
        ObjectiveSense::Minimize
    }

    fn solution_to_configuration(&self, sol: &Self::Solution) -> ConfigData<Self::VariableName>;
    fn configuration_to_solution(&self, config: &ConfigData<Self::VariableName>) -> Self::Solution;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExtraVariable<B: UsableData, E: UsableData> {
    Base(B),
    Extra(E),
}

impl<B: UsableData, E: UsableData> std::fmt::Display for ExtraVariable<B, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(v) => write!(f, "{}", v),
            Self::Extra(v) => write!(f, "{}", v),
        }
    }
}

pub trait ExtraConstraints<T: BaseConstraints> {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;

    fn extra_variables(&self, base: &T) -> Vec<(Self::VariableName, Variable)>;
    fn structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
    fn extra_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
}

pub trait ExtraObjective<T: BaseConstraints> {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;

    fn extra_variables(&self, base: &T) -> Vec<Self::VariableName>;
    fn structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
    fn objective_func(
        &self,
        base: &T,
    ) -> LinExpr<ExtraVariable<T::VariableName, Self::VariableName>>;
    fn objective_sense(&self, _base: &T) -> ObjectiveSense {
        ObjectiveSense::Minimize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternalId(u64);

impl std::fmt::Display for InternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct IdIssuer {
    available: u64,
}

impl IdIssuer {
    fn new() -> Self {
        Self::default()
    }

    fn get(&mut self) -> InternalId {
        let new_id = InternalId(self.available);
        self.available += 1;
        new_id
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum VariableName<V: UsableData> {
    Base(V),
    Extra(InternalId, String),
    Soft(InternalId),
}

impl<V: UsableData> std::fmt::Display for VariableName<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(v) => write!(f, "{}", v),
            Self::Extra(_id, desc) => write!(f, "{}", desc),
            Self::Soft(id) => write!(f, "soft_{}", id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemBuilder<V, T, P = collomatique_ilp::DefaultRepr<VariableName<V>>>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    base: T,
    id_issuer: IdIssuer,
    phantom_v: std::marker::PhantomData<V>,
    phantom_p: std::marker::PhantomData<P>,
}

impl<V, T, P> ProblemBuilder<V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub fn new(base: T) -> Self {
        ProblemBuilder {
            base,
            id_issuer: IdIssuer::new(),
            phantom_v: std::marker::PhantomData,
            phantom_p: std::marker::PhantomData,
        }
    }

    pub fn add_hard_constraints<E: ExtraConstraints<T>>(
        &mut self,
        _extra: E,
    ) -> ConstraintsTransator<E::ConstraintDesc> {
        todo!()
    }

    pub fn add_soft_constraints<E: ExtraConstraints<T>>(
        &mut self,
        _extra: E,
    ) -> ConstraintsTransator<E::ConstraintDesc> {
        todo!()
    }

    pub fn add_objective<E: ExtraObjective<T>>(
        &mut self,
        _extra: E,
    ) -> ConstraintsTransator<E::ConstraintDesc> {
        todo!()
    }

    pub fn build_problem(self) -> Problem<V, T, P> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintsTransator<C: UsableData> {
    phantom: std::marker::PhantomData<C>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem<V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    ilp_problem: collomatique_ilp::Problem<VariableName<V>, InternalId, P>,
    base: T,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solution<'a, V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    problem: &'a Problem<V, T, P>,
    internal_solution: T::Solution,
    ilp_solution: collomatique_ilp::Config<'a, VariableName<V>, InternalId, P>,
}

impl<'a, V, T, P> Solution<'a, V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub fn blame(&self) -> impl ExactSizeIterator<Item = &T::ConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }

    pub fn blame_with_translator<'b, C: UsableData>(
        &self,
        _translator: &'b ConstraintsTransator<C>,
    ) -> impl ExactSizeIterator<Item = &'b C> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeLimitSolution<'a, V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub solution: Solution<'a, V, T, P>,
    pub time_limit_reached: bool,
}

impl<V, T, P> Problem<V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub fn solve<
        'a,
        S: collomatique_ilp::solvers::Solver<VariableName<T::VariableName>, InternalId, P>,
    >(
        &'a self,
        _solver: &S,
    ) -> Option<Solution<'a, V, T, P>> {
        todo!()
    }

    pub fn solve_with_time_limit<
        'a,
        S: collomatique_ilp::solvers::SolverWithTimeLimit<
            VariableName<T::VariableName>,
            InternalId,
            P,
        >,
    >(
        &'a self,
        _solver: &S,
        _time_limit_in_seconds: u32,
    ) -> Option<TimeLimitSolution<'a, V, T, P>> {
        todo!()
    }
}
