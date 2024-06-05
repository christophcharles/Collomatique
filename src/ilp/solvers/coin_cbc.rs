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
        max_steps: Option<usize>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        // When everything is solved for some reason this is sometimes an issue...
        if let Some(result) = config.clone().into_feasable() {
            return Some(result);
        }

        let problem = config.get_problem();

        let mut cbc_model = self.build_model(problem, config);
        if let Some(ms) = max_steps {
            cbc_model.model.set_parameter("maxN", &format!("{}", ms));
        }
        if let Some(o) = origin {
            Self::add_origin_constraints(&mut cbc_model, config, &o);
        }

        let sol = cbc_model.model.solve();

        Self::reconstruct_config(problem, &sol, &cbc_model.cols)
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
            disable_logging: false,
        }
    }

    pub fn with_disable_logging(disable_logging: bool) -> Self {
        Solver { disable_logging }
    }

    fn build_model<V: VariableName, P: ProblemRepr<V>>(
        &self,
        problem: &Problem<V, P>,
        config: &Config<'_, V, P>,
    ) -> CbcModel<V> {
        use coin_cbc::{Model, Sense};
        use std::collections::BTreeMap;

        let mut model = Model::default();

        let cols: BTreeMap<_, _> = problem
            .get_variables()
            .iter()
            .map(|v| (v.clone(), model.add_binary()))
            .collect();

        model.set_obj_sense(Sense::Minimize);
        for (var, col) in &cols {
            let value = if config.get(var).expect("Variable should be valid") {
                1.
            } else {
                0.
            };
            model.set_col_initial_solution(*col, value);

            // Try minimizing the number of changes with respect to the config
            // So if a variable is true in the config, false should be penalized
            // And if a variable is false in the config, true should be penalized
            // So 1-2*value as a coefficient should work (it gives 1 for false and -1 for true).
            model.set_obj_coeff(*col, 1. - 2. * value);
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

    fn add_origin_constraints<'a, V: VariableName, P: ProblemRepr<V>>(
        cbc_model: &mut CbcModel<V>,
        config: &Config<'a, V, P>,
        origin: &FeasableConfig<'a, V, P>,
    ) {
        let changed_variables = config
            .get_problem()
            .get_variables()
            .iter()
            .filter(|var| config.get(var) != origin.get(var));

        for var in changed_variables {
            let col = cbc_model.cols[var];
            let value = if config.get(var).expect("Variable should be valid") {
                1.
            } else {
                0.
            };

            let row = cbc_model.model.add_row();
            cbc_model.model.set_weight(row, col, 1.);
            cbc_model.model.set_row_equal(row, value);
        }
    }

    fn reconstruct_config<'a, 'b, 'c, V: VariableName, P: ProblemRepr<V>>(
        problem: &'a Problem<V, P>,
        sol: &'b coin_cbc::Solution,
        cols: &'c std::collections::BTreeMap<V, coin_cbc::Col>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        use coin_cbc::raw::{SecondaryStatus, Status};
        use std::collections::BTreeSet;

        if sol.raw().status() != Status::Finished {
            return None;
        }
        if sol.raw().secondary_status() != SecondaryStatus::HasSolution {
            return None;
        }

        let vars: BTreeSet<_> = cols
            .iter()
            .filter_map(|(v, col)| {
                if sol.col(*col) == 1. {
                    Some(v.clone())
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
                .expect("Config from coin_cbc should be feasable"),
        )
    }
}
