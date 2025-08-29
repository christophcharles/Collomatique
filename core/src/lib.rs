//! Collomatique-core
//! ---
//!
//! This crate contains the main logic of Collomatique for solving colloscopes related problems.
//! The goal for this crate is to fulfill the role of a translator. It takes a description
//! of a colloscope (or at least the various constraints of a colloscope) and returns
//! an ILP problem as described by the crate [collomatique-ilp].
//!
//! Similarly, it can translate a solution of an ILP problem into the description of
//! an actual colloscope and conversly, it can take the description of a colloscope
//! and turn it into an ILP configuration. This is useful to check in real time if
//! a colloscope satisfies all the constraints and helps when constructing a colloscope
//! incrementally.
//!
//! This crate however does not expose a user-friendly interface. The reason is, to
//! make the translation algorithm as thin as possible, and its verification as easy as
//! possible, I strive to make the colloscopes constraints and the actual colloscopes
//! representation the least redundant possible.
//!
//! Also to keep this part lean, a lot of information is not represented as it is not
//! needed to build the constraint system. For instance, the name of the students or
//! the name of the teachers are not stored in the structures of this modules. Students
//! and teachers are represented with numbers and that's it. It is the job of other crates
//! from collomatique to provide necessary utilities to make working the algorithm
//! somewhat pleasant.
//!
//! The main struct is [ProblemBuilder] and you should start from there to see how this crate
//! works.

pub mod colloscopes;
pub mod time;

use std::collections::BTreeMap;

use collomatique_ilp::{ConfigData, Constraint, LinExpr, ObjectiveSense, UsableData, Variable};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum BaseVariable<M: UsableData, S: UsableData> {
    Main(M),
    Structure(S),
}

impl<M: UsableData + std::fmt::Display, S: UsableData + std::fmt::Display> std::fmt::Display
    for BaseVariable<M, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Main(v) => write!(f, "{}", v),
            Self::Structure(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum ReconstructionDesc<S: UsableData, V: UsableData> {
    Structure(S),
    ValueFixer(V, ordered_float::OrderedFloat<f64>),
}

impl<S: UsableData + std::fmt::Display, V: UsableData + std::fmt::Display> std::fmt::Display
    for ReconstructionDesc<S, V>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Structure(d) => write!(f, "{}", d),
            Self::ValueFixer(var, value) => {
                write!(f, "Fixes variable {} to value {}", var, value.into_inner())
            }
        }
    }
}

fn default_structure_variable_reconstruction<
    V: UsableData,
    S: UsableData,
    C: UsableData,
    F: FnMut(V) -> Option<S>,
>(
    structure_constraints: Vec<(Constraint<V>, C)>,
    structure_variables: BTreeMap<V, Variable>,
    main_values: ConfigData<V>,
    mut f: F,
) -> Option<ConfigData<S>> {
    let ilp_problem: collomatique_ilp::Problem<V, ReconstructionDesc<C, V>> =
        collomatique_ilp::ProblemBuilder::new()
            .set_variables(structure_variables)
            .set_variables(main_values.get_values().into_iter().map(
                |(var, _value)| (var, Variable::continuous()), // We don't care about the type of variable, the value will be fixed
            ))
            .add_constraints(
                structure_constraints
                    .into_iter()
                    .map(|(constraint, desc)| (constraint, ReconstructionDesc::Structure(desc))),
            )
            .add_constraints(main_values.get_values().into_iter().map(|(var, value)| {
                let lhs = LinExpr::var(var.clone());
                let rhs = LinExpr::constant(value);

                (
                    lhs.eq(&rhs),
                    ReconstructionDesc::ValueFixer(var, ordered_float::OrderedFloat(value)),
                )
            }))
            .build()
            .ok()?;

    use collomatique_ilp::solvers::{self, Solver};

    let cbc_solver = solvers::coin_cbc::CbcSolver::new();
    let feasable_config = cbc_solver.solve(&ilp_problem)?;

    let filtered_variables = feasable_config
        .get_values()
        .into_iter()
        .filter_map(|(var, value)| f(var).map(|x| (x, value)));

    let config_data = ConfigData::new().set_iter(filtered_variables);

    Some(config_data)
}

