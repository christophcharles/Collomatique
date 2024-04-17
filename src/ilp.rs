pub mod dbg;
pub mod linexpr;
pub mod optimizers;
pub mod random;
pub mod solvers;

mod ndtools;

#[cfg(test)]
mod tests;

use thiserror::Error;

use linexpr::VariableName;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error<V: VariableName> {
    #[error("Variable {1} is used in constraint {0} but not explicitly declared")]
    UndeclaredVariable(usize, V),
    #[error("Variable {0} is not valid for this problem")]
    InvalidVariable(V),
}

pub type Result<T, V> = std::result::Result<T, Error<V>>;

pub type EvalFn<V> = dbg::Debuggable<dyn Fn(&FeasableConfig<V>) -> f64>;

impl<V: VariableName> Default for EvalFn<V> {
    fn default() -> Self {
        crate::debuggable!(|_x| 0.)
    }
}

#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: VariableName> {
    constraints: BTreeSet<linexpr::Constraint<V>>,
    eval_fn: EvalFn<V>,
    variables: BTreeSet<V>,
}

impl<V: VariableName> Default for ProblemBuilder<V> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: BTreeSet::new(),
            eval_fn: EvalFn::default(),
            variables: BTreeSet::new(),
        }
    }
}

impl<V: VariableName> ProblemBuilder<V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_constraint(mut self, constraint: linexpr::Constraint<V>) -> Self {
        self.constraints.insert(constraint.cleaned());
        self
    }

    pub fn add_constraints<T: IntoIterator<Item = linexpr::Constraint<V>>>(
        mut self,
        constraints: T,
    ) -> Self {
        self.constraints
            .extend(constraints.into_iter().map(|x| x.cleaned()));
        self
    }

    pub fn eval_fn(mut self, func: EvalFn<V>) -> Self {
        self.eval_fn = func;
        self
    }

    pub fn add_variable<T: Into<V>>(mut self, var: T) -> Self {
        self.variables.insert(var.into());
        self
    }

    pub fn add_variables<U: Into<V>, T: IntoIterator<Item = U>>(mut self, vars: T) -> Self {
        let mut temp = vars.into_iter().map(|x| x.into()).collect();
        self.variables.append(&mut temp);
        self
    }

    pub fn build(self) -> Result<Problem<V>, V> {
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

        let constraints_vec: Vec<_> = self.constraints.iter().cloned().collect();

        let nd_problem = ndtools::NdProblem::new(&variables_vec, &constraints_vec);
        Ok(Problem {
            variables: self.variables,
            variables_vec,
            variables_lookup,
            constraints: self.constraints,
            eval_fn: self.eval_fn,
            nd_problem,
        })
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Problem<V: VariableName> {
    variables: BTreeSet<V>,
    variables_vec: Vec<V>,
    variables_lookup: BTreeMap<V, usize>,
    constraints: BTreeSet<linexpr::Constraint<V>>,
    eval_fn: EvalFn<V>,
    nd_problem: ndtools::NdProblem,
}

impl<V: VariableName> std::fmt::Display for Problem<V> {
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

impl<V: VariableName> Problem<V> {
    pub fn default_config<'a>(&'a self) -> Config<'a, V> {
        Config {
            problem: self,
            nd_config: self.nd_problem.default_nd_config(),
        }
    }

    pub fn config_from<'a, U: Into<V>, T: IntoIterator<Item = U>>(
        &'a self,
        vars: T,
    ) -> Result<Config<'a, V>, V> {
        let mut config = self.default_config();

        for v in vars.into_iter() {
            config.set(v, true)?;
        }

        Ok(config)
    }

    pub fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Config<'_, V> {
        Config {
            problem: self,
            nd_config: self.nd_problem.random_nd_config(random_gen),
        }
    }
}

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Config<'a, V: VariableName> {
    problem: &'a Problem<V>,
    nd_config: ndtools::NdConfig,
}

impl<'a, V: VariableName> Config<'a, V> {
    pub fn get<T: Into<V>>(&self, var: T) -> Result<bool, V> {
        let name = var.into();
        let i = match self.problem.variables_lookup.get(&name) {
            Some(i) => i,
            None => return Err(Error::InvalidVariable(name)),
        };
        Ok(unsafe { self.nd_config.get_unchecked(*i) == 1 })
    }

    pub fn set<T: Into<V>>(&mut self, var: T, val: bool) -> Result<(), V> {
        let name = var.into();
        let i = match self.problem.variables_lookup.get(&name) {
            Some(i) => i,
            None => return Err(Error::InvalidVariable(name)),
        };
        unsafe {
            self.nd_config.set_unchecked(*i, if val { 1 } else { 0 });
        }
        Ok(())
    }

    pub fn random_neighbour<T: random::RandomGen>(&self, random_gen: &mut T) -> Config<'a, V> {
        Config {
            problem: self.problem,
            nd_config: self.nd_config.random_neighbour(random_gen),
        }
    }

    pub fn neighbours(&self) -> Vec<Config<'a, V>> {
        self.nd_config
            .neighbours()
            .into_iter()
            .map(|x| Config {
                problem: self.problem,
                nd_config: x,
            })
            .collect()
    }

    pub fn max_distance_to_constraint(&self) -> f32 {
        self.nd_config
            .max_distance_to_constraint(&self.problem.nd_problem)
    }

    pub fn is_feasable(&self) -> bool {
        self.nd_config.is_feasable(&self.problem.nd_problem)
    }

    pub fn into_feasable(self) -> Option<FeasableConfig<'a, V>> {
        if !self.is_feasable() {
            return None;
        }

        Some(unsafe { self.into_feasable_unchecked() })
    }

    pub unsafe fn into_feasable_unchecked(self) -> FeasableConfig<'a, V> {
        FeasableConfig(self)
    }
}

impl<'a, V: VariableName> std::fmt::Display for Config<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ ")?;
        let slice: Vec<_> = self
            .problem
            .variables_vec
            .iter()
            .enumerate()
            .map(|(i, var)| format!("{}: {}", var, unsafe { self.nd_config.get_unchecked(i) }))
            .collect();
        write!(f, "{}", slice.join(", "))?;
        write!(f, " ]")?;

        Ok(())
    }
}

impl<'a, V: VariableName> PartialEq for Config<'a, V> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a, V: VariableName> Eq for Config<'a, V> {}

impl<'a, V: VariableName> Ord for Config<'a, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let p1: *const Problem<V> = &*self.problem;
        let p2: *const Problem<V> = &*other.problem;

        let problem_ord = p1.cmp(&p2);
        if problem_ord != std::cmp::Ordering::Equal {
            return problem_ord;
        }

        return self.nd_config.cmp(&other.nd_config);
    }
}

impl<'a, V: VariableName> PartialOrd for Config<'a, V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeasableConfig<'a, V: VariableName>(Config<'a, V>);

impl<'a, V: VariableName> FeasableConfig<'a, V> {
    pub fn set<T: Into<V>>(&mut self, var: T, val: bool) -> Result<(), V> {
        self.0.set(var, val)
    }

    pub fn get<T: Into<V>>(&self, var: T) -> Result<bool, V> {
        self.0.get(var)
    }

    pub fn into_inner(self) -> Config<'a, V> {
        self.0
    }

    pub fn inner(&self) -> &Config<'a, V> {
        &self.0
    }
}
