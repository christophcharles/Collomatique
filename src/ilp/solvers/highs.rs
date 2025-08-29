#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig, Problem};

#[derive(Debug, Clone)]
pub struct Solver {
    disable_logging: bool,
}

use super::{FeasabilitySolver, ProblemRepr, VariableName};
impl<V: VariableName, P: ProblemRepr<V>> FeasabilitySolver<V, P> for Solver {
    fn find_closest_solution<'a>(
        &self,
        config: &Config<'a, V, P>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.solve_internal(config, true)
    }

    fn solve<'a>(&self, problem: &'a Problem<V, P>) -> Option<FeasableConfig<'a, V, P>> {
        let init_config = problem.default_config();
        self.solve_internal(&init_config, false)
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
        minimize_distance_to_init_config: bool,
    ) -> Option<FeasableConfig<'a, V, P>> {
        // When everything is solved for some reason this is sometimes an issue...
        if let Some(result) = init_config.clone().into_feasable() {
            return Some(result);
        }

        let problem = init_config.get_problem();

        let highs_problem =
            self.build_problem(problem, init_config, minimize_distance_to_init_config);

        use highs::Sense;
        let mut model = highs_problem.problem.try_optimise(Sense::Minimise).ok()?;
        if self.disable_logging {
            model.make_quiet();
        }

        let solved_problem = model.try_solve().ok()?;

        Self::reconstruct_config(problem, &solved_problem)
    }

    fn build_problem<V: VariableName, P: ProblemRepr<V>>(
        &self,
        problem: &Problem<V, P>,
        init_config: &Config<'_, V, P>,
        minimize_distance_to_init_config: bool,
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
                        let value = if init_config.get_bool(var).expect("Variable should be valid")
                        {
                            1.
                        } else {
                            0.
                        };
                        // Try minimizing the number of changes with respect to the config
                        // So if a variable is true in the config, false should be penalized
                        // And if a variable is false in the config, true should be penalized
                        // So 1-2*value as a coefficient should work (it gives 1 for false and -1 for true).
                        let col_factor = if minimize_distance_to_init_config {
                            1. - 2. * value
                        } else {
                            0.
                        };

                        let col = highs_problem.add_integer_column(col_factor, 0..=1);
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
        use highs::HighsModelStatus;
        use std::collections::BTreeMap;

        if solved_model.status() != HighsModelStatus::Optimal {
            return None;
        }
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
                Some((var.clone(), columns[i] == 1.))
            })
            .collect();

        let config = problem
            .config_from(bool_vars)
            .expect("Variables should be valid");
        Some(
            config
                .into_feasable()
                .expect("Config from highs should be feasable"),
        )
    }
}
