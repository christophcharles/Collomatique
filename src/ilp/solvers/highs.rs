#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig, Problem};

#[derive(Debug, Clone)]
pub struct Solver {
    disable_logging: bool,
}

enum Objective {
    None,
    MinimumDistance,
    MinimumObjectiveFn,
}

use super::{FeasabilitySolver, ProblemRepr, VariableName};
impl<V: VariableName, P: ProblemRepr<V>> FeasabilitySolver<V, P> for Solver {
    fn find_closest_solution_with_time_limit<'a>(
        &self,
        config: &Config<'a, V, P>,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.solve_internal(config, Objective::MinimumDistance, time_limit_in_seconds)
    }

    fn solve<'a>(
        &self,
        config_hint: &Config<'a, V, P>,
        minimize_objective: bool,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.solve_internal(
            config_hint,
            if minimize_objective {
                Objective::MinimumObjectiveFn
            } else {
                Objective::None
            },
            time_limit_in_seconds,
        )
    }
}

struct HighsProblem {
    problem: highs::RowProblem,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new()
    }
}

impl Solver {
    pub fn new() -> Self {
        Solver {
            disable_logging: true,
        }
    }

    pub fn with_disable_logging(disable_logging: bool) -> Self {
        Solver { disable_logging }
    }

    fn solve_internal<'a, V: VariableName, P: ProblemRepr<V>>(
        &self,
        init_config: &Config<'a, V, P>,
        objective: Objective,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        // When everything is solved for some reason this is sometimes an issue...
        if let Some(result) = init_config.clone().into_feasable() {
            return Some(result);
        }

        let problem = init_config.get_problem();

        let highs_problem = self.build_problem(problem, init_config, objective);

        use highs::Sense;
        let mut model = highs_problem.problem.try_optimise(Sense::Minimise).ok()?;
        if self.disable_logging {
            model.make_quiet();
        }

        if let Some(time_limit) = time_limit_in_seconds {
            model.set_option("time_limit", f64::from(time_limit));
        }

        let solved_problem = model.try_solve().ok()?;

        Self::reconstruct_config(problem, &solved_problem)
    }

    fn build_problem<V: VariableName, P: ProblemRepr<V>>(
        &self,
        problem: &Problem<V, P>,
        init_config: &Config<'_, V, P>,
        objective: Objective,
    ) -> HighsProblem {
        use crate::ilp::VariableType;
        use highs::RowProblem;
        use std::collections::BTreeMap;

        let mut highs_problem = RowProblem::default();

        let cols: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|(var, var_type)| {
                match var_type {
                    VariableType::Bool => {
                        let col_factor = match objective {
                            Objective::MinimumDistance => {
                                let value =
                                    if init_config.get_bool(var).expect("Variable should be valid")
                                    {
                                        1.
                                    } else {
                                        0.
                                    };
                                // Try minimizing the number of changes with respect to the config
                                // So if a variable is true in the config, false should be penalized
                                // And if a variable is false in the config, true should be penalized
                                // So 1-2*value as a coefficient should work (it gives 1 for false and -1 for true).
                                1. - 2. * value
                            }
                            Objective::MinimumObjectiveFn => {
                                match problem.get_objective_fn().get(var) {
                                    Some(coef) => coef.into(),
                                    None => 0.,
                                }
                            }
                            Objective::None => 0.,
                        };

                        let col = highs_problem.add_integer_column(col_factor, 0..=1);
                        (var.clone(), col)
                    }
                    VariableType::Integer(range) => {
                        let col_factor = match objective {
                            Objective::MinimumDistance => {
                                // Ignore dist minimization for integers variables
                                0.
                            }
                            Objective::MinimumObjectiveFn => {
                                match problem.get_objective_fn().get(var) {
                                    Some(coef) => coef.into(),
                                    None => 0.,
                                }
                            }
                            Objective::None => 0.,
                        };

                        let col = highs_problem.add_integer_column(col_factor, range.clone());
                        (var.clone(), col)
                    }
                }
            })
            .collect();

        for constraint in problem.get_constraints() {
            let variables = constraint.variables();
            let row_factors = variables.iter().map(|var| {
                let col = cols[var];
                let weight = f64::from(constraint.get_var(var).unwrap());

                (col, weight)
            });

            let neg_constant = f64::from(-constraint.get_constant());
            match constraint.get_sign() {
                crate::ilp::linexpr::Sign::Equals => {
                    highs_problem.add_row(neg_constant..=neg_constant, row_factors);
                }
                crate::ilp::linexpr::Sign::LessThan => {
                    highs_problem.add_row(..=neg_constant, row_factors);
                }
            };
        }

        HighsProblem {
            problem: highs_problem,
        }
    }

    fn reconstruct_config<'a, 'b, 'c, V: VariableName, P: ProblemRepr<V>>(
        problem: &'a Problem<V, P>,
        solved_model: &'b highs::SolvedModel,
    ) -> Option<FeasableConfig<'a, V, P>> {
        use crate::ilp::VariableType;
        use std::collections::BTreeMap;

        let solution = solved_model.get_solution();
        let columns = solution.columns();

        let bool_vars: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .enumerate()
            .filter_map(|(i, (var, var_type))| {
                if *var_type != VariableType::Bool {
                    return None;
                }
                Some((var.clone(), columns[i] > 0.5))
            })
            .collect();

        let i32_vars: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .enumerate()
            .filter_map(|(i, (var, var_type))| {
                let VariableType::Integer(_range) = var_type else {
                    return None;
                };
                Some((var.clone(), columns[i] as i32))
            })
            .collect();

        let config = problem
            .config_from(bool_vars, i32_vars)
            .expect("Variables should be valid");
        config.into_feasable()
    }
}
