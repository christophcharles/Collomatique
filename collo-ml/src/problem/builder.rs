//! Problem builder for constructing ILP problems.
//!
//! This module defines:
//! - `ProblemBuilder`: Builder pattern for constructing optimization problems
//! - `EvalData`: Internal data structure for evaluation

use super::solution::Problem;
use super::types::{
    ConstraintDesc, ExtraDesc, HashedProblemVar, ProblemError, ProblemVar, ReifiedVar,
};
use crate::database::DatabaseDriver;
use crate::eval::{
    CheckedAST, CustomValue, EvalError, ExprValue, ExternVar, HashedIlpVar, IlpVar, ScriptVar,
};
use crate::semantics::ArgsType;
use crate::traits::VarConversionError;
use crate::{EvalVar, ExprType, SemWarning, SimpleType};
use collomatique_ilp::linexpr::EqSymbol;
use collomatique_ilp::{Constraint, Hashed, LinExpr, Objective, ObjectiveSense, Variable};
use derivative::Derivative;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct ProblemBuilder<
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
> {
    /// Compiled AST (all modules compiled together)
    pub(crate) ast: CheckedAST<D>,

    /// Variables that define the problem
    /// The set of possible values of these variables is one-to-one with
    /// the set of solutions to our problem
    pub(crate) base_vars: HashMap<String, ArgsType>,

    /// Pending constraint function calls (validated but not yet evaluated)
    /// Format: (module, fn_name, args, needs_db)
    pending_constraints: Vec<(String, String, Vec<ExprValue<D::Connection>>, bool)>,

    /// Pending objective function calls (validated but not yet evaluated)
    /// Format: (module, fn_name, args, needs_db, coefficient, sense)
    pending_objectives: Vec<(
        String,
        String,
        Vec<ExprValue<D::Connection>>,
        bool,
        ordered_float::OrderedFloat<f64>,
        ObjectiveSense,
    )>,

    phantom: std::marker::PhantomData<V>,
}

pub(crate) struct EvalData<
    'a,
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
> {
    pub(crate) builder: ProblemBuilder<D, V>,

    /// Reference to the evaluation environment
    pub(crate) env: &'a V::Env,

    /// List of constraints incrementally built (populated during build())
    pub(crate) constraints: Vec<(
        Constraint<HashedProblemVar<D::Connection, V>>,
        ConstraintDesc<D::Connection>,
    )>,

    /// Objective function (populated during build())
    pub(crate) objective: Objective<HashedProblemVar<D::Connection, V>>,

    /// Internal ID.
    ///
    /// When reifying variables, we might need intermediate variables.
    /// In that case, we define a numbered variable with [ProblemVar::Helper].
    /// This variable keeps track of the next id to use.
    current_helper_id: u64,

    /// Definition of all the variables used.
    ///
    /// This starts with the variables from V.
    /// Then reified variables as well as
    /// helpers variables are added as needed.
    pub(crate) vars_desc: HashMap<HashedProblemVar<D::Connection, V>, Variable>,

    /// base variables list
    pub(crate) original_var_list: HashMap<V, Variable>,
}

