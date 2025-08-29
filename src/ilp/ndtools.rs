#[cfg(test)]
mod tests;

use super::*;

use ndarray::{Array, Array1, Array2, ArrayView};

#[derive(Debug, Clone)]
pub struct MatRepr<'a> {
    problem: &'a Problem,
    leq_mat: Array2<i32>,
    leq_constants: Array1<i32>,
    eq_mat: Array2<i32>,
    eq_constants: Array1<i32>,
}

impl<'a> MatRepr<'a> {
    pub fn new(problem: &'a Problem) -> MatRepr<'a> {
        let p = problem.variables.len();

        let mut leq_mat = Array2::zeros((0, p));
        let mut eq_mat = Array2::zeros((0, p));

        let mut leq_constants_vec = vec![];
        let mut eq_constants_vec = vec![];

        for c in &problem.constraints {
            let mut current_row = Array::zeros(p);
            for (j, var) in problem.variables.iter().enumerate() {
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

        MatRepr {
            problem,
            leq_mat,
            leq_constants,
            eq_mat,
            eq_constants,
        }
    }

    pub fn config<'b>(&'b self, config: &Config) -> Option<ConfigRepr<'a, 'b>> {
        let p1 = self.problem as *const Problem;
        let p2 = config.problem as *const Problem;

        if p1 != p2 {
            return None;
        }

        let p = self.problem.variables.len();

        let mut values = Array1::zeros(p);

        for (i, var) in self.problem.variables.iter().enumerate() {
            if config.get(var).expect("Variable declared for config") {
                values[i] = 1;
            }
        }

        Some(ConfigRepr {
            mat_repr: self,
            values,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConfigRepr<'a, 'b> {
    mat_repr: &'b MatRepr<'a>,
    values: Array1<i32>,
}

impl<'a, 'b> PartialEq for ConfigRepr<'a, 'b> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a, 'b> Eq for ConfigRepr<'a, 'b> {}

impl<'a, 'b> Ord for ConfigRepr<'a, 'b> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let p1: *const MatRepr<'a> = &*self.mat_repr;
        let p2: *const MatRepr<'a> = &*other.mat_repr;

        let mat_repr_ord = p1.cmp(&p2);
        if mat_repr_ord != std::cmp::Ordering::Equal {
            return mat_repr_ord;
        }

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

impl<'a, 'b> PartialOrd for ConfigRepr<'a, 'b> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, 'b> From<ConfigRepr<'a, 'b>> for Config<'a> {
    fn from(value: ConfigRepr<'a, 'b>) -> Self {
        let mut config = value.mat_repr.problem.default_config();
        for (i, var) in value.mat_repr.problem.variables.iter().enumerate() {
            *config.get_mut(var).expect("Variable declared in config") = value.values[i] == 1;
        }
        config
    }
}

impl<'a, 'b> ConfigRepr<'a, 'b> {
    pub fn is_feasable(&self) -> bool {
        let leq_column = self.mat_repr.leq_mat.dot(&self.values) + &self.mat_repr.leq_constants;
        let eq_column = self.mat_repr.eq_mat.dot(&self.values) + &self.mat_repr.eq_constants;

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

    pub fn neighbours(&self) -> Vec<ConfigRepr<'a, 'b>> {
        let mut output = vec![];

        for i in 0..self.values.len() {
            let mut neighbour = self.clone();

            neighbour.values[i] = 1 - neighbour.values[i];

            output.push(neighbour);
        }

        output
    }
}
