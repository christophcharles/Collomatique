//! Solution types for optimization problems.
//!
//! This module defines:
//! - `Problem`: The constructed optimization problem
//! - `Solution`: A (possibly infeasible) solution
//! - `FeasableSolution`: A verified feasible solution

use super::types::ReifiedVar;
use crate::EvalVar;
use crate::database::DatabaseConnection;
use crate::eval::Origin;
use collomatique_ilp::solvers::Solver;
use collomatique_ilp::{ConfigData, DefaultRepr, Variable};
use collomatique_ilp_modeler::{ConstraintSource, InternalVar, Model};
use derivative::Derivative;
use std::collections::HashMap;

/// Type alias for the constraint source type used in our problems.
pub type ProblemConstraintSource<D> = ConstraintSource<ReifiedVar<D>, Option<Origin<D>>>;

/// Type alias for the internal variable type used in our problems.
pub type ProblemInternalVar<D, V> = InternalVar<V, ReifiedVar<D>>;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct Problem<D: DatabaseConnection, V: EvalVar> {
    model: Model<V, ReifiedVar<D>, Option<Origin<D>>>,
    pub(crate) original_var_list: HashMap<V, Variable>,
}

impl<D: DatabaseConnection, V: EvalVar> Problem<D, V> {
    pub(crate) fn new(
        model: Model<V, ReifiedVar<D>, Option<Origin<D>>>,
        original_var_list: HashMap<V, Variable>,
    ) -> Self {
        Problem {
            model,
            original_var_list,
        }
    }

    pub fn get_inner_problem(
        &self,
    ) -> &collomatique_ilp::Problem<ProblemInternalVar<D, V>, ProblemConstraintSource<D>> {
        self.model.problem()
    }

    pub fn solve<'a, S>(&'a self, solver: &S) -> Option<FeasableSolution<'a, D, V>>
    where
        S: Solver<
                ProblemInternalVar<D, V>,
                ProblemConstraintSource<D>,
                DefaultRepr<ProblemInternalVar<D, V>>,
            >,
    {
        solver
            .solve(self.model.problem())
            .map(|x| FeasableSolution { feasable_config: x })
    }

    pub fn solution_from_data<'a, S>(
        &'a self,
        config_data: &ConfigData<V>,
        solver: &S,
    ) -> Option<Solution<'a, D, V>>
    where
        S: Solver<
                ProblemInternalVar<D, V>,
                ProblemConstraintSource<D>,
                DefaultRepr<ProblemInternalVar<D, V>>,
            >,
    {
        if !self.check_no_missing_variables(config_data) {
            return None;
        }

        let base_values: HashMap<V, f64> = config_data.get_values().into_iter().collect();
        let recon_problem = self.model.reconstruction_problem(&base_values).ok()?;
        let recon_sol = solver
            .solve(&recon_problem)
            .expect("There should always be a (unique!) solution to the reconstruction problem");

        // Merge base variable values with extra/helper values from reconstruction.
        // reconstruction_problem only includes non-base variables, so we need to
        // add the base values back to create a complete config.
        let mut complete_values: HashMap<ProblemInternalVar<D, V>, f64> = base_values
            .into_iter()
            .map(|(b, v)| (InternalVar::Base(b), v))
            .collect();
        complete_values.extend(recon_sol.get_values());
        let new_config_data = ConfigData::from(complete_values);

        Some(
            self.solution_from_complete_data(new_config_data)
                .expect("The configuration data should be valid!"),
        )
    }

    pub fn solution_from_complete_data<'a>(
        &'a self,
        config_data: ConfigData<ProblemInternalVar<D, V>>,
    ) -> Option<Solution<'a, D, V>> {
        Some(Solution {
            config: self.model.problem().build_config(config_data).ok()?,
        })
    }

    fn check_variables_valid(&self, config_data: &ConfigData<V>) -> bool {
        config_data
            .get_values()
            .keys()
            .all(|x| self.original_var_list.contains_key(x))
    }

    fn check_no_missing_variables(&self, config_data: &ConfigData<V>) -> bool {
        if !self.check_variables_valid(config_data) {
            return false;
        }

        self.original_var_list
            .iter()
            .all(|(var, var_def)| match config_data.get(var.clone()) {
                Some(v) => var_def.checks_value(v),
                None => false,
            })
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""), Clone(bound = ""))]
pub struct Solution<'a, D: DatabaseConnection, V: EvalVar> {
    config: collomatique_ilp::Config<
        'a,
        ProblemInternalVar<D, V>,
        ProblemConstraintSource<D>,
        DefaultRepr<ProblemInternalVar<D, V>>,
    >,
}

impl<'a, D: DatabaseConnection, V: EvalVar> Solution<'a, D, V> {
    pub fn get_data(&self) -> ConfigData<V> {
        ConfigData::from(self.config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                InternalVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemInternalVar<D, V>> {
        ConfigData::from(self.config.get_values())
    }

    pub fn is_feasable(&self) -> bool {
        self.config.is_feasable()
    }

    pub fn into_feasable(self) -> Option<FeasableSolution<'a, D, V>> {
        Some(FeasableSolution {
            feasable_config: self.config.into_feasable()?,
        })
    }

    pub fn blame<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<
        Item = &'b (
            collomatique_ilp::Constraint<ProblemInternalVar<D, V>>,
            ProblemConstraintSource<D>,
        ),
    > + use<'a, 'b, D, V> {
        self.config.blame()
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""), Clone(bound = ""))]
pub struct FeasableSolution<'a, D: DatabaseConnection, V: EvalVar> {
    feasable_config: collomatique_ilp::FeasableConfig<
        'a,
        ProblemInternalVar<D, V>,
        ProblemConstraintSource<D>,
        DefaultRepr<ProblemInternalVar<D, V>>,
    >,
}

impl<'a, D: DatabaseConnection, V: EvalVar> FeasableSolution<'a, D, V> {
    pub fn into_solution(self) -> Solution<'a, D, V> {
        Solution {
            config: self.feasable_config.into_inner(),
        }
    }

    pub fn get_data(&self) -> ConfigData<V> {
        ConfigData::from(self.feasable_config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                InternalVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemInternalVar<D, V>> {
        ConfigData::from(self.feasable_config.get_values())
    }
}
