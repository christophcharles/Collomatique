#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig, Problem, VariableType};

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
    fn find_closest_solution<'a>(
        &self,
        config: &Config<'a, V, P>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.solve_internal(config, Objective::MinimumDistance)
    }

    fn solve<'a>(
        &self,
        problem: &'a Problem<V, P>,
        minimize_objective: bool,
    ) -> Option<FeasableConfig<'a, V, P>> {
        let init_config = problem.default_config();
        self.solve_internal(
            &init_config,
            if minimize_objective {
                Objective::MinimumObjectiveFn
            } else {
                Objective::None
            },
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
            Objective::MinimumObjectiveFn => self.add_objective_fn(&mut cbc_model, problem),
            Objective::None => {}
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
            .map(|(v, t)| match t {
                VariableType::Bool => (v.clone(), model.add_binary()),
                VariableType::Integer(range) => {
                    let col = model.add_integer();
                    model.set_col_lower(col, (*range.start()).into());
                    model.set_col_upper(col, (*range.end()).into());
                    (v.clone(), col)
                }
            })
            .collect();

        for (var, col) in &cols {
            match problem
                .get_variables()
                .get(var)
                .expect("Variable should be valid")
            {
                VariableType::Bool => {
                    let value = if init_config.get_bool(var).expect("Variable should be valid") {
                        1.
                    } else {
                        0.
                    };
                    model.set_col_initial_solution(*col, value);
                }
                VariableType::Integer(_range) => {
                    let value = init_config
                        .get_i32(var)
                        .expect("Variable should be valid")
                        .into();
                    model.set_col_initial_solution(*col, value);
                }
            }
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
            match init_config
                .get_problem()
                .get_variables()
                .get(var)
                .expect("Variable should be valid")
            {
                VariableType::Bool => {
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
                VariableType::Integer(_range) => {
                    // We have to ignore...
                }
            }
        }
    }

    fn add_objective_fn<V: VariableName, P: ProblemRepr<V>>(
        &self,
        cbc_model: &mut CbcModel<V>,
        problem: &Problem<V, P>,
    ) {
        use coin_cbc::Sense;
        cbc_model.model.set_obj_sense(Sense::Minimize);
        for (var, col) in &cbc_model.cols {
            match problem.get_objective_fn().get(var) {
                Some(coef) => {
                    cbc_model.model.set_obj_coeff(*col, coef.into());
                }
                None => {
                    cbc_model.model.set_obj_coeff(*col, 0.);
                }
            }
        }
    }

    fn reconstruct_config<'a, 'b, 'c, V: VariableName, P: ProblemRepr<V>>(
        problem: &'a Problem<V, P>,
        sol: &'b coin_cbc::Solution,
        cols: &'c std::collections::BTreeMap<V, coin_cbc::Col>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        use coin_cbc::raw::{SecondaryStatus, Status};
        use std::collections::BTreeMap;

        if sol.raw().status() != Status::Finished {
            return None;
        }
        if sol.raw().secondary_status() != SecondaryStatus::HasSolution {
            return None;
        }

        let var_types = problem.get_variables();
        let bool_vars: BTreeMap<_, _> = cols
            .iter()
            .filter_map(|(v, col)| {
                let var_type = var_types.get(v).expect("Variable should be declared");
                if *var_type != VariableType::Bool {
                    return None;
                }

                Some((v.clone(), sol.col(*col) == 1.))
            })
            .collect();

        let i32_vars: BTreeMap<_, _> = cols
            .iter()
            .filter_map(|(v, col)| {
                let var_type = var_types.get(v).expect("Variable should be declared");
                let VariableType::Integer(_range) = var_type else {
                    return None;
                };

                Some((v.clone(), sol.col(*col) as i32))
            })
            .collect();

        let config = problem
            .config_from(bool_vars, i32_vars)
            .expect("Variables should be valid");
        Some(
            config
                .into_feasable()
                .expect("Config from coin_cbc should be feasable"),
        )
    }
}
