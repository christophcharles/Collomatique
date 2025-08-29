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

use std::collections::{BTreeMap, BTreeSet};

use collomatique_ilp::{ConfigData, Constraint, LinExpr, ObjectiveSense, UsableData, Variable};

pub trait BaseConstraints: Send + Sync + std::fmt::Debug + PartialEq + Eq {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;
    type Solution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq;

    fn variables(&self) -> Vec<(Self::VariableName, Variable)>;
    fn constraints(&self) -> Vec<(Constraint<Self::VariableName>, Self::ConstraintDesc)>;

    fn objective_func(&self) -> LinExpr<Self::VariableName>;
    fn objective_sense(&self) -> ObjectiveSense {
        ObjectiveSense::Minimize
    }

    fn solution_to_configuration(&self, sol: &Self::Solution) -> ConfigData<Self::VariableName>;
    fn configuration_to_solution(&self, config: &ConfigData<Self::VariableName>) -> Self::Solution;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExtraVariable<B: UsableData, E: UsableData> {
    Base(B),
    Extra(E),
}

impl<B: UsableData, E: UsableData> std::fmt::Display for ExtraVariable<B, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(v) => write!(f, "{}", v),
            Self::Extra(v) => write!(f, "{}", v),
        }
    }
}

pub trait ExtraConstraints<T: BaseConstraints> {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;

    fn extra_variables(&self, base: &T) -> Vec<(Self::VariableName, Variable)>;
    fn structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
    fn extra_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
}

pub trait ExtraObjective<T: BaseConstraints> {
    type VariableName: UsableData;
    type ConstraintDesc: UsableData;

    fn extra_variables(&self, base: &T) -> Vec<(Self::VariableName, Variable)>;
    fn structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::VariableName, Self::VariableName>>,
        Self::ConstraintDesc,
    )>;
    fn objective_func(
        &self,
        base: &T,
    ) -> LinExpr<ExtraVariable<T::VariableName, Self::VariableName>>;
    fn objective_sense(&self, _base: &T) -> ObjectiveSense {
        ObjectiveSense::Minimize
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
pub enum VariableName<V: UsableData> {
    Base(V),
    Extra(InternalId, String),
    Soft(InternalId, String),
}

impl<V: UsableData> std::fmt::Display for VariableName<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(v) => write!(f, "{}", v),
            Self::Extra(_id, desc) => write!(f, "{}", desc),
            Self::Soft(id, desc) => write!(f, "soft_{} ({})", id, desc),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemBuilder<V, T, P = collomatique_ilp::DefaultRepr<VariableName<V>>>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    base: T,
    id_issuer: IdIssuer,
    phantom_p: std::marker::PhantomData<P>,

    objective_func: LinExpr<VariableName<V>>,
    objective_sense: ObjectiveSense,

    variables: Vec<(VariableName<V>, Variable)>,

    constraint_descs: BTreeMap<InternalId, T::ConstraintDesc>,
    constraints: Vec<(Constraint<VariableName<V>>, InternalId)>,
}

impl<V, T, P> ProblemBuilder<V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub fn new(base: T) -> Option<Self> {
        let orig_variables = base.variables();
        let variable_set = orig_variables
            .iter()
            .map(|(v_name, _v_desc)| v_name.clone())
            .collect::<BTreeSet<_>>();
        let variables = orig_variables
            .into_iter()
            .map(|(v_name, v_desc)| (VariableName::Base(v_name), v_desc))
            .collect();

        let original_objective_func = base.objective_func();
        let mut objective_func = LinExpr::constant(original_objective_func.get_constant());
        for (v, value) in original_objective_func.coefficients() {
            if !variable_set.contains(v) {
                return None;
            }
            objective_func = objective_func + value * LinExpr::var(VariableName::Base(v.clone()));
        }

        let objective_sense = base.objective_sense();

        let mut id_issuer = IdIssuer::new();

        let mut constraint_descs = BTreeMap::new();
        let mut constraints = Vec::new();

        for (orig_constraint, c_desc) in base.constraints() {
            let mut expr = LinExpr::constant(orig_constraint.get_constant());
            for (v, value) in orig_constraint.coefficients() {
                if !variable_set.contains(v) {
                    return None;
                }

                expr = expr + value * LinExpr::var(VariableName::Base(v.clone()));
            }

            let constraint = match orig_constraint.get_symbol() {
                collomatique_ilp::linexpr::EqSymbol::Equals => expr.eq(&LinExpr::constant(0.)),
                collomatique_ilp::linexpr::EqSymbol::LessThan => expr.leq(&LinExpr::constant(0.)),
            };

            let desc_id = id_issuer.get_id();

            constraints.push((constraint, desc_id));
            constraint_descs.insert(desc_id, c_desc);
        }

