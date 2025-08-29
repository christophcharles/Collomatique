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

/// Matrix representation of a problem
/// 
/// This trait is implemented by internal matrix representation of problems.
/// Indeed, in order to make computations (most notably to check if a configuration
/// is feasable), [crate::Problem] uses an internal representation, usually a matrix
/// representation.
/// 
/// This trait must be implemented by any such internal representation.
/// 
/// The default representation if you don't specify it is given by [crate::DefaultRepr].
pub trait ProblemRepr<V: UsableData>:
    Clone + std::fmt::Debug + Send + Sync + PartialEq + Eq + PartialOrd + Ord
{
    /// The corresponding representation for configurations.
    /// 
    /// A configuration must always point to its parent [ProblemRepr].
    type Config<'a>: ConfigRepr<'a, V>
    where
        Self: 'a;

    /// Builds a new representation from variables description
    /// and iterator over constraints.
    fn new<'a, T>(variables: &BTreeMap<V, Variable>, constraints: T) -> Self
    where
        V: 'a,
        T: ExactSizeIterator<Item = &'a Constraint<V>>;

    /// Builds the representation for a specific configuration
    /// defined by a map between variables and their values.
    fn config_from<'a>(
        &'a self,
        vars: &BTreeMap<V, ordered_float::OrderedFloat<f64>>,
    ) -> Self::Config<'a>;
}

/// Matrix representation of a configuration
/// 
/// This trait is implemented by internal matrix representation of configurations ([crate::Config]).
/// This is the configuration equivalent to [ProblemRepr].
/// 
/// Each [ConfigRepr] is associated with a [ProblemRepr] through [ProblemRepr::Config].

pub trait ConfigRepr<'a, V: UsableData>:
    PartialEq + Eq + PartialOrd + Ord + Sized + Clone + std::fmt::Debug + Send + Sync
{
    /// Returns the list of unsatisfied constraints by the current configuration
    /// for its parent [ProblemRepr].
    /// 
    /// The list can be empty if all the constraints are satisfied.
    fn unsatisfied_constraints(&self) -> Vec<usize>;

    /// Returns true if the configuration is feasable.
    /// 
    /// Here, the definition of feasable is restricted:
    /// we do not have to check the domain of the different variables.
    /// 
    /// The default implementation uses [ConfigRepr::unsatisfied_constraints].
    fn is_feasable(&self) -> bool {
        let unsatisfied_constraints = self.unsatisfied_constraints();

        unsatisfied_constraints.is_empty()
    }
}
