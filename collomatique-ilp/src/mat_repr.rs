//! Matrix representation of problems and configurations
//!
//! This module contains the definition of two traits [ProblemRepr] and [ConfigRepr].
//! These traits are used to represent ILP problem and corresponding configurations into
//! some actual numerical representation, usually matrix repr.
//!
//! In addition to these traits, the module contains two submodules [nd] and [sparse]
//! which contain the implementation using the ndarray crate and a sparse matrix representation
//! respectively.
//!
//! Technically, it should be possible to represent problems and configurations using something else
//! than matrices. But this is the straightforward way to do it and only this way was indeed
//! implemented.

pub mod nd;

use super::{Constraint, UsableData, Variable};

use std::collections::BTreeMap;

pub trait ProblemRepr<V: UsableData>:
    Clone + std::fmt::Debug + Send + Sync + PartialEq + Eq + PartialOrd + Ord
{
    type Config<'a>: ConfigRepr<'a, V>
    where
        Self: 'a;

    fn new<'a, T>(variables: &BTreeMap<V, Variable>, constraints: T) -> Self
    where
        V: 'a,
        T: ExactSizeIterator<Item = &'a Constraint<V>>;
    fn config_from<'a>(
        &'a self,
        vars: &BTreeMap<V, ordered_float::OrderedFloat<f64>>,
    ) -> Self::Config<'a>;
}

pub trait ConfigRepr<'a, V: UsableData>:
    PartialEq + Eq + PartialOrd + Ord + Sized + Clone + std::fmt::Debug + Send + Sync
{
    fn unsatisfied_constraints(&self) -> Vec<usize>;
    fn is_feasable(&self) -> bool {
        let unsatisfied_constraints = self.unsatisfied_constraints();

        unsatisfied_constraints.is_empty()
    }
}
