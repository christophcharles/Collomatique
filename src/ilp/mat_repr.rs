pub mod nd;

use super::linexpr::{self, VariableName};
use super::random;
use std::collections::{BTreeMap, BTreeSet};

pub trait ProblemRepr<V: VariableName>: Clone + std::fmt::Debug {
    type Config: ConfigRepr<V, Problem = Self>;

    fn new(variables_vec: &Vec<V>, constraints: &BTreeSet<linexpr::Constraint<V>>) -> Self;

    fn default_config(&self) -> Self::Config;

    fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Self::Config;
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
