//! Tools module.
//!
//! This modules gives a few tool to implement problem description
//! and constraints.
//!
//! Its main purpose is to help define structure variables
//! by providing tools for such a task.
//!
//! The main trait is [AggregatedVariables]. Any type that
//! implements [AggregatedVariables] defines a new structure variables
//! built from other variables.
//!
//! Two such cases are already implemented [AndVariable] that
//! defines a logical 'and' (using binary variables) and [OrVariable]
//! which similarly defines a logical 'or'.

#[cfg(test)]
mod tests;

use collomatique_ilp::{ConfigData, Constraint, LinExpr, UsableData, Variable};
use std::{collections::BTreeSet, ops::RangeInclusive};

/// Constraint description for [AggregatedVariables]
///
/// Aggregate variables are variables defined through structure constraints.
/// These structure constraints will need description.
///
/// Because we want the trait to be dyn compatible, the same description must be
/// used for all constraints. To have something generic *enough*, we can specify
/// the name of the variable we are defining through the constraint, a number identifying
/// the constraint and a plain text description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatedVariablesConstraintDesc<ProblemVariable: UsableData + 'static> {
    /// Name of the variable being definied through the constraint
    pub variable_names: Vec<ProblemVariable>,
    /// Internal number of the structure constraint
    pub internal_number: usize,
    /// Plain text description of the constraintSized
    pub desc: String,
}

/// AgregateVariable trait
///
/// This trait must be implemented by aggregate variable helpers.
/// A type that implements [AggregatedVariables] signals that it defines
/// new variables from some other variables.
///
/// It defines such a variable through 3 functions that:
/// - provides the type and name of the ouput variable
/// - returns a list of linear constraints for the ILP problem
/// - provides a reconstruction function for programmatic reconstruction.
pub trait AggregatedVariables<ProblemVariable>: Send + Sync
where
    ProblemVariable: UsableData + 'static,
{
    /// Returns the list of names and types of the variables being defined
    fn get_variables_def(&self) -> Vec<(ProblemVariable, Variable)>;
    /// Return a list of structure constraints in order to define the new variable
    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariablesConstraintDesc<ProblemVariable>,
    )>;
    /// Reconstructs the variable value from a [ConfigData].
    ///
    /// Some values might be missing in the provided [ConfigData].
    /// The function should still attempt the reconstruction. If it is
    /// impossible, it should simply return `None`.
    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<ProblemVariable>,
    ) -> Vec<Option<f64>>;
}

/// Variable implementing a logical 'AND'.
///
/// [AndVariable] implements [AggregatedVariables] and can help
/// build a logical 'AND' in problems. It takes a name for
/// the new variable and a list of variables (that should be binary
/// otherwise the result is undefined) that should be ANDed together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndVariable<ProblemVariable>
where
    ProblemVariable: UsableData + 'static,
{
    /// Name for the construction variable
    pub variable_name: ProblemVariable,
    /// List of variables that should be ANDed together
    pub original_variables: BTreeSet<ProblemVariable>,
}

impl<ProblemVariable: UsableData + 'static> AggregatedVariables<ProblemVariable>
    for AndVariable<ProblemVariable>
{
    fn get_variables_def(&self) -> Vec<(ProblemVariable, Variable)> {
        vec![(self.variable_name.clone(), Variable::binary())]
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariablesConstraintDesc<ProblemVariable>,
    )> {
        let var_expr = LinExpr::<ProblemVariable>::var(self.variable_name.clone());
        let mut add_expr = LinExpr::constant(1. - self.original_variables.len() as f64);

        for orig_var in &self.original_variables {
            add_expr = add_expr + LinExpr::var(orig_var.clone());
        }

        let mut constraints = vec![(
            var_expr.geq(&add_expr),
            AggregatedVariablesConstraintDesc {
                variable_names: vec![self.variable_name.clone()],
                internal_number: 0,
                desc: "Variable should be 1 if all are 1".into(),
            },
        )];

        for (i, orig_var) in self.original_variables.iter().enumerate() {
            let orig_var_expr = LinExpr::var(orig_var.clone());
            constraints.push((
                var_expr.leq(&orig_var_expr),
                AggregatedVariablesConstraintDesc {
                    variable_names: vec![self.variable_name.clone()],
                    internal_number: i,
                    desc: "Variable should be 0 if one is 0".into(),
                },
            ));
        }

        constraints
    }

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<ProblemVariable>,
    ) -> Vec<Option<f64>> {
        let mut at_least_one_none = false;
        for orig_var in &self.original_variables {
            match config.get(orig_var.clone()) {
                Some(val) => {
                    if val < 0.5 {
                        return vec![Some(0.)];
                    }
                }
                None => {
                    at_least_one_none = true;
                }
            }
        }
        vec![if at_least_one_none { None } else { Some(1.) }]
    }
}

