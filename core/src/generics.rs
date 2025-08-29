//! This module defines useful traits that should be implemented to represent a problem.
//!
//! A problem is represented by a structure that implements [BaseConstraints].
//! Extension to this problem can be implemented using other structures that implement [ExtraConstraints].
//!
//! It also implements a few generic [ExtraConstraints] that are useful in a lot of situations.
//! See [SoftConstraints] and [FixedPartialSolution].

use collomatique_ilp::{
    ConfigData, Constraint, LinExpr, Objective, ObjectiveSense, UsableData, Variable,
};
use std::collections::BTreeMap;

/// Variable type used in [BaseConstraints] trait definition.
///
/// [BaseConstraints] distinguishes between [BaseConstraints::MainVariable]
/// and [BaseConstraints::StructureVariable]. To express some constraints,
/// we need to mix these variables and this is the purpose of this type.
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

/// Base trait that should be implemented by any structure representing a problem.
///
/// The type of problems that can be represented is volontarily restricted. But it is
/// large enough to represent most schedule (and colloscope !) resolution problem.
///
/// We use the following restrictions :
/// - we can define a finite set of ILP variables such that the space of possible
///   values for this set is one-to-one with the space of possible solutions of the problem.
///
///   This constraint means we can convert back and forth between two representations of the problem.
///   The conversion is done by the two functions [BaseConstraints::partial_solution_to_configuration]
///   and [BaseConstraints::configuration_to_partial_solution].
///   One representation is the one we use in the rest of the program and is described
///   by [BaseConstraints::PartialSolution] and the other one is a set of values for these
///   variables (represented using [ConfigData]).
///
///   A few things must be noted here. First, we are talking about a space of possible solutions
///   not a space of actual solutions. For instance, for a soduko grid, that might be associating
///   a number into each cell of a grid. It does not have to satisfy the rules of the soduko. And
///   this is by design: we don't know what are correct solutions yet!
///
///   Then, this is actually quite constrained. For instance, for a soduko grid of size 9x9,
///   we usually represent the problem with 9x9x9 boolean variables. The variable x<sub>ijk</sub>
///   is `1` if the number `k` is in the cell `(i,j)`. This allows for multiple numbers to be written
///   in each cell (or none at all!). If we use such a choice for the ILP variables, we then *must*
///   enlarge our description of possible solutions by allowing multiple numbers to be written into
///   each cell (and also allow no number at all in a given cell). This is needed to maintain the
///   one-to-one correspond between the ILP description and the programmatic description for the
///   rest of the program.
///
///   Finally the type is called [BaseConstraints::PartialSolution] because the description might
///   be partial and not complete. In the ILP realm, this means some variables do not have a definite
///   value set (and must still be solved in some way). The programatic description must be adapted
///   accordingly.
///
///   This is actually not completely counter-intuitive: this is helpful in at least two cases.
///   First, we want to build a colloscope building software. We need the possibility of partially
///   built colloscopes, that still need to be completed. The same situation exists for a soduko grid:
///   we want to be able to describe a grid that has not fully been solved yet.
///   Second, this is actually useful to complete an started solution. In the case of soduko, this might
///   represent the numbers put on the initial grid as help. In the case of a colloscope, that might
///   be a partial descriptions of student groups because some students want to be together.
///
/// - we can define a second set of (ILP) variables that we call [BaseConstraints::StructureVariable].
///   These variables can be entirely deduced from the [BaseConstraints::MainVariable].
///   There are useful only to write the problem in a linear fashion.
///
///   Two functions are noteworthy with regard to these variables. First there is [BaseConstraints::structure_constraints].
///   It returns a set of (ILP) constraints such that, if given a set of [BaseConstraints::MainVariable],
///   when solved will lead to the unique corresponding set of values fot the [BaseConstraints::StructureVariable]
///   variables.
///
///   Second, there is [BaseConstraints::reconstruct_structure_variables] which does basically the same thing
///   but programmatically but allows for partial solutions (and so gives only a partial set of structure
///   variables).
///
/// - The problem itself can be described linearly using both [BaseConstraints::MainVariable] and
///   [BaseConstraints::MainVariable] with linear constraints. These are returned by
///   [BaseConstraints::general_constraints].
pub trait BaseConstraints: Send + Sync {
    /// Type to represent the main variables
    ///
    /// The main variables are the variables whose set of values is in one to one correspondance
    /// with the set of possible solutions.
    ///
    /// See [BaseConstraints] for the full discussion.
    type MainVariable: UsableData + 'static;

    /// Type to represent the structure variables
    ///
    /// The structure variables do not provide any new information and can entirely
    /// be rebuild from the main variables (represented by [BaseConstraints::MainVariable]).
    /// They only have a practical utility and help representing the problem as an ILP problem.
    ///
    /// See [BaseConstraints] for the full discussion.
    type StructureVariable: UsableData + 'static;

