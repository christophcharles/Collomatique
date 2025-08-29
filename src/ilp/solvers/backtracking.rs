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

    fn compute_next_step<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
    ) -> NextStep<V> {
        if config.is_feasable() {
            return NextStep::Solved;
        }

        let vars = self
            .heuristic
            .compute_guess_list(config, available_variables);
        if vars.is_empty() {
            NextStep::Backtrack
        } else {
            NextStep::StepInto(vars)
        }
    }

    fn select_variable<'a, V: VariableName>(
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        guess_list_stack: &Vec<(Vec<V>, usize)>,
    ) {
        let (vars, num) = guess_list_stack
            .last()
            .expect("There should be a guess to select");
        let var = vars.get(*num).expect("guess_list cursor should be valid");

        let current_val = config
            .get(&var)
            .expect("Variable should be available to get");
        config
            .set(&var, !current_val)
            .expect("Variable should be available to set");
        available_variables.remove(&var);
    }

    fn unselect_variable<'a, V: VariableName>(
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        guess_list_stack: &Vec<(Vec<V>, usize)>,
    ) {
        let (vars, num) = guess_list_stack
            .last()
            .expect("There should be a guess to unselect");
        let var = vars.get(*num).expect("guess_list cursor should be valid");

        let current_val = config
            .get(&var)
            .expect("Variable should be available to get");
        config
            .set(&var, !current_val)
            .expect("Variable should be available to set");
        available_variables.insert(var.clone());
    }

    fn backtrack<'a, V: VariableName>(
        &self,
        config: &mut Config<'a, V>,
        available_variables: &mut BTreeSet<V>,
        guess_list_stack: &mut Vec<(Vec<V>, usize)>,
    ) -> bool {
        loop {
            match guess_list_stack.last() {
                Some((guess_list, current_num)) => {
                    Self::unselect_variable(config, available_variables, &*guess_list_stack);

                    assert!(*current_num < guess_list.len());
                    if *current_num != guess_list.len() - 1 {
                        guess_list_stack.last_mut().unwrap().1 += 1;
                        Self::select_variable(config, available_variables, guess_list_stack);
                        return true;
                    } else {
                        guess_list_stack.pop();
                    }
                }
                None => {
                    return false;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum NextStep<V: VariableName> {
    StepInto(Vec<V>),
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
        let mut guess_list_stack = vec![];
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
                NextStep::StepInto(vars) => {
                    assert!(!vars.is_empty());

                    guess_list_stack.push((vars, 0));
                    Self::select_variable(
                        &mut temp_config,
                        &mut available_variables,
                        &guess_list_stack,
                    );
                }
                NextStep::Backtrack => {
                    if !self.backtrack(
                        &mut temp_config,
                        &mut available_variables,
                        &mut guess_list_stack,
                    ) {
                        return None;
                    }
                }
            }
        }
    }
}
