pub mod nd;
pub mod sparse;

use super::linexpr::{self, VariableName};
use std::collections::{BTreeMap, BTreeSet};

pub trait ProblemRepr<V: VariableName>: Clone + std::fmt::Debug + Send + Sync {
    type Config: ConfigRepr<V, Problem = Self>;

    fn new(variables_vec: &Vec<V>, constraints: &BTreeSet<linexpr::Constraint<V>>) -> Self;

    fn config_from(&self, vars: &BTreeMap<usize, i32>) -> Self::Config;
}

pub trait ConfigRepr<V: VariableName>:
    PartialEq + Eq + Ord + PartialOrd + Sized + Clone + std::fmt::Debug + Send + Sync
{
    type Problem: ProblemRepr<V, Config = Self>;
    type Precomputation: std::fmt::Debug + Clone + Send + Sync;

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

    unsafe fn get_unchecked(&self, i: usize) -> i32;
    unsafe fn set_unchecked(&mut self, i: usize, val: i32);
}
