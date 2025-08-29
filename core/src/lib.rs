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

use collomatique_ilp::{Constraint, LinExpr, ObjectiveSense, UsableData};

pub trait BaseConstraints: Send + Sync + std::fmt::Debug + PartialEq + Eq {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;

    fn variables(&self) -> Vec<Self::VariableName>;
    fn constraints(&self) -> Vec<(Constraint<Self::VariableName>, Self::ConstraintDesc)>;

    fn objective_func(&self) -> LinExpr<Self::VariableName>;
    fn objective_sense(&self) -> ObjectiveSense {
        ObjectiveSense::Minimize
    }
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

    fn extra_variables(&self) -> Vec<Self::VariableName>;
    fn structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
    fn extra_constraints(
        &self,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
}

pub trait ExtraObjective<T: BaseConstraints> {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;

    fn extra_variables(&self) -> Vec<Self::VariableName>;
    fn structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
    fn objective_func(&self) -> LinExpr<ExtraVariable<T::VariableName, Self::VariableName>>;
    fn objective_sense(&self) -> ObjectiveSense {
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
pub struct ProblemBuilder<T: BaseConstraints> {
    base: T,
    id_issuer: IdIssuer,
}

impl<T: BaseConstraints> ProblemBuilder<T> {
    pub fn new(base: T) -> Self {
        ProblemBuilder {
            base,
            id_issuer: IdIssuer::new(),
        }
    }

    pub fn add_hard_constraints<'a, E: ExtraConstraints<T>>(
        &'a mut self,
        extra: &E,
    ) -> HardConstraintsChecker<'a, T, E> {
        todo!()
    }

    pub fn add_soft_constraints<'a, E: ExtraConstraints<T>>(
        &'a mut self,
        extra: &E,
    ) -> SoftConstraintsChecker<'a, T, E> {
        todo!()
    }

    pub fn add_objective<'a, E: ExtraObjective<T>>(
        &'a mut self,
        extra: &E,
    ) -> ObjectiveChecker<'a, T, E> {
        todo!()
    }

    pub fn build_problem(
        &self,
    ) -> collomatique_ilp::Problem<VariableName<T::VariableName>, InternalId> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardConstraintsChecker<'a, T: BaseConstraints, E: ExtraConstraints<T>> {
    phantom: std::marker::PhantomData<&'a ProblemBuilder<T>>,
    phantom2: std::marker::PhantomData<E>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoftConstraintsChecker<'a, T: BaseConstraints, E: ExtraConstraints<T>> {
    phantom: std::marker::PhantomData<&'a ProblemBuilder<T>>,
    phantom2: std::marker::PhantomData<E>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveChecker<'a, T: BaseConstraints, E: ExtraObjective<T>> {
    phantom: std::marker::PhantomData<&'a ProblemBuilder<T>>,
    phantom2: std::marker::PhantomData<E>,
}
