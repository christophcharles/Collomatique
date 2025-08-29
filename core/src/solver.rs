//! Solver module.
//!
//! This module implements most of the logic for solving problems
//! defined using [BaseConstraints] and [ExtraConstraints].
//!
//! The standard process to solve such a problem is to start
//! by building a [ProblemBuilder] from a [BaseConstraints].
//! See [ProblemBuilder] or [Problem] for more details.

use super::*;

use std::collections::BTreeMap;

use collomatique_ilp::{ConfigData, Constraint, LinExpr, Objective, UsableData, Variable};

/// Internal IDs
///
/// These IDs are issued automatically and are
/// used internally to type-erase some data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternalId(u64);

impl std::fmt::Display for InternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Id issuer
///
/// This is used to issue internal ids as needed
/// to uniquely represent type erased data.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct IdIssuer {
    /// New available ID
    available: u64,
}

impl IdIssuer {
    /// Create a new ID issuer (first ID will be 0).
    fn new() -> Self {
        Self::default()
    }

    /// Get an ID from the ID issuer.
    ///
    /// Each ID from the (same) ID issuer is garanteed to be different.
    fn get_id(&mut self) -> InternalId {
        let new_id = InternalId(self.available);
        self.available += 1;
        new_id
    }
}

/// Variables used internally in [Problem] and [ProblemBuilder].
///
/// When [ExtraConstraints] define structure variables, they
/// are typed erased behind a generic ID (to simplify the API).
/// This type represents such a type erased variable.
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

/// Builder for [Problem].
///
/// This allows the progressive building of a [Problem]
/// from a [BaseConstraints] and possibly multiple [ExtraConstraints].
///
/// One starts by calling [ProblemBuilder::new] and providing a [BaseConstraints].
/// Extensions to the problem can be added with [ProblemBuilder::add_constraints].
/// Once the problem is entirely described, it can be built using [ProblemBuilder::build].
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
    /// base [BaseConstraints] the problem is built from.
    base: T,
    /// Internal [IdIssuer] used for type eraser when adding
    /// problem extensions (described by [ExtraConstraints]).
    id_issuer: IdIssuer,
    /// Phantom data for generic `P`.
    phantom_p: std::marker::PhantomData<P>,

    /// Internal objective that is progressively built from the problem and its
    /// possible extensions.
    objective: Objective<ExtraVariable<M, S, IdVariable>>,

    /// Definitions of the variables for the final ILP problem.
    variables: BTreeMap<ExtraVariable<M, S, IdVariable>, Variable>,

    /// Descriptions of the general constraints of the original [BaseConstraints] problem.
    general_constraint_descs: BTreeMap<InternalId, T::GeneralConstraintDesc>,
    /// Descriptions of the structure constraints of the original [BaseConstraints] problem.
    structure_constraint_descs: BTreeMap<InternalId, T::StructureConstraintDesc>,
    /// Constraints that are gradually built for the full ILP problem.
    /// It includes general constraints from the original problem but also the structure constraints
    /// of the base problem.
    /// It will also include possible structure and general constraints from problem extensions.
    constraints: Vec<(Constraint<ExtraVariable<M, S, IdVariable>>, InternalId)>,

    /// functions to reconstruct extra structure variables
    ///
    /// These are used to rebuild a complete configuration together with
    /// extra structure variables coming from problem extensions.
    reconstruction_funcs: Vec<
        Box<
            dyn Fn(
                    &T,
                    &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
                ) -> Option<ConfigData<IdVariable>>
                + Send
                + Sync,
        >,
    >,
}

