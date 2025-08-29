#[cfg(test)]
mod tests;

use super::*;

use ndarray::{Array, Array1, Array2, ArrayView};

#[derive(Debug, Clone, Default)]
pub struct NdProblem {
    leq_mat: Array2<i32>,
    leq_constants: Array1<i32>,
    eq_mat: Array2<i32>,
    eq_constants: Array1<i32>,
}

impl NdProblem {
    pub fn new(variables_vec: &Vec<String>, constraints: &Vec<linexpr::Constraint>) -> NdProblem {
        let p = variables_vec.len();

        let mut leq_mat = Array2::zeros((0, p));
        let mut eq_mat = Array2::zeros((0, p));

        let mut leq_constants_vec = vec![];
        let mut eq_constants_vec = vec![];

        for c in constraints {
            let mut current_row = Array::zeros(p);
            for (j, var) in variables_vec.iter().enumerate() {
                if let Some(val) = c.get_var(var) {
                    current_row[j] = val;
                }
            }

            let cst = c.get_constant();
            match c.get_sign() {
                linexpr::Sign::Equals => {
                    eq_mat.push_row(ArrayView::from(&current_row)).unwrap();
                    eq_constants_vec.push(cst);
                }
                linexpr::Sign::LessThan => {
                    leq_mat.push_row(ArrayView::from(&current_row)).unwrap();
                    leq_constants_vec.push(cst);
                }
            }
        }

        let leq_constants = Array::from_vec(leq_constants_vec);
        let eq_constants = Array::from_vec(eq_constants_vec);

        NdProblem {
            leq_mat,
            leq_constants,
            eq_mat,
            eq_constants,
        }
    }

    pub fn default_nd_config(&self) -> NdConfig {
        let p = self.leq_mat.shape()[1];

        let values = Array1::zeros(p);

        NdConfig { values }
    }

    pub fn random_nd_config<T: random::RandomGen>(&self, random_gen: &mut T) -> NdConfig {
        let p = self.leq_mat.shape()[1];

        let mut values = Array1::zeros(p);

        for i in 0..p {
            values[i] = if random_gen.randbool() { 1 } else { 0 };
        }

        NdConfig { values }
    }
}

#[derive(Debug, Clone)]
pub struct NdConfig {
    values: Array1<i32>,
}

impl PartialEq for NdConfig {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for NdConfig {}

impl Ord for NdConfig {
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

impl PartialOrd for NdConfig {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl NdConfig {
    pub fn max_distance_to_constraint(&self, nd_problem: &NdProblem) -> f32 {
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

    pub fn is_feasable(&self, nd_problem: &NdProblem) -> bool {
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

    pub fn neighbours(&self) -> Vec<NdConfig> {
        let mut output = vec![];

        for i in 0..self.values.len() {
            let mut neighbour = self.clone();

            neighbour.values[i] = 1 - neighbour.values[i];

            output.push(neighbour);
        }

        output
    }

    pub fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> NdConfig {
        let mut output = self.clone();

        let i = random_gen.rand_in_range(0..self.values.len());
        output.values[i] = 1 - output.values[i];

        output
    }

    pub unsafe fn get_unchecked(&self, i: usize) -> i32 {
        self.values[i]
    }

    pub unsafe fn set_unchecked(&mut self, i: usize, val: i32) {
        self.values[i] = val
    }
}
