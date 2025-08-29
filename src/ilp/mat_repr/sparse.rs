#[cfg(test)]
mod tests;

use crate::ilp::{linexpr, random};
use std::collections::{BTreeMap, BTreeSet};

use sprs::{CsMat, CsVec};

use linexpr::VariableName;
#[derive(Debug, Clone)]
pub struct SprsProblem<V: VariableName> {
    leq_mat: CsMat<i32>,
    leq_constants: CsVec<i32>,
    eq_mat: CsMat<i32>,
    eq_constants: CsVec<i32>,
    leq_constraints_vec: Vec<linexpr::Constraint<V>>,
    eq_constraints_vec: Vec<linexpr::Constraint<V>>,
}

impl<V: VariableName> super::ProblemRepr<V> for SprsProblem<V> {
    type Config = SprsConfig<V>;

    fn new(
        variables_vec: &Vec<V>,
        constraints: &BTreeSet<linexpr::Constraint<V>>,
    ) -> SprsProblem<V> {
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

        use sprs::TriMat;

        let mut leq_mat_tri = TriMat::new((leq_count, p));
        let mut eq_mat_tri = TriMat::new((eq_count, p));

        let mut leq_constants_indices = Vec::new();
        let mut leq_constants_data = Vec::new();
        let mut eq_constants_indices = Vec::new();
        let mut eq_constants_data = Vec::new();

        let mut leq_constraints_vec = Vec::with_capacity(leq_count);
        let mut eq_constraints_vec = Vec::with_capacity(eq_count);

        let mut leq_index = 0usize;
        let mut eq_index = 0usize;

        for c in constraints {
            match c.get_sign() {
                linexpr::Sign::Equals => {
                    for (var, val) in c.coefs() {
                        let j = variable_map[var];
                        eq_mat_tri.add_triplet(eq_index, j, *val);
                    }
                    eq_constraints_vec.push(c.clone());

                    let constant = c.get_constant();
                    if constant != 0 {
                        eq_constants_indices.push(eq_index);
                        eq_constants_data.push(constant);
                    }

                    eq_index += 1;
                }
                linexpr::Sign::LessThan => {
                    for (var, val) in c.coefs() {
                        let j = variable_map[var];
                        leq_mat_tri.add_triplet(leq_index, j, *val);
                    }
                    leq_constraints_vec.push(c.clone());

                    let constant = c.get_constant();
                    if constant != 0 {
                        leq_constants_indices.push(leq_index);
                        leq_constants_data.push(constant);
                    }

                    leq_index += 1;
                }
            }
        }

        let leq_mat = leq_mat_tri.to_csr();
        let eq_mat = eq_mat_tri.to_csr();
        let leq_constants = CsVec::new(leq_count, leq_constants_indices, leq_constants_data);
        let eq_constants = CsVec::new(eq_count, eq_constants_indices, eq_constants_data);

        SprsProblem {
            leq_mat,
            leq_constants,
            eq_mat,
            eq_constants,
            leq_constraints_vec,
            eq_constraints_vec,
        }
    }

    fn default_config(&self) -> SprsConfig<V> {
        let p = self.leq_mat.shape().1;

        let values = CsVec::empty(p);

        SprsConfig {
            values,
            _phantom: std::marker::PhantomData,
        }
    }

    fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> SprsConfig<V> {
        let mut indices = vec![];
        let mut data = vec![];

        let p = self.leq_mat.shape().1;

        for i in 0..p {
            if random_gen.randbool() {
                indices.push(i);
                data.push(1);
            }
        }

        let values = CsVec::new(p, indices, data);

        SprsConfig {
            values,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SprsConfig<V: VariableName> {
    values: CsVec<i32>,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: VariableName> PartialEq for SprsConfig<V> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<V: VariableName> Eq for SprsConfig<V> {}

impl<V: VariableName> Ord for SprsConfig<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let l1 = self.values.dim();
        let l2 = other.values.dim();

        assert_eq!(l1, l2);

        let diff = &self.values - &other.values;

        for (_i, v) in diff.iter() {
            if *v < 0 {
                return std::cmp::Ordering::Less;
            } else if *v > 0 {
                return std::cmp::Ordering::Greater;
            }
        }
        return std::cmp::Ordering::Equal;
    }
}

impl<V: VariableName> PartialOrd for SprsConfig<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<V: VariableName> super::ConfigRepr<V> for SprsConfig<V> {
    type Problem = SprsProblem<V>;

    fn max_distance_to_constraint(&self, problem: &SprsProblem<V>) -> f32 {
        let mut max_dist = 0.0f32;

        let leq_column = &problem.leq_mat * &self.values + &problem.leq_constants;

        for (i, v) in leq_column.iter() {
            let slice = problem.leq_mat.slice_outer(i..i + 1);
            let mut norm2 = 0.0f32;
            for (v, _) in slice.iter() {
                norm2 += ((*v) as f32).powi(2);
            }
            let dist = ((*v as f32) / norm2.sqrt()).min(0.0f32);

            if dist > max_dist {
                max_dist = dist;
            }
        }

        let eq_column = &problem.eq_mat * &self.values + &problem.eq_constants;

        for (i, v) in eq_column.iter() {
            let slice = problem.eq_mat.slice_outer(i..i + 1);
            let mut norm2 = 0.0f32;
            for (v, _) in slice.iter() {
                norm2 += ((*v) as f32).powi(2);
            }
            let dist = ((*v as f32) / norm2.sqrt()).abs();

            if dist > max_dist {
                max_dist = dist;
            }
        }

        max_dist
    }

    fn compute_lhs(&self, problem: &SprsProblem<V>) -> BTreeMap<linexpr::Constraint<V>, i32> {
        let leq_column = &problem.leq_mat * &self.values + &problem.leq_constants;
        let eq_column = &problem.eq_mat * &self.values + &problem.eq_constants;

        let mut output = BTreeMap::new();

        let mut prev = 0usize;
        for (i, v) in leq_column.iter() {
            for j in prev..i {
                output.insert(problem.leq_constraints_vec[j].clone(), 0);
            }
            output.insert(problem.leq_constraints_vec[i].clone(), *v);
            prev = i + 1;
        }
        let leq_count = problem.leq_constraints_vec.len();
        for j in prev..leq_count {
            output.insert(problem.leq_constraints_vec[j].clone(), 0);
        }

        let mut prev = 0usize;
        for (i, v) in eq_column.iter() {
            for j in prev..i {
                output.insert(problem.eq_constraints_vec[j].clone(), 0);
            }
            output.insert(problem.eq_constraints_vec[i].clone(), *v);
            prev = i + 1;
        }
        let eq_count = problem.eq_constraints_vec.len();
        for j in prev..eq_count {
            output.insert(problem.eq_constraints_vec[j].clone(), 0);
        }

        output
    }

    fn is_feasable(&self, problem: &SprsProblem<V>) -> bool {
        let leq_column = &problem.leq_mat * &self.values + &problem.leq_constants;
        let eq_column = &problem.eq_mat * &self.values + &problem.eq_constants;

        for (_, v) in leq_column.iter() {
            if *v > 0 {
                return false;
            }
        }
        for (_, v) in eq_column.iter() {
            if *v != 0 {
                return false;
            }
        }
        true
    }

    fn neighbours(&self) -> Vec<SprsConfig<V>> {
        let mut output = vec![];

        for i in 0..self.values.dim() {
            let neighbour = self.flip(i);
            output.push(neighbour);
        }

        output
    }

    fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> SprsConfig<V> {
        let i = random_gen.rand_in_range(0..self.values.dim());
        self.flip(i)
    }

    unsafe fn get_unchecked(&self, i: usize) -> i32 {
        self.values.get(i).copied().unwrap_or(0)
    }

    unsafe fn set_unchecked(&mut self, i: usize, val: i32) {
        assert!(val >= 0);
        assert!(val <= 1);

        match self.values.get(i) {
            Some(v) => {
                assert!(*v == 1);
                if val == 0 {
                    *self = self.flip(i);
                }
            }
            None => {
                if val == 1 {
                    *self = self.flip(i);
                }
            }
        }
    }
}

impl<V: VariableName> SprsConfig<V> {
    fn flip(&self, i: usize) -> SprsConfig<V> {
        let mut indices = vec![];
        let mut data = vec![];

        let mut prev = 0usize;
        for (j, v) in self.values.iter() {
            assert!(*v == 1);

            if j == i {
                prev = j + 1;
                continue;
            }

            if prev <= i && j > i {
                indices.push(i);
                data.push(1);
            }

            indices.push(j);
            data.push(*v);

            prev = j + 1;
        }
        if prev <= i && self.values.dim() > i {
            indices.push(i);
            data.push(1);
        }

        SprsConfig {
            values: CsVec::new(self.values.dim(), indices, data),
            _phantom: std::marker::PhantomData,
        }
    }
}
