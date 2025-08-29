use collomatique_ilp::{
    ConfigData, Constraint, LinExpr, Objective, ObjectiveSense, UsableData, Variable,
};
use std::collections::BTreeMap;

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

pub trait BaseConstraints: Send + Sync + std::fmt::Debug + PartialEq + Eq {
    type MainVariable: UsableData;
    type StructureVariable: UsableData;
    type StructureConstraintDesc: UsableData;
    type GeneralConstraintDesc: UsableData;
    type PartialSolution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq;

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

    fn objective(&self) -> Objective<BaseVariable<Self::MainVariable, Self::StructureVariable>> {
        Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize)
    }

    fn partial_solution_to_configuration(&self, sol: &Self::PartialSolution) -> ConfigData<Self::MainVariable>;
    fn configuration_to_partial_solution(&self, config: &ConfigData<Self::MainVariable>) -> Self::PartialSolution;

    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> ConfigData<Self::StructureVariable>;
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

    fn extra_objective(
        &self,
        base: &T,
    ) -> Objective<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>;

    fn reconstruct_extra_structure_variables(
        &self,
        base: &T,
        config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> ConfigData<Self::StructureVariable>;
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

impl<T: BaseConstraints, E: ExtraConstraints<T>> ExtraConstraints<T> for SoftConstraints<T, E> {
    type StructureConstraintDesc =
        SoftConstraint<E::StructureConstraintDesc, E::GeneralConstraintDesc>;
    type StructureVariable = SoftVariable<E::StructureVariable, E::GeneralConstraintDesc>;
    type GeneralConstraintDesc = ();

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

    fn extra_general_constraints(
            &self,
            _base: &T,
        ) -> Vec<(
            Constraint<ExtraVariable<<T as BaseConstraints>::MainVariable, <T as BaseConstraints>::StructureVariable, Self::StructureVariable>>,
            Self::GeneralConstraintDesc,
        )> {
        vec![]
    }

    fn extra_objective(
        &self,
        base: &T,
    ) -> Objective<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>
    {
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

        Objective::new(new_obj, ObjectiveSense::Minimize)
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