    /// Type to represent the description of structure constraints
    ///
    /// Structure constraints define the structure variables ([BaseConstraints::StructureVariable])
    /// from the main variables ([BaseConstraints::MainVariable]).
    ///
    /// See [BaseConstraints] for the full discussion.
    type StructureConstraintDesc: UsableData + 'static;

    /// Type to represent the description of general constraints
    ///
    /// Genral constraints actually define the problem using both main variables ([BaseConstraints::MainVariable])
    /// and the structure variables ([BaseConstraints::StructureVariable]).
    ///
    /// See [BaseConstraints] for the full discussion.
    type GeneralConstraintDesc: UsableData + 'static;

    /// Partial solution type associated to the problem.
    ///
    /// This type is used in the rest of the program to represent a solution (actually a partial
    /// solution) to the problem.
    ///
    /// See [BaseConstraints] for the full discussion.
    type PartialSolution: Send + Sync + Clone + std::fmt::Debug + PartialEq + Eq;

    /// Definition of the main variables for the problem.
    ///
    /// See [BaseConstraints] for the full discussion.
    fn main_variables(&self) -> BTreeMap<Self::MainVariable, Variable>;

    /// Definition of the structure variables for the problem.
    ///
    /// See [BaseConstraints] for the full discussion.
    fn structure_variables(&self) -> BTreeMap<Self::StructureVariable, Variable>;

