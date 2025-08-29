pub mod linexpr;
mod tools;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct ProblemBuilder {
    constraints: Vec<linexpr::Constraint>,
}

impl ProblemBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(mut self, constraint: linexpr::Constraint) -> Self {
        self.constraints.push(constraint);
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
        }
    }
}

use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Problem {
    variables: BTreeSet<String>,
    constraints: Vec<linexpr::Constraint>,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "variables : [")?;
        for v in &self.variables {
            write!(f, " {}", v)?;
        }
        write!(f, " ]\n")?;

        write!(f, "constraints :")?;
        for (i, c) in self.constraints.iter().enumerate() {
            write!(f, "\n{}) {}", i, c)?;
        }
        Ok(())
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
            config.set(v.into(), true);
        }

        config
    }
}