        Some(ProblemBuilder {
            base,
            id_issuer,
            phantom_p: std::marker::PhantomData,
            objective_func,
            objective_sense,
            variables,
            constraint_descs,
            constraints,
        })
    }

    fn scan_variables<U: UsableData>(
        &mut self,
        variables: Vec<(U, Variable)>,
    ) -> (
        BTreeMap<U, VariableName<V>>,
        Vec<(VariableName<V>, Variable)>,
    ) {
        let mut v_map = BTreeMap::new();
        let mut vars = Vec::new();

        for (v, v_desc) in variables {
            let v_id = self.id_issuer.get_id();
            let v_name = VariableName::Extra(v_id, format!("{}", v));

            vars.push((v_name.clone(), v_desc));
            v_map.insert(v, v_name);
        }

        (v_map, vars)
    }

    fn add_variables(&mut self, vars: Vec<(VariableName<V>, Variable)>) {
        self.variables.extend(vars);
    }

    fn check_variables_expr<U: UsableData>(
        &self,
        expr: &LinExpr<ExtraVariable<V, U>>,
        v_map: &BTreeMap<U, VariableName<V>>,
    ) -> bool {
        for (v, _value) in expr.coefficients() {
            if let ExtraVariable::Extra(v_extra) = v {
                if !v_map.contains_key(v_extra) {
                    return false;
                }
            }
        }

        true
    }

    fn check_variables<U: UsableData, C: UsableData>(
        &self,
        constraints: &Vec<(Constraint<ExtraVariable<V, U>>, C)>,
        v_map: &BTreeMap<U, VariableName<V>>,
    ) -> bool {
        for (c, _c_desc) in constraints {
            if !self.check_variables_expr(c.get_lhs(), v_map) {
                return false;
            }
        }

        true
    }

    fn update_var_in_expr<U: UsableData>(
        &self,
        e: &LinExpr<ExtraVariable<V, U>>,
        v_map: &BTreeMap<U, VariableName<V>>,
    ) -> LinExpr<VariableName<V>> {
        let mut expr = LinExpr::constant(e.get_constant());

        for (v, value) in e.coefficients() {
            let var = match v {
                ExtraVariable::Base(v_base) => VariableName::Base(v_base.clone()),
                ExtraVariable::Extra(v_extra) => v_map.get(v_extra)
                    .expect("consistency between variables and constraints should be checked beforehand")
                    .clone(),
            };
            expr = expr + value * LinExpr::var(var);
        }

        expr
    }

    fn add_constraints<U: UsableData, C: UsableData>(
        &mut self,
        constraints: Vec<(Constraint<ExtraVariable<V, U>>, C)>,
        v_map: &BTreeMap<U, VariableName<V>>,
    ) -> BTreeMap<InternalId, C> {
        let mut c_map = BTreeMap::new();

        for (c, c_desc) in constraints {
            let expr = self.update_var_in_expr(c.get_lhs(), v_map);

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
    ) -> Option<ConstraintsTranslator<E::ConstraintDesc>> {
        let extra_variables = extra.extra_variables(&self.base);
        let structure_constraints = extra.structure_constraints(&self.base);
        let extra_constraints = extra.extra_constraints(&self.base);

        let (v_map, vars) = self.scan_variables(extra_variables);

        if !self.check_variables(&structure_constraints, &v_map)
            || !self.check_variables(&extra_constraints, &v_map)
        {
            return None;
        }

        self.add_variables(vars);
        let mut c_map = self.add_constraints(structure_constraints, &v_map);
        c_map.extend(self.add_constraints(extra_constraints, &v_map));

        Some(ConstraintsTranslator { c_map })
    }

    fn convert_constraints_to_soft<U: UsableData, C: UsableData>(
        &mut self,
        constraints: Vec<(Constraint<ExtraVariable<V, U>>, C)>,
        v_map: &BTreeMap<U, VariableName<V>>,
    ) -> LinExpr<VariableName<V>> {
        let mut obj = LinExpr::constant(0.);

        for (c, c_desc) in constraints {
            let expr = self.update_var_in_expr(c.get_lhs(), v_map);

            let soft_variable_id = self.id_issuer.get_id();
            let soft_variable = VariableName::Soft(soft_variable_id, format!("{}", c_desc));

            self.variables
                .push((soft_variable.clone(), Variable::non_negative()));

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
    ) -> Option<ConstraintsTranslator<E::ConstraintDesc>> {
        let extra_variables = extra.extra_variables(&self.base);
        let structure_constraints = extra.structure_constraints(&self.base);
        let extra_constraints = extra.extra_constraints(&self.base);

        let (v_map, vars) = self.scan_variables(extra_variables);

        if !self.check_variables(&structure_constraints, &v_map)
            || !self.check_variables(&extra_constraints, &v_map)
        {
            return None;
        }

        self.add_variables(vars);
        let c_map = self.add_constraints(structure_constraints, &v_map);

        let obj_func = self.convert_constraints_to_soft(extra_constraints, &v_map);
        match self.objective_sense {
            ObjectiveSense::Minimize => {
                self.objective_func = &self.objective_func + obj_coef * obj_func;
            }
            ObjectiveSense::Maximize => {
                self.objective_func = &self.objective_func - obj_coef * obj_func;
            }
        }

        Some(ConstraintsTranslator { c_map })
    }

    pub fn add_objective<E: ExtraObjective<T>>(
        &mut self,
        extra: E,
        obj_coef: f64,
    ) -> Option<ConstraintsTranslator<E::ConstraintDesc>> {
        let extra_variables = extra.extra_variables(&self.base);
        let structure_constraints = extra.structure_constraints(&self.base);
        let objective_func = extra.objective_func(&self.base);

        let (v_map, vars) = self.scan_variables(extra_variables);

        if !self.check_variables(&structure_constraints, &v_map)
            || !self.check_variables_expr(&objective_func, &v_map)
        {
            return None;
        }

        self.add_variables(vars);
        let c_map = self.add_constraints(structure_constraints, &v_map);

        let obj_func = self.update_var_in_expr(&objective_func, &v_map);
        if self.objective_sense == extra.objective_sense(&self.base) {
            self.objective_func = &self.objective_func + obj_coef * obj_func;
        } else {
            self.objective_func = &self.objective_func - obj_coef * obj_func;
        }

        Some(ConstraintsTranslator { c_map })
    }

    pub fn build(self) -> Problem<V, T, P> {
        let ilp_problem = collomatique_ilp::ProblemBuilder::new()
            .set_variables(self.variables)
            .add_constraints(self.constraints)
            .set_objective_function(self.objective_func, self.objective_sense)
            .build()
            .expect("Variables good definition should have already been checked");

        Problem {
            ilp_problem,
            base: self.base,
            constraint_descs: self.constraint_descs,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintsTranslator<C: UsableData> {
    c_map: BTreeMap<InternalId, C>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem<V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    ilp_problem: collomatique_ilp::Problem<VariableName<V>, InternalId, P>,
    base: T,
    constraint_descs: BTreeMap<InternalId, T::ConstraintDesc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecoratedSolution<'a, V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    problem: &'a Problem<V, T, P>,
    internal_solution: T::Solution,
    ilp_config: collomatique_ilp::Config<'a, VariableName<V>, InternalId, P>,
}

impl<'a, V, T, P> DecoratedSolution<'a, V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub fn inner(&self) -> &T::Solution {
        &self.internal_solution
    }

    pub fn into_innter(self) -> T::Solution {
        self.internal_solution
    }

    pub fn blame(&self) -> impl ExactSizeIterator<Item = &T::ConstraintDesc> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }

    pub fn blame_with_translator<'b, C: UsableData>(
        &self,
        _translator: &'b ConstraintsTranslator<C>,
    ) -> impl ExactSizeIterator<Item = &'b C> {
        if false {
            return vec![].into_iter();
        }
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeLimitSolution<'a, V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    pub solution: DecoratedSolution<'a, V, T, P>,
    pub time_limit_reached: bool,
}

impl<V, T, P> Problem<V, T, P>
where
    V: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<VariableName<V>>,
    T: BaseConstraints<VariableName = V>,
{
    fn feasable_config_into_config_data(
        &self,
        feasable_config: &collomatique_ilp::FeasableConfig<'_, VariableName<V>, InternalId, P>,
    ) -> ConfigData<V> {
        let mut config_data = ConfigData::new();

        for (var, value) in feasable_config.get_values() {
            if let VariableName::Base(v_name) = var {
                config_data = config_data.set(v_name, value);
            }
        }

        config_data
    }

    pub fn solve<
        'a,
        S: collomatique_ilp::solvers::Solver<VariableName<T::VariableName>, InternalId, P>,
    >(
        &'a self,
        solver: &S,
    ) -> Option<DecoratedSolution<'a, V, T, P>> {
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
        S: collomatique_ilp::solvers::SolverWithTimeLimit<
            VariableName<T::VariableName>,
            InternalId,
            P,
        >,
    >(
        &'a self,
        solver: &S,
        time_limit_in_seconds: u32,
    ) -> Option<TimeLimitSolution<'a, V, T, P>> {
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
