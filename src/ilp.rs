pub mod dbg;
pub mod linexpr;
pub mod random;
pub mod solvers;

pub mod mat_repr;

#[cfg(test)]
mod tests;

use std::sync::RwLock;

use thiserror::Error;

use linexpr::VariableName;
use mat_repr::{ConfigRepr, ProblemRepr};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error<V: VariableName> {
    #[error("Variable {0} is not valid for this problem")]
    InvalidVariable(V),
    #[error("Variable {0} does not have the right type")]
    InvalidVariableType(V),
}

pub type Result<T, V> = std::result::Result<T, Error<V>>;

pub type EvalFn<V, P> = dbg::Debuggable<dyn Sync + Send + Fn(&FeasableConfig<V, P>) -> f64>;

impl<V: VariableName, P: ProblemRepr<V>> Default for EvalFn<V, P> {
    fn default() -> Self {
        crate::debuggable!(|_x| 0.)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariableType {
    Bool,
}

pub type DefaultRepr<V> = mat_repr::sparse::SprsProblem<V>;

#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: VariableName, P: ProblemRepr<V> = DefaultRepr<V>> {
    constraints: BTreeSet<linexpr::Constraint<V>>,
    eval_fn: EvalFn<V, P>,
    variables: BTreeMap<V, VariableType>,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VarError<V: VariableName> {
    #[error("Variable {0} already declared")]
    VariableAlreadyDeclared(V),
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
            variables: BTreeMap::new(),
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
        for var in constraint_vars {
            if !self.variables.contains_key(&var) {
                return Err(ConstraintError::UndeclaredVariable(var));
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
        if self.variables.contains_key(var) {
            return Err(VarError::VariableAlreadyDeclared(var.into()));
        }
        Ok(())
    }

    pub fn add_bool_variable<T: Into<V>>(self, var: T) -> VarResult<Self, V> {
        self.add_variable(var, VariableType::Bool)
    }

    pub fn add_bool_variables<U: Into<V>, T: IntoIterator<Item = U>>(
        mut self,
        vars: T,
    ) -> VarResult<Self, V> {
        for var in vars {
            self = self.add_bool_variable(var)?;
        }
        Ok(self)
    }

    pub fn add_variable<T: Into<V>>(
        mut self,
        var: T,
        var_type: VariableType,
    ) -> VarResult<Self, V> {
        let var = var.into();
        self.check_var(&var)?;
        self.variables.insert(var, var_type);
        Ok(self)
    }

    pub fn add_variables<U: Into<(V, VariableType)>, T: IntoIterator<Item = U>>(
        mut self,
        vars: T,
    ) -> VarResult<Self, V> {
        for var in vars {
            let (var_name, var_type) = var.into();
            self = self.add_variable(var_name, var_type)?;
        }
        Ok(self)
    }

    pub fn get_variables(&self) -> &BTreeMap<V, VariableType> {
        &self.variables
    }

    pub fn build(self) -> Problem<V, P> {
        let variables_vec: Vec<_> = self.variables.iter().map(|(v, _t)| v.clone()).collect();
        let mut variables_lookup = BTreeMap::new();
        for (i, var) in variables_vec.iter().enumerate() {
            variables_lookup.insert(var.clone(), i);
        }

        let pb_repr = P::new(&variables_vec, &self.constraints);

        Problem {
            variables: self.variables,
            variables_vec,
            variables_lookup,
            constraints: self.constraints,
            eval_fn: self.eval_fn,
            pb_repr,
        }
    }

    pub fn filter_variables<F>(self, mut predicate: F) -> ProblemBuilder<V, P>
    where
        F: FnMut(&V) -> bool,
    {
        use linexpr::Constraint;
        let constraints = self
            .constraints
            .into_iter()
            .filter(|c: &Constraint<V>| c.variables().iter().all(&mut predicate))
            .collect();
        let variables = self
            .variables
            .into_iter()
            .filter(|(v, _t)| predicate(v))
            .collect();

        ProblemBuilder {
            constraints,
            eval_fn: self.eval_fn,
            variables,
        }
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Problem<V: VariableName, P: ProblemRepr<V> = DefaultRepr<V>> {
    variables: BTreeMap<V, VariableType>,
    variables_vec: Vec<V>,
    variables_lookup: BTreeMap<V, usize>,
    constraints: BTreeSet<linexpr::Constraint<V>>,
    eval_fn: EvalFn<V, P>,
    pb_repr: P,
}

impl<V: VariableName, P: ProblemRepr<V>> std::fmt::Display for Problem<V, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "variables : [")?;
        for (v, t) in &self.variables {
            write!(f, " {}: {:?}, ", v, t)?;
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
    pub fn into_builder(self) -> ProblemBuilder<V, P> {
        ProblemBuilder {
            constraints: self.constraints,
            eval_fn: self.eval_fn,
            variables: self.variables,
        }
    }

    pub fn default_config<'a>(&'a self) -> Config<'a, V, P> {
        let bool_vars = self.variables.iter().filter_map(|(var, var_type)| {
            if *var_type != VariableType::Bool {
                return None;
            }

            Some((var.clone(), false))
        });
        self.config_from(bool_vars)
            .expect("Valid variables as no variables are used")
    }

    pub fn config_from<'a, U, T: IntoIterator<Item = (U, bool)>>(
        &'a self,
        bool_vars: T,
    ) -> Result<Config<'a, V, P>, V>
    where
        U: Into<V>,
    {
        let mut vars_repr = BTreeMap::new();

        for (var, value) in bool_vars {
            let v = var.into();
            let var_type = match self.variables.get(&v) {
                Some(t) => t.clone(),
                None => return Err(Error::InvalidVariable(v.clone())),
            };
            if var_type != VariableType::Bool {
                return Err(Error::InvalidVariableType(v.clone()));
            }

            let num = self
                .variables_lookup
                .get(&v)
                .copied()
                .expect("Variable should exist as it is in variables map");
            vars_repr.insert(num, if value { 1 } else { 0 });
        }

        Ok(Config {
            problem: self,
            cfg_repr: self.pb_repr.config_from(&vars_repr),
            precomputation: RwLock::new(None),
        })
    }

    pub fn get_constraints(&self) -> &BTreeSet<linexpr::Constraint<V>> {
        &self.constraints
    }

    pub fn get_variables(&self) -> &BTreeMap<V, VariableType> {
        &self.variables
    }
}

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct Precomputation<V: VariableName, P: ProblemRepr<V>> {
    data: <P::Config as mat_repr::ConfigRepr<V>>::Precomputation,
    invalidated_vars: BTreeSet<usize>,
}

#[derive(Debug)]
pub struct Config<'a, V: VariableName, P: ProblemRepr<V> = DefaultRepr<V>> {
    problem: &'a Problem<V, P>,
    cfg_repr: P::Config,
    precomputation: RwLock<Option<Precomputation<V, P>>>,
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Config<'a, V, P> {
    fn clone_precomputation(&self) -> RwLock<Option<Precomputation<V, P>>> {
        let guard = self.precomputation.read().unwrap();
        RwLock::new(guard.clone())
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Clone for Config<'a, V, P> {
    fn clone(&self) -> Self {
        Config {
            problem: self.problem,
            cfg_repr: self.cfg_repr.clone(),
            precomputation: self.clone_precomputation(),
        }
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Config<'a, V, P> {
    pub fn get_problem(&self) -> &'a Problem<V, P> {
        self.problem
    }

    pub fn get_bool<'b, T>(&self, var: &'b T) -> Result<bool, V>
    where
        V: std::borrow::Borrow<T>,
        T: Ord + ?Sized,
        &'b T: Into<V>,
    {
        let var_type = match self.problem.variables.get(var) {
            Some(t) => t.clone(),
            None => return Err(Error::InvalidVariable(var.into())),
        };

        if var_type != VariableType::Bool {
            return Err(Error::InvalidVariableType(var.into()));
        }

        let i = self
            .problem
            .variables_lookup
            .get(var)
            .expect("Variable should exist since it is in variables map");
        Ok(unsafe { self.cfg_repr.get_unchecked(*i) == 1 })
    }

