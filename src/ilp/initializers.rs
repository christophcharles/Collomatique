use super::{Config, Problem, ProblemRepr, VariableName};

pub trait ConfigInitializer<V: VariableName, P: ProblemRepr<V>> {
    fn build_init_config<'a, 'b>(&'a mut self, problem: &'b Problem<V, P>) -> Config<'b, V, P>;
}

use crate::ilp::random::RandomGen;

#[derive(Clone, Debug)]
pub struct Random<T: RandomGen> {
    random_gen: T,
}

impl<T: RandomGen> Random<T> {
    pub fn new(random_gen: T) -> Self {
        Random { random_gen }
    }
}

impl<V: VariableName, P: ProblemRepr<V>, T: RandomGen> ConfigInitializer<V, P> for Random<T> {
    fn build_init_config<'a, 'b>(&'a mut self, problem: &'b Problem<V, P>) -> Config<'b, V, P> {
        use std::collections::BTreeSet;
        let mut vars = BTreeSet::new();

        for var in problem.get_variables() {
            if self.random_gen.randbool() {
                vars.insert(var);
            }
        }

        problem.config_from(vars).expect("Valid variables")
    }
}
