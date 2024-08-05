#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig, Problem};

#[derive(Debug, Clone)]
pub struct Solver {
    disable_logging: bool,
}

use super::{FeasabilitySolver, ProblemRepr, VariableName};
impl<V: VariableName, P: ProblemRepr<V>> FeasabilitySolver<V, P> for Solver {
    fn restore_feasability_with_origin_and_max_steps<'a>(
        &self,
        config: &Config<'a, V, P>,
        origin: Option<&FeasableConfig<'a, V, P>>,
        _max_steps: Option<usize>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        // When everything is solved for some reason this is sometimes an issue...
        if let Some(result) = config.clone().into_feasable() {
            return Some(result);
        }

        let problem = config.get_problem();

        let mut highs_problem = self.build_problem(problem, config);
        if let Some(o) = origin {
            Self::add_origin_constraints(&mut highs_problem, config, &o);
        }

        use highs::Sense;
        let mut model = highs_problem.problem.try_optimise(Sense::Minimise).ok()?;
        if self.disable_logging {
            model.make_quiet();
        }

        let solved_problem = model.try_solve().ok()?;

        Self::reconstruct_config(problem, &solved_problem)
    }
}

struct HighsProblem<V: VariableName> {
    problem: highs::RowProblem,
    cols: std::collections::BTreeMap<V, highs::Col>,
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

    fn build_problem<V: VariableName, P: ProblemRepr<V>>(
        &self,
        problem: &Problem<V, P>,
        config: &Config<'_, V, P>,
    ) -> HighsProblem<V> {
        use highs::RowProblem;
        use std::collections::BTreeMap;

        let mut highs_problem = RowProblem::default();

        let cols: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|var| {
                let value = if config.get(var).expect("Variable should be valid") {
                    1.
                } else {
                    0.
                };
                // Try minimizing the number of changes with respect to the config
                // So if a variable is true in the config, false should be penalized
                // And if a variable is false in the config, true should be penalized
                // So 1-2*value as a coefficient should work (it gives 1 for false and -1 for true).
                let col_factor = 1. - 2. * value;

                let col = highs_problem.add_integer_column(col_factor, 0..=1);
                (var.clone(), col)
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
            cols,
        }
    }

    fn add_origin_constraints<'a, V: VariableName, P: ProblemRepr<V>>(
        highs_problem: &mut HighsProblem<V>,
        config: &Config<'a, V, P>,
        origin: &FeasableConfig<'a, V, P>,
    ) {
        let changed_variables = config
            .get_problem()
            .get_variables()
            .iter()
            .filter(|var| config.get(var) != origin.get(var));

        for var in changed_variables {
            let col = highs_problem.cols[var];
            let value = if config.get(var).expect("Variable should be valid") {
                1.
            } else {
                0.
            };

            let row_factors = [(col, 1.0)];
            highs_problem.problem.add_row(value..=value, row_factors);
        }
    }

    fn reconstruct_config<'a, 'b, 'c, V: VariableName, P: ProblemRepr<V>>(
        problem: &'a Problem<V, P>,
        solved_model: &'b highs::SolvedModel,
    ) -> Option<FeasableConfig<'a, V, P>> {
        use highs::HighsModelStatus;
        use std::collections::BTreeSet;

        if solved_model.status() != HighsModelStatus::Optimal {
            return None;
        }
        let solution = solved_model.get_solution();
        let columns = solution.columns();

        let vars: BTreeSet<_> = problem
            .get_variables()
            .iter()
            .enumerate()
            .filter_map(|(i, var)| {
                if columns[i] == 1. {
                    Some(var.clone())
                } else {
                    None
                }
            })
            .collect();

        let config = problem
            .config_from(&vars)
            .expect("Variables should be valid");
        Some(
            config
                .into_feasable()
                .expect("Config from highs should be feasable"),
        )
    }
}
