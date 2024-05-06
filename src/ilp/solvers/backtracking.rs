#[cfg(test)]
mod tests;

pub mod heuristics;

use crate::ilp::{Config, FeasableConfig};

use super::VariableName;

#[derive(Debug, Clone)]
pub struct Solver<H: heuristics::Heuristic> {
    heuristic: H,
}

use std::collections::BTreeSet;

impl<H: heuristics::Heuristic> Solver<H> {
    pub fn new(heuristic: H) -> Self {
        Solver { heuristic }
    }

    fn choose_variable<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
        previous_var: Option<&V>,
    ) -> Option<V> {
        let guess_list = self
            .heuristic
            .compute_guess_list(config, available_variables);

        match previous_var {
            Some(var) => {
                let mut iterator = guess_list.iter();
                while let Some(v) = iterator.next() {
                    if *v == *var {
                        break;
                    }
                }
                iterator.next().cloned()
            }
            None => guess_list.first().cloned(),
        }
    }

    fn compute_next_step<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
    ) -> NextStep<V> {
        if config.is_feasable() {
            return NextStep::Solved;
        }

        match self.choose_variable(config, available_variables, None) {
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
        &self,
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        choice_stack: &mut Vec<V>,
    ) -> bool {
        loop {
            match choice_stack.pop() {
                Some(old_var) => {
                    Self::unselect_variable(config, available_variables, old_var.clone());

                    if let Some(new_var) =
                        self.choose_variable(config, available_variables, Some(&old_var))
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

impl<V: VariableName, H: heuristics::Heuristic> FeasabilitySolver<V> for Solver<H> {
    fn restore_feasability_with_origin_and_max_steps<'a>(
        &self,
        config: &Config<'a, V>,
        origin: Option<&FeasableConfig<'a, V>>,
        mut max_steps: Option<usize>,
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
            if let Some(ms) = max_steps {
                if ms == 0 {
                    return None;
                } else {
                    max_steps = Some(ms - 1);
                }
            }
            match self.compute_next_step(&temp_config, &available_variables) {
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
                    if !self.backtrack(
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
