pub mod linexpr;
pub mod random;
pub mod solvers;
mod tools;

#[cfg(test)]
mod tests;

use std::sync::Arc;

#[derive(Clone)]
pub struct EvalFn {
    func: Arc<dyn Fn(&Config) -> Option<f64>>,
    debug_payload: &'static str,
}

impl EvalFn {
    pub fn new(func: Arc<dyn Fn(&Config) -> Option<f64>>, debug_payload: &'static str) -> EvalFn {
        EvalFn {
            func,
            debug_payload,
        }
    }
}

#[macro_export]
macro_rules! eval_fn {
    ($($body:tt)+) => {
        $crate::ilp::EvalFn::new(
            std::sync::Arc::new($($body)+),
            stringify!($($body)+)
        )
    };
}

impl std::fmt::Debug for EvalFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug_payload)
    }
}

impl std::ops::Deref for EvalFn {
    type Target = Arc<dyn Fn(&Config) -> Option<f64>>;
    fn deref(&self) -> &Arc<dyn Fn(&Config) -> Option<f64>> {
        &self.func
    }
}

#[derive(Debug, Default, Clone)]
pub struct ProblemBuilder {
    constraints: Vec<linexpr::Constraint>,
    eval_fn: Option<EvalFn>,
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
        self.eval_fn = Some(func);
        self
    }

    pub fn build(mut self) -> Problem {
        let mut variables = BTreeSet::new();

        for c in self.constraints.iter_mut() {
            c.clean();
        }

        for c in self.constraints.iter() {
            variables.append(&mut c.variables());
        }

        Problem {
            variables,
            constraints: self.constraints,
            eval_fn: self.eval_fn,
        }
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub struct Problem {
    variables: BTreeSet<String>,
    constraints: Vec<linexpr::Constraint>,
    eval_fn: Option<EvalFn>,
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