impl<M, S, T, P> ProblemBuilder<M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    /// Starts building a problem from a given [BaseConstraints].
    ///
    /// This functions can actually fail if there is a mismatch between the
    /// declared variables and the variables that appear in the constraints
    /// and the objective.
    ///
    /// In that case, it returns `None`.
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
            reconstruction_funcs: vec![],
        })
    }

    /// Used internally
    ///
    /// Takes a map of variable definition and builds
    /// a two way map between internal IDs and these variables.
    ///
    /// This changes the internal state of the builder because of the internal
    /// IDs that are issued. But the variables are not committed yet to the problem
    /// to allow for failure in the calling function.
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

    /// Used internally
    ///
    /// Commit variables as returned by [ProblemBuilder::scan_variables] into the ILP problem.
    fn add_variables(&mut self, mut v_map: BTreeMap<ExtraVariable<M, S, IdVariable>, Variable>) {
        self.variables.append(&mut v_map);
    }

    /// Used internally
    ///
    /// Checks that all the variables in an expression were correctly declared
    /// using the rev_map returned by [ProblemBuilder::scan_variables]
    fn check_variables_in_expr<U: UsableData>(
        &self,
        expr: &LinExpr<ExtraVariable<M, S, U>>,
        rev_v_map: &BTreeMap<U, ExtraVariable<M, S, IdVariable>>,
    ) -> bool {
        for (v, _value) in expr.coefficients() {
            match v {
                ExtraVariable::BaseMain(v) => {
                    if !self
                        .variables
                        .contains_key(&ExtraVariable::BaseMain(v.clone()))
                    {
                        return false;
                    }
                }
                ExtraVariable::BaseStructure(v) => {
                    if !self
                        .variables
                        .contains_key(&ExtraVariable::BaseStructure(v.clone()))
                    {
                        return false;
                    }
                }
                ExtraVariable::Extra(v_extra) => {
                    if !rev_v_map.contains_key(v_extra) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Used internally
    ///
    /// Checks that all the variables in a list of constraints were correctly declared
    /// using the rev_map returned by [ProblemBuilder::scan_variables]
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

    /// Used internally
    ///
    /// Update a variable type to the internal [ExtraVariable] that uses
    /// IDs to type erase.
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

    /// Used internally
    ///
    /// Commit constraints to the ILP problem but transmute them in the process
    /// to use the correct internal [ExtraVariable] (for type erasure).
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

    /// Add a problem extension defined by an [ExtraConstraints] to the problem.
    ///
    /// The first parameter is a struct `extra` describing the extension and must implement [ExtraConstraints]
    /// for the specific [BaseConstraints] of the problem.
    /// The extension can provide further constraints and an additional (linear) objective.
    ///
    /// The second parameter is the weight that should be given to the additional objective.
    /// The sign of this parameter is basicaly ignored.
    ///
    /// This function can fail and in this case returns `None`. This happens if there is a mismatch
    /// between the variables that appear in the constraints or the objective and the declared variables.
    ///
    /// If the function succeeds, it returns a translator of type [ExtraTranslator]. This structure contains
    /// the necessary data to identify which extra constraints is not correctly satisfied in a (non-feasible) solution.
    pub fn add_constraints<E: ExtraConstraints<T> + 'static>(
        &mut self,
        extra: E,
        obj_coef: f64,
    ) -> Option<ExtraTranslator<T, E>> {
        let extra_variables = extra.extra_structure_variables(&self.base);
        let extra_structure_constraints = extra.extra_structure_constraints(&self.base);
        let extra_general_constraints = extra.extra_general_constraints(&self.base);
        let objective = extra.extra_objective(&self.base);

        let (rev_v_map, v_map) = self.scan_variables(extra_variables);

        if !self.check_variables_in_constraints(&extra_structure_constraints, &rev_v_map)
            || !self.check_variables_in_constraints(&extra_general_constraints, &rev_v_map)
            || !self.check_variables_in_expr(objective.get_function(), &rev_v_map)
        {
            return None;
        }

        self.add_variables(v_map);
        let general_c_map = self.add_constraints_internal(extra_general_constraints, &rev_v_map);
        let structure_c_map =
            self.add_constraints_internal(extra_structure_constraints, &rev_v_map);

        let new_obj = objective.transmute(|x| self.update_var(x, &rev_v_map));
        self.objective = &self.objective + obj_coef * new_obj;

        let lean_rev_v_map: BTreeMap<_, _> = rev_v_map
            .into_iter()
            .map(|(e, fluff)| match fluff {
                ExtraVariable::Extra(id) => (e, id),
                _ => panic!("Should only have extra variables"),
            })
            .collect();

        let reconstruct_func = move |base: &T, config: &ConfigData<BaseVariable<M, S>>| {
            let pre_output = extra.reconstruct_extra_structure_variables(base, config)?;
            pre_output.try_transmute(|x| lean_rev_v_map.get(x).cloned())
        };

        self.reconstruction_funcs.push(Box::new(reconstruct_func));

        Some(ExtraTranslator {
            general_c_map,
            structure_c_map,
        })
    }

    /// Builds the [Problem].
    ///
    /// This is the last function to call in the builder pattern (the first one being [ProblemBuilder::new]).
    /// It consumes the [ProblemBuilder] and builds the [Problem] as described.
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
            reconstruction_funcs: self.reconstruction_funcs,
        }
    }
}

