#[cfg(test)]
mod tests;

use crate::ilp::{linexpr, random};
use std::collections::{BTreeMap, BTreeSet};

use ndarray::{Array1, Array2};

use linexpr::VariableName;
#[derive(Debug, Clone, Default)]
pub struct NdProblem<V: VariableName> {
    leq_mat: Array2<i32>,
    leq_constants: Array1<i32>,
    eq_mat: Array2<i32>,
    eq_constants: Array1<i32>,
    constraints_map: BTreeMap<linexpr::Constraint<V>, ConstraintRef>,
}

#[derive(Debug, Clone)]
enum ConstraintRef {
    Leq(usize),
    Eq(usize),
}

impl<V: VariableName> super::ProblemRepr<V> for NdProblem<V> {
    type Config = NdConfig<V>;

    fn new(variables_vec: &Vec<V>, constraints: &BTreeSet<linexpr::Constraint<V>>) -> NdProblem<V> {
        let p = variables_vec.len();

        let mut leq_count = 0usize;
        let mut eq_count = 0usize;

        for c in constraints {
            match c.get_sign() {
                linexpr::Sign::Equals => {
                    eq_count += 1;
                }
                linexpr::Sign::LessThan => {
                    leq_count += 1;
                }
            }
        }

        let mut leq_mat = Array2::zeros((leq_count, p));
        let mut eq_mat = Array2::zeros((eq_count, p));

        let mut leq_constants = Array1::zeros(leq_count);
        let mut eq_constants = Array1::zeros(eq_count);

        let mut constraints_map = BTreeMap::new();

        let mut leq_index = 0usize;
        let mut eq_index = 0usize;

        for c in constraints {
            match c.get_sign() {
                linexpr::Sign::Equals => {
                    for (j, var) in variables_vec.iter().enumerate() {
                        if let Some(val) = c.get_var(var.clone()) {
                            eq_mat[(eq_index, j)] = val;
                        }
                    }
                    constraints_map.insert(c.clone(), ConstraintRef::Eq(eq_index));
                    eq_constants[eq_index] = c.get_constant();
                    eq_index += 1;
                }
                linexpr::Sign::LessThan => {
                    for (j, var) in variables_vec.iter().enumerate() {
                        if let Some(val) = c.get_var(var.clone()) {
                            leq_mat[(leq_index, j)] = val;
                        }
                    }
                    constraints_map.insert(c.clone(), ConstraintRef::Leq(leq_index));
                    leq_constants[leq_index] = c.get_constant();
                    leq_index += 1;
                }
            }
        }

        NdProblem {
            leq_mat,
            leq_constants,
            eq_mat,
            eq_constants,
            constraints_map,
        }
    }

    fn default_nd_config(&self) -> NdConfig<V> {
        let p = self.leq_mat.shape()[1];

        let values = Array1::zeros(p);

        NdConfig {
            values,
            _phantom: std::marker::PhantomData,
        }
    }

    fn random_nd_config<T: random::RandomGen>(&self, random_gen: &mut T) -> NdConfig<V> {
        let p = self.leq_mat.shape()[1];

        let mut values = Array1::zeros(p);

        for i in 0..p {
            values[i] = if random_gen.randbool() { 1 } else { 0 };
        }

        NdConfig {
            values,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NdConfig<V: VariableName> {
    values: Array1<i32>,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: VariableName> PartialEq for NdConfig<V> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<V: VariableName> Eq for NdConfig<V> {}

impl<V: VariableName> Ord for NdConfig<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let l1 = self.values.len();
        let l2 = other.values.len();

        assert_eq!(l1, l2);

        for i in 0..l1 {
            let ord = self.values[i].cmp(&other.values[i]);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        return std::cmp::Ordering::Equal;
    }
}

impl<V: VariableName> PartialOrd for NdConfig<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<V: VariableName> super::ConfigRepr<V> for NdConfig<V> {
    type Problem = NdProblem<V>;

    fn max_distance_to_constraint(&self, nd_problem: &NdProblem<V>) -> f32 {
        let mut max_dist = 0.0f32;
        let p = nd_problem.leq_mat.shape()[1];

        let leq_column = nd_problem.leq_mat.dot(&self.values) + &nd_problem.leq_constants;

        for (i, v) in leq_column.iter().copied().enumerate() {
            let mut norm2 = 0.0f32;
            for j in 0..p {
                norm2 += nd_problem.leq_mat[(i, j)] as f32;
            }
            let dist = ((v as f32) / norm2.sqrt()).min(0.0f32);

            if dist > max_dist {
                max_dist = dist;
            }
        }

        let eq_column = nd_problem.eq_mat.dot(&self.values) + &nd_problem.eq_constants;

        for (i, v) in eq_column.iter().copied().enumerate() {
            let mut norm2 = 0.0f32;
            for j in 0..p {
                norm2 += nd_problem.eq_mat[(i, j)] as f32;
            }
            let dist = ((v as f32) / norm2.sqrt()).abs();

            if dist > max_dist {
                max_dist = dist;
            }
        }

        max_dist
    }

    fn compute_lhs(&self, nd_problem: &NdProblem<V>) -> BTreeMap<linexpr::Constraint<V>, i32> {
        let leq_column = nd_problem.leq_mat.dot(&self.values) + &nd_problem.leq_constants;
        let eq_column = nd_problem.eq_mat.dot(&self.values) + &nd_problem.eq_constants;

        let mut output = BTreeMap::new();

        for (c, r) in &nd_problem.constraints_map {
            let val = match r {
                ConstraintRef::Eq(num) => eq_column[*num],
                ConstraintRef::Leq(num) => leq_column[*num],
            };
            output.insert(c.clone(), val);
        }

        output
    }

    fn is_feasable(&self, nd_problem: &NdProblem<V>) -> bool {
        let leq_column = nd_problem.leq_mat.dot(&self.values) + &nd_problem.leq_constants;
        let eq_column = nd_problem.eq_mat.dot(&self.values) + &nd_problem.eq_constants;

        for v in &leq_column {
            if *v > 0 {
                return false;
            }
        }
        for v in &eq_column {
            if *v != 0 {
                return false;
            }
        }
        true
    }

    fn neighbours(&self) -> Vec<NdConfig<V>> {
        let mut output = vec![];

        for i in 0..self.values.len() {
            let mut neighbour = self.clone();

            neighbour.values[i] = 1 - neighbour.values[i];

            output.push(neighbour);
        }

        output
    }

    fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> NdConfig<V> {
        let mut output = self.clone();

        let i = random_gen.rand_in_range(0..self.values.len());
        output.values[i] = 1 - output.values[i];

        output
    }

    unsafe fn get_unchecked(&self, i: usize) -> i32 {
        self.values[i]
    }

    unsafe fn set_unchecked(&mut self, i: usize, val: i32) {
        self.values[i] = val
    }
}
