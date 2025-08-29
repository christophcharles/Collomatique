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
use crate::{f64_is_positive, f64_is_zero, linexpr::EqSymbol, Constraint, UsableData, Variable};

use sprs::{CsMat, CsVec, TriMat};
use std::collections::{BTreeMap, BTreeSet};

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
    variable_map: BTreeMap<V, usize>,
}

impl<V: UsableData> ProblemRepr<V> for SprsProblem<V> {
    type Config<'a> = SprsConfig<'a, V>
    where
        V: 'a,
        Self: 'a;

    fn new<'a, T>(variables: &BTreeMap<V, Variable>, constraints: T) -> Self
    where
        V: 'a,
        T: ExactSizeIterator<Item = &'a Constraint<V>>,
    {
        let n = constraints.len();
        let p = variables.len();

        let variable_map: BTreeMap<_, _> = variables
            .iter()
            .enumerate()
            .map(|(i, v)| (v.0.clone(), i))
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
        vars: &BTreeMap<V, ordered_float::OrderedFloat<f64>>,
    ) -> SprsConfig<'a, V> {
        let p = self.mat.shape().1;

        let mut indices = Vec::new();
        let mut data = Vec::new();

        let mut last_i = 0usize;
        for (name, value) in vars {
            let v = value.into_inner();
            if f64_is_zero(v) {
                continue;
            }

            let i = self.variable_map[name];

            // Consistency check
            assert!(i >= last_i);
            last_i = i;

            indices.push(i);
            data.push(v);
        }

        let values = CsVec::new(p, indices, data);

        SprsConfig {
            pb_repr: self,
            values,
        }
    }
}

impl<V: UsableData> PartialEq for SprsProblem<V> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<V: UsableData> Eq for SprsProblem<V> {}

impl<V: UsableData> Ord for SprsProblem<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ord_symb = self.constraint_symbols.cmp(&other.constraint_symbols);
        if ord_symb != std::cmp::Ordering::Equal {
            return ord_symb;
        }

        let ord_var_map = self.variable_map.cmp(&other.variable_map);
        if ord_var_map != std::cmp::Ordering::Equal {
            return ord_var_map;
        }

        let s1 = self.mat.shape();
        let s2 = other.mat.shape();

        assert_eq!(s1, s2);

        let diff = &self.mat - &other.mat;

        for (f, _coord) in diff.iter() {
            let v = ordered_float::OrderedFloat(*f);
            let ord = v.cmp(&ordered_float::OrderedFloat(0.0));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }

        let l1 = self.constants.dim();
        let l2 = other.constants.dim();

        assert_eq!(l1, l2);

        let diff = &self.constants - &other.constants;

        for (_i, f) in diff.iter() {
            let v = ordered_float::OrderedFloat(*f);
            let ord = v.cmp(&ordered_float::OrderedFloat(0.0));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }

        return std::cmp::Ordering::Equal;
    }
}

impl<V: UsableData> PartialOrd for SprsProblem<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

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
    fn unsatisfied_constraints(&self) -> BTreeSet<usize> {
        let column = &self.pb_repr.mat * &self.values + &self.pb_repr.constants;

        assert_eq!(column.dim(), self.pb_repr.constraint_symbols.len());

        let mut result = BTreeSet::new();

        for (i, v) in column.iter() {
            let symb = self.pb_repr.constraint_symbols[i];

            match symb {
                EqSymbol::Equals => {
                    if !f64_is_zero(*v) {
                        result.insert(i);
                    }
                }
                EqSymbol::LessThan => {
                    if f64_is_positive(*v) {
                        result.insert(i);
                    }
                }
            }
        }

        result
    }
}

impl<'a, V: UsableData> PartialEq for SprsConfig<'a, V> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a, V: UsableData> Eq for SprsConfig<'a, V> {}

impl<'a, V: UsableData> Ord for SprsConfig<'a, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ord = self.pb_repr.cmp(&other.pb_repr);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }

        let l1 = self.values.dim();
        let l2 = other.values.dim();

        assert_eq!(l1, l2);

        let diff = &self.values - &other.values;

        for (_i, f) in diff.iter() {
            let v = ordered_float::OrderedFloat(*f);
            let ord = v.cmp(&ordered_float::OrderedFloat(0.0));
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }

        return std::cmp::Ordering::Equal;
    }
}

impl<'a, V: UsableData> PartialOrd for SprsConfig<'a, V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
