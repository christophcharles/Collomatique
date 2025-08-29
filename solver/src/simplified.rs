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

use crate::{tools::AggregatedVariableConstraintDesc, BaseProblem, BaseVariable, ExtraVariable};
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

    /// Definition of the aggregated variables for the problem
    ///
    /// This is a list of variables all satisfying the [crate::tools::AggregatedVariable]
    /// constructed, directly or indirectly, from the main variables
    /// (returned by [SimpleBaseProblem::main_variables]).
    ///
    /// The order of variables is important: each variable should be reconstructable
    /// from the main variables and the variables before it in the list. We
    /// cannot have circular dependancies.
    ///
    /// The variable names should all be of type [SimpleBaseProblem::StructureVariable].
    /// Doing otherwise will lead to failed assertion in rebuilding the structure variables.
    fn aggregated_variables(
        &self,
    ) -> Vec<
        Box<
            dyn crate::tools::AggregatedVariable<
                crate::generics::BaseVariable<Self::MainVariable, Self::StructureVariable>,
            >,
        >,
    >;

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
pub trait SimpleProblemConstraints: Send + Sync {
    type Problem: BaseProblem;
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
    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool;

    /// Definition of the aggregated variables specific to this constraint set.
    ///
    /// This is a list of variables all satisfying the [crate::tools::AggregatedVariable]
    /// constructed, directly or indirectly, from the main variables
    /// (returned by [crate::BaseProblem::main_variables]) and possibly from
    /// the base problem structure variables (returned by [crate::BaseProblem::structure_variables]).
    ///
    /// The order of variables is important: each variable should be reconstructable
    /// from the main variables and the variables before it in the list. We
    /// cannot have circular dependancies.
    ///
    /// The variable names should all be of type [SimpleProblemConstraints::StructureVariable].
    /// Doing otherwise will lead to failed assertion in rebuilding the structure variables.
    fn extra_aggregated_variables(
        &self,
        desc: &Self::Problem,
    ) -> Vec<
        Box<
            dyn crate::tools::AggregatedVariable<
                crate::generics::ExtraVariable<
                    <Self::Problem as BaseProblem>::MainVariable,
                    <Self::Problem as BaseProblem>::StructureVariable,
                    Self::StructureVariable,
                >,
            >,
        >,
    >;

    /// Definition of the general constraints
    ///
    /// See [crate::ProblemConstraints] for the full discussion.
    fn general_constraints(
        &self,
        desc: &Self::Problem,
    ) -> Vec<(
        Constraint<
            ExtraVariable<
                <Self::Problem as BaseProblem>::MainVariable,
                <Self::Problem as BaseProblem>::StructureVariable,
                Self::StructureVariable,
            >,
        >,
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
        _desc: &Self::Problem,
    ) -> Objective<
        ExtraVariable<
            <Self::Problem as BaseProblem>::MainVariable,
            <Self::Problem as BaseProblem>::StructureVariable,
            Self::StructureVariable,
        >,
    > {
        Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize)
    }
}

impl<T: SimpleBaseProblem> crate::BaseProblem for T {
    type MainVariable = <Self as SimpleBaseProblem>::MainVariable;
    type StructureVariable = <Self as SimpleBaseProblem>::StructureVariable;
    type StructureConstraintDesc = AggregatedVariableConstraintDesc<
        crate::generics::BaseVariable<Self::MainVariable, Self::StructureVariable>,
    >;
    type PartialSolution = <Self as SimpleBaseProblem>::PartialSolution;

    fn main_variables(&self) -> BTreeMap<Self::MainVariable, Variable> {
        <Self as SimpleBaseProblem>::main_variables(self)
    }
    fn structure_variables(&self) -> BTreeMap<Self::StructureVariable, Variable> {
        let mut output = BTreeMap::new();
        for aggregated_var in self.aggregated_variables() {
            let (name, desc) = aggregated_var.get_variable_def();
            let BaseVariable::Structure(v) = name else {
                panic!(
                    "An aggregated variable has a main variable name: {:?}",
                    name
                );
            };

            if output.insert(v.clone(), desc).is_some() {
                panic!("Duplicated name for aggregated variable: {:?}", v);
            }
        }
        output
    }
    fn structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )> {
        let mut constraints = vec![];

        for aggregated_var in self.aggregated_variables() {
            constraints.extend(aggregated_var.get_structure_constraints());
        }

        constraints
    }
    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> Option<ConfigData<Self::MainVariable>> {
        <Self as SimpleBaseProblem>::partial_solution_to_configuration(self, sol)
    }
    fn configuration_to_partial_solution(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution {
        <Self as SimpleBaseProblem>::configuration_to_partial_solution(self, config)
    }
    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> ConfigData<Self::StructureVariable> {
        let mut temp_config = config.transmute(|x| BaseVariable::Main(x.clone()));

        for aggregated_var in self.aggregated_variables() {
            if let Some(value) = aggregated_var.reconstruct_structure_variable(&temp_config) {
                temp_config = temp_config.set(aggregated_var.get_variable_def().0, value);
            }
        }

        temp_config
            .retain(|name, _val| match name {
                BaseVariable::Main(_) => false,
                BaseVariable::Structure(_) => true,
            })
            .into_transmuted(|x| match x {
                BaseVariable::Structure(s) => s,
                _ => unreachable!(),
            })
    }
}

