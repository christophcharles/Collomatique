#[cfg(test)]
mod tests;

use crate::ilp::dbg::Debuggable;
use crate::ilp::linexpr::VariableName;
use crate::ilp::mat_repr::ProblemRepr;
use crate::ilp::random::RandomGen;
use crate::ilp::solvers::FeasabilitySolver;
use crate::ilp::{Config, FeasableConfig};

use super::MutationPolicy;

pub type TemperatureFn = Debuggable<dyn Fn(usize) -> f64>;

impl Default for TemperatureFn {
    fn default() -> Self {
        crate::debuggable!(|k: usize| 1000. / (k as f64))
    }
}

#[derive(Debug, Clone)]
pub struct Optimizer<'a, V: VariableName, P: ProblemRepr<V>> {
    init_config: Config<'a, V, P>,
    temp_profile: TemperatureFn,
    max_steps: Option<usize>,
    init_max_steps: Option<usize>,
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Optimizer<'a, V, P> {
    pub fn new(init_config: Config<'a, V, P>) -> Self {
        Optimizer {
            init_config,
            temp_profile: TemperatureFn::default(),
            max_steps: None,
            init_max_steps: None,
        }
    }

    pub fn set_init_config(&mut self, init_config: Config<'a, V, P>) {
        self.init_config = init_config;
    }

    pub fn set_temp_profile(&mut self, temp_profile: TemperatureFn) {
        self.temp_profile = temp_profile;
    }

    pub fn set_max_steps(&mut self, max_steps: Option<usize>) {
        self.max_steps = max_steps;
    }

    pub fn set_init_max_steps(&mut self, init_max_steps: Option<usize>) {
        self.init_max_steps = init_max_steps;
    }

    pub fn iterate<R: RandomGen, S: FeasabilitySolver<V, P>, M: MutationPolicy<V, P>>(
        &self,
        solver: S,
        random_gen: R,
        mutation_policy: M,
    ) -> OptimizerIterator<'a, V, P, R, S, M> {
        OptimizerIterator {
            random_gen,
            solver,
            previous_config: None,
            current_config: self.init_config.clone(),
            k: 0,
            temp_profile: self.temp_profile.clone(),
            max_steps: self.max_steps.clone(),
            current_max_steps: self.init_max_steps.clone(),
            mutation_policy,
        }
    }
}

use std::rc::Rc;

#[derive(Debug)]
pub struct OptimizerIterator<
    'a,
    V: VariableName,
    P: ProblemRepr<V>,
    R: RandomGen,
    S: FeasabilitySolver<V, P>,
    M: MutationPolicy<V, P>,
> {
    solver: S,
    random_gen: R,

    previous_config: Option<(Rc<FeasableConfig<'a, V, P>>, f64)>,
    current_config: Config<'a, V, P>,

    k: usize,
    temp_profile: TemperatureFn,
    max_steps: Option<usize>,
    current_max_steps: Option<usize>,
    mutation_policy: M,
}

impl<
        'a,
        V: VariableName,
        P: ProblemRepr<V>,
        R: RandomGen,
        S: FeasabilitySolver<V, P>,
        M: MutationPolicy<V, P>,
    > Iterator for OptimizerIterator<'a, V, P, R, S, M>
{
    type Item = (Rc<FeasableConfig<'a, V, P>>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        let origin = match &self.previous_config {
            Some((c, _)) => Some(c.as_ref()),
            None => None,
        };

        // First the first step, if we can't restore then the iterator stops
        // So "None" should be propagated upwards
        //
        // But if we are on a step after that, we might have chosen a bad current_config
        // So we must try again with a "better" neighbour on next try.
        let sol = match self.solver.restore_feasability_with_origin_and_max_steps(
            &self.current_config,
            origin,
            self.current_max_steps,
        ) {
            Some(s) => s,
            None => match &self.previous_config {
                Some((c, _)) => c.as_ref().clone(),
                None => return None,
            },
        };
        self.current_max_steps = self.max_steps;
        let config = Rc::new(sol);

        let config_cost = config.as_ref().eval();

        let acceptance = match self.previous_config {
            Some((_, old_cost)) => {
                let temp = (self.temp_profile)(self.k);
                (-(config_cost - old_cost) / temp).exp()
            }
            None => 1.0,
        };
        self.k += 1;

        if let Some(neighbour) = self.mutation_policy.mutate(config.as_ref()) {
            self.current_config = neighbour;
            if acceptance >= self.random_gen.random() {
                self.previous_config = Some((config, config_cost));
            }
            self.previous_config.clone()
        } else {
            Some((config, config_cost))
        }
    }
}

impl<
        'a,
        V: VariableName,
        P: ProblemRepr<V>,
        R: RandomGen,
        S: FeasabilitySolver<V, P>,
        M: MutationPolicy<V, P>,
    > OptimizerIterator<'a, V, P, R, S, M>
{
    pub fn best_in(self, max_iter: usize) -> Option<(Rc<FeasableConfig<'a, V, P>>, f64)> {
        self.take(max_iter)
            .min_by(|x, y| x.1.partial_cmp(&y.1).expect("Non NaN"))
    }
}