pub trait BaseConstraints: Send + Sync + std::fmt::Debug + PartialEq + Eq {
    type MainVariable: UsableData;
    type StructureVariable: UsableData;
    type StructureConstraintDesc: UsableData;
    type GeneralConstraintDesc: UsableData;
    type Solution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq;

    fn main_variables(&self) -> BTreeMap<Self::MainVariable, Variable>;
    fn structure_variables(&self) -> BTreeMap<Self::StructureVariable, Variable>;

    fn structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )>;
    fn general_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )>;

    fn objective_func(&self) -> LinExpr<BaseVariable<Self::MainVariable, Self::StructureVariable>>;
    fn objective_sense(&self) -> ObjectiveSense {
        ObjectiveSense::Minimize
    }

    fn solution_to_configuration(&self, sol: &Self::Solution) -> ConfigData<Self::MainVariable>;
    fn configuration_to_solution(&self, config: &ConfigData<Self::MainVariable>) -> Self::Solution;

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> Option<ConfigData<Self::StructureVariable>> {
        let structure_constraints = self.structure_constraints();
        let structure_variables = self
            .structure_variables()
            .into_iter()
            .map(|(v_name, v_desc)| (BaseVariable::Structure(v_name), v_desc))
            .collect();
        let main_values = ConfigData::new().set_iter(
            config
                .get_values()
                .into_iter()
                .map(|(var, value)| (BaseVariable::Main(var), value)),
        );

        let f = |v| match v {
            BaseVariable::Main(_) => None,
            BaseVariable::Structure(v) => Some(v),
        };

        default_structure_variable_reconstruction(
            structure_constraints,
            structure_variables,
            main_values,
            f,
        )
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExtraVariable<M: UsableData, S: UsableData, E: UsableData> {
    BaseMain(M),
    BaseStructure(S),
    Extra(E),
}

impl<M: UsableData + std::fmt::Display, S: UsableData + std::fmt::Display, E: UsableData + std::fmt::Display>
    std::fmt::Display for ExtraVariable<M, S, E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BaseMain(v) => write!(f, "{}", v),
            Self::BaseStructure(v) => write!(f, "{}", v),
            Self::Extra(v) => write!(f, "{}", v),
        }
    }
}

pub trait ExtraConstraints<T: BaseConstraints> {
    type StructureVariable: UsableData;
    type StructureConstraintDesc: UsableData;
    type GeneralConstraintDesc: UsableData;

    fn extra_structure_variables(&self, base: &T) -> BTreeMap<Self::StructureVariable, Variable>;
    fn extra_structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )>;

    fn extra_general_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )>;

    fn reconstruct_extra_structure_variables(
        &self,
        base: &T,
        config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> Option<ConfigData<Self::StructureVariable>> {
        let structure_constraints = self.extra_structure_constraints(base);
        let structure_variables = self
            .extra_structure_variables(base)
            .into_iter()
            .map(|(v_name, v_desc)| (ExtraVariable::Extra(v_name), v_desc))
            .collect();
        let main_values =
            ConfigData::new().set_iter(config.get_values().into_iter().map(|(var, value)| {
                (
                    match var {
                        BaseVariable::Main(v) => ExtraVariable::BaseMain(v),
                        BaseVariable::Structure(v) => ExtraVariable::BaseStructure(v),
                    },
                    value,
                )
            }));

        let f = |v| match v {
            ExtraVariable::Extra(v) => Some(v),
            _ => None,
        };

        default_structure_variable_reconstruction(
            structure_constraints,
            structure_variables,
            main_values,
            f,
        )
    }
}

