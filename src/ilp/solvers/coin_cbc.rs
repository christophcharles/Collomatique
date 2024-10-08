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

struct CbcModel<V: VariableName> {
    model: coin_cbc::Model,
    cols: std::collections::BTreeMap<V, coin_cbc::Col>,
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
        // cbc does not seem to shut up even if logging is disabled
        // we block output directly
        let stdout_gag = gag::Gag::stdout();
        // We allow for errors in case this is run in multiple threads
        if !self.disable_logging {
            if let Ok(gag) = stdout_gag {
                drop(gag);
            }
        }

        let problem = init_config.get_problem();

        let mut cbc_model = self.build_model(problem, init_config);
        match objective {
            Objective::MinimumDistance => {
                self.add_minimize_dist_constraint(&mut cbc_model, init_config)
            }
            Objective::MinimumObjectiveFn => self.add_objective_terms(&mut cbc_model, problem),
            Objective::None => {}
        }

        if let Some(time_limit) = time_limit_in_seconds {
            cbc_model.model.set_parameter("timeMode", "elapsed");
            cbc_model
                .model
                .set_parameter("seconds", &time_limit.to_string());
        }

        let sol = cbc_model.model.solve();

        Self::reconstruct_config(problem, &sol, &cbc_model.cols)
    }

    fn build_model<V: VariableName, P: ProblemRepr<V>>(
        &self,
        problem: &Problem<V, P>,
        init_config: &Config<'_, V, P>,
    ) -> CbcModel<V> {
        use coin_cbc::Model;
        use std::collections::BTreeMap;

        let mut model = Model::default();

        let cols: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|v| (v.clone(), model.add_binary()))
            .collect();

        for (var, col) in &cols {
            let value = if init_config.get_bool(var).expect("Variable should be valid") {
                1.
            } else {
                0.
            };
            model.set_col_initial_solution(*col, value);
        }

        for constraint in problem.get_constraints() {
            let row = model.add_row();
            for v in constraint.variables() {
                let col = cols[&v];
                let weight = constraint.get_var(v).unwrap();
                model.set_weight(row, col, weight.into());
            }
            match constraint.get_sign() {
                crate::ilp::linexpr::Sign::Equals => {
                    model.set_row_equal(row, (-constraint.get_constant()).into());
                }
                crate::ilp::linexpr::Sign::LessThan => {
                    model.set_row_upper(row, (-constraint.get_constant()).into());
                }
            }
        }

        if self.disable_logging {
            model.set_parameter("log", "0");
            model.set_parameter("slog", "0");
        }

        CbcModel { model, cols }
    }

    fn add_minimize_dist_constraint<V: VariableName, P: ProblemRepr<V>>(
        &self,
        cbc_model: &mut CbcModel<V>,
        init_config: &Config<'_, V, P>,
    ) {
        use coin_cbc::Sense;
        cbc_model.model.set_obj_sense(Sense::Minimize);
        for (var, col) in &cbc_model.cols {
            let value = if init_config.get_bool(var).expect("Variable should be valid") {
                1.
            } else {
                0.
            };
            // Try minimizing the number of changes with respect to the config
            // So if a variable is true in the config, false should be penalized
            // And if a variable is false in the config, true should be penalized
            // So 1-2*value as a coefficient should work (it gives 1 for false and -1 for true).
            cbc_model.model.set_obj_coeff(*col, 1. - 2. * value);
        }
    }

    fn add_objective_terms<V: VariableName, P: ProblemRepr<V>>(
        &self,
        cbc_model: &mut CbcModel<V>,
        problem: &Problem<V, P>,
    ) {
        use coin_cbc::Sense;
        cbc_model.model.set_obj_sense(Sense::Minimize);

        for objective_term in problem.get_objective_terms() {
            let new_col = cbc_model.model.add_col();
            cbc_model.model.set_col_lower(new_col, -f64::INFINITY);
            cbc_model.model.set_col_upper(new_col, f64::INFINITY);

            cbc_model.model.set_obj_coeff(new_col, objective_term.coef);

            for expr in &objective_term.exprs {
                let row = cbc_model.model.add_row();

                cbc_model.model.set_weight(row, new_col, -1.);

                for v in expr.variables() {
                    let col = cbc_model.cols[&v];
                    let weight = expr.get(v).unwrap();

                    cbc_model.model.set_weight(row, col, weight.into());
                }

                cbc_model
                    .model
                    .set_row_upper(row, (-expr.get_constant()).into());
            }
        }

        for (v, coef) in problem.get_objective_contribs() {
            let col = cbc_model.cols[&v];
            cbc_model.model.set_obj_coeff(col, *coef);
        }
    }

    fn reconstruct_config<'a, 'b, 'c, V: VariableName, P: ProblemRepr<V>>(
        problem: &'a Problem<V, P>,
        sol: &'b coin_cbc::Solution,
        cols: &'c std::collections::BTreeMap<V, coin_cbc::Col>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        use std::collections::BTreeMap;

        let bool_vars: BTreeMap<_, _> = cols
            .iter()
            .map(|(v, col)| (v.clone(), sol.col(*col) == 1.))
            .collect();

        let config = problem
            .config_from(bool_vars)
            .expect("Variables should be valid");
        config.into_feasable()
    }
}