/// Translator
///
/// This is used to restore types. The structure is returned by [ProblemBuilder::add_constraints]
/// and can be passed to [DecoratedSolution::blame_extra] and [DecoratedSolution::check_structure_extra].
///
/// It is used to restore the correct types for the constraints descriptions associated to a problem extension
/// (described by [ExtraConstraints]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraTranslator<T: BaseConstraints, E: ExtraConstraints<T>> {
    /// Map between ids and general constraints descriptions for the problem extension
    general_c_map: BTreeMap<InternalId, E::GeneralConstraintDesc>,
    /// Map between ids and structure constraints descriptions for the problem extension
    structure_c_map: BTreeMap<InternalId, E::StructureConstraintDesc>,
}

/// Represents a complete problem.
///
/// A problem is constructed using [ProblemBuilder] from a [BaseConstraints]
/// and possibly multiple [ExtraConstraints] using the builder pattern.
///
/// Once built, a problem is essentially fixed and is non-mutable.
///
/// You can call [Problem::solve] or [Problem::solve_with_time_limit] to try
/// and solve the problem using a solver.
///
/// You can also provide a potentiel solution to [Problem::decorate_solution]
/// and use its blaming functions ([DecoratedSolution::blame] and [DecoratedSolution::blame_extra])
/// to find out which constraints is not satisfied.

pub struct Problem<M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    /// ILP representation of the problem
    ilp_problem: collomatique_ilp::Problem<ExtraVariable<M, S, IdVariable>, InternalId, P>,
    /// Original problem of type [BaseConstraints]
    base: T,
    /// Map between ids and general constraints descriptions for the problem
    general_constraint_descs: BTreeMap<InternalId, T::GeneralConstraintDesc>,
    /// Map between ids and structure constraints descriptions for the problem
    structure_constraint_descs: BTreeMap<InternalId, T::StructureConstraintDesc>,
    /// Reconstruction functions.
    ///
    /// These are used to reconstruct the structure variables associated to
    /// the various problem extensions.
    reconstruction_funcs: Vec<
        Box<
            dyn Fn(
                    &T,
                    &ConfigData<BaseVariable<T::MainVariable, T::StructureVariable>>,
                ) -> Option<ConfigData<IdVariable>>
                + Send
                + Sync,
        >,
    >,
}

/// Represents a partial solution
///
/// Normally a solution of a problem is described by the type [BaseConstraints::PartialSolution].
/// This type will be used in the rest of the program. However, this representation usually lacks
/// some information to properly test the solution.
///
/// This is the decoration provided here: all the necessary variables are correctly reconstructed
/// and stored using [BaseConstraints::reconstruct_structure_variables] and [ExtraConstraints::reconstruct_extra_structure_variables].
///
/// Such a decorated partial solution is constructued from a [BaseConstraints::PartialSolution]
/// by calling [Problem::decorate_partial_solution].
#[derive(Clone)]
pub struct DecoratedPartialSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    problem: &'a Problem<M, S, T, P>,
    internal_solution: T::PartialSolution,
    config_data: ConfigData<ExtraVariable<M, S, IdVariable>>,
}