/// Variable implementing a logical 'OR'.
///
/// [OrVariable] implements [AggregatedVariables] and can help
/// build a logical 'OR' in problems. It takes a name for
/// the new variable and a list of variables (that should be binary
/// otherwise the result is undefined) that should be ORed together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrVariable<ProblemVariable>
where
    ProblemVariable: UsableData + 'static,
{
    /// Name for the construction variable
    pub variable_name: ProblemVariable,
    /// List of variables that should be ORed together
    pub original_variables: BTreeSet<ProblemVariable>,
}

impl<ProblemVariable: UsableData + 'static> AggregatedVariables<ProblemVariable>
    for OrVariable<ProblemVariable>
{
    fn get_variables_def(&self) -> Vec<(ProblemVariable, Variable)> {
        vec![(self.variable_name.clone(), Variable::binary())]
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariablesConstraintDesc<ProblemVariable>,
    )> {
        let var_expr = LinExpr::<ProblemVariable>::var(self.variable_name.clone());
        let mut add_expr = LinExpr::constant(0.);

        for orig_var in &self.original_variables {
            add_expr = add_expr + LinExpr::var(orig_var.clone());
        }

        let mut constraints = vec![(
            var_expr.leq(&add_expr),
            AggregatedVariablesConstraintDesc {
                variable_names: vec![self.variable_name.clone()],
                internal_number: 0,
                desc: "Variable should be 0 if all are 0".into(),
            },
        )];

        for (i, orig_var) in self.original_variables.iter().enumerate() {
            let orig_var_expr = LinExpr::var(orig_var.clone());
            constraints.push((
                var_expr.geq(&orig_var_expr),
                AggregatedVariablesConstraintDesc {
                    variable_names: vec![self.variable_name.clone()],
                    internal_number: i,
                    desc: "Variable should be 1 if one is 1".into(),
                },
            ));
        }

        constraints
    }

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<ProblemVariable>,
    ) -> Vec<Option<f64>> {
        let mut at_least_one_none = false;
        for orig_var in &self.original_variables {
            match config.get(orig_var.clone()) {
                Some(val) => {
                    if val > 0.5 {
                        return vec![Some(1.)];
                    }
                }
                None => {
                    at_least_one_none = true;
                }
            }
        }
        vec![if at_least_one_none { None } else { Some(0.) }]
    }
}

/// Variable implementing a logical 'NOT'.
///
/// [NotVariable] implements [AggregatedVariables] and can help
/// build a logical 'NOT' in problems. It takes a name for
/// the new variable and an existing variable (that should be binary
/// otherwise the result is undefined) that should be NOTed against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotVariable<ProblemVariable>
where
    ProblemVariable: UsableData + 'static,
{
    /// Name for the construction variable
    pub variable_name: ProblemVariable,
    /// Name of the original variable to inverse
    pub original_variable: ProblemVariable,
}

impl<ProblemVariable: UsableData + 'static> AggregatedVariables<ProblemVariable>
    for NotVariable<ProblemVariable>
{
    fn get_variables_def(&self) -> Vec<(ProblemVariable, Variable)> {
        vec![(self.variable_name.clone(), Variable::binary())]
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariablesConstraintDesc<ProblemVariable>,
    )> {
        let var_expr = LinExpr::<ProblemVariable>::var(self.variable_name.clone());
        let orig_var_expr = LinExpr::var(self.original_variable.clone());
        let one = LinExpr::constant(1.);

        let constraint = var_expr.eq(&(one - orig_var_expr));

        vec![(
            constraint,
            AggregatedVariablesConstraintDesc {
                variable_names: vec![self.variable_name.clone()],
                internal_number: 0,
                desc: "New variable should be 1 minus the old one".into(),
            },
        )]
    }

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<ProblemVariable>,
    ) -> Vec<Option<f64>> {
        vec![match config.get(self.original_variable.clone()) {
            Some(val) => Some(1. - val),
            None => None,
        }]
    }
}

