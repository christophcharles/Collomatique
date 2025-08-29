#[cfg(test)]
mod tests;

use crate::ilp::linexpr;
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
    constraints_ref: Vec<BTreeSet<ConstraintRef>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ConstraintRef {
    Leq(usize),
    Eq(usize),
}

impl<V: VariableName> super::ProblemRepr<V> for NdProblem<V> {
    type Config = NdConfig<V>;

    fn new(variables_vec: &Vec<V>, constraints: &BTreeSet<linexpr::Constraint<V>>) -> NdProblem<V> {
        let p = variables_vec.len();

        let variable_map: BTreeMap<_, _> = variables_vec
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();

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

        let mut constraints_ref = vec![BTreeSet::new(); p];

        let mut leq_index = 0usize;
        let mut eq_index = 0usize;

        for c in constraints {
            match c.get_sign() {
                linexpr::Sign::Equals => {
                    for (var, val) in c.coefs() {
                        let j = variable_map[var];
                        eq_mat[(eq_index, j)] = *val;

                        constraints_ref[j].insert(ConstraintRef::Eq(eq_index));
                    }
                    constraints_map.insert(c.clone(), ConstraintRef::Eq(eq_index));
                    eq_constants[eq_index] = c.get_constant();
                    eq_index += 1;
                }
                linexpr::Sign::LessThan => {
                    for (var, val) in c.coefs() {
                        let j = variable_map[var];
                        leq_mat[(leq_index, j)] = *val;

                        constraints_ref[j].insert(ConstraintRef::Leq(leq_index));
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
            constraints_ref,
        }
    }

    fn config_from(&self, vars: &BTreeSet<usize>) -> Self::Config {
        let p = self.leq_mat.shape()[1];

        let mut values = Array1::zeros(p);

        for i in 0..p {
            if vars.contains(&i) {
                values[i] = 1;
            }
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
    type Precomputation = (Array1<i32>, Array1<i32>);

    fn max_distance_to_constraint(&self, problem: &NdProblem<V>) -> f32 {
        let mut max_dist = 0.0f32;
        let p = problem.leq_mat.shape()[1];

        let leq_column = problem.leq_mat.dot(&self.values) + &problem.leq_constants;

        for (i, v) in leq_column.iter().copied().enumerate() {
            let mut norm2 = 0.0f32;
            for j in 0..p {
                norm2 += (problem.leq_mat[(i, j)] as f32).powi(2);
            }
            let dist = ((v as f32) / norm2.sqrt()).min(0.0f32);

            if dist > max_dist {
                max_dist = dist;
            }
        }

        let eq_column = problem.eq_mat.dot(&self.values) + &problem.eq_constants;

        for (i, v) in eq_column.iter().copied().enumerate() {
            let mut norm2 = 0.0f32;
            for j in 0..p {
                norm2 += (problem.eq_mat[(i, j)] as f32).powi(2);
            }
            let dist = ((v as f32) / norm2.sqrt()).abs();

            if dist > max_dist {
                max_dist = dist;
            }
        }

        max_dist
    }

    fn precompute(&self, problem: &Self::Problem) -> Self::Precomputation {
        let leq_column = problem.leq_mat.dot(&self.values) + &problem.leq_constants;
        let eq_column = problem.eq_mat.dot(&self.values) + &problem.eq_constants;

        (leq_column, eq_column)
    }

    fn update_precomputation(
        &self,
        problem: &Self::Problem,
        data: &mut Self::Precomputation,
        vars: &BTreeSet<usize>,
    ) {
        let lines_to_update: BTreeSet<_> = vars
            .iter()
            .flat_map(|x| problem.constraints_ref[*x].iter())
            .collect();

        for line in lines_to_update {
            use ndarray::s;
            match line {
                ConstraintRef::Eq(c) => {
                    let partial_mat = problem.eq_mat.slice(s![*c..*c + 1, ..]);
                    let partial_constants = problem.eq_constants[*c];

                    let temp = partial_mat.dot(&self.values);
                    assert_eq!(temp.dim(), 1);

                    let new_val = temp[0] + partial_constants;

                    data.1[*c] = new_val;
                }
                ConstraintRef::Leq(c) => {
                    let partial_mat = problem.leq_mat.slice(s![*c..*c + 1, ..]);
                    let partial_constants = problem.leq_constants[*c];

                    let temp = partial_mat.dot(&self.values);
                    assert_eq!(temp.dim(), 1);

                    let new_val = temp[0] + partial_constants;

                    data.0[*c] = new_val;
                }
            }
        }
    }

    fn compute_lhs(
        &self,
        problem: &NdProblem<V>,
        precomputation: &Self::Precomputation,
    ) -> BTreeMap<linexpr::Constraint<V>, i32> {
        let (leq_column, eq_column) = precomputation;

        let mut output = BTreeMap::new();

        for (c, r) in &problem.constraints_map {
            let val = match r {
                ConstraintRef::Eq(num) => eq_column[*num],
                ConstraintRef::Leq(num) => leq_column[*num],
            };
            output.insert(c.clone(), val);
        }

        output
    }

    fn is_feasable(&self, _problem: &NdProblem<V>, precomputation: &Self::Precomputation) -> bool {
        let (leq_column, eq_column) = precomputation;

        for v in leq_column {
            if *v > 0 {
                return false;
            }
        }
        for v in eq_column {
            if *v != 0 {
                return false;
            }
        }
        true
    }

    fn neighbour(&self, i: usize) -> Self {
        let mut neighbour = self.clone();

        neighbour.values[i] = 1 - neighbour.values[i];

        neighbour
    }

    unsafe fn get_unchecked(&self, i: usize) -> i32 {
        self.values[i]
    }

    unsafe fn set_unchecked(&mut self, i: usize, val: i32) {
        self.values[i] = val
    }
}