    pub fn get_bool_vars(&self) -> BTreeMap<V, bool> {
        let mut output = BTreeMap::new();
        for (var, i) in &self.problem.variables_lookup {
            let var_type = self
                .problem
                .variables
                .get(var)
                .expect("Variable should exist since it is in variables_lookup")
                .clone();
            if var_type != VariableType::Bool {
                continue;
            }

            let is_true = unsafe { self.cfg_repr.get_unchecked(*i) == 1 };
            output.insert(var.clone(), is_true);
        }
        output
    }

    pub fn set_bool<'b, T>(&mut self, var: &'b T, val: bool) -> Result<(), V>
    where
        V: std::borrow::Borrow<T>,
        T: Ord + ?Sized,
        &'b T: Into<V>,
    {
        let var_type = match self.problem.variables.get(var) {
            Some(t) => t.clone(),
            None => return Err(Error::InvalidVariable(var.into())),
        };

        if var_type != VariableType::Bool {
            return Err(Error::InvalidVariableType(var.into()));
        }

        let i = self
            .problem
            .variables_lookup
            .get(var)
            .expect("Variable should exist since it is in variables map");
        unsafe {
            self.cfg_repr.set_unchecked(*i, if val { 1 } else { 0 });
        }
        self.invalidate_precomputation(*i);
        Ok(())
    }

    pub fn compute_lhs(&self) -> BTreeMap<linexpr::Constraint<V>, i32> {
        let precomputation = self.get_precomputation();
        self.cfg_repr
            .compute_lhs(&self.problem.pb_repr, &*precomputation)
    }

    pub fn compute_lhs_sq_norm2(&self) -> f64 {
        let lhs = self.compute_lhs();
        let mut tot = 0.;
        for (_constraint, val) in lhs {
            tot += f64::from(val * val);
        }
        tot
    }

    pub fn is_feasable(&self) -> bool {
        let precomputation = self.get_precomputation();
        self.cfg_repr
            .is_feasable(&self.problem.pb_repr, &*precomputation)
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

// MappedRwLockReadGuard is still in nightly...
pub struct PrecomputationGuard<'a, V: VariableName, P: ProblemRepr<V>> {
    guard: std::sync::RwLockReadGuard<'a, Option<Precomputation<V, P>>>,
}

