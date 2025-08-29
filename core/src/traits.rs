use collomatique_ilp::{ConfigData, Constraint, LinExpr, ObjectiveSense, UsableData, Variable};
use std::collections::BTreeMap;

pub trait PartialSolution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq {
    fn is_complete(&self) -> bool;
}

pub struct CompleteSolution<T: PartialSolution>(T);

impl<T: PartialSolution> CompleteSolution<T> {
    pub fn new(sol: T) -> Option<Self> {
        if !sol.is_complete() {
            return None;
        }

        Some(CompleteSolution(sol))
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: PartialSolution> std::ops::Deref for CompleteSolution<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

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
) -> ConfigData<S> {
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
            .expect("Variables and constraints should match");

    use collomatique_ilp::solvers::{self, Solver};

    let cbc_solver = solvers::coin_cbc::CbcSolver::new();
    let feasable_config = cbc_solver
        .solve(&ilp_problem)
        .expect("There should always be a solution for reconstructing structure variables");

    let filtered_variables = feasable_config
        .get_values()
        .into_iter()
        .filter_map(|(var, value)| f(var).map(|x| (x, value)));

    ConfigData::new().set_iter(filtered_variables)
}

pub trait BaseConstraints: Send + Sync + std::fmt::Debug + PartialEq + Eq {
    type MainVariable: UsableData;
    type StructureVariable: UsableData;
    type StructureConstraintDesc: UsableData;
    type GeneralConstraintDesc: UsableData;
    type Solution: PartialSolution;

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

    fn objective(
        &self,
    ) -> (
        LinExpr<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        ObjectiveSense,
    ) {
        (LinExpr::constant(0.), ObjectiveSense::Minimize)
    }

    fn solution_to_configuration(&self, sol: &Self::Solution) -> ConfigData<Self::MainVariable>;
    fn configuration_to_solution(&self, config: &ConfigData<Self::MainVariable>) -> Self::Solution;

    fn reconstruct_structure_variables(
        &self,
        config: &CompleteSolution<Self::Solution>,
    ) -> ConfigData<Self::StructureVariable> {
        let config_data = self.solution_to_configuration(config.inner());

        let structure_constraints = self.structure_constraints();
        let structure_variables = self
            .structure_variables()
            .into_iter()
            .map(|(v_name, v_desc)| (BaseVariable::Structure(v_name), v_desc))
            .collect();
        let main_values = ConfigData::new().set_iter(
            config_data
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

impl<
        M: UsableData + std::fmt::Display,
        S: UsableData + std::fmt::Display,
        E: UsableData + std::fmt::Display,
    > std::fmt::Display for ExtraVariable<M, S, E>
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
    ) -> ConfigData<Self::StructureVariable> {
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

    fn extra_objective(
        &self,
        base: &T,
    ) -> (
        LinExpr<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        ObjectiveSense,
    );

    fn reconstruct_extra_structure_variables(
        &self,
        base: &T,
        config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> ConfigData<Self::StructureVariable> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftConstraints<T: BaseConstraints, E: ExtraConstraints<T>> {
    internal_extra: E,
    phantom: std::marker::PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoftVariable<S: UsableData, C: UsableData> {
    Orig(S),
    Soft(usize, C),
}

impl<S: UsableData + std::fmt::Display, C: UsableData + std::fmt::Display> std::fmt::Display
    for SoftVariable<S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftVariable::Orig(x) => write!(f, "{}", x),
            SoftVariable::Soft(i, d) => write!(f, "soft_{} ({})", i, d),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoftConstraint<S: UsableData, C: UsableData> {
    Orig(S),
    Soft(usize, C, bool),
}

impl<S: UsableData + std::fmt::Display, C: UsableData + std::fmt::Display> std::fmt::Display
    for SoftConstraint<S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftConstraint::Orig(x) => write!(f, "{}", x),
            SoftConstraint::Soft(i, d, geq) => {
                write!(f, "Soft constraint n°{} for {} (geq = {})", i, d, geq)
            }
        }
    }
}

impl<T: BaseConstraints, E: ExtraConstraints<T>> ExtraObjective<T> for SoftConstraints<T, E> {
    type StructureConstraintDesc =
        SoftConstraint<E::StructureConstraintDesc, E::GeneralConstraintDesc>;
    type StructureVariable = SoftVariable<E::StructureVariable, E::GeneralConstraintDesc>;

    fn extra_structure_variables(&self, base: &T) -> BTreeMap<Self::StructureVariable, Variable> {
        self.internal_extra
            .extra_structure_variables(base)
            .into_iter()
            .map(|(x, v)| (SoftVariable::Orig(x), v))
            .chain(
                self.internal_extra
                    .extra_general_constraints(base)
                    .into_iter()
                    .enumerate()
                    .map(|(i, (_c, desc))| (SoftVariable::Soft(i, desc), Variable::non_negative())),
            )
            .collect()
    }

    fn extra_structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )> {
        self.internal_extra
            .extra_structure_constraints(base)
            .into_iter()
            .map(|(c, desc)| {
                (
                    c.transmute(|x| match x {
                        ExtraVariable::BaseMain(m) => ExtraVariable::BaseMain(m.clone()),
                        ExtraVariable::BaseStructure(s) => ExtraVariable::BaseStructure(s.clone()),
                        ExtraVariable::Extra(e) => {
                            ExtraVariable::Extra(SoftVariable::Orig(e.clone()))
                        }
                    }),
                    SoftConstraint::Orig(desc),
                )
            })
            .chain(
                self.internal_extra
                    .extra_general_constraints(base)
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, (c, desc))| {
                        let expr = c.get_lhs().transmute(|x| match x {
                            ExtraVariable::BaseMain(m) => ExtraVariable::BaseMain(m.clone()),
                            ExtraVariable::BaseStructure(s) => {
                                ExtraVariable::BaseStructure(s.clone())
                            }
                            ExtraVariable::Extra(e) => {
                                ExtraVariable::Extra(SoftVariable::Orig(e.clone()))
                            }
                        });
                        let v = ExtraVariable::Extra(SoftVariable::Soft(i, desc.clone()));

                        let mut output = Vec::new();
                        output.push((
                            expr.leq(&LinExpr::var(v.clone())),
                            SoftConstraint::Soft(i, desc.clone(), false),
                        ));

                        if c.get_symbol() == collomatique_ilp::linexpr::EqSymbol::Equals {
                            output.push((
                                expr.geq(&(-LinExpr::var(v))),
                                SoftConstraint::Soft(i, desc, true),
                            ));
                        }

                        output
                    }),
            )
            .collect()
    }

