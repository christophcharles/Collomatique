pub mod random;

use super::{Config, Problem, ProblemRepr, VariableName};

pub trait ConfigInitializer<V: VariableName, P: ProblemRepr<V>> {
    fn build_init_config<'a, 'b>(&'a mut self, problem: &'b Problem<V, P>) -> Config<'b, V, P>;
}
