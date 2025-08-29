#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig};

use super::VariableName;

#[derive(Debug, Clone, Default)]
pub struct Solver {}

use std::collections::BTreeSet;

impl Solver {
    pub fn new() -> Self {
        Solver {}
    }

    fn choose_variable<'a, V: VariableName>(
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
        previous_var: Option<&V>,
    ) -> Option<V> {
        match previous_var {
            Some(v) => {
                let mut iterator = available_variables.iter();
                while let Some(val) = iterator.next() {
                    if v == val {
                        break;
                    }
                }
                iterator.next().cloned()
            }
            None => available_variables.first().cloned(),
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