    /// Definition of the structure constraints for the problem.
    ///
    /// See [BaseConstraints] for the full discussion.
    fn structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )>;

    /// Definition of the general constraints for the problem.
    ///
    /// See [BaseConstraints] for the full discussion.
    fn general_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )>;

    /// Definition of a linear objective for the problem.
    ///
    /// An ILP problem has an objective to optimize. By default it is
    /// a constant function which means there is nothing to optimize.
    /// But you can define such an objective by using both main and structure variables.
    ///
    /// See [BaseConstraints] for the full discussion on the different types of variables.
    fn objective(&self) -> Objective<BaseVariable<Self::MainVariable, Self::StructureVariable>> {
        Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize)
    }

    /// Converts a [BaseConstraints::PartialSolution] into a set of values for the main variables.
    ///
    /// The description should be exactly one to one. This means two things:
    /// - first, [BaseConstraints::partial_solution_to_configuration] and [BaseConstraints::configuration_to_partial_solution]
    ///   should be reciprocal to each other.
    /// - second, if the solution is partial, this should be correctly reflected by not setting the value of some
    ///   main variables.
    ///
    /// This method can fail if the partial solution does not fit the problem. In that cas, `None` is returned.
    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> Option<ConfigData<Self::MainVariable>>;

    /// Converts a set of values for the main variables into a [BaseConstraints::PartialSolution].
    ///
    /// The description should be exactly one to one. This means two things:
    /// - first, [BaseConstraints::partial_solution_to_configuration] and [BaseConstraints::configuration_to_partial_solution]
    ///   should be reciprocal to each other.
    /// - second, if the set of values is partial, this should be correctly reflected in a partial solution output.
    fn configuration_to_partial_solution(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution;

    /// Reconstructs as many structure variables as possible from the main variables.
    ///
    /// A value should only be given if it can indeed be fixed. If the solution is complete (meaning
    /// all main variables have a fixed value) then all structure variables should have a value too
    /// and it should uniquely be fixed by the main variables.
    fn reconstruct_structure_variables(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> ConfigData<Self::StructureVariable>;
}

/// Variable type used in [ExtraConstraints] trait definition.
///
/// [ExtraConstraints] can introduce its own structure variables.
/// To express some constraints, we need to mix these variables
/// with the main and structure variables of the corresponding
/// [BaseConstraints]. This is the purpose of this type.
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

/// Extra constraints for a given [BaseConstraints].
///
/// Sometimes a generic problem might have possible extensions.
/// For a scheduling problem for instance, we might have a generic
/// problem of distributing courses to different students.
/// But there can be extra constraints. For instance a student might not
/// be available on mondays because of personnal reasons.
/// This is an extension of the *constraints* of the problem.
///
/// The form of the solution itself does not but the set of possible
/// solutions is reduced due to some extra constraints.
///
/// Sometimes these extra constraints are so prevalent that they
/// might as well be represented as part of the main problem. But
/// often, these are exceptional cases and we should not burden the user
/// into describing constraints he does not need.
///
/// For exceptional cases like this, we can define extra structures
/// implementing the current trait [ExtraConstraints].
///
/// Such a trait does not define any new main variables (described by [BaseConstraints::MainVariable]).
/// Indeed, the space of possible solutions does not change.
/// However, we might need extra constraints and, to express them linearly,
/// extra structure variables. These extra structure variables will be described by
/// [ExtraConstraints::StructureVariable].
///
/// The corresponding structure constraints will be given by [ExtraConstraints::extra_structure_constraints]
/// and the structure variables can be rebuilt programmatically using [ExtraConstraints::reconstruct_extra_structure_variables].
///
/// The additionnal constraints will be given by [ExtraConstraints::extra_general_constraints].
///
/// It is also possible to add an objective with [ExtraConstraints::extra_objective].
///
/// Because the space of solutions does not change, there is no equivalent to [BaseConstraints::configuration_to_partial_solution]
/// and [BaseConstraints::partial_solution_to_configuration] in [ExtraConstraints].
pub trait ExtraConstraints<T: BaseConstraints>: Send + Sync {
    /// Type to represent the structure variables specific to this problem extension.
    ///
    /// The structure variables do not provide any new information and can entirely
    /// be rebuild from the main variables (represented by [BaseConstraints::MainVariable]).
    /// They only have a practical utility and help representing the problem as an ILP problem.
    ///
    /// See [ExtraConstraints] and [BaseConstraints] for the full discussion.
    type StructureVariable: UsableData + 'static;

    /// Type to represent the description of the extra structure constraints
    ///
    /// Structure constraints define the structure variables ([ExtraConstraints::StructureVariable])
    /// from the main variables ([BaseConstraints::MainVariable]) and possibly the already
    /// existing structure constraints from the main problem ([BaseConstraints::StructureVariable]).
    ///
    /// See [ExtraConstraints] and [BaseConstraints] for the full discussion.
    type StructureConstraintDesc: UsableData + 'static;

    /// Type to represent the description of general constraints for the extension.
    ///
    /// Genral constraints actually define the extension to the problem using main variables
    /// ([BaseConstraints::MainVariable]), structure variables ([BaseConstraints::StructureVariable])
    /// from the original problem but also extra structure variables ([ExtraConstraints::StructureVariable])
    /// from the problem extension.
    ///
    /// See [ExtraConstraints] for the full discussion.
    type GeneralConstraintDesc: UsableData + 'static;

    /// Checks if the extension is compatible with the given problem
    fn is_fit_for_problem(&self, base: &T) -> bool;

    /// Definition of the structure variables for the problem extension.
    ///
    /// See [ExtraConstraints] for the full discussion.
    fn extra_structure_variables(&self, base: &T) -> BTreeMap<Self::StructureVariable, Variable>;

    /// Definition of the structure constraints for the problem extension.
    ///
    /// See [ExtraConstraints] for the full discussion.
    fn extra_structure_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )>;

    /// Definition of the general constraints for the problem extension.
    ///
    /// See [ExtraConstraints] for the full discussion.
    fn extra_general_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )>;

    /// Definition of a linear objective for the problem extension.
    ///
    /// This objective will be added (with a weight) to the overall objective.
    /// By default, you do not have to implement this function and it returns
    /// a trivial objective.
    ///
    /// See [ExtraConstraints] for the full discussion.
    fn extra_objective(
        &self,
        _base: &T,
    ) -> Objective<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>
    {
        Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize)
    }

    /// Reconstructs as many extra structure variables as possible from the main variables and generic structure variables.
    ///
    /// A value should only be given if it can indeed be fixed. If the solution is complete (meaning
    /// all main variables have a fixed value) then all extra structure variables should have a value too
    /// and it should uniquely be fixed by the main variables.
    ///
    /// Here we only want to build the structure variables specific to the problem extension ([ExtraConstraints::StructureVariable]).
    /// The structure variable for the generic problem are already handled by [BaseConstraints::reconstruct_structure_variables].
    ///
    /// As a convenience, it is possible to use the structure variables from the generic problem ([BaseConstraints::StructureVariable])
    /// to rebuild the extra structure variables.
    /// See [ExtraConstraints] for the full discussion.
    fn reconstruct_extra_structure_variables(
        &self,
        base: &T,
        config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> ConfigData<Self::StructureVariable>;
}

/// Soft enforcement of extra constraints.
///
/// Sometimes, we do not want to implement strictly a set of constraints.
/// A typical example is regularity constraints in school schedules.
///
/// We might want the courses to be fairly regular in a schedule. It is
/// usually somewhat easy to write a set of constraints enforcing *perfect*
/// regularity. However, such a strict regularity might not be possible
/// or even desirable if it conflicts with some other constraints.
///
/// This structure is built from some [ExtraConstraints] that implements
/// strictly a set of constraints. It transforms those constraints into
/// an objective that should be optimized. If the objective is perfectly
/// optimized then the constraints are perfectly satisfied. But it is
/// also possible to not completly satisfy the constraints.
///
/// This also allows the introduction of weights between different objectives
/// and thus fine-tune which schedule is preferable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftConstraints<T: BaseConstraints, E: ExtraConstraints<T>> {
    /// Original [ExtraConstraints] describing the strict constraints.
    internal_extra: E,
    /// Phantom type because of generic `T`.
    phantom: std::marker::PhantomData<T>,
}

