//! Simplified submodule
//!
//! This module defines two traits: [SimpleBaseProblem] and [SimpleProblemConstraints].
//!
//! You can implement these traits. They rely on the [crate::tools] module and particularly
//! the [crate::tools::AggregatedVariable] trait to provide a simplified (but more limited)
//! interface.
//!
//! If you implement [SimpleBaseProblem], the trait [crate::BaseProblem] will automatically
//! be implemented. Similarly if you implement [SimpleProblemConstraints], [crate::ProblemConstraints]
//! will automatically be implemented.

use crate::{BaseProblem, ExtraVariable};
use collomatique_ilp::{
    ConfigData, Constraint, LinExpr, Objective, ObjectiveSense, UsableData, Variable,
};
use std::collections::BTreeMap;

/// Simplified base trait for a problem description.
///
/// You should refer to [crate::BaseProblem] for further documentation on inner intracacies.
///
/// [SimpleBaseProblem] is simpler to implement as long as the following restrictions are
/// observed: all structure variables are defined through [crate::tools::AggregatedVariable].
/// If your problem can be expressed this way, the [crate::BaseProblem::structure_variables],
/// [crate::BaseProblem::structure_constraints] and [crate::BaseProblem::reconstruct_structure_variables]
/// will be implemented automagically.
pub trait SimpleBaseProblem: Send + Sync {
    /// Type to represent the main variables
    ///
    /// The main variables are the variables whose set of values is in one to one correspondance
    /// with the set of possible solutions.
    ///
    /// See [crate::BaseProblem] for the full discussion.
    type MainVariable: UsableData + 'static;

    /// Type to represent the structure variables
    ///
    /// The structure variables do not provide any new information and can entirely
    /// be rebuild from the main variables (represented by [SimpleBaseProblem::MainVariable]).
    /// They only have a practical utility and help representing the problem as an ILP problem.
    ///
    /// See [crate::BaseProblem] for the full discussion.
    type StructureVariable: UsableData + 'static;

    /// Partial solution type associated to the problem.
    ///
    /// This type is used in the rest of the program to represent a solution (actually a partial
    /// solution) to the problem.
    ///
    /// See [crate::BaseProblem] for the full discussion.
    type PartialSolution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq;

    /// Definition of the main variables for the problem.
    ///
    /// See [crate::BaseProblem] for the full discussion.
    fn main_variables(&self) -> BTreeMap<Self::MainVariable, Variable>;

    /// Converts a [SimpleBaseProblem::PartialSolution] into a set of values for the main variables.
    ///
    /// The description should be exactly one to one. This means two things:
    /// - first, [SimpleBaseProblem::partial_solution_to_configuration] and
    ///   [SimpleBaseProblem::configuration_to_partial_solution] should be reciprocal to each other.
    /// - second, if the solution is partial, this should be correctly reflected by not setting the value of some
    ///   main variables.
    ///
    /// This method can fail if the partial solution does not fit the problem. In that cas, `None` is returned.
    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> Option<ConfigData<Self::MainVariable>>;

    /// Converts a set of values for the main variables into a [SimpleBaseProblem::PartialSolution].
    ///
    /// The description should be exactly one to one. This means two things:
    /// - first, [SimpleBaseProblem::partial_solution_to_configuration] and
    ///   [SimpleBaseProblem::configuration_to_partial_solution] should be reciprocal to each other.
    /// - second, if the set of values is partial, this should be correctly reflected in a partial solution output.
    fn configuration_to_partial_solution(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution;
}

/// Simplified trait for a problem constraints description.
///
/// You should refer to [crate::ProblemConstraints] for further documentation on inner intracacies.
///
/// [SimpleProblemConstraints] is simpler to implement as long as the following restrictions are
/// observed: all structure variables are defined through [crate::tools::AggregatedVariable].
/// If your problem can be expressed this way, the [crate::ProblemConstraints::extra_structure_variables],
/// [crate::ProblemConstraints::extra_structure_constraints] and
/// [crate::ProblemConstraints::reconstruct_extra_structure_variables] will be implemented automagically.
pub trait SimpleProblemConstraints<T: BaseProblem>: Send + Sync {
    /// Type to represent the structure variables specific to this problem extension.
    ///
    /// The structure variables do not provide any new information and can entirely
    /// be rebuild from the main variables (represented by [BaseProblem::MainVariable]).
    /// They only have a practical utility and help representing the problem as an ILP problem.
    ///
    /// See [crate::ProblemConstraints] and [crate::BaseProblem] for the full discussion.
    type StructureVariable: UsableData + 'static;

    /// Type to represent the description of general constraints for the extension.
    ///
    /// Genral constraints actually define the extension to the problem using main variables
    /// ([crate::BaseProblem::MainVariable]), structure variables ([crate::BaseProblem::StructureVariable])
    /// from the original problem but also extra structure variables ([SimpleProblemConstraints::StructureVariable])
    /// from the problem extension.
    ///
    /// See [crate::ProblemConstraints] for the full discussion.
    type GeneralConstraintDesc: UsableData + 'static;

    /// Checks if the extension is compatible with the given problem
    fn is_fit_for_problem(&self, desc: &T) -> bool;

    /// Definition of the general constraints
    ///
    /// See [crate::ProblemConstraints] for the full discussion.
    fn general_constraints(
        &self,
        desc: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )>;

    /// Definition of a linear objective for the problem extension.
    ///
    /// This objective will be added (with a weight) to the overall objective.
    /// By default, you do not have to implement this function and it returns
    /// a trivial objective.
    ///
    /// See [crate::ProblemConstraints] for the full discussion.
    fn objective(
        &self,
        _desc: &T,
    ) -> Objective<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>
    {
        Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize)
    }
}