pub trait ExtraObjective<T: BaseConstraints> {
    type StructureVariable: UsableData;
    type StructureConstraintDesc: UsableData;

    fn extra_structure_variables(&self, base: &T) -> BTreeMap<Self::StructureVariable, Variable>;
    fn extra_structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )>;

    fn objective_func(
        &self,
        base: &T,
    ) -> LinExpr<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>;
    fn objective_sense(&self, _base: &T) -> ObjectiveSense {
        ObjectiveSense::Minimize
    }

    fn reconstruct_extra_structure_variables(
        &self,
        base: &T,
        config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> Option<ConfigData<Self::StructureVariable>> {
        let structure_constraints = self.extra_structure_constraints(base);
        let structure_variables = self
            .extra_structure_variables(base)
            .into_iter()
            .map(|(v_name, v_desc)| (ExtraVariable::Extra(v_name), v_desc))
            .collect();
        let main_values =
            ConfigData::new().set_iter(config.get_values().into_iter().map(|(var, value)| {
                (
                    match var {
                        BaseVariable::Main(v) => ExtraVariable::BaseMain(v),
                        BaseVariable::Structure(v) => ExtraVariable::BaseStructure(v),
                    },
                    value,
                )
            }));

        let f = |v| match v {
            ExtraVariable::Extra(v) => Some(v),
            _ => None,
        };

        default_structure_variable_reconstruction(
            structure_constraints,
            structure_variables,
            main_values,
            f,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternalId(u64);

impl std::fmt::Display for InternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct IdIssuer {
    available: u64,
}

impl IdIssuer {
    fn new() -> Self {
        Self::default()
    }

    fn get_id(&mut self) -> InternalId {
        let new_id = InternalId(self.available);
        self.available += 1;
        new_id
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct IdVariable {
    id: InternalId,
    desc: String,
}

impl std::fmt::Display for IdVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.desc)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemBuilder<
    M,
    S,
    T,
    P = collomatique_ilp::DefaultRepr<ExtraVariable<M, S, IdVariable>>,
> where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    base: T,
    id_issuer: IdIssuer,
    phantom_p: std::marker::PhantomData<P>,

    objective_func: LinExpr<ExtraVariable<M, S, IdVariable>>,
    objective_sense: ObjectiveSense,

    variables: BTreeMap<ExtraVariable<M, S, IdVariable>, Variable>,

    general_constraint_descs: BTreeMap<InternalId, T::GeneralConstraintDesc>,
    structure_constraint_descs: BTreeMap<InternalId, T::StructureConstraintDesc>,
    constraints: Vec<(Constraint<ExtraVariable<M, S, IdVariable>>, InternalId)>,
}

impl<M, S, T, P> ProblemBuilder<M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    pub fn new(base: T) -> Option<Self> {
        let orig_main_variables = base.main_variables();
        let orig_structure_variables = base.structure_variables();

        let variables = orig_main_variables
            .into_iter()
            .map(|(v_name, v_desc)| (ExtraVariable::BaseMain(v_name), v_desc))
            .chain(
                orig_structure_variables
                    .into_iter()
                    .map(|(v_name, v_desc)| (ExtraVariable::BaseStructure(v_name), v_desc)),
            )
            .collect::<BTreeMap<_, _>>();

        let original_objective_func = base.objective_func();
        let mut objective_func = LinExpr::constant(original_objective_func.get_constant());
        for (v, value) in original_objective_func.coefficients() {
            let new_v = match v {
                BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
                BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
            };

            if !variables.contains_key(&new_v) {
                return None;
            }
            objective_func = objective_func + value * LinExpr::var(new_v);
        }

        let objective_sense = base.objective_sense();

        let mut id_issuer = IdIssuer::new();

        let mut general_constraint_descs = BTreeMap::new();
        let mut structure_constraint_descs = BTreeMap::new();
        let mut constraints = Vec::new();

        for (orig_constraint, c_desc) in base.structure_constraints() {
            let mut expr = LinExpr::constant(orig_constraint.get_constant());
            for (v, value) in orig_constraint.coefficients() {
                let new_v = match v {
                    BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
                    BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
                };

                if !variables.contains_key(&new_v) {
                    return None;
                }

                expr = expr + value * LinExpr::var(new_v);
            }

            let constraint = match orig_constraint.get_symbol() {
                collomatique_ilp::linexpr::EqSymbol::Equals => expr.eq(&LinExpr::constant(0.)),
                collomatique_ilp::linexpr::EqSymbol::LessThan => expr.leq(&LinExpr::constant(0.)),
            };

            let desc_id = id_issuer.get_id();

            constraints.push((constraint, desc_id));
            structure_constraint_descs.insert(desc_id, c_desc);
        }

        for (orig_constraint, c_desc) in base.general_constraints() {
            let mut expr = LinExpr::constant(orig_constraint.get_constant());
            for (v, value) in orig_constraint.coefficients() {
                let new_v = match v {
                    BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
                    BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
                };

                if !variables.contains_key(&new_v) {
                    return None;
                }

                expr = expr + value * LinExpr::var(new_v);
            }

            let constraint = match orig_constraint.get_symbol() {
                collomatique_ilp::linexpr::EqSymbol::Equals => expr.eq(&LinExpr::constant(0.)),
                collomatique_ilp::linexpr::EqSymbol::LessThan => expr.leq(&LinExpr::constant(0.)),
            };

            let desc_id = id_issuer.get_id();

            constraints.push((constraint, desc_id));
            general_constraint_descs.insert(desc_id, c_desc);
        }

        Some(ProblemBuilder {
            base,
            id_issuer,
            phantom_p: std::marker::PhantomData,
            objective_func,
            objective_sense,
            variables,
            general_constraint_descs,
            structure_constraint_descs,
            constraints,
        })
    }

    fn scan_variables<U: UsableData>(
        &mut self,
        variables: BTreeMap<U, Variable>,
    ) -> (
        BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
        BTreeMap<ExtraVariable<M, S, IdVariable>, Variable>,
    ) {
        let mut rev_v_map = BTreeMap::new();
        let mut v_map = BTreeMap::new();

        for (v, v_desc) in variables {
            let v_id = self.id_issuer.get_id();
            let v_name = ExtraVariable::Extra(IdVariable {
                id: v_id,
                desc: format!("{:?}", v),
            });

            v_map.insert(v_name.clone(), v_desc);
            rev_v_map.insert(v, v_name);
        }

        (rev_v_map, v_map)
    }

    fn add_variables(&mut self, mut v_map: BTreeMap<ExtraVariable<M, S, IdVariable>, Variable>) {
        self.variables.append(&mut v_map);
    }

    fn check_variables_in_expr<U: UsableData>(
        &self,
        expr: &LinExpr<ExtraVariable<M, S, U>>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> bool {
        for (v, _value) in expr.coefficients() {
            if let ExtraVariable::Extra(v_extra) = v {
                if !rev_v_map.contains_key(v_extra) {
                    return false;
                }
            }
        }

        true
    }

    fn check_variables_in_constraints<U: UsableData, C: UsableData>(
        &self,
        constraints: &Vec<(Constraint<ExtraVariable<M, S, U>>, C)>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> bool {
        for (c, _c_desc) in constraints {
            if !self.check_variables_in_expr(c.get_lhs(), rev_v_map) {
                return false;
            }
        }

        true
    }

    fn update_var_in_expr<U: UsableData>(
        &self,
        e: &LinExpr<ExtraVariable<M, S, U>>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> LinExpr<ExtraVariable<M, S, IdVariable>> {
        let mut expr = LinExpr::constant(e.get_constant());

        for (v, value) in e.coefficients() {
            let var = match v {
                ExtraVariable::BaseMain(v_main) => ExtraVariable::BaseMain(v_main.clone()),
                ExtraVariable::BaseStructure(v_struct) => ExtraVariable::BaseStructure(v_struct.clone()),
                ExtraVariable::Extra(v_extra) => rev_v_map.get(v_extra)
                    .expect("consistency between variables and constraints should be checked beforehand")
                    .clone(),
            };
            expr = expr + value * LinExpr::var(var);
        }

        expr
    }

    fn add_constraints<U: UsableData, C: UsableData>(
        &mut self,
        constraints: Vec<(Constraint<ExtraVariable<M, S, U>>, C)>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> BTreeMap<InternalId, C> {
        let mut c_map = BTreeMap::new();

        for (c, c_desc) in constraints {
            let expr = self.update_var_in_expr(c.get_lhs(), rev_v_map);

            let constraint = match c.get_symbol() {
                collomatique_ilp::linexpr::EqSymbol::Equals => expr.eq(&LinExpr::constant(0.)),
                collomatique_ilp::linexpr::EqSymbol::LessThan => expr.leq(&LinExpr::constant(0.)),
            };

            let c_id = self.id_issuer.get_id();
            self.constraints.push((constraint, c_id));
            c_map.insert(c_id, c_desc);
        }

        c_map
    }

    pub fn add_hard_constraints<E: ExtraConstraints<T>>(
        &mut self,
        extra: E,
    ) -> Option<ConstraintsTranslator<T, E>> {
        let extra_variables = extra.extra_structure_variables(&self.base);
        let extra_structure_constraints = extra.extra_structure_constraints(&self.base);
        let extra_general_constraints = extra.extra_general_constraints(&self.base);

        let (rev_v_map, v_map) = self.scan_variables(extra_variables);

        if !self.check_variables_in_constraints(&extra_structure_constraints, &rev_v_map)
            || !self.check_variables_in_constraints(&extra_general_constraints, &rev_v_map)
        {
            return None;
        }

        self.add_variables(v_map);
        let general_c_map = self.add_constraints(extra_general_constraints, &rev_v_map);
        let structure_c_map = self.add_constraints(extra_structure_constraints, &rev_v_map);

        Some(ConstraintsTranslator {
            extra,
            general_c_map,
            structure_c_map,
        })
    }

    fn convert_constraints_to_soft<U: UsableData, C: UsableData>(
        &mut self,
        constraints: Vec<(Constraint<ExtraVariable<M, S, U>>, C)>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> LinExpr<ExtraVariable<M, S, IdVariable>> {
        let mut obj = LinExpr::constant(0.);

        for (c, c_desc) in constraints {
            let expr = self.update_var_in_expr(c.get_lhs(), rev_v_map);

            let soft_variable_id = self.id_issuer.get_id();
            let soft_variable = ExtraVariable::Extra(IdVariable {
                id: soft_variable_id,
                desc: format!("soft_{} ({:?})", soft_variable_id, c_desc),
            });

            self.variables
                .insert(soft_variable.clone(), Variable::non_negative());

            match c.get_symbol() {
                collomatique_ilp::linexpr::EqSymbol::Equals => {
                    let soft_constraint1 = expr.leq(&LinExpr::var(soft_variable.clone()));
                    let soft_constraint1_id = self.id_issuer.get_id(); // We'll loose this as this constraint always has a solution

                    let soft_constraint2 = expr.geq(&(-LinExpr::var(soft_variable.clone())));
                    let soft_constraint2_id = self.id_issuer.get_id(); // We'll loose this as this constraint always has a solution

                    self.constraints
                        .push((soft_constraint1, soft_constraint1_id));
                    self.constraints
                        .push((soft_constraint2, soft_constraint2_id));
                }
                collomatique_ilp::linexpr::EqSymbol::LessThan => {
                    let soft_constraint = expr.leq(&LinExpr::var(soft_variable.clone()));
                    let soft_constraint_id = self.id_issuer.get_id(); // We'll loose this as this constraint always has a solution

                    self.constraints.push((soft_constraint, soft_constraint_id));
                }
            }

            obj = obj + LinExpr::var(soft_variable);
        }

        obj
    }

    pub fn add_soft_constraints<E: ExtraConstraints<T>>(
        &mut self,
        extra: E,
        obj_coef: f64,
    ) -> Option<ConstraintsTranslator<T, E>> {
        let extra_variables = extra.extra_structure_variables(&self.base);
        let extra_structure_constraints = extra.extra_structure_constraints(&self.base);
        let extra_general_constraints = extra.extra_general_constraints(&self.base);

        let (rev_v_map, v_map) = self.scan_variables(extra_variables);

        if !self.check_variables_in_constraints(&extra_structure_constraints, &rev_v_map)
            || !self.check_variables_in_constraints(&extra_general_constraints, &rev_v_map)
        {
            return None;
        }

        self.add_variables(v_map);
        let structure_c_map = self.add_constraints(extra_structure_constraints, &rev_v_map);

        let obj_func = self.convert_constraints_to_soft(extra_general_constraints, &rev_v_map);
        match self.objective_sense {
            ObjectiveSense::Minimize => {
                self.objective_func = &self.objective_func + obj_coef * obj_func;
            }
            ObjectiveSense::Maximize => {
                self.objective_func = &self.objective_func - obj_coef * obj_func;
            }
        }

        let general_c_map = BTreeMap::new();
        Some(ConstraintsTranslator {
            extra,
            general_c_map,
            structure_c_map,
        })
    }

    pub fn add_objective<E: ExtraObjective<T>>(
        &mut self,
        extra: E,
        obj_coef: f64,
    ) -> Option<ObjectiveTranslator<T, E>> {
        let extra_structure_variables = extra.extra_structure_variables(&self.base);
        let extra_structure_constraints = extra.extra_structure_constraints(&self.base);
        let objective_func = extra.objective_func(&self.base);

        let (rev_v_map, v_map) = self.scan_variables(extra_structure_variables);

        if !self.check_variables_in_constraints(&extra_structure_constraints, &rev_v_map)
            || !self.check_variables_in_expr(&objective_func, &rev_v_map)
        {
            return None;
        }

        self.add_variables(v_map);
        let structure_c_map = self.add_constraints(extra_structure_constraints, &rev_v_map);

        let obj_func = self.update_var_in_expr(&objective_func, &rev_v_map);
        if self.objective_sense == extra.objective_sense(&self.base) {
            self.objective_func = &self.objective_func + obj_coef * obj_func;
        } else {
            self.objective_func = &self.objective_func - obj_coef * obj_func;
        }

        Some(ObjectiveTranslator {
            extra,
            structure_c_map,
        })
    }

    pub fn build(self) -> Problem<M, S, T, P> {
        let ilp_problem = collomatique_ilp::ProblemBuilder::new()
            .set_variables(self.variables)
            .add_constraints(self.constraints)
            .set_objective_function(self.objective_func, self.objective_sense)
            .build()
            .expect("Variables good definition should have already been checked");

        Problem {
            ilp_problem,
            base: self.base,
            general_constraint_descs: self.general_constraint_descs,
            structure_constraint_descs: self.structure_constraint_descs,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintsTranslator<T: BaseConstraints, E: ExtraConstraints<T>> {
    extra: E,
    general_c_map: BTreeMap<InternalId, E::GeneralConstraintDesc>,
    structure_c_map: BTreeMap<InternalId, E::StructureConstraintDesc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveTranslator<T: BaseConstraints, E: ExtraObjective<T>> {
    extra: E,
    structure_c_map: BTreeMap<InternalId, E::StructureConstraintDesc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem<M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    ilp_problem: collomatique_ilp::Problem<ExtraVariable<M, S, IdVariable>, InternalId, P>,
    base: T,
    general_constraint_descs: BTreeMap<InternalId, T::GeneralConstraintDesc>,
    structure_constraint_descs: BTreeMap<InternalId, T::StructureConstraintDesc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecoratedSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    problem: &'a Problem<M, S, T, P>,
    internal_solution: T::Solution,
    ilp_config: collomatique_ilp::Config<'a, ExtraVariable<M, S, IdVariable>, InternalId, P>,
}

impl<'a, M, S, T, P> DecoratedSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    pub fn inner(&self) -> &T::Solution {
        &self.internal_solution
    }

    pub fn into_innter(self) -> T::Solution {
        self.internal_solution
    }

    pub fn blame(&self) -> impl ExactSizeIterator<Item = &T::GeneralConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }

    pub fn blame_with_extra_constraint<'b, E: ExtraConstraints<T>>(
        &self,
        _translator: &'b ConstraintsTranslator<T, E>,
    ) -> impl ExactSizeIterator<Item = &'b E::GeneralConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }

    pub fn check_structure(&self) -> impl ExactSizeIterator<Item = &T::StructureConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }

    pub fn check_structure_with_extra_constraint<'b, E: ExtraConstraints<T>>(
        &self,
        _translator: &'b ConstraintsTranslator<T, E>,
    ) -> impl ExactSizeIterator<Item = &'b E::StructureConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }

    pub fn check_structure_with_extra_objective<'b, E: ExtraObjective<T>>(
        &self,
        _translator: &'b ObjectiveTranslator<T, E>,
    ) -> impl ExactSizeIterator<Item = &'b E::StructureConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeLimitSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    pub solution: DecoratedSolution<'a, M, S, T, P>,
    pub time_limit_reached: bool,
}

impl<M, S, T, P> Problem<M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    fn feasable_config_into_config_data(
        &self,
        feasable_config: &collomatique_ilp::FeasableConfig<
            '_,
            ExtraVariable<M, S, IdVariable>,
            InternalId,
            P,
        >,
    ) -> ConfigData<M> {
        let mut config_data = ConfigData::new();

        for (var, value) in feasable_config.get_values() {
            if let ExtraVariable::BaseMain(v_name) = var {
                config_data = config_data.set(v_name, value);
            }
        }

        config_data
    }

    pub fn solve<
        'a,
        Solver: collomatique_ilp::solvers::Solver<ExtraVariable<M, S, IdVariable>, InternalId, P>,
    >(
        &'a self,
        solver: &Solver,
    ) -> Option<DecoratedSolution<'a, M, S, T, P>> {
        let feasable_config = solver.solve(&self.ilp_problem)?;
        let internal_solution = self
            .base
            .configuration_to_solution(&self.feasable_config_into_config_data(&feasable_config));

        Some(DecoratedSolution {
            problem: self,
            internal_solution,
            ilp_config: feasable_config.into_inner(),
        })
    }

    pub fn solve_with_time_limit<
        'a,
        Solver: collomatique_ilp::solvers::SolverWithTimeLimit<
            ExtraVariable<M, S, IdVariable>,
            InternalId,
            P,
        >,
    >(
        &'a self,
        solver: &Solver,
        time_limit_in_seconds: u32,
    ) -> Option<TimeLimitSolution<'a, M, S, T, P>> {
        let time_limit_sol =
            solver.solve_with_time_limit(&self.ilp_problem, time_limit_in_seconds)?;
        let internal_solution = self.base.configuration_to_solution(
            &self.feasable_config_into_config_data(&time_limit_sol.config),
        );

        Some(TimeLimitSolution {
            solution: DecoratedSolution {
                problem: self,
                internal_solution,
                ilp_config: time_limit_sol.config.into_inner(),
            },
            time_limit_reached: time_limit_sol.time_limit_reached,
        })
    }
}