impl<
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
> ProblemBuilder<D, V>
{
    /// Validate that a function exists with the correct signature.
    /// Returns `Ok(true)` if the first argument is a database schema (needs_db),
    /// `Ok(false)` otherwise.
    fn validate_function(
        &self,
        module: &str,
        fn_name: &str,
        args: &[ExprValue<D::Connection>],
        expected_return: &ExprType,
    ) -> Result<bool, ProblemError<D::Connection>> {
        let functions = self.ast.get_functions();
        let key = (module.to_string(), fn_name.to_string());

        let (args_type, output_type) = functions
            .get(&key)
            .ok_or_else(|| ProblemError::UnknownFunction(format!("{}::{}", module, fn_name)))?;

        // Determine if the first parameter is a database schema
        let needs_db =
            !args_type.is_empty() && self.ast.global_env.is_database_schema_deep(&args_type[0]);
        let expected_args = if needs_db {
            args_type.len() - 1
        } else {
            args_type.len()
        };

        if args.len() != expected_args {
            return Err(ProblemError::ArgumentCountMismatch {
                func: format!("{}::{}", module, fn_name),
                expected: expected_args,
                found: args.len(),
            });
        }

        // Check return type
        if !output_type.is_subtype_of(expected_return)
            && !expected_return.is_subtype_of(output_type)
        {
            return Err(ProblemError::WrongReturnType {
                func: format!("{}::{}", module, fn_name),
                returned: output_type.clone(),
                expected: expected_return.clone(),
            });
        }

        Ok(needs_db)
    }

    pub async fn new(modules: &BTreeMap<&str, &str>) -> Result<Self, ProblemError<D::Connection>> {
        let base_vars = V::field_schema();

        // Compile all modules upfront
        let ast = CheckedAST::new(modules, base_vars.clone()).await?;

        Ok(ProblemBuilder {
            ast,
            base_vars,
            pending_constraints: vec![],
            pending_objectives: vec![],
            phantom: std::marker::PhantomData,
        })
    }

    /// Get compilation warnings from the AST
    pub fn get_warnings(&self) -> &[SemWarning] {
        self.ast.get_warnings()
    }

    /// Get the signature of a function by module and name.
    /// Returns the list of argument types and the output type,
    /// or `None` if the function does not exist.
    pub fn get_fn_signature(&self, module: &str, fn_name: &str) -> Option<(ArgsType, ExprType)> {
        let functions = self.ast.get_functions();
        let key = (module.to_string(), fn_name.to_string());
        functions.get(&key).cloned()
    }

    /// Get all public function signatures in a given module.
    /// Returns a map from function name to (argument types, return type).
    /// The map is empty if the module does not exist or has no public functions.
    pub fn get_fn_from_module(&self, module: &str) -> BTreeMap<String, (ArgsType, ExprType)> {
        self.ast
            .get_functions()
            .into_iter()
            .filter(|((mod_name, _), _)| mod_name == module)
            .map(|((_, fn_name), sig)| (fn_name, sig))
            .collect()
    }

    /// Add a constraint function to be evaluated at build time.
    ///
    /// Validates that the function exists and has the correct signature,
    /// but does not evaluate it yet.
    pub fn add_constraint(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<D::Connection>>,
    ) -> Result<(), ProblemError<D::Connection>> {
        // Validate function exists and has correct signature
        // Constraints can return Constraint or [Constraint]
        let constraint_type = ExprType::from_variants([
            SimpleType::Constraint,
            SimpleType::List(SimpleType::Constraint.into()),
        ]);

        let needs_db = self.validate_function(module, fn_name, &args, &constraint_type)?;

        // Store for later evaluation
        self.pending_constraints
            .push((module.to_string(), fn_name.to_string(), args, needs_db));
        Ok(())
    }

    /// Add an objective function to be evaluated at build time.
    ///
    /// Validates that the function exists and has the correct signature,
    /// but does not evaluate it yet.
    pub fn add_objective(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<D::Connection>>,
        coefficient: f64,
        sense: ObjectiveSense,
    ) -> Result<(), ProblemError<D::Connection>> {
        // Validate function exists and has correct signature
        // Objectives can return LinExpr or Constraint or a list of those
        let obj_types = ExprType::from_variants([
            SimpleType::LinExpr,
            SimpleType::Constraint,
            SimpleType::List(ExprType::from_variants([
                SimpleType::LinExpr,
                SimpleType::Constraint,
            ])),
        ]);

        let needs_db = self.validate_function(module, fn_name, &args, &obj_types)?;

        // Store for later evaluation
        self.pending_objectives.push((
            module.to_string(),
            fn_name.to_string(),
            args,
            needs_db,
            ordered_float::OrderedFloat(coefficient),
            sense,
        ));
        Ok(())
    }

    pub async fn build(
        self,
        env: &V::Env,
        db_connection: Option<D::Connection>,
    ) -> Result<Problem<D::Connection, V>, ProblemError<D::Connection>> {
        // Evaluate all pending constraints and objectives
        let mut eval_data = EvalData::new(self, env, db_connection).await?;

        for (constraint, _desc) in eval_data.constraints.iter_mut() {
            let mut fixed_variables = HashMap::new();
            for var in constraint.variables() {
                if fixed_variables.contains_key(&var) {
                    continue;
                }
                let ProblemVar::Base(v) = &*var else {
                    continue;
                };
                let v = v.clone();
                let Some(value) = v.fix(eval_data.env) else {
                    continue;
                };
                fixed_variables.insert(var, value);
            }
            if fixed_variables.is_empty() {
                continue;
            }
            *constraint = constraint.reduce(&fixed_variables);
        }
        let mut fixed_variables = HashMap::new();
        for var in eval_data.objective.get_function().variables() {
            if fixed_variables.contains_key(&var) {
                continue;
            }
            let ProblemVar::Base(v) = &*var else {
                continue;
            };
            let v = v.clone();
            let Some(value) = v.fix(eval_data.env) else {
                continue;
            };
            fixed_variables.insert(var, value);
        }
        if !fixed_variables.is_empty() {
            eval_data.objective = eval_data.objective.reduce(&fixed_variables);
        }
        eval_data
            .constraints
            .retain(|(c, _d)| !c.is_trivially_true());

        let reification_constraints: Vec<_> = eval_data
            .constraints
            .iter()
            .filter_map(|(c, d)| match d {
                ConstraintDesc::InScript { origin: _ } => None,
                ConstraintDesc::Objectify { origin: _ } => {
                    Some((c.clone(), ExtraDesc::Orig(d.clone())))
                }
                ConstraintDesc::Reified {
                    var_name: _,
                    origin: _,
                } => Some((c.clone(), ExtraDesc::Orig(d.clone()))),
            })
            .collect();

        let mut problem_builder = collomatique_ilp::ProblemBuilder::new()
            .set_variables(eval_data.vars_desc.clone())
            .add_constraints(eval_data.constraints);
        problem_builder = problem_builder.set_objective(eval_data.objective);

        let reification_problem_builder = collomatique_ilp::ProblemBuilder::new()
            .set_variables(eval_data.vars_desc)
            .add_constraints(reification_constraints);

        Ok(Problem::new(
            problem_builder.build().expect("Problem should be valid"),
            reification_problem_builder,
            eval_data.original_var_list,
        ))
    }
}

