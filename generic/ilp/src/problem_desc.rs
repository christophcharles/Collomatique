#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::linexpr::EqSymbol;
use crate::mat_repr::ProblemRepr;
use crate::objectives::ObjectiveSense;
use crate::{ConfigData, LinExpr, Objective, Problem, ProblemBuilder, UsableData, Variable};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProblemDesc {
    pub variables: Vec<Variable>,
    pub constraints: Vec<ConstraintDesc>,
    pub objective: ObjectiveDesc,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConstraintDesc {
    pub coeffs: Vec<(usize, f64)>,
    pub constant: f64,
    pub eq_symbol: EqSymbol,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ObjectiveDesc {
    pub coeffs: Vec<(usize, f64)>,
    pub constant: f64,
    pub sense: ObjectiveSense,
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Problem<V, C, P> {
    pub fn get_desc(&self) -> (ProblemDesc, Vec<V>) {
        let variables_map = self.get_variables();
        let var_order: Vec<V> = variables_map.keys().cloned().collect();
        let var_to_index: HashMap<&V, usize> =
            var_order.iter().enumerate().map(|(i, v)| (v, i)).collect();

        let variables: Vec<Variable> = var_order.iter().map(|v| variables_map[v].clone()).collect();

        let constraints: Vec<ConstraintDesc> = self
            .get_constraints()
            .iter()
            .map(|(constraint, _desc)| {
                let coeffs: Vec<(usize, f64)> = constraint
                    .coefficients()
                    .map(|(v, coef)| (var_to_index[v], coef))
                    .collect();
                ConstraintDesc {
                    coeffs,
                    constant: constraint.get_constant(),
                    eq_symbol: constraint.get_symbol(),
                }
            })
            .collect();

        let obj = self.get_objective();
        let obj_coeffs: Vec<(usize, f64)> = obj
            .get_function()
            .coefficients()
            .map(|(v, coef)| (var_to_index[v], coef))
            .collect();
        let objective = ObjectiveDesc {
            coeffs: obj_coeffs,
            constant: obj.get_function().get_constant(),
            sense: obj.get_sense(),
        };

        (
            ProblemDesc {
                variables,
                constraints,
                objective,
            },
            var_order,
        )
    }
}

impl<P: ProblemRepr<usize>> ProblemBuilder<usize, (), P> {
    pub fn from_desc(desc: ProblemDesc) -> Self {
        let mut builder = ProblemBuilder::new();

        for (i, var) in desc.variables.into_iter().enumerate() {
            builder = builder.set_variable(i, var);
        }

        for constraint_desc in desc.constraints {
            let expr = LinExpr::from_coefficients(
                constraint_desc.coeffs.into_iter(),
                constraint_desc.constant,
            );
            let constraint = match constraint_desc.eq_symbol {
                EqSymbol::Equals => expr.eq(&LinExpr::default()),
                EqSymbol::LessThan => expr.leq(&LinExpr::default()),
            };
            builder = builder.add_constraint(constraint, ());
        }

        let obj_expr =
            LinExpr::from_coefficients(desc.objective.coeffs.into_iter(), desc.objective.constant);
        let objective = Objective::new(obj_expr, desc.objective.sense);
        builder = builder.set_objective(objective);

        builder
    }
}

pub fn solution_to_config_data<V: UsableData>(solution: &[f64], var_order: &[V]) -> ConfigData<V> {
    ConfigData::from(
        var_order
            .iter()
            .zip(solution.iter())
            .map(|(v, &val)| (v.clone(), val)),
    )
}

pub fn config_data_to_hint<V: UsableData>(hint: &ConfigData<V>, var_order: &[V]) -> Vec<f64> {
    var_order
        .iter()
        .map(|v| hint.get(v.clone()).unwrap_or(0.0))
        .collect()
}
