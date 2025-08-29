pub mod dbg;
pub mod linexpr;
pub mod optimizers;
pub mod random;
pub mod solvers;

mod mat_repr;

#[cfg(test)]
mod tests;

use thiserror::Error;

use linexpr::VariableName;
use mat_repr::{ConfigRepr, ProblemRepr};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error<V: VariableName> {
    #[error("Variable {0} is not valid for this problem")]
    InvalidVariable(V),
    #[error("Variable {0} is actually a constant and cannot be set")]
    ConstantNotVariable(V),
}

pub type Result<T, V> = std::result::Result<T, Error<V>>;

pub type EvalFn<V, P> = dbg::Debuggable<dyn Fn(&FeasableConfig<V, P>) -> f64>;

impl<V: VariableName, P: ProblemRepr<V>> Default for EvalFn<V, P> {
    fn default() -> Self {
        crate::debuggable!(|_x| 0.)
    }
}

#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: VariableName, P: ProblemRepr<V> = mat_repr::nd::NdProblem<V>> {
    constraints: BTreeSet<linexpr::Constraint<V>>,
    eval_fn: EvalFn<V, P>,
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
pub enum ConstraintError<V: VariableName> {
    #[error("Variable {0} is used in constraint but not explicitly declared")]
    UndeclaredVariable(V),
}

pub type ConstraintResult<T, V> = std::result::Result<T, ConstraintError<V>>;

impl<V: VariableName, P: ProblemRepr<V>> Default for ProblemBuilder<V, P> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: BTreeSet::new(),
            eval_fn: EvalFn::default(),
            variables: BTreeSet::new(),
            constants: BTreeMap::new(),
        }
    }
}

