pub mod dbg;
pub mod linexpr;
mod ndtools;
pub mod optimizers;
pub mod random;
pub mod solvers;

#[cfg(test)]
mod tests;

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProblemError {
    #[error("Variable {1} is used in constraint {0} but not explicitly declared")]
    UndeclaredVariable(usize, String),
}

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

    pub fn add_variables<T: Into<BTreeSet<String>>>(mut self, vars: T) -> Self {
        let mut temp = vars.into();
        self.variables.append(&mut temp);
        self
    }

    pub fn build(mut self) -> Result<Problem, ProblemError> {
        for c in self.constraints.iter_mut() {
            c.clean();
        }

        for (i, c) in self.constraints.iter().enumerate() {
            let constraint_vars = c.variables();
            if !self.variables.is_superset(&constraint_vars) {
                for var in constraint_vars {
                    if !self.variables.contains(&var) {
                        return Err(ProblemError::UndeclaredVariable(i, var));
                    }
                }
            }
        }

        Ok(Problem {
            variables: self.variables,
            constraints: self.constraints,
            eval_fn: self.eval_fn,
        })
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub struct Problem {
    variables: BTreeSet<String>,
    constraints: Vec<linexpr::Constraint>,
    eval_fn: EvalFn,
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
    pub fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Config {
        let mut config = Config::new();
        for v in &self.variables {
            config.set(v, random_gen.randbool());
        }
        config
    }

    pub fn random_neighbour<T: random::RandomGen>(
        &self,
        config: &Config,
        random_gen: &mut T,
    ) -> Config {
        let mut output = config.clone();

        let variables_vec: Vec<_> = self.variables.iter().collect();
        let var = random_gen.rand_elem(&variables_vec[..]);

        output.set(var, !config.get(var));

        output
    }

    fn into_linexpr_config(&self, config: &Config) -> Option<linexpr::Config> {
        let mut cfg = linexpr::Config::new();

        for v in &self.variables {
            cfg.set(v, config.get(v));
        }

        Some(cfg)
    }

    pub fn is_feasable(&self, config: &Config) -> bool {
        let cfg = match self.into_linexpr_config(config) {
            Some(c) => c,
            None => return false,
        };

        for c in &self.constraints {
            let res = match c.eval(&cfg) {
                Some(r) => r,
                None => return false,
            };
            if !res {
                return false;
            }
        }

        true
    }

    pub fn into_feasable<'a, 'b>(&'a self, config: &'b Config) -> Option<FeasableConfig<'a>> {
        if !self.is_feasable(config) {
            return None;
        }

        Some(unsafe { self.into_feasable_unchecked(config) })
    }

    pub unsafe fn into_feasable_unchecked<'a, 'b>(
        &'a self,
        config: &'b Config,
    ) -> FeasableConfig<'a> {
        FeasableConfig {
            variables: config.variables.clone(),
            problem: self,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Config {
    variables: BTreeSet<String>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            variables: BTreeSet::new(),
        }
    }

    pub fn set<T: Into<String>>(&mut self, var: T, val: bool) {
        if val {
            self.variables.insert(var.into());
        } else {
            self.variables.remove(&var.into());
        }
    }

    pub fn get<T: Into<String>>(&self, var: T) -> bool {
        self.variables.contains(&var.into())
    }
}

impl<A> FromIterator<A> for Config
where
    A: Into<String>,
{
    fn from_iter<I>(iterable: I) -> Config
    where
        I: IntoIterator<Item = A>,
    {
        let mut config = Config::new();

        for v in iterable {
            config.set(v, true);
        }

        config
    }
}

#[derive(Debug, Clone)]
pub struct FeasableConfig<'a> {
    variables: BTreeSet<String>,
    problem: &'a Problem,
}

impl<'a> FeasableConfig<'a> {
    pub fn get<T: Into<String>>(&self, var: T) -> bool {
        self.variables.contains(&var.into())
    }
}

impl<'a> From<&FeasableConfig<'a>> for Config {
    fn from(value: &FeasableConfig<'a>) -> Self {
        Config {
            variables: value.variables.clone(),
        }
    }
}

impl<'a> From<FeasableConfig<'a>> for Config {
    fn from(value: FeasableConfig<'a>) -> Self {
        Config::from(&value)
    }
}

impl<'a> PartialEq for FeasableConfig<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a> Eq for FeasableConfig<'a> {}

impl<'a> Ord for FeasableConfig<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let p1: *const Problem = &*self.problem;
        let p2: *const Problem = &*other.problem;

        let mat_repr_ord = p1.cmp(&p2);
        if mat_repr_ord != std::cmp::Ordering::Equal {
            return mat_repr_ord;
        }

        for v in &self.problem.variables {
            let v1 = self.get(v);
            let v2 = other.get(v);

            if v1 != v2 {
                if v2 {
                    return std::cmp::Ordering::Less;
                } else {
                    return std::cmp::Ordering::Greater;
                }
            }
        }

        return std::cmp::Ordering::Equal;
    }
}

impl<'a> PartialOrd for FeasableConfig<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