/// Structure variable used for the definition of [SoftConstraints].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoftVariable<S: UsableData, C: UsableData> {
    /// This represents a structure variable from the original
    /// strict constraint set.
    Orig(S),
    /// This is a new structure variable used to measure the degree
    /// of non-satisfaction of a constraint.
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

/// Structure constraint used for the definition of [SoftConstraints].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoftConstraint<S: UsableData, C: UsableData> {
    /// This represents a structure constraint from the original
    /// strict constraint set.
    Orig(S),
    /// This is a new structure constraint used to define a [SoftVariable::Soft] variable.
    ///
    /// The first two parameters define the corresponding soft variable. The last one
    /// defines the equation symbol (either `<=` if `false` or `>=` if `true`).
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

    fn is_fit_for_problem(&self, base: &T) -> bool {
        self.internal_extra.is_fit_for_problem(base)
    }

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
        Constraint<
            ExtraVariable<
                <T as BaseConstraints>::MainVariable,
                <T as BaseConstraints>::StructureVariable,
                Self::StructureVariable,
            >,
        >,
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
            let Ok(value) = c.get_lhs().eval(&values) else {
                continue;
            };
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
    /// Builds a [SoftConstraints] from an existing strict
    /// constraint set defined in a structure implementing [ExtraConstraints].
    pub fn new(extra: E) -> Self {
        SoftConstraints {
            internal_extra: extra,
            phantom: std::marker::PhantomData,
        }
    }
}

/// Fixes a partial solution
///
/// In a lot of problems, we actually want to complete
/// an already existing partial solution.
///
/// This can be the case for instance if we have a partial list of
/// interrogation groups (for colloscopes), or if we have a partially
/// completed schedule (because of other constraints).
///
/// This structure implements [ExtraConstraints] and for any problem
/// can force a partial solution to be completed.
///
/// It can also be combined with [SoftConstraints] to rather look for the closest
/// solution possible to the partial solution rather than looking for an exact match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedPartialSolution<T: BaseConstraints> {
    /// Partial solution represented as [BaseConstraints::PartialSolution]
    /// to enforce
    partial_solution: T::PartialSolution,
}

/// General constraint used for the definition of [FixedPartialSolution].
/// It takes two parameters: the variable being enforced and the corresponding set value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartialConstraint<V: UsableData>(pub V, pub ordered_float::OrderedFloat<f64>);

impl<V: UsableData + std::fmt::Display> std::fmt::Display for PartialConstraint<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.0, self.1)
    }
}

impl<T: BaseConstraints> ExtraConstraints<T> for FixedPartialSolution<T> {
    type StructureConstraintDesc = ();
    type StructureVariable = ();
    type GeneralConstraintDesc = PartialConstraint<T::MainVariable>;

    fn is_fit_for_problem(&self, base: &T) -> bool {
        base.partial_solution_to_configuration(&self.partial_solution)
            .is_some()
    }

    fn extra_structure_variables(&self, _base: &T) -> BTreeMap<Self::StructureVariable, Variable> {
        BTreeMap::new()
    }

    fn extra_structure_constraints(
        &self,
        _base: &T,
    ) -> Vec<(
        Constraint<ExtraVariable<T::MainVariable, T::StructureVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )> {
        vec![]
    }

    fn extra_general_constraints(
        &self,
        base: &T,
    ) -> Vec<(
        Constraint<
            ExtraVariable<
                <T as BaseConstraints>::MainVariable,
                <T as BaseConstraints>::StructureVariable,
                Self::StructureVariable,
            >,
        >,
        Self::GeneralConstraintDesc,
    )> {
        let config_data = base
            .partial_solution_to_configuration(&self.partial_solution)
            .expect("Compatibility should be tested with is_fit_for_problem");

        config_data
            .get_values()
            .into_iter()
            .map(|(var, value)| {
                let lhs = LinExpr::var(ExtraVariable::BaseMain(var.clone()));
                let rhs = LinExpr::constant(value);
                let constraint = lhs.eq(&rhs);

                let desc = PartialConstraint(var, ordered_float::OrderedFloat(value));

                (constraint, desc)
            })
            .collect()
    }

    fn reconstruct_extra_structure_variables(
        &self,
        _base: &T,
        _config: &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
    ) -> ConfigData<Self::StructureVariable> {
        ConfigData::new()
    }
}
