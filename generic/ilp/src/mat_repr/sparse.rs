//! Matrix representation based on [sprs].
//!
//! This representation represents matrices using a sparse matrix
//! representation from [sprs].
//!
//! Indeed, in Collomatique, the ILP problems can have tens of thousands (or even
//! hundreds of thousands) of constraints and variables. As such, the naive matrix
//! representations can quickly lead to *gigabytes* of data.
//!
//! Therefore, it is usually better to use a sparse matrix representation
//! which is more efficient and well-suited to typical scheduling problems.
//!
//! In those problems, there are still ten of thousands of constraints but each constraint
//! only conerns a few variables (maybe ten or so). So a matrix representation suited
//! for sparse matrices will only lead to *megabytes* of data.
//!
//! It is still huge but can easily be fitted in a usual computer memory (even a bad one - heck
//! it fits on most modern smartphones).

use super::{ConfigRepr, ProblemRepr};
use crate::{Constraint, UsableData, Variable, f64_is_positive, f64_is_zero, linexpr::EqSymbol};

use sprs::{CsMat, CsVec, TriMat};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// Implementation of a problem representation ([ProblemRepr])
/// using [sprs] as a backend.
///
/// See [super::sparse] documentation for more details.
#[derive(Debug, Clone)]
pub struct SprsProblem<V: UsableData> {
    mat: CsMat<f64>,
    constants: CsVec<f64>,
    constraint_symbols: Vec<EqSymbol>,
    variable_map: HashMap<V, usize>,
}

impl<V: UsableData> ProblemRepr<V> for SprsProblem<V> {
    type Config<'a>
        = SprsConfig<'a, V>
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

        let variable_map: HashMap<V, usize> = variables
            .keys()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();

        let mut mat_tri = TriMat::new((n, p));
        let mut constants_indices = Vec::new();
        let mut constants_data = Vec::new();

        let mut constraint_symbols = Vec::with_capacity(n);

        for (i, c) in constraints.enumerate() {
            constraint_symbols.push(c.get_symbol());
            for (var, val) in c.coefficients() {
                if f64_is_zero(val) {
                    continue;
                }

                let j = variable_map[var];
                mat_tri.add_triplet(i, j, val);
            }

            let constant = c.get_constant();
            if !f64_is_zero(constant) {
                constants_indices.push(i);
                constants_data.push(constant);
            }
        }

        let mat = mat_tri.to_csr();
        let constants = CsVec::new(n, constants_indices, constants_data);

        SprsProblem {
            mat,
            constants,
            constraint_symbols,
            variable_map,
        }
    }

    fn config_from<'a>(
        &'a self,
        vars: &HashMap<V, ordered_float::OrderedFloat<f64>>,
    ) -> SprsConfig<'a, V> {
        let p = self.mat.shape().1;

        // Collect (index, value) pairs and sort by index — CsVec requires sorted indices
        let mut entries: Vec<(usize, f64)> = vars
            .iter()
            .filter_map(|(name, value)| {
                let v = value.into_inner();
                if f64_is_zero(v) {
                    return None;
                }
                let i = self.variable_map[name];
                Some((i, v))
            })
            .collect();
        entries.sort_by_key(|(i, _)| *i);

        let (indices, data): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let values = CsVec::new(p, indices, data);

        SprsConfig {
            pb_repr: self,
            values,
        }
    }
}

impl<V: UsableData> PartialEq for SprsProblem<V> {
    fn eq(&self, other: &Self) -> bool {
        self.constraint_symbols == other.constraint_symbols
            && self.variable_map == other.variable_map
            && self.mat.shape() == other.mat.shape()
            && (&self.mat - &other.mat)
                .iter()
                .all(|(f, _)| f64_is_zero(*f))
            && self.constants.dim() == other.constants.dim()
            && (&self.constants - &other.constants)
                .iter()
                .all(|(_, f)| f64_is_zero(*f))
    }
}

impl<V: UsableData> Eq for SprsProblem<V> {}

/// Implementation of a configuration representation ([ConfigRepr])
/// using [sprs] as a backend.
///
/// See [super::sparse] documentation for more details.
#[derive(Debug, Clone)]
pub struct SprsConfig<'a, V: UsableData> {
    pb_repr: &'a SprsProblem<V>,
    values: CsVec<f64>,
}

impl<'a, V: UsableData> ConfigRepr<'a, V> for SprsConfig<'a, V> {
    fn unsatisfied_constraints(&self) -> Vec<usize> {
        let column = &self.pb_repr.mat * &self.values + &self.pb_repr.constants;

        assert_eq!(column.dim(), self.pb_repr.constraint_symbols.len());

        let mut result = Vec::new();

        for (i, v) in column.iter() {
            let symb = self.pb_repr.constraint_symbols[i];

            match symb {
                EqSymbol::Equals => {
                    if !f64_is_zero(*v) {
                        result.push(i);
                    }
                }
                EqSymbol::LessThan => {
                    if f64_is_positive(*v) {
                        result.push(i);
                    }
                }
            }
        }

        result
    }
}

impl<'a, V: UsableData> PartialEq for SprsConfig<'a, V> {
    fn eq(&self, other: &Self) -> bool {
        self.pb_repr == other.pb_repr
            && self.values.dim() == other.values.dim()
            && (&self.values - &other.values)
                .iter()
                .all(|(_, f)| f64_is_zero(*f))
    }
}

impl<'a, V: UsableData> Eq for SprsConfig<'a, V> {}
