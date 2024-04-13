#[cfg(test)]
mod tests;

use super::*;

use ndarray::{Array, Array1, Array2, ArrayView};

#[derive(Debug, Clone, Default)]
pub struct MatRepr {
    leq_mat: Array2<i32>,
    leq_constants: Array1<i32>,
    eq_mat: Array2<i32>,
    eq_constants: Array1<i32>,
}

impl MatRepr {
    pub fn new(variables_vec: &Vec<String>, constraints: &Vec<linexpr::Constraint>) -> MatRepr {
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

        MatRepr {
            leq_mat,
            leq_constants,
            eq_mat,
            eq_constants,
        }
    }

    pub fn config<'a>(&self, config: &Config<'a>) -> ConfigRepr<'a> {
        let p = config.problem.variables.len();

        let mut values = Array1::zeros(p);

        for (i, var) in config.problem.variables.iter().enumerate() {
            if config.get(var).expect("Variable declared for config") {
                values[i] = 1;
            }
        }

        ConfigRepr {
            problem: config.problem,
            values,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigRepr<'a> {
    problem: &'a Problem,
    values: Array1<i32>,
}

impl<'a> PartialEq for ConfigRepr<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a> Eq for ConfigRepr<'a> {}

impl<'a> Ord for ConfigRepr<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let p1: *const Problem = &*self.problem;
        let p2: *const Problem = &*other.problem;

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

impl<'a> PartialOrd for ConfigRepr<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> From<&ConfigRepr<'a>> for Config<'a> {
    fn from(value: &ConfigRepr<'a>) -> Self {
        let mut config = value.problem.default_config();
        for (i, var) in value.problem.variables.iter().enumerate() {
            *config.get_mut(var).expect("Variable declared in config") = value.values[i] == 1;
        }
        config
    }
}

impl<'a> From<ConfigRepr<'a>> for Config<'a> {
    fn from(value: ConfigRepr<'a>) -> Self {
        Config::from(&value)
    }
}

impl<'a> ConfigRepr<'a> {
    pub fn is_feasable(&self) -> bool {
        let leq_column =
            self.problem.mat_repr.leq_mat.dot(&self.values) + &self.problem.mat_repr.leq_constants;
        let eq_column =
            self.problem.mat_repr.eq_mat.dot(&self.values) + &self.problem.mat_repr.eq_constants;

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

    pub unsafe fn into_feasable_unchecked(self) -> FeasableConfigRepr<'a> {
        FeasableConfigRepr(self)
    }

    pub fn into_feasable(self) -> Option<FeasableConfigRepr<'a>> {
        if !self.is_feasable() {
            return None;
        }

        Some(unsafe { self.into_feasable_unchecked() })
    }

    pub fn neighbours(&self) -> Vec<ConfigRepr<'a>> {
        let mut output = vec![];

        for i in 0..self.values.len() {
            let mut neighbour = self.clone();

            neighbour.values[i] = 1 - neighbour.values[i];

            output.push(neighbour);
        }

        output
    }

    pub fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> ConfigRepr<'a> {
        let mut output = self.clone();

        let i = random_gen.rand_in_range(0..self.values.len());
        output.values[i] = 1 - output.values[i];

        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeasableConfigRepr<'a>(ConfigRepr<'a>);

impl<'a> FeasableConfigRepr<'a> {
    pub fn into_inner(self) -> ConfigRepr<'a> {
        self.0
    }

    pub fn inner(&self) -> &ConfigRepr<'a> {
        &self.0
    }
}

impl<'a> From<&FeasableConfigRepr<'a>> for FeasableConfig<'a> {
    fn from(value: &FeasableConfigRepr<'a>) -> Self {
        unsafe { Config::from(value.inner()).into_feasable_unchecked() }
    }
}

impl<'a> From<FeasableConfigRepr<'a>> for FeasableConfig<'a> {
    fn from(value: FeasableConfigRepr<'a>) -> Self {
        FeasableConfig::from(&value)
    }
}
