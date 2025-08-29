use super::{ProblemRepr, ConfigRepr};
use crate::{Variable, UsableData, Constraint, linexpr::EqSymbol};

use ndarray::{Array1, Array2};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default)]
pub struct NdProblem<V: UsableData> {
    mat: Array2<f64>,
    constants: Array1<f64>,
    constraint_symbols: Vec<EqSymbol>,
    variable_map: BTreeMap<V, usize>,
}

impl<V: UsableData> ProblemRepr<V> for NdProblem<V> {
    fn new<'a, T>(variables: &BTreeMap<V, Variable>, constraints: T) -> Self
    where
        V: 'a,
        T: ExactSizeIterator<Item = &'a Constraint<V>>
    {
        let n = constraints.len();
        let p = variables.len();

        let variable_map: BTreeMap<_, _> = variables
            .iter()
            .enumerate()
            .map(|(i, v)| (v.0.clone(), i))
            .collect();

        let mut mat = Array2::zeros((n, p));
        let mut constants = Array1::zeros(n);

        let mut constraints_symbols = Vec::with_capacity(n);

        for (i,c) in constraints.enumerate() {
            constraints_symbols.push(c.get_symbol());
            for (var, val) in c.coefficients() {
                let j = variable_map[var];
                mat[(i,j)] = val;
                constants[i] = c.get_constant();
            }
        }

        NdProblem {
            mat,
            constants,
            constraint_symbols: constraints_symbols,
            variable_map,
        }
    }

    fn config_from<'a>(&'a self, vars: &BTreeMap<V, ordered_float::OrderedFloat<f64>>) -> impl ConfigRepr<'a, V> {
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
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<V: UsableData> Eq for NdProblem<V> {}

impl<V: UsableData> Ord for NdProblem<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ord_symb = self.constraint_symbols.cmp(&other.constraint_symbols);
        if ord_symb != std::cmp::Ordering::Equal {
            return ord_symb;
        }

        let ord_var_map = self.variable_map.cmp(&other.variable_map);
        if ord_var_map != std::cmp::Ordering::Equal {
            return ord_var_map;
        }

        let l1 = self.mat.len();
        let l2 = other.mat.len();

        assert_eq!(l1, l2);

        for (f1,f2) in self.mat.iter().zip(other.mat.iter()) {
            let v1 = ordered_float::OrderedFloat(*f1);
            let v2 = ordered_float::OrderedFloat(*f2);

            let ord = v1.cmp(&v2);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }

        let l1 = self.constants.len();
        let l2 = other.constants.len();

        assert_eq!(l1, l2);

        for (f1,f2) in self.constants.iter().zip(other.constants.iter()) {
            let v1 = ordered_float::OrderedFloat(*f1);
            let v2 = ordered_float::OrderedFloat(*f2);

            let ord = v1.cmp(&v2);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }

        return std::cmp::Ordering::Equal;
    }
}

impl<V: UsableData> PartialOrd for NdProblem<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

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
                    if v != 0.0 {
                        result.push(i);
                    }
                }
                EqSymbol::LessThan => {
                    if v > 0.0 {
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
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a, V: UsableData> Eq for NdConfig<'a, V> {}

impl<'a, V: UsableData> Ord for NdConfig<'a, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let l1 = self.values.len();
        let l2 = other.values.len();

        assert_eq!(l1, l2);

        for (f1,f2) in self.values.iter().zip(other.values.iter()) {
            let v1 = ordered_float::OrderedFloat(*f1);
            let v2 = ordered_float::OrderedFloat(*f2);

            let ord = v1.cmp(&v2);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        return std::cmp::Ordering::Equal;
    }
}

impl<'a, V: UsableData> PartialOrd for NdConfig<'a, V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}