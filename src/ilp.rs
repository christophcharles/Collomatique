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
}

pub type Result<T, V> = std::result::Result<T, Error<V>>;

pub type DefaultRepr<V> = mat_repr::sparse::SprsProblem<V>;

#[derive(Debug, Clone)]
pub struct ObjectiveTerm<V: VariableName> {
    coef: f64,
    exprs: BTreeSet<linexpr::Expr<V>>,
}

#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: VariableName> {
    constraints: BTreeSet<linexpr::Constraint<V>>,
    variables: BTreeSet<V>,
    objective_terms: Vec<ObjectiveTerm<V>>,
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

impl<V: VariableName> Default for ProblemBuilder<V> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: BTreeSet::new(),
            variables: BTreeSet::new(),
            objective_terms: Vec::new(),
        }
    }
}

impl<V: VariableName> ProblemBuilder<V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_constraint(
        mut self,
        constraint: linexpr::Constraint<V>,
    ) -> ConstraintResult<Self, V> {
        let constraint_vars = constraint.variables();
        for var in constraint_vars {
            if !self.variables.contains(&var) {
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

    pub fn add_objective_term<U, I>(mut self, coef: f64, exprs: I) -> ConstraintResult<Self, V>
    where
        I: IntoIterator<Item = linexpr::Expr<V>>,
    {
        let exprs = BTreeSet::from_iter(exprs);
        for expr in &exprs {
            for var in expr.variables() {
                if !self.variables.contains(&var) {
                    return Err(ConstraintError::UndeclaredVariable(var));
                }
            }
        }

        self.objective_terms.push(ObjectiveTerm { coef, exprs });
        Ok(self)
    }

    pub fn add_bool_variable<T: Into<V>>(mut self, var: T) -> VarResult<Self, V> {
        let v = var.into();
        if self.variables.contains(&v) {
            return Err(VarError::VariableAlreadyDeclared(v.into()));
        }
        self.variables.insert(v);
        Ok(self)
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

    pub fn get_variables(&self) -> &BTreeSet<V> {
        &self.variables
    }

    pub fn build<P: ProblemRepr<V>>(self) -> Problem<V, P> {
        let variables_vec: Vec<_> = self.variables.iter().cloned().collect();
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
            pb_repr,
            objective_terms: self.objective_terms,
        }
    }

    pub fn filter_variables<F>(self, mut predicate: F) -> ProblemBuilder<V>
    where
        F: FnMut(&V) -> bool,
    {
        use linexpr::Constraint;
        let constraints = self
            .constraints
            .into_iter()
            .filter(|c: &Constraint<V>| c.variables().iter().all(&mut predicate))
            .collect();
        let variables = self.variables.into_iter().filter(&mut predicate).collect();
        let objective_terms = self
            .objective_terms
            .into_iter()
            .filter_map(|obj_term| {
                let exprs: BTreeSet<_> = obj_term
                    .exprs
                    .into_iter()
                    .filter(|e| e.variables().iter().all(&mut predicate))
                    .collect();

                if exprs.is_empty() {
                    return None;
                }

                Some(ObjectiveTerm {
                    coef: obj_term.coef,
                    exprs,
                })
            })
            .collect();

        ProblemBuilder {
            constraints,
            variables,
            objective_terms,
        }
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Problem<V: VariableName, P: ProblemRepr<V> = DefaultRepr<V>> {
    variables: BTreeSet<V>,
    variables_vec: Vec<V>,
    variables_lookup: BTreeMap<V, usize>,
    constraints: BTreeSet<linexpr::Constraint<V>>,
    pb_repr: P,
    objective_terms: Vec<ObjectiveTerm<V>>,
}

impl<V: VariableName, P: ProblemRepr<V>> std::fmt::Display for Problem<V, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "variables : [")?;
        for v in &self.variables {
            write!(f, " {}", v)?;
        }
        write!(f, " ]\n")?;

        write!(f, "constraints :\n")?;
        for (i, c) in self.constraints.iter().enumerate() {
            write!(f, "{}) {}\n", i, c)?;
        }

        write!(f, "objectives terms :\n")?;
        for (i, o) in self.objective_terms.iter().enumerate() {
            let strings: Vec<_> = o.exprs.iter().map(|x| x.to_string()).collect();
            write!(
                f,
                "{}) coefficient: {}, expressions: {}",
                i,
                o.coef,
                strings.join(", ")
            )?;
        }

        Ok(())
    }
}

impl<V: VariableName, P: ProblemRepr<V>> Problem<V, P> {
    pub fn into_builder(self) -> ProblemBuilder<V> {
        ProblemBuilder {
            constraints: self.constraints,
            variables: self.variables,
            objective_terms: self.objective_terms,
        }
    }

    pub fn default_config<'a>(&'a self) -> Config<'a, V, P> {
        self.config_from::<V, _>([])
            .expect("Valid variables as no variables are used")
    }

    pub fn config_from<'a, U, I1>(&'a self, bool_vars: I1) -> Result<Config<'a, V, P>, V>
    where
        U: Into<V>,
        I1: IntoIterator<Item = (U, bool)>,
    {
        let mut vars_repr = BTreeMap::new();

        for (var, value) in bool_vars {
            let v = var.into();

            let num = self
                .variables_lookup
                .get(&v)
                .copied()
                .ok_or(Error::InvalidVariable(v.clone()))?;

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

    pub fn get_variables(&self) -> &BTreeSet<V> {
        &self.variables
    }

    pub fn get_objective_terms(&self) -> &Vec<ObjectiveTerm<V>> {
        &self.objective_terms
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
        let i = self
            .problem
            .variables_lookup
            .get(var)
            .ok_or(Error::InvalidVariable(var.into()))?;
        Ok(unsafe { self.cfg_repr.get_unchecked(*i) == 1 })
    }

    pub fn get_bool_vars(&self) -> BTreeMap<V, bool> {
        let mut output = BTreeMap::new();
        for (var, i) in &self.problem.variables_lookup {
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
        let i = self
            .problem
            .variables_lookup
            .get(var)
            .ok_or(Error::InvalidVariable(var.into()))?;
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