impl<'a, M, S, T, P> DecoratedPartialSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    /// Returns the inner solution
    ///
    /// This returns the [BaseConstraints::PartialSolution]
    /// the rest of the program usually cares about.
    pub fn inner(&self) -> &T::PartialSolution {
        &self.internal_solution
    }

    /// Returns the inner solution
    ///
    /// This method works like [Self::inner] but consumes the [DecoratedSolution].
    pub fn into_inner(self) -> T::PartialSolution {
        self.internal_solution
    }

    /// Checks whether the solution is complete
    ///
    /// A partial solution is a solution for which all the variables have not been defined.
    /// A complete solution is a solution for which all the variables have a defined value.
    ///
    /// This functions returns `true` if the partial solution actually has all its variables defined.
    pub fn is_complete(&self) -> bool {
        use std::collections::BTreeSet;

        let defined_variables: BTreeSet<_> = self.config_data.get_variables().collect();
        let problem_variables: BTreeSet<_> =
            self.problem.ilp_problem.get_variables().keys().collect();

        assert!(defined_variables.is_subset(&problem_variables));
        defined_variables == problem_variables
    }

    /// Promotes the [DecoratedPartialSolution] into a [DecoratedCompleteSolution].
    ///
    /// If the solution is complete (see [DecoratedPartialSolution::is_complete]),
    /// this functions returns a [DecoratedCompleteSolution] that represents it.
    ///
    /// Otherwise, the function fails and returns `None`.
    pub fn into_complete(self) -> Option<DecoratedCompleteSolution<'a, M, S, T, P>> {
        let ilp_config = self
            .problem
            .ilp_problem
            .build_config(self.config_data)
            .ok()?;

        Some(DecoratedCompleteSolution {
            problem: self.problem,
            internal_solution: self.internal_solution,
            ilp_config,
        })
    }

    /// Blame general constraints
    ///
    /// Returns a list of descriptions of general constraints that cannot be satisfied.
    /// If no such constraints exist, an empty list is returned.
    ///
    /// Because the solution is potentially partial, it is not always possible
    /// to check if all constraints are indeed satisfied or possibly satisfied.
    ///
    /// So the returned list can be empty and still the solution has no feasable extension.
    pub fn partial_blame(&self) -> Vec<T::GeneralConstraintDesc> {
        self.problem
            .ilp_problem
            .partial_blame(&self.config_data)
            .iter()
            .filter_map(|(_c, d)| self.problem.general_constraint_descs.get(d).cloned())
            .collect()
    }

    /// Blame general constraints for problem extension
    ///
    /// Returns a list of descriptions of general constraints from a problem extension that cannot be satisfied.
    /// The problem extension to consider is given by the translator used.
    ///
    /// Because the solution is potentially partial, it is not always possible
    /// to check if all constraints are indeed satisfied or possibly satisfied.
    ///
    /// If no such constraints exist, an empty list is returned.
    pub fn partial_blame_extra<E: ExtraConstraints<T>>(
        &self,
        translator: &ExtraTranslator<T, E>,
    ) -> Vec<E::GeneralConstraintDesc> {
        self.problem
            .ilp_problem
            .partial_blame(&self.config_data)
            .iter()
            .filter_map(|(_c, d)| translator.general_c_map.get(d).cloned())
            .collect()
    }

    /// Blame structure constraints
    ///
    /// Returns a list of descriptions of structure constraints that cannot be satisfied.
    /// If no such constraints exist, an empty list is returned.
    ///
    /// Because the solution is potentially partial, it is not always possible
    /// to check if all constraints are indeed satisfied or possibly satisfied.
    ///
    /// If no programming error is present in the reconstruction functions, this should
    /// *always* return an empty list.
    pub fn partial_check_structure(&self) -> Vec<T::StructureConstraintDesc> {
        self.problem
            .ilp_problem
            .partial_blame(&self.config_data)
            .iter()
            .filter_map(|(_c, d)| self.problem.structure_constraint_descs.get(d).cloned())
            .collect()
    }

    /// Blame structure constraints for problem extension
    ///
    /// Returns a list of descriptions of structure constraints from a problem extension that cannot be satisfied.
    /// The problem extension to consider is given by the translator used.
    ///
    /// Because the solution is potentially partial, it is not always possible
    /// to check if all constraints are indeed satisfied or possibly satisfied.
    ///
    /// If no such constraints exist, an empty list is returned.
    /// If no programming error is present in the reconstruction functions, this should
    /// *always* return an empty list.
    pub fn partial_check_structure_extra<E: ExtraConstraints<T>>(
        &self,
        translator: &ExtraTranslator<T, E>,
    ) -> Vec<E::StructureConstraintDesc> {
        self.problem
            .ilp_problem
            .partial_blame(&self.config_data)
            .iter()
            .filter_map(|(_c, d)| translator.structure_c_map.get(d).cloned())
            .collect()
    }
}

