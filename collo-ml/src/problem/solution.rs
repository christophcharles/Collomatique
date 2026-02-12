//! Solution types for optimization problems.
//!
//! This module defines:
//! - `Problem`: The constructed optimization problem
//! - `Solution`: A (possibly infeasible) solution
//! - `FeasableSolution`: A verified feasible solution

use super::types::{ConstraintDesc, ExtraDesc, ProblemVar};
use crate::EvalVar;
use crate::database::DatabaseConnection;
use crate::traits::EvalObject;
use collomatique_ilp::solvers::Solver;
use collomatique_ilp::{ConfigData, Constraint, DefaultRepr, LinExpr, Variable};
use derivative::Derivative;
use std::collections::BTreeMap;

#[derive(Derivative)]
#[derivative(
    Debug(bound = "T: EvalObject"),
    Clone(bound = "T: EvalObject"),
    PartialEq(bound = "T: EvalObject"),
    Eq(bound = "T: EvalObject")
)]
pub struct Problem<T: EvalObject, D: DatabaseConnection, V: EvalVar> {
    problem: collomatique_ilp::Problem<ProblemVar<T, D, V>, ConstraintDesc<T, D>>,
    pub(crate) reification_problem_builder:
        collomatique_ilp::ProblemBuilder<ProblemVar<T, D, V>, ExtraDesc<T, D, V>>,
    pub(crate) original_var_list: BTreeMap<V, Variable>,
}

impl<T: EvalObject, D: DatabaseConnection, V: EvalVar> Problem<T, D, V> {
    pub(crate) fn new(
        problem: collomatique_ilp::Problem<ProblemVar<T, D, V>, ConstraintDesc<T, D>>,
        reification_problem_builder: collomatique_ilp::ProblemBuilder<
            ProblemVar<T, D, V>,
            ExtraDesc<T, D, V>,
        >,
        original_var_list: BTreeMap<V, Variable>,
    ) -> Self {
        Problem {
            problem,
            reification_problem_builder,
            original_var_list,
        }
    }

    pub fn get_inner_problem(
        &self,
    ) -> &collomatique_ilp::Problem<ProblemVar<T, D, V>, ConstraintDesc<T, D>> {
        &self.problem
    }

    pub fn solve<
        'a,
        S: Solver<ProblemVar<T, D, V>, ConstraintDesc<T, D>, DefaultRepr<ProblemVar<T, D, V>>>,
    >(
        &'a self,
        solver: &S,
    ) -> Option<FeasableSolution<'a, T, D, V>> {
        solver
            .solve(&self.problem)
            .map(|x| FeasableSolution { feasable_config: x })
    }

    pub fn solution_from_data<
        'a,
        S: Solver<ProblemVar<T, D, V>, ExtraDesc<T, D, V>, DefaultRepr<ProblemVar<T, D, V>>>,
    >(
        &'a self,
        config_data: &ConfigData<V>,
        solver: &S,
    ) -> Option<Solution<'a, T, D, V>> {
        if !self.check_no_missing_variables(config_data) {
            return None;
        }

        let reification_problem = self
            .reification_problem_builder
            .clone()
            .add_constraints(Self::build_equality_constraints_from_config_data(
                config_data,
            ))
            .build()
            .expect("The reification problem should always be valid");
        let reification_sol = solver
            .solve(&reification_problem)
            .expect("There should always be a (unique!) solution to the reification problem");
        let inner_data = reification_sol.get_values();
        let new_config_data = ConfigData::from(inner_data);

        Some(
            self.solution_from_complete_data(new_config_data)
                .expect("The configuration data should be valid!"),
        )
    }

    pub fn solution_from_complete_data<'a>(
        &'a self,
        config_data: ConfigData<ProblemVar<T, D, V>>,
    ) -> Option<Solution<'a, T, D, V>> {
        Some(Solution {
            config: self.problem.build_config(config_data).ok()?,
        })
    }

    fn build_equality_constraints_from_config_data(
        config_data: &ConfigData<V>,
    ) -> impl Iterator<Item = (Constraint<ProblemVar<T, D, V>>, ExtraDesc<T, D, V>)> {
        config_data.get_values().into_iter().map(|(var, value)| {
            let var_expr = LinExpr::var(ProblemVar::Base(var.clone()));
            let value_expr = LinExpr::constant(value);
            let constraint = var_expr.eq(&value_expr);
            let desc = ExtraDesc::InitCond(var);
            (constraint, desc)
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
#[derivative(Debug(bound = "T: EvalObject"), Clone(bound = "T: EvalObject"))]
pub struct Solution<'a, T: EvalObject, D: DatabaseConnection, V: EvalVar> {
    config: collomatique_ilp::Config<
        'a,
        ProblemVar<T, D, V>,
        ConstraintDesc<T, D>,
        DefaultRepr<ProblemVar<T, D, V>>,
    >,
}

impl<'a, T: EvalObject, D: DatabaseConnection, V: EvalVar> Solution<'a, T, D, V> {
    pub fn get_data(&self) -> ConfigData<V> {
        ConfigData::from(self.config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                ProblemVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemVar<T, D, V>> {
        ConfigData::from(self.config.get_values())
    }

    pub fn is_feasable(&self) -> bool {
        self.config.is_feasable()
    }

    pub fn into_feasable(self) -> Option<FeasableSolution<'a, T, D, V>> {
        Some(FeasableSolution {
            feasable_config: self.config.into_feasable()?,
        })
    }

    pub fn blame(&self) -> Vec<(Constraint<ProblemVar<T, D, V>>, ConstraintDesc<T, D>)> {
        self.config.blame().cloned().collect()
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "T: EvalObject"), Clone(bound = "T: EvalObject"))]
pub struct FeasableSolution<'a, T: EvalObject, D: DatabaseConnection, V: EvalVar> {
    feasable_config: collomatique_ilp::FeasableConfig<
        'a,
        ProblemVar<T, D, V>,
        ConstraintDesc<T, D>,
        DefaultRepr<ProblemVar<T, D, V>>,
    >,
}

impl<'a, T: EvalObject, D: DatabaseConnection, V: EvalVar> FeasableSolution<'a, T, D, V> {
    pub fn into_solution(self) -> Solution<'a, T, D, V> {
        Solution {
            config: self.feasable_config.into_inner(),
        }
    }

    pub fn get_data(&self) -> ConfigData<V> {
        ConfigData::from(self.feasable_config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                ProblemVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemVar<T, D, V>> {
        ConfigData::from(self.feasable_config.get_values())
    }
}
