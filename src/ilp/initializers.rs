use super::{Config, Problem, ProblemRepr, VariableName};

pub trait ConfigInitializer<V: VariableName, P: ProblemRepr<V>> {
    fn build_init_config<'a, 'b>(&'a self, problem: &'b Problem<V, P>) -> Option<Config<'b, V, P>>;
}

use crate::ilp::random::RandomGen;

#[derive(Clone, Debug)]
pub struct Random<T: RandomGen> {
    random_gen: T,
    one_out_of: usize,
}

impl<T: RandomGen> Random<T> {
    pub fn new(random_gen: T) -> Self {
        Random {
            random_gen,
            one_out_of: 2,
        }
    }

    pub fn with_one_out_of(random_gen: T, one_out_of: usize) -> Option<Self> {
        if one_out_of < 2 {
            return None;
        }
        Some(Random {
            random_gen,
            one_out_of,
        })
    }
}

impl<V: VariableName, P: ProblemRepr<V>, T: RandomGen> ConfigInitializer<V, P> for Random<T> {
    fn build_init_config<'a, 'b>(&'a self, problem: &'b Problem<V, P>) -> Option<Config<'b, V, P>> {
        use std::collections::BTreeSet;
        let mut vars = BTreeSet::new();

        for var in problem.get_variables() {
            if self.random_gen.rand_in_range(0..self.one_out_of) == 0 {
                vars.insert(var);
            }
        }

        Some(problem.config_from(vars).expect("Valid variables"))
    }
}

#[derive(Clone, Debug)]
pub struct Null {}

impl Null {
    pub fn new() -> Self {
        Null {}
    }
}

impl<V: VariableName, P: ProblemRepr<V>> ConfigInitializer<V, P> for Null {
    fn build_init_config<'a, 'b>(&'a self, problem: &'b Problem<V, P>) -> Option<Config<'b, V, P>> {
        Some(problem.default_config())
    }
}