/// Represents a possibly non-feasable but complete solution
///
/// Normally a solution of a problem is described by the type [BaseConstraints::PartialSolution].
/// This type will be used in the rest of the program. However, this representation usually lacks
/// some information to properly test the solution.
///
/// This is the decoration provided here: all the necessary variables are correctly reconstructed
/// and stored.
///
/// Compared to [DecoratedPartialSolution], this type garantees that the solution is *complete*
/// meaning that all variables are defined and checking if it is a feasable solution is actually
/// trivial.
#[derive(Clone)]
pub struct DecoratedCompleteSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    problem: &'a Problem<M, S, T, P>,
    internal_solution: T::PartialSolution,
    ilp_config: collomatique_ilp::Config<'a, ExtraVariable<M, S, IdVariable>, InternalId, P>,
}

impl<'a, M, S, T, P> DecoratedCompleteSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    /// Returns the inner solution
    ///
    /// This returns the [BaseConstraints::PartialSolution]
    /// the rest of the program usually cares about.
    pub fn inner(&self) -> &T::PartialSolution {
        &self.internal_solution
    }

    /// Returns the inner solution
    ///
    /// This method works like [Self::inner] but consumes the [DecoratedSolution].
    pub fn into_inner(self) -> T::PartialSolution {
        self.internal_solution
    }

    /// Blame general constraints
    ///
    /// Returns a list of descriptions of general constraints that are not satisfied.
    /// If no such constraints exist, an empty list is returned.
    pub fn blame(&self) -> Vec<T::GeneralConstraintDesc> {
        self.ilp_config
            .blame()
            .filter_map(|(_c, d)| self.problem.general_constraint_descs.get(d).cloned())
            .collect()
    }

    /// Blame general constraints for problem extension
    ///
    /// Returns a list of descriptions of general constraints from a problem extension that are not satisfied.
    /// The problem extension to consider is given by the translator used.
    ///
    /// If no such constraints exist, an empty list is returned.
    pub fn blame_extra<E: ExtraConstraints<T>>(
        &self,
        translator: &ExtraTranslator<T, E>,
    ) -> Vec<E::GeneralConstraintDesc> {
        self.ilp_config
            .blame()
            .filter_map(|(_c, d)| translator.general_c_map.get(d).cloned())
            .collect()
    }

    /// Blame structure constraints
    ///
    /// Returns a list of descriptions of structure constraints that are not satisfied.
    /// If no such constraints exist, an empty list is returned.
    ///
    /// If no programming error is present in the reconstruction functions, this should
    /// *always* return an empty list.
    pub fn check_structure(&self) -> Vec<T::StructureConstraintDesc> {
        self.ilp_config
            .blame()
            .filter_map(|(_c, d)| self.problem.structure_constraint_descs.get(d).cloned())
            .collect()
    }

    /// Blame structure constraints for problem extension
    ///
    /// Returns a list of descriptions of structure constraints from a problem extension that are not satisfied.
    /// The problem extension to consider is given by the translator used.
    ///
    /// If no such constraints exist, an empty list is returned.
    /// If no programming error is present in the reconstruction functions, this should
    /// *always* return an empty list.
    pub fn check_structure_extra<E: ExtraConstraints<T>>(
        &self,
        translator: &ExtraTranslator<T, E>,
    ) -> Vec<E::StructureConstraintDesc> {
        self.ilp_config
            .blame()
            .filter_map(|(_c, d)| translator.structure_c_map.get(d).cloned())
            .collect()
    }
}