impl<V: VariableName, P: ProblemRepr<V>> ProblemBuilder<V, P> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_constraint(
        mut self,
        constraint: linexpr::Constraint<V>,
    ) -> ConstraintResult<Self, V> {
        let constraint_vars = constraint.variables();
        if !self.variables.is_superset(&constraint_vars) {
            for var in constraint_vars {
                if !self.variables.contains(&var) {
                    return Err(ConstraintError::UndeclaredVariable(var));
                }
            }
        }

        self.constraints.insert(constraint.cleaned());
        Ok(self)
    }

    pub fn add_constraints<T: IntoIterator<Item = linexpr::Constraint<V>>>(
        mut self,
        constraints: T,
    ) -> ConstraintResult<Self, V> {
        for constraint in constraints {
            self = self.add_constraint(constraint)?;
        }
        Ok(self)
    }

    pub fn eval_fn(mut self, func: EvalFn<V, P>) -> Self {
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

    pub fn build(self) -> Problem<V, P> {
        let variables_vec: Vec<_> = self.variables.iter().cloned().collect();
        let mut variables_lookup = BTreeMap::new();
        for (i, var) in variables_vec.iter().enumerate() {
            variables_lookup.insert(var.clone(), i);
        }

        let nd_problem = P::new(&variables_vec, &self.constraints);

        Problem {
            variables: self.variables,
            variables_vec,
            variables_lookup,
            constraints: self.constraints,
            constants: self.constants,
            eval_fn: self.eval_fn,
            pb_repr: nd_problem,
        }
    }

    pub fn simplify_trivial_constraints(self) -> ProblemBuilder<V, P> {
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

impl<V: VariableName, P: ProblemRepr<V>> ProblemBuilder<V, P> {
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
pub struct Problem<V: VariableName, P: ProblemRepr<V> = mat_repr::nd::NdProblem<V>> {
    variables: BTreeSet<V>,
    variables_vec: Vec<V>,
    variables_lookup: BTreeMap<V, usize>,
    constraints: BTreeSet<linexpr::Constraint<V>>,
    constants: BTreeMap<V, bool>,
    eval_fn: EvalFn<V, P>,
    pb_repr: P,
}

impl<V: VariableName, P: ProblemRepr<V>> std::fmt::Display for Problem<V, P> {
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

impl<V: VariableName, P: ProblemRepr<V>> Problem<V, P> {
    pub fn default_config<'a>(&'a self) -> Config<'a, V, P> {
        Config {
            problem: self,
            cfg_repr: self.pb_repr.default_config(),
        }
    }

    pub fn config_from<'a, 'b, U, T: IntoIterator<Item = &'b U>>(
        &'a self,
        vars: T,
    ) -> Result<Config<'a, V, P>, V>
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

    pub fn random_config<T: random::RandomGen>(&self, random_gen: &mut T) -> Config<'_, V, P> {
        Config {
            problem: self,
            cfg_repr: self.pb_repr.random_config(random_gen),
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
pub struct Config<'a, V: VariableName, P: ProblemRepr<V>> {
    problem: &'a Problem<V, P>,
    cfg_repr: P::Config,
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Config<'a, V, P> {
    pub fn get_problem(&self) -> &Problem<V, P> {
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
        Ok(unsafe { self.cfg_repr.get_unchecked(*i) == 1 })
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
            self.cfg_repr.set_unchecked(*i, if val { 1 } else { 0 });
        }
        Ok(())
    }

    pub fn random_neighbour<T: random::RandomGen>(
        &self,
        random_gen: &mut T,
    ) -> Option<Config<'a, V, P>> {
        if self.problem.variables.is_empty() {
            return None;
        }

        Some(Config {
            problem: self.problem,
            cfg_repr: self.cfg_repr.random_neighbour(random_gen),
        })
    }

    pub fn neighbours(&self) -> Vec<Config<'a, V, P>> {
        self.cfg_repr
            .neighbours()
            .into_iter()
            .map(|x| Config {
                problem: self.problem,
                cfg_repr: x,
            })
            .collect()
    }

    pub fn max_distance_to_constraint(&self) -> f32 {
        self.cfg_repr
            .max_distance_to_constraint(&self.problem.pb_repr)
    }

    pub fn compute_lhs(&self) -> BTreeMap<linexpr::Constraint<V>, i32> {
        self.cfg_repr.compute_lhs(&self.problem.pb_repr)
    }

    pub fn is_feasable(&self) -> bool {
        self.cfg_repr.is_feasable(&self.problem.pb_repr)
    }

    pub fn into_feasable(self) -> Option<FeasableConfig<'a, V, P>> {
        if !self.is_feasable() {
            return None;
        }

        Some(unsafe { self.into_feasable_unchecked() })
    }

    pub unsafe fn into_feasable_unchecked(self) -> FeasableConfig<'a, V, P> {
        FeasableConfig(self)
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> std::fmt::Display for Config<'a, V, P> {
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
                .map(|(i, var)| (var.clone(), unsafe { self.cfg_repr.get_unchecked(i) })),
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

impl<'a, V: VariableName, P: ProblemRepr<V>> PartialEq for Config<'a, V, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Eq for Config<'a, V, P> {}

impl<'a, V: VariableName, P: ProblemRepr<V>> Ord for Config<'a, V, P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let p1: *const Problem<V, P> = &*self.problem;
        let p2: *const Problem<V, P> = &*other.problem;

        let problem_ord = p1.cmp(&p2);
        if problem_ord != std::cmp::Ordering::Equal {
            return problem_ord;
        }

        return self.cfg_repr.cmp(&other.cfg_repr);
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> PartialOrd for Config<'a, V, P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct FeasableConfig<'a, V: VariableName, P: ProblemRepr<V> = mat_repr::nd::NdProblem<V>>(
    Config<'a, V, P>,
);

impl<'a, V: VariableName, P: ProblemRepr<V>> PartialEq for FeasableConfig<'a, V, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Eq for FeasableConfig<'a, V, P> {}

impl<'a, V: VariableName, P: ProblemRepr<V>> Ord for FeasableConfig<'a, V, P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner().cmp(other.inner())
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> PartialOrd for FeasableConfig<'a, V, P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> FeasableConfig<'a, V, P> {
    pub fn into_inner(self) -> Config<'a, V, P> {
        self.0
    }

    pub fn inner(&self) -> &Config<'a, V, P> {
        &self.0
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> std::ops::Deref for FeasableConfig<'a, V, P> {
    type Target = Config<'a, V, P>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> std::fmt::Display for FeasableConfig<'a, V, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner().fmt(f)
    }
}
