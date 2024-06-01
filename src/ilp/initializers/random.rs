use super::ConfigInitializer;
use crate::ilp::random::RandomGen;
use crate::ilp::{Config, Problem, ProblemRepr, VariableName};

#[derive(Clone, Debug)]
pub struct Initializer<T: RandomGen> {
    random_gen: T,
}

impl<T: RandomGen> Initializer<T> {
    pub fn new(random_gen: T) -> Self {
        Initializer { random_gen }
    }
}

impl<V: VariableName, P: ProblemRepr<V>, T: RandomGen> ConfigInitializer<V, P> for Initializer<T> {
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