impl<'a, V: VariableName, P: ProblemRepr<V>> std::ops::Deref for PrecomputationGuard<'a, V, P> {
    type Target = <P::Config as mat_repr::ConfigRepr<V>>::Precomputation;
    fn deref(&self) -> &Self::Target {
        let opt = self.guard.deref();
        &opt.as_ref().unwrap().data
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Config<'a, V, P> {
    fn get_precomputation(&self) -> PrecomputationGuard<'_, V, P> {
        let mut write_guard = self.precomputation.write().unwrap();
        match write_guard.as_ref() {
            Some(x) => {
                if !x.invalidated_vars.is_empty() {
                    write_guard.as_mut().map(|y| {
                        self.cfg_repr.update_precomputation(
                            &self.problem.pb_repr,
                            &mut y.data,
                            &y.invalidated_vars,
                        );
                        y.invalidated_vars.clear();
                    });
                }
            }
            None => {
                let data = self.cfg_repr.precompute(&self.problem.pb_repr);
                *write_guard = Some(Precomputation {
                    data,
                    invalidated_vars: BTreeSet::new(),
                });
            }
        }
        std::mem::drop(write_guard);
        let guard = self.precomputation.read().unwrap();
        PrecomputationGuard { guard }
    }

    fn invalidate_precomputation(&mut self, i: usize) {
        let mut b = self.precomputation.write().unwrap();

        b.as_mut().map(|x| {
            x.invalidated_vars.insert(i);
        });
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> std::fmt::Display for Config<'a, V, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variables_iter = self
            .problem
            .variables_vec
            .iter()
            .enumerate()
            .map(|(i, var)| (var.clone(), unsafe { self.cfg_repr.get_unchecked(i) }));

        write!(f, "[ ")?;
        let slice: Vec<_> = variables_iter
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
pub struct FeasableConfig<'a, V: VariableName, P: ProblemRepr<V> = DefaultRepr<V>>(
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

    pub fn eval(&self) -> f64 {
        (self.0.problem.eval_fn)(self)
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
