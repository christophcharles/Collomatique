pub mod nd;
pub mod sparse;

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
    type Precomputation: std::fmt::Debug + Clone;

    fn max_distance_to_constraint(&self, problem: &Self::Problem) -> f32;

    fn precompute(&self, problem: &Self::Problem) -> Self::Precomputation;
    fn update_precomputation(
        &self,
        problem: &Self::Problem,
        data: &mut Self::Precomputation,
        vars: &BTreeSet<usize>,
    );
    fn compute_lhs(
        &self,
        problem: &Self::Problem,
        precomputation: &Self::Precomputation,
    ) -> BTreeMap<linexpr::Constraint<V>, i32>;
    fn is_feasable(&self, problem: &Self::Problem, precomputation: &Self::Precomputation) -> bool;
    fn neighbour(&self, i: usize) -> Self;
    unsafe fn get_unchecked(&self, i: usize) -> i32;
    unsafe fn set_unchecked(&mut self, i: usize, val: i32);
}
