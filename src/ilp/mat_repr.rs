pub mod nd;

use super::linexpr::{self, VariableName};
use super::random;
use std::collections::BTreeMap;

pub trait ProblemRepr<V: VariableName>: Clone + std::fmt::Debug {
    type Config: ConfigRepr<V, Problem = Self>;

    fn new<'a, I: IntoIterator<Item = &'a linexpr::Constraint<V>>>(
        variables_vec: &'a Vec<V>,
        constraints: I,
    ) -> Self;

    fn default_nd_config(&self) -> Self::Config;

    fn random_nd_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Self::Config;
}

pub trait ConfigRepr<V: VariableName>:
    PartialEq + Eq + Ord + PartialOrd + Sized + Clone + std::fmt::Debug
{
    type Problem: ProblemRepr<V, Config = Self>;

    fn max_distance_to_constraint(&self, problem: &Self::Problem) -> f32;

    fn compute_lhs(&self, problem: &Self::Problem) -> BTreeMap<linexpr::Constraint<V>, i32>;

    fn is_feasable(&self, problem: &Self::Problem) -> bool;
    fn neighbours(&self) -> Vec<Self>;
    fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> Self;
    unsafe fn get_unchecked(&self, i: usize) -> i32;
    unsafe fn set_unchecked(&mut self, i: usize, val: i32);
}
