use std::collections::{HashMap, HashSet};

use collomatique_ilp::linexpr::EqSymbol;
use collomatique_ilp::{
    ConstraintDesc, LinExpr, Objective, ObjectiveDesc, ProblemBuilder, ProblemDesc, UsableData,
    Variable,
};

use crate::{ConstraintSource, HelperId, InternalVar, Model};

// ---------------------------------------------------------------------------
// Desc types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InternalVarDesc {
    Base(usize),
    Extra(usize),
    Helper { owner: usize, id: u64 },
}

impl InternalVarDesc {
    fn to_internal_var(&self) -> InternalVar<usize, usize> {
        match self {
            InternalVarDesc::Base(b) => InternalVar::Base(*b),
            InternalVarDesc::Extra(e) => InternalVar::Extra(*e),
            InternalVarDesc::Helper { owner, id } => InternalVar::Helper {
                owner: *owner,
                id: HelperId(*id),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConstraintSourceDesc {
    User(usize),
    DefiningExtra {
        extra: usize,
        index: usize,
        for_constraints: bool,
    },
}

impl ConstraintSourceDesc {
    fn to_constraint_source(&self) -> ConstraintSource<usize, usize> {
        match self {
            ConstraintSourceDesc::User(c) => ConstraintSource::User(*c),
            ConstraintSourceDesc::DefiningExtra {
                extra,
                index,
                for_constraints,
            } => ConstraintSource::DefiningExtra {
                extra: *extra,
                index: *index,
                for_constraints: *for_constraints,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubModelDesc {
    pub problem_desc: ProblemDesc,
    pub problem_constraint_sources: Vec<ConstraintSourceDesc>,
    pub var_descs: Vec<InternalVarDesc>,
    pub reconstruction_constraints: Vec<(ConstraintDesc, ConstraintSourceDesc)>,
    pub reconstruction_variables: Vec<(usize, Variable)>,
    pub base_variable_set: Vec<usize>,
    pub reconstruction_objective: ObjectiveDesc,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelDesc {
    pub main: SubModelDesc,
    pub checker: SubModelDesc,
    pub base_var_list: Vec<Variable>,
}

// ---------------------------------------------------------------------------
// Conversion helpers: Constraint/Objective ↔ Desc
// ---------------------------------------------------------------------------

fn constraint_to_desc<V: UsableData>(
    c: &collomatique_ilp::Constraint<V>,
    var_to_index: &HashMap<&V, usize>,
) -> ConstraintDesc {
    let coeffs: Vec<(usize, f64)> = c
        .coefficients()
        .map(|(v, coef)| (var_to_index[v], coef))
        .collect();
    ConstraintDesc {
        coeffs,
        constant: c.get_constant(),
        eq_symbol: c.get_symbol(),
    }
}

fn objective_to_desc<V: UsableData>(
    o: &Objective<V>,
    var_to_index: &HashMap<&V, usize>,
) -> ObjectiveDesc {
    let coeffs: Vec<(usize, f64)> = o
        .get_function()
        .coefficients()
        .map(|(v, coef)| (var_to_index[v], coef))
        .collect();
    ObjectiveDesc {
        coeffs,
        constant: o.get_function().get_constant(),
        sense: o.get_sense(),
    }
}

fn constraint_from_desc<V: UsableData>(
    d: &ConstraintDesc,
    index_to_var: &[V],
) -> collomatique_ilp::Constraint<V> {
    let expr = LinExpr::from_coefficients(
        d.coeffs.iter().map(|(i, c)| (index_to_var[*i].clone(), *c)),
        d.constant,
    );
    match d.eq_symbol {
        EqSymbol::Equals => expr.eq(&LinExpr::default()),
        EqSymbol::LessThan => expr.leq(&LinExpr::default()),
    }
}

fn objective_from_desc<V: UsableData>(d: &ObjectiveDesc, index_to_var: &[V]) -> Objective<V> {
    let expr = LinExpr::from_coefficients(
        d.coeffs.iter().map(|(i, c)| (index_to_var[*i].clone(), *c)),
        d.constant,
    );
    Objective::new(expr, d.sense)
}

// ---------------------------------------------------------------------------
// InternalVar / ConstraintSource → Desc conversion
// ---------------------------------------------------------------------------

fn internal_var_to_desc<B: UsableData, E: UsableData>(
    iv: &InternalVar<B, E>,
    base_to_idx: &HashMap<&B, usize>,
    extra_to_idx: &HashMap<&E, usize>,
) -> InternalVarDesc {
    match iv {
        InternalVar::Base(b) => InternalVarDesc::Base(base_to_idx[b]),
        InternalVar::Extra(e) => InternalVarDesc::Extra(extra_to_idx[e]),
        InternalVar::Helper { owner, id } => InternalVarDesc::Helper {
            owner: extra_to_idx[owner],
            id: id.0,
        },
    }
}

fn constraint_source_to_desc<E: UsableData, C: UsableData>(
    cs: &ConstraintSource<E, C>,
    extra_to_idx: &HashMap<&E, usize>,
    user_constraint_to_idx: &HashMap<&C, usize>,
) -> ConstraintSourceDesc {
    match cs {
        ConstraintSource::User(c) => ConstraintSourceDesc::User(user_constraint_to_idx[c]),
        ConstraintSource::DefiningExtra {
            extra,
            index,
            for_constraints,
        } => ConstraintSourceDesc::DefiningExtra {
            extra: extra_to_idx[extra],
            index: *index,
            for_constraints: *for_constraints,
        },
    }
}

// ---------------------------------------------------------------------------
// Model::to_desc
// ---------------------------------------------------------------------------

fn build_sub_model_desc<B: UsableData, E: UsableData, C: UsableData>(
    problem: &collomatique_ilp::Problem<
        InternalVar<B, E>,
        ConstraintSource<E, C>,
        collomatique_ilp::DefaultRepr<InternalVar<B, E>>,
    >,
    reconstruction_constraints: &[(
        collomatique_ilp::Constraint<InternalVar<B, E>>,
        ConstraintSource<E, C>,
    )],
    reconstruction_variables: &HashMap<InternalVar<B, E>, Variable>,
    base_variable_set: &HashSet<B>,
    reconstruction_objective: &Objective<InternalVar<B, E>>,
    base_to_idx: &HashMap<&B, usize>,
    extra_to_idx: &HashMap<&E, usize>,
    user_constraint_to_idx: &HashMap<&C, usize>,
) -> SubModelDesc {
    let (problem_desc, var_order) = problem.get_desc();

    let var_to_index: HashMap<&InternalVar<B, E>, usize> =
        var_order.iter().enumerate().map(|(i, v)| (v, i)).collect();

    let var_descs: Vec<InternalVarDesc> = var_order
        .iter()
        .map(|iv| internal_var_to_desc(iv, base_to_idx, extra_to_idx))
        .collect();

    let problem_constraint_sources: Vec<ConstraintSourceDesc> = problem
        .get_constraints()
        .iter()
        .map(|(_, src)| constraint_source_to_desc(src, extra_to_idx, user_constraint_to_idx))
        .collect();

    let recon_constraints: Vec<(ConstraintDesc, ConstraintSourceDesc)> = reconstruction_constraints
        .iter()
        .map(|(c, src)| {
            (
                constraint_to_desc(c, &var_to_index),
                constraint_source_to_desc(src, extra_to_idx, user_constraint_to_idx),
            )
        })
        .collect();

    let recon_variables: Vec<(usize, Variable)> = reconstruction_variables
        .iter()
        .map(|(iv, var)| (var_to_index[iv], var.clone()))
        .collect();

    let base_set: Vec<usize> = base_variable_set
        .iter()
        .map(|b| var_to_index[&InternalVar::Base(b.clone())])
        .collect();

    let recon_objective = objective_to_desc(reconstruction_objective, &var_to_index);

    SubModelDesc {
        problem_desc,
        problem_constraint_sources,
        var_descs,
        reconstruction_constraints: recon_constraints,
        reconstruction_variables: recon_variables,
        base_variable_set: base_set,
        reconstruction_objective: recon_objective,
    }
}

impl<B, E, C> Model<B, E, C>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    pub fn to_desc(&self) -> ModelDesc {
        let base_to_idx: HashMap<&B, usize> = self
            .base_var_list
            .keys()
            .enumerate()
            .map(|(i, b)| (b, i))
            .collect();

        let mut extra_set: HashSet<&E> = HashSet::new();
        for (iv, _) in self.problem.get_variables() {
            match iv {
                InternalVar::Extra(e) | InternalVar::Helper { owner: e, .. } => {
                    extra_set.insert(e);
                }
                InternalVar::Base(_) => {}
            }
        }
        let extra_to_idx: HashMap<&E, usize> = extra_set
            .into_iter()
            .enumerate()
            .map(|(i, e)| (e, i))
            .collect();

        let mut user_constraint_set: HashSet<&C> = HashSet::new();
        for (_, src) in self.problem.get_constraints() {
            if let ConstraintSource::User(c) = src {
                user_constraint_set.insert(c);
            }
        }
        let user_constraint_to_idx: HashMap<&C, usize> = user_constraint_set
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c, i))
            .collect();

        let main = build_sub_model_desc(
            &self.problem,
            &self.reconstruction_constraints,
            &self.reconstruction_variables,
            &self.base_variable_set,
            &self.reconstruction_objective,
            &base_to_idx,
            &extra_to_idx,
            &user_constraint_to_idx,
        );

        let checker = build_sub_model_desc(
            &self.checker_problem,
            &self.checker_reconstruction_constraints,
            &self.checker_reconstruction_variables,
            &self.checker_base_variable_set,
            &self.checker_reconstruction_objective,
            &base_to_idx,
            &extra_to_idx,
            &user_constraint_to_idx,
        );

        let mut base_var_list = vec![Variable::binary(); base_to_idx.len()];
        for (b, var) in &self.base_var_list {
            base_var_list[base_to_idx[b]] = var.clone();
        }

        ModelDesc {
            main,
            checker,
            base_var_list,
        }
    }
}

// ---------------------------------------------------------------------------
// ModelDesc::to_model
// ---------------------------------------------------------------------------

type IV = InternalVar<usize, usize>;
type CS = ConstraintSource<usize, usize>;

fn rebuild_sub_model(
    sub: &SubModelDesc,
) -> (
    collomatique_ilp::Problem<IV, CS>,
    Vec<(collomatique_ilp::Constraint<IV>, CS)>,
    HashMap<IV, Variable>,
    HashSet<usize>,
    Objective<IV>,
) {
    let index_to_var: Vec<IV> = sub.var_descs.iter().map(|d| d.to_internal_var()).collect();

    // Rebuild problem
    let mut builder = ProblemBuilder::new();
    for (i, var) in sub.problem_desc.variables.iter().enumerate() {
        builder = builder.set_variable(index_to_var[i].clone(), var.clone());
    }
    for (constraint_desc, source_desc) in sub
        .problem_desc
        .constraints
        .iter()
        .zip(sub.problem_constraint_sources.iter())
    {
        let constraint = constraint_from_desc(constraint_desc, &index_to_var);
        let source = source_desc.to_constraint_source();
        builder = builder.add_constraint(constraint, source);
    }
    let objective = objective_from_desc(&sub.problem_desc.objective, &index_to_var);
    builder = builder.set_objective(objective);
    let problem = builder
        .build()
        .expect("rebuilding problem from ModelDesc should not fail");

    // Rebuild reconstruction constraints
    let recon_constraints: Vec<(collomatique_ilp::Constraint<IV>, CS)> = sub
        .reconstruction_constraints
        .iter()
        .map(|(cd, sd)| {
            (
                constraint_from_desc(cd, &index_to_var),
                sd.to_constraint_source(),
            )
        })
        .collect();

    // Rebuild reconstruction variables
    let recon_variables: HashMap<IV, Variable> = sub
        .reconstruction_variables
        .iter()
        .map(|(idx, var)| (index_to_var[*idx].clone(), var.clone()))
        .collect();

    // Rebuild base variable set
    let base_set: HashSet<usize> = sub.base_variable_set.iter().copied().collect();

    // Rebuild reconstruction objective
    let recon_objective = objective_from_desc(&sub.reconstruction_objective, &index_to_var);

    (
        problem,
        recon_constraints,
        recon_variables,
        base_set,
        recon_objective,
    )
}

impl ModelDesc {
    pub fn to_model(self) -> Model<usize, usize, usize> {
        let (
            problem,
            reconstruction_constraints,
            reconstruction_variables,
            base_variable_set,
            reconstruction_objective,
        ) = rebuild_sub_model(&self.main);

        let (
            checker_problem,
            checker_reconstruction_constraints,
            checker_reconstruction_variables,
            checker_base_variable_set,
            checker_reconstruction_objective,
        ) = rebuild_sub_model(&self.checker);

        let base_var_list: HashMap<usize, Variable> =
            self.base_var_list.into_iter().enumerate().collect();

        Model {
            problem,
            reconstruction_constraints,
            reconstruction_variables,
            base_variable_set,
            checker_problem,
            checker_reconstruction_constraints,
            checker_reconstruction_variables,
            checker_base_variable_set,
            reconstruction_objective,
            checker_reconstruction_objective,
            base_var_list,
        }
    }
}
