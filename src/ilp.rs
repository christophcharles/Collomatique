pub mod dbg;
pub mod linexpr;
pub mod optimizers;
pub mod random;
pub mod solvers;

mod ndtools;

#[cfg(test)]
mod tests;

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Variable {1} is used in constraint {0} but not explicitly declared")]
    UndeclaredVariable(usize, String),
    #[error("Variable {0} is not valid for this problem")]
    InvalidVariable(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub type EvalFn = dbg::Debuggable<dyn Fn(&FeasableConfig) -> f64>;

impl Default for EvalFn {
    fn default() -> Self {
        crate::debuggable!(|_x| 0.)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ProblemBuilder {
    constraints: Vec<linexpr::Constraint>,
    eval_fn: EvalFn,
    variables: BTreeSet<String>,
}

impl ProblemBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(mut self, constraint: linexpr::Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn eval_fn(mut self, func: EvalFn) -> Self {
        self.eval_fn = func;
        self
    }

    pub fn add_variable<T: Into<String>>(mut self, var: T) -> Self {
        self.variables.insert(var.into());
        self
    }

    pub fn add_variables<U: Into<String>, T: IntoIterator<Item = U>>(mut self, vars: T) -> Self {
        let mut temp = vars.into_iter().map(|x| x.into()).collect();
        self.variables.append(&mut temp);
        self
    }

    pub fn build(mut self) -> Result<Problem> {
        for c in self.constraints.iter_mut() {
            c.clean();
        }

        for (i, c) in self.constraints.iter().enumerate() {
            let constraint_vars = c.variables();
            if !self.variables.is_superset(&constraint_vars) {
                for var in constraint_vars {
                    if !self.variables.contains(&var) {
                        return Err(Error::UndeclaredVariable(i, var));
                    }
                }
            }
        }

        let variables_vec: Vec<_> = self.variables.iter().cloned().collect();

        let mut variables_lookup = BTreeMap::new();
        for (i, var) in variables_vec.iter().enumerate() {
            variables_lookup.insert(var.clone(), i);
        }

        let mat_repr = ndtools::MatRepr::new(&variables_vec, &self.constraints);
        Ok(Problem {
            variables: self.variables,
            variables_vec,
            variables_lookup,
            constraints: self.constraints,
            eval_fn: self.eval_fn,
            mat_repr,
        })
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub struct Problem {
    variables: BTreeSet<String>,
    variables_vec: Vec<String>,
    variables_lookup: BTreeMap<String, usize>,
    constraints: Vec<linexpr::Constraint>,
    eval_fn: EvalFn,
    mat_repr: ndtools::MatRepr,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "variables : [")?;
        for v in &self.variables {
            write!(f, " {}", v)?;
        }
        write!(f, " ]\n")?;

        write!(f, "evaluation function : {:?}\n", self.eval_fn)?;

        write!(f, "constraints :")?;
        for (i, c) in self.constraints.iter().enumerate() {
            write!(f, "\n{}) {}", i, c)?;
        }

        Ok(())
    }
}

impl Problem {
    pub fn default_config<'a>(&'a self) -> Config<'a> {
        Config {
            problem: self,
            repr: self.mat_repr.default_config_repr(),
        }
    }

    pub fn config_from<'a, U: Into<String>, T: IntoIterator<Item = U>>(
        &'a self,
        vars: T,
    ) -> Result<Config<'a>> {
        let mut config = self.default_config();

        for v in vars.into_iter() {
            config.set(v, true)?;
        }

        Ok(config)
    }

    pub fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Config {
        Config {
            problem: self,
            repr: self.mat_repr.random_config_repr(random_gen),
        }
    }
}

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Config<'a> {
    problem: &'a Problem,
    repr: ndtools::ConfigRepr,
}

impl<'a> Config<'a> {
    pub fn get<T: Into<String>>(&self, var: T) -> Result<bool> {
        let name = var.into();
        let i = match self.problem.variables_lookup.get(&name) {
            Some(i) => i,
            None => return Err(Error::InvalidVariable(name)),
        };
        Ok(unsafe { self.repr.get_unchecked(*i) == 1 })
    }

    pub fn set<T: Into<String>>(&mut self, var: T, val: bool) -> Result<()> {
        let name = var.into();
        let i = match self.problem.variables_lookup.get(&name) {
            Some(i) => i,
            None => return Err(Error::InvalidVariable(name)),
        };
        unsafe {
            self.repr.set_unchecked(*i, if val { 1 } else { 0 });
        }
        Ok(())
    }

    pub fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> Config<'a> {
        Config {
            problem: self.problem,
            repr: self.repr.random_neighbour(random_gen),
        }
    }

    pub fn neighbours(&self) -> Vec<Config<'a>> {
        self.repr
            .neighbours()
            .into_iter()
            .map(|x| Config {
                problem: self.problem,
                repr: x,
            })
            .collect()
    }

    pub fn max_distance_to_constraint(&self) -> f32 {
        self.repr.max_distance_to_constraint(&self.problem.mat_repr)
    }

    pub fn is_feasable(&self) -> bool {
        self.repr.is_feasable(&self.problem.mat_repr)
    }

    pub fn into_feasable(self) -> Option<FeasableConfig<'a>> {
        if !self.is_feasable() {
            return None;
        }

        Some(unsafe { self.into_feasable_unchecked() })
    }

    pub unsafe fn into_feasable_unchecked(self) -> FeasableConfig<'a> {
        FeasableConfig(self)
    }
}

impl<'a> std::fmt::Display for Config<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ ")?;
        let slice: Vec<_> = self
            .problem
            .variables_vec
            .iter()
            .enumerate()
            .map(|(i, var)| format!("{}: {}", var, unsafe { self.repr.get_unchecked(i) }))
            .collect();
        write!(f, "{}", slice.join(", "))?;
        write!(f, " ]")?;

        Ok(())
    }
}

impl<'a> PartialEq for Config<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a> Eq for Config<'a> {}

impl<'a> Ord for Config<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let p1: *const Problem = &*self.problem;
        let p2: *const Problem = &*other.problem;

        let mat_repr_ord = p1.cmp(&p2);
        if mat_repr_ord != std::cmp::Ordering::Equal {
            return mat_repr_ord;
        }

        return self.repr.cmp(&other.repr);
    }
}

impl<'a> PartialOrd for Config<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeasableConfig<'a>(Config<'a>);

impl<'a> FeasableConfig<'a> {
    pub fn set<T: Into<String>>(&mut self, var: T, val: bool) -> Result<()> {
        self.0.set(var, val)
    }

    pub fn get<T: Into<String>>(&self, var: T) -> Result<bool> {
        self.0.get(var)
    }

    pub fn into_inner(self) -> Config<'a> {
        self.0
    }

    pub fn inner(&self) -> &Config<'a> {
        &self.0
    }
}
