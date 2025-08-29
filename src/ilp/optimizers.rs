pub mod genetic;
pub mod sa;

use crate::ilp::linexpr::VariableName;
use crate::ilp::mat_repr::ProblemRepr;
use crate::ilp::random::RandomGen;
use crate::ilp::Config;

pub trait MutationPolicy<V: VariableName, P: ProblemRepr<V>>: Sync + Send {
    fn mutate<'a>(&self, config: &Config<'a, V, P>) -> Option<Config<'a, V, P>>;
}

#[derive(Clone, Debug)]
pub struct RandomMutationPolicy<R: RandomGen> {
    random_gen: R,
    p: f64,
}

impl<R: RandomGen> RandomMutationPolicy<R> {
    pub fn new(random_gen: R, p: f64) -> Self {
        RandomMutationPolicy { random_gen, p }
    }
}

impl<V: VariableName, P: ProblemRepr<V>, R: RandomGen> MutationPolicy<V, P>
    for RandomMutationPolicy<R>
{
    fn mutate<'a>(&self, config: &Config<'a, V, P>) -> Option<Config<'a, V, P>> {
        let mut vars = std::collections::BTreeSet::new();
        for var in config.get_problem().get_variables() {
            let mut value = config.get(var).expect("Variable should be valid");

            if self.random_gen.random() < self.p {
                value = !value;
            }

            if value {
                vars.insert(var.clone());
            }
        }
        Some(
            config
                .get_problem()
                .config_from(&vars)
                .expect("Variables should be valid"),
        )
    }
}

#[derive(Clone, Debug)]
pub struct NeighbourMutationPolicy<R: RandomGen> {
    random_gen: R,
}

impl<R: RandomGen> NeighbourMutationPolicy<R> {
    pub fn new(random_gen: R) -> Self {
        NeighbourMutationPolicy { random_gen }
    }
}

impl<V: VariableName, P: ProblemRepr<V>, R: RandomGen> MutationPolicy<V, P>
    for NeighbourMutationPolicy<R>
{
    fn mutate<'a>(&self, config: &Config<'a, V, P>) -> Option<Config<'a, V, P>> {
        config.random_neighbour(&self.random_gen)
    }
}
