//! Tools module.
//!
//! This modules gives a few tool to implement problem description
//! and constraints.
//!
//! Its main purpose is to help define structure variables
//! by providing tools for such a task.
//!
//! The main trait is [AggregatedVariable]. Any type that
//! implements [AggregatedVariable] defines a new structure variables
//! built from other variables.
//!
//! Two such cases are already implemented [AndVariable] that
//! defines a logical 'and' (using binary variables) and [OrVariable]
//! which similarly defines a logical 'or'.

#[cfg(test)]
mod tests;

use collomatique_ilp::{ConfigData, Constraint, LinExpr, UsableData, Variable};
use std::collections::BTreeSet;

/// Constraint description for [AggregatedVariable]
///
/// Aggregate variables are variables defined through structure constraints.
/// These structure constraints will need description.
///
/// Because we want the trait to be dyn compatible, the same description must be
/// used for all constraints. To have something generic *enough*, we can specify
/// the name of the variable we are defining through the constraint, a number identifying
/// the constraint and a plain text description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatedVariableConstraintDesc<ProblemVariable> {
    /// Name of the variable being definied through the constraint
    pub variable_name: ProblemVariable,
    /// Internal number of the structure constraint
    pub internal_number: usize,
    /// Plain text description of the constraint
    pub desc: String,
}

/// AgregateVariable trait
///
/// This trait must be implemnted by aggregate variable helpers.
/// A type that implements [AggregatedVariable] signals that it defines
/// a new variables from some other variables.
///
/// It defines such a variable through 3 functions that:
/// - provides the type and name of the ouput variable
/// - returns a list of linear constraints for the ILP problem
/// - provides a reconstruction function for programmatic reconstruction.
pub trait AggregatedVariable<ProblemVariable>: Send + Sync
where
    ProblemVariable: UsableData + 'static,
{
    /// Returns the name and type of the variable being defined
    fn get_variable_def(&self) -> (ProblemVariable, Variable);
    /// Return a list of structure constraints in order to define the new variable
    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariableConstraintDesc<ProblemVariable>,
    )>;
    /// Reconstructs the variable value from a [ConfigData].
    ///
    /// Some values might be missing in the provided [ConfigData].
    /// The function should still attempt the reconstruction. If it is
    /// impossible, it should simply return `None`.
    fn reconstruct_structure_variable(&self, config: &ConfigData<ProblemVariable>) -> Option<f64>;
}

/// Variable implementing a logical 'AND'.
///
/// [AndVariable] implements [AggregatedVariable] and can help
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

impl<ProblemVariable: UsableData + 'static> AggregatedVariable<ProblemVariable>
    for AndVariable<ProblemVariable>
{
    fn get_variable_def(&self) -> (ProblemVariable, Variable) {
        (self.variable_name.clone(), Variable::binary())
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariableConstraintDesc<ProblemVariable>,
    )> {
        let var_expr = LinExpr::<ProblemVariable>::var(self.variable_name.clone());
        let mut add_expr = LinExpr::constant(1. - self.original_variables.len() as f64);

        for orig_var in &self.original_variables {
            add_expr = add_expr + LinExpr::var(orig_var.clone());
        }

        let mut constraints = vec![(
            var_expr.geq(&add_expr),
            AggregatedVariableConstraintDesc {
                variable_name: self.variable_name.clone(),
                internal_number: 0,
                desc: "Variable should be 1 if all are 1".into(),
            },
        )];

        for (i, orig_var) in self.original_variables.iter().enumerate() {
            let orig_var_expr = LinExpr::var(orig_var.clone());
            constraints.push((
                var_expr.leq(&orig_var_expr),
                AggregatedVariableConstraintDesc {
                    variable_name: self.variable_name.clone(),
                    internal_number: i,
                    desc: "Variable should be 0 if one is 0".into(),
                },
            ));
        }

        constraints
    }

    fn reconstruct_structure_variable(&self, config: &ConfigData<ProblemVariable>) -> Option<f64> {
        let mut at_least_one_none = false;
        for orig_var in &self.original_variables {
            match config.get(orig_var.clone()) {
                Some(val) => {
                    if val < 0.5 {
                        return Some(0.);
                    }
                }
                None => {
                    at_least_one_none = true;
                }
            }
        }
        if at_least_one_none {
            None
        } else {
            Some(1.)
        }
    }
}

/// Variable implementing a logical 'OR'.
///
/// [OrVariable] implements [AggregatedVariable] and can help
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

impl<ProblemVariable: UsableData + 'static> AggregatedVariable<ProblemVariable>
    for OrVariable<ProblemVariable>
{
    fn get_variable_def(&self) -> (ProblemVariable, Variable) {
        (self.variable_name.clone(), Variable::binary())
    }

    fn get_structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<ProblemVariable>,
        AggregatedVariableConstraintDesc<ProblemVariable>,
    )> {
        let var_expr = LinExpr::<ProblemVariable>::var(self.variable_name.clone());
        let mut add_expr = LinExpr::constant(0.);

        for orig_var in &self.original_variables {
            add_expr = add_expr + LinExpr::var(orig_var.clone());
        }

        let mut constraints = vec![(
            var_expr.leq(&add_expr),
            AggregatedVariableConstraintDesc {
                variable_name: self.variable_name.clone(),
                internal_number: 0,
                desc: "Variable should be 0 if all are 0".into(),
            },
        )];

        for (i, orig_var) in self.original_variables.iter().enumerate() {
            let orig_var_expr = LinExpr::var(orig_var.clone());
            constraints.push((
                var_expr.geq(&orig_var_expr),
                AggregatedVariableConstraintDesc {
                    variable_name: self.variable_name.clone(),
                    internal_number: i,
                    desc: "Variable should be 1 if one is 1".into(),
                },
            ));
        }

        constraints
    }

    fn reconstruct_structure_variable(&self, config: &ConfigData<ProblemVariable>) -> Option<f64> {
        let mut at_least_one_none = false;
        for orig_var in &self.original_variables {
            match config.get(orig_var.clone()) {
                Some(val) => {
                    if val > 0.5 {
                        return Some(1.);
                    }
                }
                None => {
                    at_least_one_none = true;
                }
            }
        }
        if at_least_one_none {
            None
        } else {
            Some(0.)
        }
    }
}
