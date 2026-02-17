//! Matrix representation based on [ndarray].
//!
//! This representation simply represents matrices using [ndarray].
//! It is quite straightforward but mostly used for testing.
//!
//! Indeed, in Collomatique, the ILP problems can have tens of thousands (or even
//! hundreds of thousands) of constraints and variables. As such, the matrix
//! representations can quickly lead to *gigabytes* of data.
//!
//! Therefore, it is usually better to use a sparse matrix representation (like [super::sparse])
//! which is more efficient and well-suited to typical scheduling problems.
//!
//! Still, this representation is sufficiently straightforward for testing purposes.

use super::{ConfigRepr, ProblemRepr};
use crate::{Constraint, UsableData, Variable, f64_is_positive, f64_is_zero, linexpr::EqSymbol};

use ndarray::{Array1, Array2};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// Implementation of a problem representation ([ProblemRepr])
/// using [ndarray] as a backend.
///
/// See [super::nd] documentation for more details.
#[derive(Debug, Clone)]
pub struct NdProblem<V: UsableData> {
    mat: Array2<f64>,
    constants: Array1<f64>,
    constraint_symbols: Vec<EqSymbol>,
    variable_map: HashMap<V, usize>,
}

impl<V: UsableData> ProblemRepr<V> for NdProblem<V> {
    type Config<'a>
        = NdConfig<'a, V>
    where
        V: 'a,
        Self: 'a;

    fn new<'a, T>(variables: &HashMap<V, Variable>, constraints: T) -> Self
    where
        V: 'a,
        T: ExactSizeIterator<Item = &'a Constraint<V>>,
    {
        let n = constraints.len();
        let p = variables.len();

        let variable_map: HashMap<_, _> = variables
            .keys()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();

        let mut mat = Array2::zeros((n, p));
        let mut constants = Array1::zeros(n);

        let mut constraint_symbols = Vec::with_capacity(n);

        for (i, c) in constraints.enumerate() {
            constraint_symbols.push(c.get_symbol());
            for (var, val) in c.coefficients() {
                let j = variable_map[var];
                mat[(i, j)] = val;
                constants[i] = c.get_constant();
            }
        }

        NdProblem {
            mat,
            constants,
            constraint_symbols,
            variable_map,
        }
    }

    fn config_from<'a>(
        &'a self,
        vars: &HashMap<V, ordered_float::OrderedFloat<f64>>,
    ) -> NdConfig<'a, V> {
        let p = self.mat.shape()[1];

        let mut values = Array1::zeros(p);

        for (name, value) in vars {
            let i = self.variable_map[name];
            values[i] = value.into_inner();
        }

        NdConfig {
            pb_repr: self,
            values,
        }
    }
}

impl<V: UsableData> PartialEq for NdProblem<V> {
    fn eq(&self, other: &Self) -> bool {
        self.constraint_symbols == other.constraint_symbols
            && self.variable_map == other.variable_map
            && self.mat.shape() == other.mat.shape()
            && self
                .mat
                .iter()
                .zip(other.mat.iter())
                .all(|(a, b)| f64_is_zero(a - b))
            && self.constants.len() == other.constants.len()
            && self
                .constants
                .iter()
                .zip(other.constants.iter())
                .all(|(a, b)| f64_is_zero(a - b))
    }
}

impl<V: UsableData> Eq for NdProblem<V> {}

/// Implementation of a configuration representation ([ConfigRepr])
/// using [ndarray] as a backend.
///
/// See [super::nd] documentation for more details.
#[derive(Debug, Clone)]
pub struct NdConfig<'a, V: UsableData> {
    pb_repr: &'a NdProblem<V>,
    values: Array1<f64>,
}

impl<'a, V: UsableData> ConfigRepr<'a, V> for NdConfig<'a, V> {
    fn unsatisfied_constraints(&self) -> Vec<usize> {
        let column = self.pb_repr.mat.dot(&self.values) + &self.pb_repr.constants;

        assert_eq!(column.len(), self.pb_repr.constraint_symbols.len());

        let mut result = Vec::new();
        for i in 0..column.len() {
            let symb = self.pb_repr.constraint_symbols[i];
            let v = column[i];

            match symb {
                EqSymbol::Equals => {
                    if !f64_is_zero(v) {
                        result.push(i);
                    }
                }
                EqSymbol::LessThan => {
                    if f64_is_positive(v) {
                        result.push(i);
                    }
                }
            }
        }

        result
    }
}

impl<'a, V: UsableData> PartialEq for NdConfig<'a, V> {
    fn eq(&self, other: &Self) -> bool {
        self.pb_repr == other.pb_repr
            && self.values.len() == other.values.len()
            && self
                .values
                .iter()
                .zip(other.values.iter())
                .all(|(a, b)| f64_is_zero(a - b))
    }
}

impl<'a, V: UsableData> Eq for NdConfig<'a, V> {}