impl<
    'a,
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
> EvalData<'a, D, V>
{
    fn generate_helper_var(&mut self) -> HashedProblemVar<D::Connection, V> {
        let new_var = Hashed::new(ProblemVar::Helper(self.current_helper_id));
        self.vars_desc
            .insert(new_var.clone(), collomatique_ilp::Variable::binary());
        self.current_helper_id += 1;
        new_var
    }

    fn generate_helper_continuous_var(&mut self) -> HashedProblemVar<D::Connection, V> {
        let new_var = Hashed::new(ProblemVar::Helper(self.current_helper_id));
        self.vars_desc
            .insert(new_var.clone(), collomatique_ilp::Variable::continuous());
        self.current_helper_id += 1;
        new_var
    }

    fn get_variable_type(&self, v: &HashedProblemVar<D::Connection, V>) -> Variable {
        match &**v {
            ProblemVar::Helper(_) | ProblemVar::Reified(_) => Variable::binary(),
            ProblemVar::Base(b) => match self.vars_desc.get(v) {
                Some(def) => def.clone(),
                None => match b.fix(self.env) {
                    Some(val) => {
                        let new_var = Variable::integer().min(val).max(val);
                        if !new_var.checks_value(val) {
                            panic!("Variable {:?} has a non-integer fixed value! ({})", b, val);
                        }
                        new_var
                    }
                    None => panic!("Unknown unfixed variable!"),
                },
            },
        }
    }

    fn objectify_single_constraint(
        constraint: &Constraint<HashedProblemVar<D::Connection, V>>,
        origin: ConstraintDesc<D::Connection>,
        var: HashedProblemVar<D::Connection, V>,
    ) -> (
        Objective<HashedProblemVar<D::Connection, V>>,
        Vec<(
            Constraint<HashedProblemVar<D::Connection, V>>,
            ConstraintDesc<D::Connection>,
        )>,
    ) {
        match constraint.get_symbol() {
            EqSymbol::LessThan => {
                let var = LinExpr::var(var);
                let lin_expr = constraint.get_lhs().clone();
                let c1 = lin_expr.leq(&var);
                let c2 = var.geq(&LinExpr::constant(0.));
                let constraints = vec![(c1, origin.clone()), (c2, origin.clone())];
                let objective = Objective::new(var, ObjectiveSense::Minimize);
                (objective, constraints)
            }
            EqSymbol::Equals => {
                let var = LinExpr::var(var);
                let lin_expr = constraint.get_lhs().clone();
                let c1 = lin_expr.leq(&var);
                let c2 = lin_expr.geq(&(-&var));
                let constraints = vec![(c1, origin.clone()), (c2, origin.clone())];
                let objective = Objective::new(var, ObjectiveSense::Minimize);
                (objective, constraints)
            }
        }
    }

    /// Takes a list of constraints and generate a linear expression
    /// to optimize as an objective. Returns the objective and the
    /// necessary constraints to enforce define the helper variables.
    fn objectify_constraints<'b>(
        &mut self,
        mut constraints: impl ExactSizeIterator<
            Item = &'b Constraint<HashedProblemVar<D::Connection, V>>,
        >,
        origin: ConstraintDesc<D::Connection>,
    ) -> (
        Objective<HashedProblemVar<D::Connection, V>>,
        Vec<(
            Constraint<HashedProblemVar<D::Connection, V>>,
            ConstraintDesc<D::Connection>,
        )>,
    )
    where
        D::Connection: 'b,
        V: 'b,
    {
        // If there is no constraints, we can have a trivial linear expression
        if constraints.len() == 0 {
            let objective = Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize);
            return (objective, vec![]);
        }
        // With a single constraint, we can just defer to objectify_single_constraint
        if constraints.len() == 1 {
            let var = self.generate_helper_continuous_var();
            return Self::objectify_single_constraint(constraints.next().unwrap(), origin, var);
        }

        let c_count = constraints.len() as f64;

        let global_var = self.generate_helper_continuous_var();
        let global_var = LinExpr::var(global_var);
        let mut obj = Objective::new(c_count * global_var.clone(), ObjectiveSense::Minimize);
        let mut output = vec![];
        for constraint in constraints {
            let var = self.generate_helper_continuous_var();
            let lin_expr = LinExpr::var(var.clone());
            output.push((lin_expr.leq(&global_var), origin.clone()));
            let (c_obj, c_constraints) =
                Self::objectify_single_constraint(constraint, origin.clone(), var);
            obj = obj + c_obj;
            output.extend(c_constraints);
        }
        obj = (0.5 / c_count) * obj; // the global weight should be one
        (obj, output)
    }

    fn reify_single_constraint(
        &mut self,
        constraint: &Constraint<HashedProblemVar<D::Connection, V>>,
        origin: ConstraintDesc<D::Connection>,
        var: HashedProblemVar<D::Connection, V>,
    ) -> Vec<(
        Constraint<HashedProblemVar<D::Connection, V>>,
        ConstraintDesc<D::Connection>,
    )> {
        let vars = constraint.get_lhs().variables();
        // Handle special cases with 0 or 1 variable in the lin_expr.
        match vars.len() {
            0 => {
                // If there are no variables, we can simply check if the constraint is satisfied
                // and fix the variable accordingly
                let var = LinExpr::var(var);
                let c = if constraint.is_trivially_true() {
                    let one = LinExpr::constant(1.);
                    var.eq(&one)
                } else {
                    let zero = LinExpr::constant(0.);
                    var.eq(&zero)
                };
                return vec![(c, origin)];
            }
            1 => {
                let single_var = vars
                    .into_iter()
                    .next()
                    .expect("There is one variable in this branch");
                let var_type = self.get_variable_type(&single_var);

                // If the variable is binary, we can check if the constraint is satisfied in each case
                // and construct a corresponding matching constraint
                if var_type == Variable::binary() {
                    let f = |val: bool| {
                        let reduced = constraint.reduce(&HashMap::from([(
                            single_var.clone(),
                            if val { 1.0 } else { 0.0 },
                        )]));
                        reduced
                            .trivially_eval()
                            .expect("Constraint should be trivial")
                    };
                    let orig_var = LinExpr::var(single_var.clone());
                    let var = LinExpr::var(var);
                    let one = LinExpr::constant(1.);
                    let zero = LinExpr::constant(0.);
                    let c = match (f(true), f(false)) {
                        (true, true) => var.eq(&one),
                        (false, false) => var.eq(&zero),
                        (true, false) => var.eq(&orig_var),
                        (false, true) => var.eq(&(&one - &orig_var)),
                    };
                    return vec![(c, origin)];
                }
            }
            _ => {} // Generic case
        }

        match constraint.get_symbol() {
            EqSymbol::LessThan => {
                let lin_expr = constraint.get_lhs().clone();
                let range = lin_expr.compute_range_with(|v| Some(self.get_variable_type(v)));
                let min = *range.start();
                let max = *range.end();
                assert!(
                    min.is_finite() && max.is_finite(),
                    "Linear expression from ColloML should always have finite ranges. But this expression is unbounded: {:?} (found range: {:?})",
                    lin_expr,
                    range,
                );
                let one = LinExpr::constant(1.);
                let epsilon = LinExpr::constant(0.1);
                let var = LinExpr::var(var);
                let constraints = vec![
                    (
                        lin_expr.leq(&(max * (&one - &var) + &epsilon)),
                        origin.clone(),
                    ),
                    (lin_expr.geq(&((min - 1.) * &var + &one - &epsilon)), origin),
                ];
                constraints
            }
            EqSymbol::Equals => {
                // For equality, the constraint is lin_expr === 0
                // we turn that into lin_expr <== 0 && lin_expr >== 0
                // and combine the two reified variables
                let v1 = self.generate_helper_var();
                let v2 = self.generate_helper_var();
                let lin_expr = constraint.get_lhs().clone();
                let c1 = lin_expr.leq(&LinExpr::constant(0.));
                let c2 = lin_expr.geq(&LinExpr::constant(0.));
                let mut constraints = self.reify_single_constraint(&c1, origin.clone(), v1.clone());
                constraints.extend(self.reify_single_constraint(&c2, origin.clone(), v2.clone()));
                // Encode var as an AND between v1 and v2
                let v1 = LinExpr::var(v1);
                let v2 = LinExpr::var(v2);
                let var = LinExpr::var(var);
                constraints.push((var.leq(&v1), origin.clone()));
                constraints.push((var.leq(&v2), origin.clone()));
                constraints.push(((&v1 + &v2).leq(&(&var + &LinExpr::constant(1.))), origin));
                constraints
            }
        }
    }

    /// Takes a list of constraints and reify them into a single
    /// a binary variable. Returns the necessary constraints
    /// to enforce this.
    fn reify_constraint<'b>(
        &mut self,
        mut constraints: impl ExactSizeIterator<
            Item = &'b Constraint<HashedProblemVar<D::Connection, V>>,
        >,
        origin: ConstraintDesc<D::Connection>,
        var: HashedProblemVar<D::Connection, V>,
    ) -> Vec<(
        Constraint<HashedProblemVar<D::Connection, V>>,
        ConstraintDesc<D::Connection>,
    )>
    where
        D::Connection: 'b,
        V: 'b,
    {
        // If there is no constraints, they are always satisfied
        // and the variable should be always 1
        if constraints.len() == 0 {
            let var = LinExpr::var(var);
            return vec![(var.eq(&LinExpr::constant(1.)), origin)];
        }
        if constraints.len() == 1 {
            return self.reify_single_constraint(constraints.next().unwrap(), origin, var);
        }

        // We reify each constraint with helper variables
        let mut output = vec![];
        let mut helpers = vec![];

        for constraint in constraints {
            let helper = self.generate_helper_var();
            helpers.push(helper.clone());
            output.extend(self.reify_single_constraint(constraint, origin.clone(), helper));
        }

        // Now let's combine all the helper variables in an AND op
        let var = LinExpr::var(var);
        for helper in &helpers {
            let h = LinExpr::var(helper.clone());
            output.push((var.leq(&h), origin.clone()));
        }
        let rhs = var + LinExpr::constant((helpers.len() - 1) as f64);
        let mut lhs = LinExpr::constant(0.);
        for helper in helpers {
            let h = LinExpr::var(helper);
            lhs = lhs + h;
        }
        output.push((lhs.leq(&rhs), origin));

        output
    }

    fn clean_var(&self, var: &HashedIlpVar<D::Connection>) -> HashedProblemVar<D::Connection, V> {
        Hashed::new(match &**var {
            IlpVar::Base(extern_var) => {
                if self.builder.base_vars.contains_key(&extern_var.name) {
                    ProblemVar::Base(match extern_var.try_into() {
                        Ok(v) => v,
                        Err(e) => match e {
                            VarConversionError::Unknown(n) => {
                                panic!("Inconsistent EvalVar, cannot convert var name {}", n)
                            }
                            VarConversionError::WrongParameterCount {
                                name: _,
                                expected: _,
                                found: _,
                            } => {
                                panic!("Inconsistent EvalVar, cannot convert var: {}", e)
                            }
                            VarConversionError::WrongParameterType {
                                name: _,
                                param: _,
                                expected: _,
                            } => {
                                panic!("Inconsistent EvalVar, cannot convert var: {}", e)
                            }
                        },
                    })
                } else {
                    panic!(
                        "Undeclared variable {}: this should have been caught in the semantic analysis",
                        extern_var.name
                    );
                }
            }
            IlpVar::Script(ScriptVar {
                module,
                name,
                params,
                ..
            }) => ProblemVar::Reified(ReifiedVar {
                module: module.clone(),
                name: name.clone(),
                params: params.clone(),
            }),
        })
    }

    fn clean_constraint(
        &self,
        constraint: &Constraint<HashedIlpVar<D::Connection>>,
    ) -> Constraint<HashedProblemVar<D::Connection, V>> {
        constraint.transmute(|v| self.clean_var(v))
    }

    fn clean_lin_expr(
        &self,
        lin_expr: &LinExpr<HashedIlpVar<D::Connection>>,
    ) -> LinExpr<HashedProblemVar<D::Connection, V>> {
        lin_expr.transmute(|v| self.clean_var(v))
    }

    fn update_origin(
        origin: Option<crate::eval::Origin<D::Connection>>,
    ) -> ConstraintDesc<D::Connection> {
        let origin = origin.expect("All constraints should have an origin");
        ConstraintDesc::InScript { origin }
    }

    pub(crate) async fn new(
        builder: ProblemBuilder<D, V>,
        env: &'a V::Env,
        db_connection: Option<D::Connection>,
    ) -> Result<EvalData<'a, D, V>, ProblemError<D::Connection>> {
        // Phase 1: Evaluate all functions and collect results
        // We need to do this first because eval_history borrows self.ast
        let (constraint_results, objective_results, var_def) = {
            let mut eval_history = builder.ast.start_eval_history();

            let db_value = db_connection
                .map(|conn| ExprValue::Database(crate::eval::database::DatabaseHandle::new(conn)));

            // Evaluate constraints
            let mut constraint_results = Vec::new();
            for (module, fn_name, args, needs_db) in builder.pending_constraints.iter() {
                let eval_args = if *needs_db {
                    let db_val = db_value.as_ref().ok_or_else(|| {
                        ProblemError::MissingDatabaseConnection(format!("{}::{}", module, fn_name))
                    })?;
                    let fn_key = (module.to_string(), fn_name.to_string());
                    let fn_desc = builder.ast.global_env.get_functions().get(&fn_key).unwrap();
                    let wrapped = wrap_db_in_custom_layers::<D>(
                        db_val.clone(),
                        &fn_desc.typ.args[0],
                        &builder.ast.global_env,
                    )
                    .expect("DB type wrapping should succeed for validated function");
                    let mut full_args = vec![wrapped];
                    full_args.extend(args.clone());
                    full_args
                } else {
                    args.clone()
                };
                let result = eval_history
                    .eval_fn(module, fn_name, eval_args)
                    .await
                    .map_err(|e| match e {
                        EvalError::Panic(v) => ProblemError::Panic(v),
                        _ => panic!(
                            "Evaluation should succeed (function was validated): {:?}",
                            e
                        ),
                    })?;
                constraint_results.push((module.clone(), fn_name.clone(), result));
            }

            // Evaluate objectives
            let mut objective_results = Vec::new();
            for (module, fn_name, args, needs_db, coef, obj_sense) in
                builder.pending_objectives.iter()
            {
                let eval_args = if *needs_db {
                    let db_val = db_value.as_ref().ok_or_else(|| {
                        ProblemError::MissingDatabaseConnection(format!("{}::{}", module, fn_name))
                    })?;
                    let fn_key = (module.to_string(), fn_name.to_string());
                    let fn_desc = builder.ast.global_env.get_functions().get(&fn_key).unwrap();
                    let wrapped = wrap_db_in_custom_layers::<D>(
                        db_val.clone(),
                        &fn_desc.typ.args[0],
                        &builder.ast.global_env,
                    )
                    .expect("DB type wrapping should succeed for validated function");
                    let mut full_args = vec![wrapped];
                    full_args.extend(args.clone());
                    full_args
                } else {
                    args.clone()
                };
                let result = eval_history
                    .eval_fn(module, fn_name, eval_args)
                    .await
                    .map_err(|e| match e {
                        EvalError::Panic(v) => ProblemError::Panic(v),
                        _ => panic!("Evaluation should succeed (function was validated)"),
                    })?;
                objective_results.push((
                    module.clone(),
                    fn_name.clone(),
                    result,
                    *coef,
                    *obj_sense,
                ));
            }

            let var_def = eval_history.into_var_def().await.map_err(|e| match e {
                EvalError::Panic(v) => ProblemError::Panic(v),
                _ => panic!(
                    "Evaluation should succeed (variables were validated): {:?}",
                    e
                ),
            })?;
            (constraint_results, objective_results, var_def)
        };

        let original_var_list: HashMap<V, Variable> = V::vars(env).into_iter().collect();
        for (name, desc) in &original_var_list {
            if !desc.is_integer() {
                return Err(ProblemError::NonIntegerVariable(format!("{:?}", name)));
            }
        }

        let vars_desc = original_var_list
            .iter()
            .map(|(name, desc)| (Hashed::new(ProblemVar::Base(name.clone())), desc.clone()))
            .collect();

        let mut eval_data = EvalData {
            builder,
            env,
            constraints: vec![],
            objective: Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize),
            current_helper_id: 0,
            vars_desc,
            original_var_list,
        };

        // Phase 2: Process constraint results
        for (module, fn_name, (constraints_expr, _origin)) in constraint_results {
            let constraints = match constraints_expr {
                ExprValue::Constraint(constraints) => constraints,
                ExprValue::List(list)
                    if list
                        .iter()
                        .all(|x| matches!(&**x, ExprValue::Constraint(_))) =>
                {
                    list.into_iter()
                        .flat_map(|x| match Arc::unwrap_or_clone(x) {
                            ExprValue::Constraint(constraints) => constraints.into_iter(),
                            _ => unreachable!(),
                        })
                        .collect()
                }
                _ => panic!(
                    "Function {}::{} returned {:?} instead of Constraint",
                    module, fn_name, constraints_expr
                ),
            };

            let new_constraints: Vec<_> = constraints
                .into_iter()
                .map(|c_with_o| {
                    (
                        eval_data.clean_constraint(&c_with_o.constraint),
                        Self::update_origin(c_with_o.origin),
                    )
                })
                .collect();
            eval_data.constraints.extend(new_constraints);
        }

        // Phase 3: Process objective results
        for (module, fn_name, (fn_result, origin), coef, obj_sense) in objective_results {
            let mut values_list: Vec<ExprValue<D::Connection>> = vec![];
            match fn_result {
                ExprValue::LinExpr(lin_expr) => values_list.push(ExprValue::LinExpr(lin_expr)),
                ExprValue::Constraint(constraint) => {
                    values_list.push(ExprValue::Constraint(constraint))
                }
                ExprValue::List(list) => {
                    values_list.extend(list.into_iter().map(Arc::unwrap_or_clone))
                }
                _ => panic!(
                    "Function {}::{} returned {:?} instead of LinExpr",
                    module, fn_name, fn_result
                ),
            }

            let mut obj = Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize);
            for value in values_list {
                match value {
                    ExprValue::LinExpr(lin_expr) => {
                        let cleaned_lin_expr = eval_data.clean_lin_expr(&lin_expr);
                        obj = obj + Objective::new(cleaned_lin_expr, obj_sense);
                    }
                    ExprValue::Constraint(c) => {
                        let cleaned_constraints: Vec<_> = c
                            .into_iter()
                            .map(|c_with_o| eval_data.clean_constraint(&c_with_o.constraint))
                            .collect();
                        let new_origin = ConstraintDesc::Objectify {
                            origin: origin.clone(),
                        };
                        let (new_obj, new_constraints) =
                            eval_data.objectify_constraints(cleaned_constraints.iter(), new_origin);
                        obj = obj + new_obj;
                        eval_data.constraints.extend(new_constraints);
                    }
                    _ => panic!(
                        "Function {}::{} returned {:?} instead of LinExpr",
                        module, fn_name, value
                    ),
                }
            }
            eval_data.objective = &eval_data.objective + coef.0 * obj;
        }

        // Phase 4: Process reified variables
        let mut constraints_to_reify = HashMap::<
            HashedProblemVar<D::Connection, V>,
            (
                Vec<Constraint<HashedProblemVar<D::Connection, V>>>,
                crate::eval::Origin<D::Connection>,
            ),
        >::new();

        for ((var_module, var_name, var_args), (constraints, new_origin)) in var_def.vars {
            let cleaned_constraints: Vec<_> = constraints
                .into_iter()
                .map(|c: Constraint<HashedIlpVar<D::Connection>>| eval_data.clean_constraint(&c))
                .collect();

            let reified_var = ReifiedVar {
                module: var_module,
                name: var_name,
                params: var_args,
            };
            let new_var = Hashed::new(ProblemVar::Reified(reified_var));

            eval_data
                .vars_desc
                .insert(new_var.clone(), Variable::binary());
            constraints_to_reify.insert(new_var, (cleaned_constraints, new_origin));
        }
        // Phase 5: Reify constraints
        for (var, (constraints, origin)) in constraints_to_reify {
            let var_name = match &*var {
                ProblemVar::Reified(ReifiedVar {
                    module: _,
                    name,
                    params: _,
                }) => name.clone(),
                _ => panic!("Unexpected variable type to reify: {:?}", var),
            };

            let new_origin = ConstraintDesc::Reified { var_name, origin };

            let reified_constraints =
                eval_data.reify_constraint(constraints.iter(), new_origin, var);

            eval_data.constraints.extend(reified_constraints);
        }

        Ok(eval_data)
    }
}

/// Wrap a database ExprValue in Custom type layers to match a declared parameter type.
/// Recursively resolves Custom types until reaching DatabaseSchema, then wraps on the way back up.
fn wrap_db_in_custom_layers<D: DatabaseDriver>(
    db_value: ExprValue<D::Connection>,
    declared_type: &ExprType,
    global_env: &crate::semantics::GlobalEnv<D>,
) -> Option<ExprValue<D::Connection>> {
    let variants: Vec<_> = declared_type.get_variants().iter().cloned().collect();
    if variants.len() != 1 {
        return None;
    }
    match &variants[0] {
        SimpleType::DatabaseSchema(_) => Some(db_value),
        SimpleType::Custom(module, root, variant) => {
            let qualified = match variant {
                Some(v) => format!("{}::{}", root, v),
                None => root.clone(),
            };
            let underlying = global_env.get_custom_type_underlying(module, &qualified)?;
            let inner = wrap_db_in_custom_layers::<D>(db_value, underlying, global_env)?;
            Some(ExprValue::Custom(CustomValue {
                module: module.clone(),
                type_name: root.clone(),
                variant: variant.clone(),
                content: Arc::new(inner),
            }))
        }
        _ => None,
    }
}