/// Type returned by [Problem::solve_with_time_limit].
///
/// This contains a normal decorated solution and says if the time limit was reached.
/// It is functionally similar to [collomatique_ilp::solvers::TimeLimitSolution].
#[derive(Clone)]
pub struct TimeLimitSolution<'a, M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    /// The actual decorated solution that is returned
    pub solution: Option<DecoratedCompleteSolution<'a, M, S, T, P>>,
    /// Wether the time limit was reached
    ///
    /// If the time limit is reached, the solution might not be optimal.
    pub time_limit_reached: bool,
}

impl<M, S, T, P> Problem<M, S, T, P>
where
    M: UsableData,
    S: UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<ExtraVariable<M, S, IdVariable>>,
    T: BaseConstraints<MainVariable = M, StructureVariable = S>,
{
    /// Decorate a partial solution
    ///
    /// This functions takes a partial solution of type [BaseConstraints::PartialSolution]
    /// and "decorates" it. This means that all the internal ILP variables are reconstructed and packaged
    /// into a [DecoratedPartialSolution].
    ///
    /// This function can fail (and return `None` in that case) if there are issues with the
    /// reconstruction (usually unexpected or undefined variables).
    pub fn decorate_partial_solution<'a>(
        &'a self,
        solution: T::PartialSolution,
    ) -> Option<DecoratedPartialSolution<'a, M, S, T, P>> {
        let starting_configuration_data = self.base.partial_solution_to_configuration(&solution)?;
        let mut base_config_data =
            starting_configuration_data.transmute(|x| BaseVariable::Main(x.clone()));
        base_config_data = base_config_data.set_iter(
            self.base
                .reconstruct_structure_variables(&starting_configuration_data)?
                .get_values()
                .into_iter()
                .map(|(var, value)| (BaseVariable::Structure(var), value)),
        );

        let mut config_data = base_config_data.transmute(|x| match x {
            BaseVariable::Main(m) => ExtraVariable::BaseMain(m.clone()),
            BaseVariable::Structure(s) => ExtraVariable::BaseStructure(s.clone()),
        });
        for func in &self.reconstruction_funcs {
            config_data = config_data.set_iter(
                func(&self.base, &base_config_data)?
                    .get_values()
                    .into_iter()
                    .map(|(var, value)| (ExtraVariable::Extra(var), value)),
            );
        }

        Some(DecoratedPartialSolution {
            problem: self,
            internal_solution: solution,
            config_data,
        })
    }

    /// Used internally
    ///
    /// Transforms the result of a solver into a usable [collomatique_ilp::ConfigData]
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

    /// Solves the problem using a solver
    ///
    /// This will use the solver to solve the ILP problem. It returns `None` if there is no solution.
    /// Otherwise it returns a [DecoratedSolution] reprenseting the solution that was found.
    pub fn solve<
        'a,
        Solver: collomatique_ilp::solvers::Solver<ExtraVariable<M, S, IdVariable>, InternalId, P>,
    >(
        &'a self,
        solver: &Solver,
    ) -> Option<DecoratedCompleteSolution<'a, M, S, T, P>> {
        let feasable_config = solver.solve(&self.ilp_problem)?;
        let internal_solution = self.base.configuration_to_partial_solution(
            &self.feasable_config_into_config_data(&feasable_config),
        )?;

        Some(DecoratedCompleteSolution {
            problem: self,
            internal_solution,
            ilp_config: feasable_config.into_inner(),
        })
    }

    /// Solves the problem using a solver
    ///
    /// This will use the solver to solve the ILP problem but with a time limit.
    /// The details of the result are returned through a [TimeLimitSolution] struct.
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
    ) -> TimeLimitSolution<'a, M, S, T, P> {
        let time_limit_sol = solver.solve_with_time_limit(&self.ilp_problem, time_limit_in_seconds);
        let Some(config) = time_limit_sol.config else {
            return TimeLimitSolution {
                solution: None,
                time_limit_reached: time_limit_sol.time_limit_reached,
            };
        };

        let internal_solution = self
            .base
            .configuration_to_partial_solution(&self.feasable_config_into_config_data(&config));

        TimeLimitSolution {
            solution: internal_solution.map(|is| DecoratedCompleteSolution {
                problem: self,
                internal_solution: is,
                ilp_config: config.into_inner(),
            }),
            time_limit_reached: time_limit_sol.time_limit_reached,
        }
    }
}