/// Variable implementing a conversion from integer variable to a collection
/// of binary variables.
///
/// [IntToBinVariable] implements [AggregatedVariable] and can help
/// convert a single variable having an integer value into a collection of binary
/// variables with value 1 if a specific value is set. It takes a closure for
/// the new variables and an initial integer variables (that should have a finite range).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UIntToBinVariables<ProblemVariable, F>
where
    ProblemVariable: UsableData + 'static,
    F: Fn(u32) -> ProblemVariable + Send + Sync,
{
    /// Name for the new variables
    pub variable_name_builder: F,
    /// Original integer variable
    pub original_variable: ProblemVariable,
    /// Range for the original variable
    pub original_range: RangeInclusive<u32>,
}

impl<ProblemVariable: UsableData + 'static, F: Fn(u32) -> ProblemVariable + Send + Sync>
    AggregatedVariables<ProblemVariable> for UIntToBinVariables<ProblemVariable, F>
{
    fn get_variables_def(&self) -> Vec<(ProblemVariable, Variable)> {
        self.original_range
            .clone()
            .into_iter()
            .map(|i| ((self.variable_name_builder)(i), Variable::binary()))
            .collect()
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariablesConstraintDesc<ProblemVariable>,
    )> {
        let rhs1 = LinExpr::<ProblemVariable>::var(self.original_variable.clone());
        let mut lhs1 = LinExpr::constant(0.);

        for i in self.original_range.clone() {
            lhs1 = lhs1 + f64::from(i) * LinExpr::var((self.variable_name_builder)(i));
        }

        let number_constraint = lhs1.eq(&rhs1);

        let rhs2 = LinExpr::<ProblemVariable>::constant(1.);
        let mut lhs2 = LinExpr::constant(0.);

        for i in self.original_range.clone() {
            lhs2 = lhs2 + LinExpr::var((self.variable_name_builder)(i));
        }

        let only_one_var_constraint = lhs2.eq(&rhs2);

        let variable_names: Vec<_> = self
            .original_range
            .clone()
            .into_iter()
            .map(|i| (self.variable_name_builder)(i))
            .collect();

        let constraints = vec![
            (
                number_constraint,
                AggregatedVariablesConstraintDesc {
                    variable_names: variable_names.clone(),
                    internal_number: 0,
                    desc: "Original variable value should be reconstructible".into(),
                },
            ),
            (
                only_one_var_constraint,
                AggregatedVariablesConstraintDesc {
                    variable_names,
                    internal_number: 1,
                    desc: "Only one binary variable should be 1".into(),
                },
            ),
        ];

        constraints
    }

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<ProblemVariable>,
    ) -> Vec<Option<f64>> {
        match config.get(self.original_variable.clone()) {
            None => self
                .original_range
                .clone()
                .into_iter()
                .map(|_| None)
                .collect(),
            Some(v) => self
                .original_range
                .clone()
                .into_iter()
                .map(|i| {
                    let test_v = f64::from(i);
                    if v > test_v - 0.5 && v < test_v + 0.5 {
                        Some(1.)
                    } else {
                        Some(0.)
                    }
                })
                .collect(),
        }
    }
}

/// Variable implementing a logical 'YES'.
///
/// [YesVariable] implements [AggregatedVariables] and can help
/// build a logical 'YES' in problems. It takes a name for
/// the new variable and an existing variable (that should be binary
/// otherwise the result is undefined). The state of the existing variable
/// will be copied. This can be useful to provide multiple names for the same
/// value or to handle some edge cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YesVariable<ProblemVariable>
where
    ProblemVariable: UsableData + 'static,
{
    /// Name for the construction variable
    pub variable_name: ProblemVariable,
    /// Name of the original variable to copy
    pub original_variable: ProblemVariable,
}

impl<ProblemVariable: UsableData + 'static> AggregatedVariables<ProblemVariable>
    for YesVariable<ProblemVariable>
{
    fn get_variables_def(&self) -> Vec<(ProblemVariable, Variable)> {
        vec![(self.variable_name.clone(), Variable::binary())]
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariablesConstraintDesc<ProblemVariable>,
    )> {
        let var_expr = LinExpr::<ProblemVariable>::var(self.variable_name.clone());
        let orig_var_expr = LinExpr::var(self.original_variable.clone());

        let constraint = var_expr.eq(&orig_var_expr);

        vec![(
            constraint,
            AggregatedVariablesConstraintDesc {
                variable_names: vec![self.variable_name.clone()],
                internal_number: 0,
                desc: "New variable should be a copy of the old one".into(),
            },
        )]
    }

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<ProblemVariable>,
    ) -> Vec<Option<f64>> {
        vec![config.get(self.original_variable.clone())]
    }
}
