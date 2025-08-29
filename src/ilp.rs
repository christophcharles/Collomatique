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
    #[error("Variable {0} is not valid for this problem")]
    InvalidVariable(V),
    #[error("Variable {0} is actually a constant and cannot be set")]
    ConstantNotVariable(V),
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
    constants: BTreeMap<V, bool>,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VarError<V: VariableName> {
    #[error("Variable {0} already declared")]
    VariableAlreadyDeclared(V),
    #[error("Constant {0} already declared")]
    ConstantAlreadyDeclared(V),
}

pub type VarResult<T, V> = std::result::Result<T, VarError<V>>;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BuildError<V: VariableName> {
    #[error("Variable {1} is used in constraint {0} but not explicitly declared")]
    UndeclaredVariable(usize, V),
}

pub type BuildResult<T, V> = std::result::Result<T, BuildError<V>>;

impl<V: VariableName> Default for ProblemBuilder<V> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: BTreeSet::new(),
            eval_fn: EvalFn::default(),
            variables: BTreeSet::new(),
            constants: BTreeMap::new(),
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

    fn check_var<'a, T>(&self, var: &'a T) -> VarResult<(), V>
    where
        V: std::borrow::Borrow<T>,
        T: Ord + ?Sized,
        &'a T: Into<V>,
    {
        if self.constants.contains_key(var) {
            return Err(VarError::ConstantAlreadyDeclared(var.into()));
        }
        if self.variables.contains(var) {
            return Err(VarError::VariableAlreadyDeclared(var.into()));
        }
        Ok(())
    }

    pub fn add_variable<T: Into<V>>(mut self, var: T) -> VarResult<Self, V> {
        let var = var.into();
        self.check_var(&var)?;
        self.variables.insert(var);
        Ok(self)
    }

    pub fn add_variables<U: Into<V>, T: IntoIterator<Item = U>>(
        mut self,
        vars: T,
    ) -> VarResult<Self, V> {
        for var in vars {
            self = self.add_variable(var)?;
        }
        Ok(self)
    }

    pub fn add_constant<T: Into<V>>(mut self, var: T, val: bool) -> VarResult<Self, V> {
        let var = var.into();
        self.check_var(&var)?;
        self.constants.insert(var, val);
        Ok(self)
    }

    pub fn add_constants<U: Into<V>, T: IntoIterator<Item = (U, bool)>>(
        mut self,
        vars: T,
    ) -> VarResult<Self, V> {
        for (var, val) in vars {
            self = self.add_constant(var, val)?;
        }
        Ok(self)
    }

    pub fn build(self) -> BuildResult<Problem<V>, V> {
        for (i, c) in self.constraints.iter().enumerate() {
            let constraint_vars = c.variables();
            if !self.variables.is_superset(&constraint_vars) {
                for var in constraint_vars {
                    if !self.variables.contains(&var) {
                        return Err(BuildError::UndeclaredVariable(i, var));
                    }
                }
            }
        }

        let variables_vec: Vec<_> = self.variables.iter().cloned().collect();
        let mut variables_lookup = BTreeMap::new();
        for (i, var) in variables_vec.iter().enumerate() {
            variables_lookup.insert(var.clone(), i);
        }

        let nd_problem = ndtools::NdProblem::new(&variables_vec, &self.constraints);

        Ok(Problem {
            variables: self.variables,
            variables_vec,
            variables_lookup,
            constraints: self.constraints,
            constants: self.constants,
            eval_fn: self.eval_fn,
            nd_problem,
        })
    }

    pub fn simplify_trivial_constraints(self) -> ProblemBuilder<V> {
        let (constraints, constants) = Self::iterate_simplify(&self.constraints);

        let variables: BTreeSet<_> = self
            .variables
            .iter()
            .filter(|&x| !constants.contains_key(x))
            .cloned()
            .collect();

        ProblemBuilder {
            constraints,
            eval_fn: self.eval_fn,
            variables,
            constants,
        }
    }
}

impl<V: VariableName> ProblemBuilder<V> {
    fn simple_simplify(
        constraints: &mut BTreeSet<linexpr::Constraint<V>>,
    ) -> linexpr::SimpleSolution<V> {
        let mut attr_opt = None;

        for constraint in constraints.iter() {
            use linexpr::SimpleSolution;
            match constraint.simple_solve() {
                SimpleSolution::NoSolution => {
                    return SimpleSolution::NoSolution;
                }
                SimpleSolution::NotSimpleSolvable => {}
                SimpleSolution::Solution(attr) => {
                    attr_opt = Some((constraint.clone(), attr));
                    break;
                }
            }
        }

        if let Some((c, attr)) = attr_opt {
            constraints.remove(&c);

            if let Some((v, val)) = &attr {
                let attr_map = BTreeMap::from([(v.clone(), *val)]);
                let output: BTreeSet<_> =
                    constraints.iter().map(|c| c.reduced(&attr_map)).collect();

                *constraints = output;
            }

            linexpr::SimpleSolution::Solution(attr)
        } else {
            linexpr::SimpleSolution::NotSimpleSolvable
        }
    }

