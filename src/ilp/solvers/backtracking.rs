#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig};

use super::VariableName;

#[derive(Debug, Clone, Default)]
pub struct Solver {}

use std::collections::{BTreeMap, BTreeSet};

impl Solver {
    pub fn new() -> Self {
        Solver {}
    }

    fn is_var_helpful<V: VariableName>(
        constraint: &crate::ilp::linexpr::Constraint<V>,
        lhs: i32,
        coef: i32,
        current_val: bool,
    ) -> bool {
        use crate::ilp::linexpr::Sign;
        match constraint.get_sign() {
            Sign::LessThan => {
                if current_val == true {
                    coef > 0
                } else {
                    coef < 0
                }
            }
            Sign::Equals => {
                if current_val == true {
                    coef * lhs > 0
                } else {
                    coef * lhs < 0
                }
            }
        }
    }

    fn compute_help<'a, V: VariableName>(
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
        constraint: &crate::ilp::linexpr::Constraint<V>,
        lhs: i32,
    ) -> i32 {
        let variables = constraint.variables();
        let mut help = 0;
        for var in variables.intersection(available_variables) {
            let coef = constraint.get_var(var).expect("Variable should be valid");
            let current_val = config.get(var).expect("Variable should be valid");

            if Self::is_var_helpful(constraint, lhs, coef, current_val) {
                help += coef.abs();
            }
        }
        help
    }

    fn compute_help_scores<'a, V: VariableName>(
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
    ) -> Result<BTreeMap<V, ordered_float::NotNan<f64>>, Option<V>> {
        let lhs_map = config.compute_lhs();

        use ordered_float::NotNan;
        let mut scores = BTreeMap::<V, NotNan<f64>>::new();

        for (c, lhs) in &lhs_map {
            use crate::ilp::linexpr::Sign;
            let infeasability = match c.get_sign() {
                Sign::LessThan => (*lhs).max(0),
                Sign::Equals => (*lhs).abs(),
            };
            assert!(infeasability >= 0);

            if infeasability == 0 {
                continue;
            }

            let help = Self::compute_help(config, available_variables, c, *lhs);
            assert!(help >= 0);

            if infeasability > help {
                return Err(None);
            }

            let inf_f64 = NotNan::new(f64::from(infeasability)).unwrap();
            let help_f64 = NotNan::new(f64::from(help)).unwrap();

            let k = inf_f64 / help_f64;

            let variables = c.variables();
            for var in variables.intersection(available_variables) {
                let coef = c.get_var(var).expect("Variable should be valid");
                let current_val = config.get(var).expect("Variable should be valid");
                if !Self::is_var_helpful(c, *lhs, coef, current_val) {
                    continue;
                }

                if infeasability > help - coef {
                    return Err(Some(var.clone()));
                }

                let coef_f64 = NotNan::new(f64::from(coef)).unwrap();
                let temp_score = k * coef_f64;
                match scores.get_mut(var) {
                    Some(score) => {
                        *score += temp_score;
                    }
                    None => {
                        scores.insert(var.clone(), temp_score);
                    }
                }
            }
        }

        Ok(scores)
    }

    fn choose_variable<'a, V: VariableName>(
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
        previous_var: Option<&V>,
    ) -> Option<V> {
        let h_s = Self::compute_help_scores(config, available_variables);
        match h_s {
            Ok(scores) => {
                let mut scores_vec: Vec<_> = scores.into_iter().collect();
                scores_vec.sort_by_key(|(_v, s)| *s);
                let scores_vec: Vec<_> = scores_vec.into_iter().map(|(v, _s)| v).collect();

                match previous_var {
                    Some(var) => {
                        let mut iterator = scores_vec.iter();
                        while let Some(v) = iterator.next_back() {
                            if *v == *var {
                                break;
                            }
                        }
                        iterator.next_back().cloned()
                    }
                    None => scores_vec.last().cloned(),
                }
            }
            Err(opt_v) => match opt_v {
                None => None,
                Some(v) => {
                    if let Some(pv) = previous_var {
                        if *pv == v {
                            return None;
                        }
                    }
                    Some(v)
                }
            },
        }
    }

    fn compute_next_step<'a, V: VariableName>(
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
    ) -> NextStep<V> {
        if config.is_feasable() {
            return NextStep::Solved;
        }

        match Self::choose_variable(config, available_variables, None) {
            Some(var) => NextStep::StepInto(var),
            None => NextStep::Backtrack,
        }
    }

    fn select_variable<'a, V: VariableName>(
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        choice_stack: &mut Vec<V>,
        var: V,
    ) {
        let current_val = config
            .get(&var)
            .expect("Variable should be available to get");
        config
            .set(&var, !current_val)
            .expect("Variable should be available to set");
        available_variables.remove(&var);
        choice_stack.push(var);
    }

    fn unselect_variable<'a, V: VariableName>(
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        var: V,
    ) {
        let current_val = config
            .get(&var)
            .expect("Variable should be available to get");
        config
            .set(&var, !current_val)
            .expect("Variable should be available to set");
        available_variables.insert(var);
    }

    fn backtrack<'a, V: VariableName>(
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        choice_stack: &mut Vec<V>,
    ) -> bool {
        loop {
            match choice_stack.pop() {
                Some(old_var) => {
                    Self::unselect_variable(config, available_variables, old_var.clone());

                    if let Some(new_var) =
                        Self::choose_variable(config, available_variables, Some(&old_var))
                    {
                        Self::select_variable(config, available_variables, choice_stack, new_var);
                        return true;
                    }
                    // No else clause: if no more variables, we loop and backtrack further
                }
                None => {
                    return false;
                }
            }
        }
    }
}

enum NextStep<V: VariableName> {
    StepInto(V),
    Backtrack,
    Solved,
}

use super::FeasabilitySolver;

impl<V: VariableName> FeasabilitySolver<V> for Solver {
    fn restore_feasability_with_origin<'a>(
        &self,
        config: &Config<'a, V>,
        origin: Option<&FeasableConfig<'a, V>>,
    ) -> Option<FeasableConfig<'a, V>> {
        let mut available_variables = match origin {
            Some(o) => config
                .get_problem()
                .get_variables()
                .iter()
                .filter(|var| config.get(var) == o.get(var))
                .cloned()
                .collect(),
            None => config.get_problem().get_variables().clone(),
        };

        let mut temp_config = config.clone();
        let mut choice_stack = vec![];
        loop {
            match Self::compute_next_step(&temp_config, &available_variables) {
                NextStep::Solved => return Some(unsafe { temp_config.into_feasable_unchecked() }),
                NextStep::StepInto(var) => {
                    Self::select_variable(
                        &mut temp_config,
                        &mut available_variables,
                        &mut choice_stack,
                        var,
                    );
                }
                NextStep::Backtrack => {
                    if !Self::backtrack(
                        &mut temp_config,
                        &mut available_variables,
                        &mut choice_stack,
                    ) {
                        return None;
                    }
                }
            }
        }
    }
}