    fn extra_objective(
        &self,
        base: &T,
    ) -> (
        LinExpr<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        ObjectiveSense,
    ) {
        let mut new_obj = LinExpr::constant(0.0);

        for (i, (_c, desc)) in self
            .internal_extra
            .extra_general_constraints(base)
            .into_iter()
            .enumerate()
        {
            let v = ExtraVariable::Extra(SoftVariable::Soft(i, desc));
            new_obj = new_obj + LinExpr::var(v);
        }

        (new_obj, ObjectiveSense::Minimize)
    }

    fn reconstruct_extra_structure_variables(
        &self,
        base: &T,
        config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> ConfigData<Self::StructureVariable> {
        let orig_structure_variables = self
            .internal_extra
            .reconstruct_extra_structure_variables(base, config);

        let values = config
            .transmute(|x| match x {
                BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
                BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
            })
            .set_iter(
                orig_structure_variables
                    .transmute(|x| ExtraVariable::Extra(x.clone()))
                    .get_values(),
            )
            .get_values();

        let mut output = orig_structure_variables.transmute(|x| SoftVariable::Orig(x.clone()));

        for (i, (c, desc)) in self
            .internal_extra
            .extra_general_constraints(base)
            .into_iter()
            .enumerate()
        {
            let value = c
                .get_lhs()
                .eval(&values)
                .expect("All variables pertinent to the problem should be fixed");
            let var = SoftVariable::Soft(i, desc);

            match c.get_symbol() {
                collomatique_ilp::linexpr::EqSymbol::Equals => {
                    output = output.set(var, value.abs())
                }
                collomatique_ilp::linexpr::EqSymbol::LessThan => {
                    output = output.set(var, value.max(0.))
                }
            }
        }

        output
    }
}

impl<T: BaseConstraints, E: ExtraConstraints<T>> SoftConstraints<T, E> {
    pub fn new(extra: E) -> Self {
        SoftConstraints {
            internal_extra: extra,
            phantom: std::marker::PhantomData,
        }
    }
}