    fn iterate_simplify(
        constraints: &BTreeSet<linexpr::Constraint<V>>,
    ) -> (BTreeSet<linexpr::Constraint<V>>, BTreeMap<V, bool>) {
        let mut constraints_output = constraints.clone();
        let mut vars_output = BTreeMap::new();

        while let linexpr::SimpleSolution::Solution(attr) =
            Self::simple_simplify(&mut constraints_output)
        {
            if let Some((v, val)) = attr {
                vars_output.insert(v, val);
            }
        }

        (constraints_output, vars_output)
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Problem<V: VariableName> {
    variables: BTreeSet<V>,
    variables_vec: Vec<V>,
    variables_lookup: BTreeMap<V, usize>,
    constraints: BTreeSet<linexpr::Constraint<V>>,
    constants: BTreeMap<V, bool>,
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

        write!(f, "constants : [")?;
        for (i, (c, val)) in self.constants.iter().enumerate() {
            if i != 0 {
                write!(f, ",")?;
            }
            write!(f, " {} = {}", c, if *val { 1 } else { 0 })?;
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

    pub fn config_from<'a, 'b, U, T: IntoIterator<Item = &'b U>>(
        &'a self,
        vars: T,
    ) -> Result<Config<'a, V>, V>
    where
        V: std::borrow::Borrow<U>,
        U: Ord + ?Sized + 'b,
        &'b U: Into<V>,
    {
        let mut config = self.default_config();

        for v in vars {
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

    pub fn get_constraints(&self) -> &BTreeSet<linexpr::Constraint<V>> {
        &self.constraints
    }

    pub fn get_variables(&self) -> &BTreeSet<V> {
        &self.variables
    }
}

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Config<'a, V: VariableName> {
    problem: &'a Problem<V>,
    nd_config: ndtools::NdConfig,
}

impl<'a, V: VariableName> Config<'a, V> {
    pub fn get_problem(&self) -> &Problem<V> {
        self.problem
    }

    pub fn get<'b, T>(&self, var: &'b T) -> Result<bool, V>
    where
        V: std::borrow::Borrow<T>,
        T: Ord + ?Sized,
        &'b T: Into<V>,
    {
        if let Some(val) = self.problem.constants.get(var) {
            return Ok(*val);
        }

        let i = match self.problem.variables_lookup.get(var) {
            Some(i) => i,
            None => return Err(Error::InvalidVariable(var.into())),
        };
        Ok(unsafe { self.nd_config.get_unchecked(*i) == 1 })
    }

    pub fn set<'b, T>(&mut self, var: &'b T, val: bool) -> Result<(), V>
    where
        V: std::borrow::Borrow<T>,
        T: Ord + ?Sized,
        &'b T: Into<V>,
    {
        if let Some(_val) = self.problem.constants.get(var) {
            return Err(Error::ConstantNotVariable(var.into()));
        }

        let i = match self.problem.variables_lookup.get(var) {
            Some(i) => i,
            None => return Err(Error::InvalidVariable(var.into())),
        };
        unsafe {
            self.nd_config.set_unchecked(*i, if val { 1 } else { 0 });
        }
        Ok(())
    }

    pub fn random_neighbour<T: random::RandomGen>(
        &self,
        random_gen: &mut T,
    ) -> Option<Config<'a, V>> {
        if self.problem.variables.is_empty() {
            return None;
        }

        Some(Config {
            problem: self.problem,
            nd_config: self.nd_config.random_neighbour(random_gen),
        })
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
        let mut variables: BTreeMap<_, _> = self
            .problem
            .constants
            .iter()
            .map(|(k, v)| (k.clone(), if *v { 1 } else { 0 }))
            .collect();
        variables.extend(
            self.problem
                .variables_vec
                .iter()
                .enumerate()
                .map(|(i, var)| (var.clone(), unsafe { self.nd_config.get_unchecked(i) })),
        );

        write!(f, "[ ")?;
        let slice: Vec<_> = variables
            .iter()
            .map(|(var, val)| format!("{}: {}", var, val))
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
    pub fn into_inner(self) -> Config<'a, V> {
        self.0
    }

    pub fn inner(&self) -> &Config<'a, V> {
        &self.0
    }
}

impl<'a, V: VariableName> std::ops::Deref for FeasableConfig<'a, V> {
    type Target = Config<'a, V>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
