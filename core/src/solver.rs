use super::*;

use std::collections::BTreeMap;

use collomatique_ilp::{ConfigData, Constraint, LinExpr, Objective, UsableData, Variable};

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

    objective: Objective<ExtraVariable<M, S, IdVariable>>,

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

        let objective = base.objective().transmute(|v| match v {
            BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
            BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
        });
        for v in objective.get_function().variables() {
            if !variables.contains_key(&v) {
                return None;
            }
        }

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
            objective,
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

    fn update_var<U: UsableData>(
        &self,
        v: &ExtraVariable<M, S, U>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> ExtraVariable<M, S, IdVariable> {
        match v {
            ExtraVariable::BaseMain(v_main) => ExtraVariable::BaseMain(v_main.clone()),
            ExtraVariable::BaseStructure(v_struct) => {
                ExtraVariable::BaseStructure(v_struct.clone())
            }
            ExtraVariable::Extra(v_extra) => rev_v_map
                .get(v_extra)
                .expect(
                    "consistency between variables and constraints should be checked beforehand",
                )
                .clone(),
        }
    }

    fn add_constraints_internal<U: UsableData, C: UsableData>(
        &mut self,
        constraints: Vec<(Constraint<ExtraVariable<M, S, U>>, C)>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> BTreeMap<InternalId, C> {
        let mut c_map = BTreeMap::new();

        for (c, c_desc) in constraints {
            let expr = c.get_lhs().transmute(|x| self.update_var(x, rev_v_map));

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

    pub fn add_constraints<E: ExtraConstraints<T>>(
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
        let general_c_map = self.add_constraints_internal(extra_general_constraints, &rev_v_map);
        let structure_c_map =
            self.add_constraints_internal(extra_structure_constraints, &rev_v_map);

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
        let objective = extra.extra_objective(&self.base);

        let (rev_v_map, v_map) = self.scan_variables(extra_structure_variables);

        if !self.check_variables_in_constraints(&extra_structure_constraints, &rev_v_map)
            || !self.check_variables_in_expr(objective.get_function(), &rev_v_map)
        {
            return None;
        }

        self.add_variables(v_map);
        let structure_c_map =
            self.add_constraints_internal(extra_structure_constraints, &rev_v_map);

        let new_obj = objective.transmute(|x| self.update_var(x, &rev_v_map));
        self.objective = &self.objective + obj_coef * new_obj;

        Some(ObjectiveTranslator {
            extra,
            structure_c_map,
        })
    }

    pub fn build(self) -> Problem<M, S, T, P> {
        let ilp_problem = collomatique_ilp::ProblemBuilder::new()
            .set_variables(self.variables)
            .add_constraints(self.constraints)
            .set_objective(self.objective)
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
