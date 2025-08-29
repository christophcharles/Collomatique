#[cfg(test)]
mod tests;

use super::*;

use ndarray::{Array, Array1, Array2, ArrayView};

#[derive(Debug,Clone,PartialEq,Eq)]
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

        let mut leq_mat = Array2::zeros((0,p));
        let mut eq_mat = Array2::zeros((0,p));

        let mut leq_constants_vec = vec![];
        let mut eq_constants_vec = vec![];

        for c in &problem.constraints {
            let mut current_row = Array::zeros(p);
            for (j,var) in problem.variables.iter().enumerate() {
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

    pub fn config_repr<'b>(&'b self, config: &Config) -> ConfigRepr<'a,'b> {
        let p = self.problem.variables.len();

        let mut values = Array1::zeros(p);

        for (i,var) in self.problem.variables.iter().enumerate() {
            if config.get(var) {
                values[i] = 1;
            }
        }

        ConfigRepr {
            mat_repr: self,
            values,
        }
    }
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ConfigRepr<'a, 'b: 'a> {
    mat_repr: &'b MatRepr<'a>,
    values: Array1<i32>,
}

impl<'a,'b: 'a> From<ConfigRepr<'a,'b>> for Config {
    fn from(value: ConfigRepr<'a,'b>) -> Self {
        let mut config = Config::new();
        for (i,var) in value.mat_repr.problem.variables.iter().enumerate() {
            config.set(
                var,
                value.values[i] == 1
            );
        }
        config
    }
}

impl<'a, 'b: 'a> ConfigRepr<'a,'b> {
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
}