impl<T: SimpleProblemConstraints> crate::ProblemConstraints for T {
    type Problem = <Self as SimpleProblemConstraints>::Problem;
    type StructureVariable = <Self as SimpleProblemConstraints>::StructureVariable;
    type StructureConstraintDesc = AggregatedVariableConstraintDesc<
        crate::generics::ExtraVariable<
            <Self::Problem as BaseProblem>::MainVariable,
            <Self::Problem as BaseProblem>::StructureVariable,
            Self::StructureVariable,
        >,
    >;
    type GeneralConstraintDesc = T::GeneralConstraintDesc;

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        <Self as SimpleProblemConstraints>::is_fit_for_problem(self, desc)
    }
    fn extra_structure_variables(
        &self,
        desc: &Self::Problem,
    ) -> BTreeMap<Self::StructureVariable, Variable> {
        let mut output = BTreeMap::new();

        for aggregated_var in self.extra_aggregated_variables(desc) {
            let (name, var_desc) = aggregated_var.get_variable_def();
            let ExtraVariable::Extra(v) = name else {
                panic!(
                    "An aggregated variable has a base problem variable name: {:?}",
                    name
                );
            };

            if output.insert(v.clone(), var_desc).is_some() {
                panic!("Duplicated name for aggregated variable: {:?}", v);
            }
        }
        output
    }
    fn extra_structure_constraints(
        &self,
        desc: &Self::Problem,
    ) -> Vec<(
        Constraint<
            ExtraVariable<
                <Self::Problem as BaseProblem>::MainVariable,
                <Self::Problem as BaseProblem>::StructureVariable,
                Self::StructureVariable,
            >,
        >,
        Self::StructureConstraintDesc,
    )> {
        let mut constraints = vec![];

        for aggregated_var in self.extra_aggregated_variables(desc) {
            constraints.extend(aggregated_var.get_structure_constraints());
        }

        constraints
    }
    fn general_constraints(
        &self,
        desc: &Self::Problem,
    ) -> Vec<(
        Constraint<
            ExtraVariable<
                <Self::Problem as BaseProblem>::MainVariable,
                <Self::Problem as BaseProblem>::StructureVariable,
                Self::StructureVariable,
            >,
        >,
        Self::GeneralConstraintDesc,
    )> {
        <Self as SimpleProblemConstraints>::general_constraints(self, desc)
    }
    fn objective(
        &self,
        desc: &Self::Problem,
    ) -> Objective<
        ExtraVariable<
            <Self::Problem as BaseProblem>::MainVariable,
            <Self::Problem as BaseProblem>::StructureVariable,
            Self::StructureVariable,
        >,
    > {
        <Self as SimpleProblemConstraints>::objective(self, desc)
    }
    fn reconstruct_extra_structure_variables(
        &self,
        desc: &Self::Problem,
        config: &ConfigData<
            BaseVariable<
                <Self::Problem as BaseProblem>::MainVariable,
                <Self::Problem as BaseProblem>::StructureVariable,
            >,
        >,
    ) -> ConfigData<Self::StructureVariable> {
        let mut temp_config = config.transmute(|x| match x {
            BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
            BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
        });

        for aggregated_var in self.extra_aggregated_variables(desc) {
            if let Some(value) = aggregated_var.reconstruct_structure_variable(&temp_config) {
                temp_config = temp_config.set(aggregated_var.get_variable_def().0, value);
            }
        }

        temp_config
            .retain(|name, _val| match name {
                ExtraVariable::BaseMain(_) => false,
                ExtraVariable::BaseStructure(_) => false,
                ExtraVariable::Extra(_) => true,
            })
            .into_transmuted(|x| match x {
                ExtraVariable::Extra(e) => e,
                _ => unreachable!(),
            })
    }
}
