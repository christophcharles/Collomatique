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
pub enum Error {
    #[error("Variable {1} is used in constraint {0} but not explicitly declared")]
    UndeclaredVariable(usize, String),
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

        let variables_vec = self.variables.iter().cloned().collect();
        Ok(Problem {
            variables: self.variables,
            variables_vec,
            constraints: self.constraints,
            eval_fn: self.eval_fn,
        })
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub struct Problem {
    variables: BTreeSet<String>,
    variables_vec: Vec<String>,
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
    pub fn default_config<'a>(&'a self) -> Config<'a> {
        Config {
            variables: self.variables.iter().map(|x| (x.clone(), false)).collect(),
            problem: self,
        }
    }

    pub fn config_from<'a, U: Into<String>, T: IntoIterator<Item = U>>(
        &'a self,
        vars: T,
    ) -> Config<'a> {
        let mut config = self.default_config();

        for v in vars.into_iter() {
            *config.get_mut(v).expect("Variable declared for config") = true;
        }

        config
    }

    pub fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Config {
        let mut config = self.default_config();
        for v in &self.variables {
            *config
                .variables
                .get_mut(v)
                .expect("Variable declared for config") = random_gen.randbool();
        }
        config
    }
}

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Config<'a> {
    variables: BTreeMap<String, bool>,
    problem: &'a Problem,
}

impl<'a> Config<'a> {
    pub fn get_mut<T: Into<String>>(&mut self, var: T) -> Option<&mut bool> {
        self.variables.get_mut(&var.into())
    }

    pub fn get<T: Into<String>>(&self, var: T) -> Option<bool> {
        self.variables.get(&var.into()).copied()
    }

    pub fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> Config<'a> {
        let mut output = self.clone();

        let var = random_gen.rand_elem(&self.problem.variables_vec[..]);
        let v = output
            .variables
            .get_mut(&var)
            .expect("Variable declared for config");
        *v = !(*v);

        output
    }

    fn into_linexpr_config(&self) -> Option<linexpr::Config> {
        let mut cfg = linexpr::Config::new();

        for v in &self.problem.variables {
            cfg.set(v, self.get(v).expect("Variable declared for config"));
        }

        Some(cfg)
    }

    pub fn is_feasable(&self) -> bool {
        let cfg = match self.into_linexpr_config() {
            Some(c) => c,
            None => return false,
        };

        for c in &self.problem.constraints {
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
            .variables
            .iter()
            .map(|(var, val)| format!("{}: {}", var, val))
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

        return self.variables.cmp(&other.variables);
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
    pub fn get_mut<T: Into<String>>(&mut self, var: T) -> Option<&mut bool> {
        self.0.get_mut(var)
    }

    pub fn get<T: Into<String>>(&self, var: T) -> Option<bool> {
        self.0.get(var)
    }

    pub fn into_inner(self) -> Config<'a> {
        self.0
    }

    pub fn inner(&self) -> &Config<'a> {
        &self.0
    }
}
