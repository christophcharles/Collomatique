use crate::eval::{
    CheckedAST, CompileError, EvalError, ExprValue, ExternVar, IlpVar, Origin, ScriptVar,
};
use crate::semantics::ArgsType;
use crate::traits::{FieldConversionError, VarConversionError};
use crate::{EvalObject, EvalVar, ExprType, SemWarning, SimpleType};
use collomatique_ilp::linexpr::EqSymbol;
use collomatique_ilp::solvers::Solver;
use collomatique_ilp::{ConfigData, Constraint, LinExpr, Objective, ObjectiveSense, Variable};
use std::collections::{BTreeMap, HashMap};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ReifiedVar<T: EvalObject> {
    module: String,
    name: String,
    from_list: Option<usize>,
    params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ProblemVar<T: EvalObject, V: EvalVar<T>> {
    Base(V),
    Reified(ReifiedVar<T>),
    Helper(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintDesc<T: EvalObject> {
    Reified { var_name: String, origin: Origin<T> },
    InScript { origin: Origin<T> },
    Objectify { origin: Origin<T> },
}

pub struct ProblemBuilder<
    T: EvalObject,
    V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
> {
    /// Compiled AST (all modules compiled together)
    ast: CheckedAST<T>,

    /// Variables that define the problem
    /// The set of possible values of these variables is one-to-one with
    /// the set of solutions to our problem
    base_vars: HashMap<String, ArgsType>,

    /// Pending constraint function calls (validated but not yet evaluated)
    /// Format: (module, fn_name, args)
    pending_constraints: Vec<(String, String, Vec<ExprValue<T>>)>,

    /// Pending objective function calls (validated but not yet evaluated)
    /// Format: (module, fn_name, args, coefficient, sense)
    pending_objectives: Vec<(String, String, Vec<ExprValue<T>>, f64, ObjectiveSense)>,

    phantom: std::marker::PhantomData<V>,
}

struct EvalData<
    'a,
    T: EvalObject,
    V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
> {
    builder: ProblemBuilder<T, V>,

    /// Reference to the evaluation environment
    env: &'a T::Env,

    /// List of constraints incrementally built (populated during build())
    constraints: Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>,

    /// Objective function (populated during build())
    objective: Objective<ProblemVar<T, V>>,

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
    vars_desc: BTreeMap<ProblemVar<T, V>, Variable>,

    /// base variables list
    original_var_list: BTreeMap<V, Variable>,
}

use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum ProblemError<T: EvalObject> {
    #[error("Variable {0} has non-integer type")]
    NonIntegerVariable(String),
    #[error("TypeId {0:?} from EvalVar cannot be represented with EvalObject")]
    EvalVarIncompatibleWithEvalObject(std::any::TypeId),
    #[error("Function \"{0}\" was not found in script (maybe it is not public?)")]
    UnknownFunction(String),
    #[error("Function \"{func}\" expects {expected} arguments but got {found}")]
    ArgumentCountMismatch {
        func: String,
        expected: usize,
        found: usize,
    },
    #[error("Value \"{0}\" is invalid")]
    InvalidExprValue(String),
    #[error("Variable \"{0}\" is already defined")]
    VariableAlreadyDefined(String),
    #[error(transparent)]
    CompileError(#[from] CompileError),
    #[error("Function {func} returns {returned} instead of {expected}")]
    WrongReturnType {
        func: String,
        returned: ExprType,
        expected: ExprType,
    },
    #[error("Function {func} has returned {returned:?} instead of type {expected}")]
    UnexpectedReturnValue {
        func: String,
        returned: ExprValue<T>,
        expected: ExprType,
    },
    #[error("Panic: {0}")]
    Panic(Box<ExprValue<T>>),
}

impl<
        T: EvalObject,
        V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > ProblemBuilder<T, V>
{
    fn build_vars() -> Result<HashMap<String, Vec<ExprType>>, ProblemError<T>> {
        V::field_schema()
            .into_iter()
            .map(|(name, typ)| {
                Ok((
                    name,
                    typ.into_iter()
                        .map(|x| Ok(x.convert_to_expr_type::<T>()?))
                        .collect::<Result<_, _>>()
                        .map_err(|e| match e {
                            FieldConversionError::UnknownTypeId(type_id) => {
                                ProblemError::EvalVarIncompatibleWithEvalObject(type_id)
                            }
                        })?,
                ))
            })
            .collect::<Result<_, _>>()
    }

    /// Validate that a function exists with the correct signature
    fn validate_function(
        &self,
        module: &str,
        fn_name: &str,
        args: &[ExprValue<T>],
        expected_return: &ExprType,
    ) -> Result<(), ProblemError<T>> {
        let functions = self.ast.get_functions();
        let key = (module.to_string(), fn_name.to_string());

        let (args_type, output_type) = functions
            .get(&key)
            .ok_or_else(|| ProblemError::UnknownFunction(format!("{}::{}", module, fn_name)))?;

        // Check argument count
        if args_type.len() != args.len() {
            return Err(ProblemError::ArgumentCountMismatch {
                func: format!("{}::{}", module, fn_name),
                expected: args_type.len(),
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

        Ok(())
    }
}

impl<
        'a,
        T: EvalObject,
        V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > EvalData<'a, T, V>
{
    fn generate_helper_var(&mut self) -> ProblemVar<T, V> {
        let new_var = ProblemVar::Helper(self.current_helper_id);
        self.vars_desc
            .insert(new_var.clone(), collomatique_ilp::Variable::binary());
        self.current_helper_id += 1;
        new_var
    }

    fn generate_helper_continuous_var(&mut self) -> ProblemVar<T, V> {
        let new_var = ProblemVar::Helper(self.current_helper_id);
        self.vars_desc
            .insert(new_var.clone(), collomatique_ilp::Variable::continuous());
        self.current_helper_id += 1;
        new_var
    }

    fn get_variable_type(&self, v: &ProblemVar<T, V>) -> Variable {
        match v {
            ProblemVar::Helper(_) | ProblemVar::Reified(_) => Variable::binary(),
            ProblemVar::Base(b) => match self.vars_desc.get(v) {
                Some(def) => def.clone(),
                None => match b.fix(&self.env) {
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
        constraint: &Constraint<ProblemVar<T, V>>,
        origin: ConstraintDesc<T>,
        var: ProblemVar<T, V>,
    ) -> (
        Objective<ProblemVar<T, V>>,
        Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>,
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
        mut constraints: impl ExactSizeIterator<Item = &'b Constraint<ProblemVar<T, V>>>,
        origin: ConstraintDesc<T>,
    ) -> (
        Objective<ProblemVar<T, V>>,
        Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>,
    )
    where
        T: 'b,
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
        constraint: &Constraint<ProblemVar<T, V>>,
        origin: ConstraintDesc<T>,
        var: ProblemVar<T, V>,
    ) -> Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)> {
        use collomatique_ilp::linexpr::EqSymbol;

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
                        let reduced = constraint.reduce(&BTreeMap::from([(
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
        mut constraints: impl ExactSizeIterator<Item = &'b Constraint<ProblemVar<T, V>>>,
        origin: ConstraintDesc<T>,
        var: ProblemVar<T, V>,
    ) -> Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>
    where
        T: 'b,
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

    fn clean_var(&self, var: &IlpVar<T>) -> ProblemVar<T, V> {
        match var {
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
                    panic!("Undeclared variable {}: this should have been caught in the semantic analysis", extern_var.name);
                }
            }
            IlpVar::Script(ScriptVar {
                module,
                name,
                from_list,
                params,
                ..
            }) => ProblemVar::Reified(ReifiedVar {
                module: module.clone(),
                name: name.clone(),
                from_list: from_list.clone(),
                params: params.clone(),
            }),
        }
    }

    fn clean_constraint(&self, constraint: &Constraint<IlpVar<T>>) -> Constraint<ProblemVar<T, V>> {
        constraint.transmute(|v| self.clean_var(v))
    }

    fn clean_lin_expr(&self, lin_expr: &LinExpr<IlpVar<T>>) -> LinExpr<ProblemVar<T, V>> {
        lin_expr.transmute(|v| self.clean_var(v))
    }

    fn update_origin(origin: Option<Origin<T>>) -> ConstraintDesc<T> {
        let origin = origin.expect("All constraints should have an origin");
        ConstraintDesc::InScript { origin }
    }

    fn new(
        builder: ProblemBuilder<T, V>,
        env: &'a T::Env,
    ) -> Result<EvalData<'a, T, V>, ProblemError<T>> {
        // Phase 1: Evaluate all functions and collect results
        // We need to do this first because eval_history borrows self.ast
        let (constraint_results, objective_results, var_def) = {
            let mut eval_history = builder
                .ast
                .start_eval_history_with_cache(env, T::Cache::default())
                .expect("Environment should be compatible with AST");

            // Evaluate constraints
            let constraint_results = builder
                .pending_constraints
                .iter()
                .map(|(module, fn_name, args)| {
                    let result = eval_history
                        .eval_fn(module, fn_name, args.clone())
                        .map_err(|e| match e {
                            EvalError::Panic(v) => ProblemError::Panic(v),
                            _ => panic!("Evaluation should succeed (function was validated)"),
                        })?;
                    Ok((module.clone(), fn_name.clone(), result))
                })
                .collect::<Result<Vec<_>, ProblemError<T>>>()?;

            // Evaluate objectives
            let objective_results = builder
                .pending_objectives
                .iter()
                .map(|(module, fn_name, args, coef, obj_sense)| {
                    let result = eval_history
                        .eval_fn(module, fn_name, args.clone())
                        .map_err(|e| match e {
                            EvalError::Panic(v) => ProblemError::Panic(v),
                            _ => panic!("Evaluation should succeed (function was validated)"),
                        })?;
                    Ok((
                        module.clone(),
                        fn_name.clone(),
                        result,
                        *coef,
                        obj_sense.clone(),
                    ))
                })
                .collect::<Result<Vec<_>, ProblemError<T>>>()?;

            let var_def = eval_history.into_var_def();
            (constraint_results, objective_results, var_def)
        };

        let original_var_list =
            V::vars(env).map_err(|id| ProblemError::EvalVarIncompatibleWithEvalObject(id))?;
        for (name, desc) in &original_var_list {
            if !desc.is_integer() {
                return Err(ProblemError::NonIntegerVariable(format!("{:?}", name)));
            }
        }

        let vars_desc = original_var_list
            .iter()
            .map(|(name, desc)| (ProblemVar::Base(name.clone()), desc.clone()))
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
                    if list.iter().all(|x| matches!(x, ExprValue::Constraint(_))) =>
                {
                    list.into_iter()
                        .flat_map(|x| match x {
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
            let mut values_list = vec![];
            match fn_result {
                ExprValue::LinExpr(lin_expr) => values_list.push(ExprValue::LinExpr(lin_expr)),
                ExprValue::Constraint(constraint) => {
                    values_list.push(ExprValue::Constraint(constraint))
                }
                ExprValue::List(list) => values_list.extend(list),
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
                        obj = obj + Objective::new(cleaned_lin_expr, obj_sense.clone());
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
            eval_data.objective = &eval_data.objective + coef * obj;
        }

        // Phase 4: Process reified variables
        let mut constraints_to_reify =
            BTreeMap::<ProblemVar<T, V>, (Vec<Constraint<ProblemVar<T, V>>>, Origin<T>)>::new();

        for ((var_module, var_name, var_args), (constraints, new_origin)) in var_def.vars {
            let cleaned_constraints: Vec<_> = constraints
                .into_iter()
                .map(|c: Constraint<IlpVar<T>>| eval_data.clean_constraint(&c))
                .collect();

            let reified_var = ReifiedVar {
                module: var_module,
                name: var_name,
                from_list: None,
                params: var_args,
            };
            let new_var = ProblemVar::Reified(reified_var);

            eval_data
                .vars_desc
                .insert(new_var.clone(), Variable::binary());
            constraints_to_reify.insert(new_var, (cleaned_constraints, new_origin));
        }
        for ((var_list_module, var_list_name, var_list_args), (constraints_list, new_origin)) in
            var_def.var_lists
        {
            for (i, constraints) in constraints_list.into_iter().enumerate() {
                let cleaned_constraints: Vec<_> = constraints
                    .into_iter()
                    .map(|c| eval_data.clean_constraint(&c))
                    .collect();

                let reified_var = ReifiedVar {
                    module: var_list_module.clone(),
                    name: var_list_name.clone(),
                    from_list: Some(i),
                    params: var_list_args.clone(),
                };
                let new_var = ProblemVar::Reified(reified_var);

                eval_data
                    .vars_desc
                    .insert(new_var.clone(), Variable::binary());
                constraints_to_reify.insert(new_var, (cleaned_constraints, new_origin.clone()));
            }
        }

        // Phase 5: Reify constraints
        for (var, (constraints, origin)) in constraints_to_reify {
            let var_name = match &var {
                ProblemVar::Reified(ReifiedVar {
                    module: _,
                    name,
                    from_list: _,
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

impl<
        T: EvalObject,
        V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > ProblemBuilder<T, V>
{
    pub fn new(modules: &BTreeMap<&str, &str>) -> Result<Self, ProblemError<T>> {
        let base_vars = Self::build_vars()?;

        // Compile all modules upfront
        let ast = CheckedAST::new(modules, base_vars.clone())?;

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

    /// Add a constraint function to be evaluated at build time.
    ///
    /// Validates that the function exists and has the correct signature,
    /// but does not evaluate it yet.
    pub fn add_constraint(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<T>>,
    ) -> Result<(), ProblemError<T>> {
        // Validate function exists and has correct signature
        // Constraints can return Constraint or [Constraint]
        let constraint_type = ExprType::from_variants([
            SimpleType::Constraint,
            SimpleType::List(SimpleType::Constraint.into()),
        ]);

        self.validate_function(module, fn_name, &args, &constraint_type)?;

        // Store for later evaluation
        self.pending_constraints
            .push((module.to_string(), fn_name.to_string(), args));
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
        args: Vec<ExprValue<T>>,
        coefficient: f64,
        sense: ObjectiveSense,
    ) -> Result<(), ProblemError<T>> {
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

        self.validate_function(module, fn_name, &args, &obj_types)?;

        // Store for later evaluation
        self.pending_objectives.push((
            module.to_string(),
            fn_name.to_string(),
            args,
            coefficient,
            sense,
        ));
        Ok(())
    }

    pub fn build(self, env: &T::Env) -> Result<Problem<T, V>, ProblemError<T>> {
        // Evaluate all pending constraints and objectives
        let mut eval_data = EvalData::new(self, env)?;

        for (constraint, _desc) in eval_data.constraints.iter_mut() {
            let mut fixed_variables = BTreeMap::new();
            for var in constraint.variables() {
                if fixed_variables.contains_key(&var) {
                    continue;
                }
                let ProblemVar::Base(v) = var else {
                    continue;
                };
                let Some(value) = v.fix(&eval_data.env) else {
                    continue;
                };
                fixed_variables.insert(ProblemVar::Base(v), value);
            }
            if fixed_variables.is_empty() {
                continue;
            }
            *constraint = constraint.reduce(&fixed_variables);
        }
        let mut fixed_variables = BTreeMap::new();
        for var in eval_data.objective.get_function().variables() {
            if fixed_variables.contains_key(&var) {
                continue;
            }
            let ProblemVar::Base(v) = var else {
                continue;
            };
            let Some(value) = v.fix(&eval_data.env) else {
                continue;
            };
            fixed_variables.insert(ProblemVar::Base(v), value);
        }
        if !fixed_variables.is_empty() {
            eval_data.objective = eval_data.objective.reduce(&fixed_variables);
        }
        eval_data.constraints = eval_data
            .constraints
            .into_iter()
            .filter(|(c, _d)| !c.is_trivially_true())
            .collect();

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

        Ok(Problem {
            problem: problem_builder.build().expect("Problem should be valid"),
            reification_problem_builder,
            original_var_list: eval_data.original_var_list,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtraDesc<T: EvalObject, V: EvalVar<T>> {
    Orig(ConstraintDesc<T>),
    InitCond(V),
}

#[derive(Debug, Clone)]
pub struct Problem<T: EvalObject, V: EvalVar<T>> {
    problem: collomatique_ilp::Problem<ProblemVar<T, V>, ConstraintDesc<T>>,
    reification_problem_builder:
        collomatique_ilp::ProblemBuilder<ProblemVar<T, V>, ExtraDesc<T, V>>,
    original_var_list: BTreeMap<V, Variable>,
}

impl<T: EvalObject, V: EvalVar<T>> Problem<T, V> {
    pub fn get_inner_problem(
        &self,
    ) -> &collomatique_ilp::Problem<ProblemVar<T, V>, ConstraintDesc<T>> {
        &self.problem
    }

    pub fn solve<
        'a,
        S: Solver<ProblemVar<T, V>, ConstraintDesc<T>, DefaultRepr<ProblemVar<T, V>>>,
    >(
        &'a self,
        solver: &S,
    ) -> Option<FeasableSolution<'a, T, V>> {
        solver
            .solve(&self.problem)
            .map(|x| FeasableSolution { feasable_config: x })
    }

    pub fn solution_from_data<
        'a,
        S: Solver<ProblemVar<T, V>, ExtraDesc<T, V>, DefaultRepr<ProblemVar<T, V>>>,
    >(
        &'a self,
        config_data: &ConfigData<V>,
        solver: &S,
    ) -> Option<Solution<'a, T, V>> {
        if !self.check_no_missing_variables(config_data) {
            return None;
        }

        let reification_problem = self
            .reification_problem_builder
            .clone()
            .add_constraints(Self::build_equality_constraints_from_config_data(
                config_data,
            ))
            .build()
            .expect("The reification problem should always be valid");
        let reification_sol = solver
            .solve(&reification_problem)
            .expect("There should always be a (unique!) solution to the reification problem");
        let inner_data = reification_sol.get_values();
        let new_config_data = ConfigData::from(inner_data);

        Some(
            self.solution_from_complete_data(new_config_data)
                .expect("The configuration data should be valid!"),
        )
    }

    pub fn solution_from_complete_data<'a>(
        &'a self,
        config_data: ConfigData<ProblemVar<T, V>>,
    ) -> Option<Solution<'a, T, V>> {
        Some(Solution {
            config: self.problem.build_config(config_data).ok()?,
        })
    }
}

impl<T: EvalObject, V: EvalVar<T>> Problem<T, V> {
    fn build_equality_constraints_from_config_data(
        config_data: &ConfigData<V>,
    ) -> impl Iterator<Item = (Constraint<ProblemVar<T, V>>, ExtraDesc<T, V>)> {
        config_data.get_values().into_iter().map(|(var, value)| {
            let var_expr = LinExpr::var(ProblemVar::Base(var.clone()));
            let value_expr = LinExpr::constant(value);
            let constraint = var_expr.eq(&value_expr);
            let desc = ExtraDesc::InitCond(var);
            (constraint, desc)
        })
    }

    fn check_variables_valid(&self, config_data: &ConfigData<V>) -> bool {
        config_data
            .get_values()
            .keys()
            .all(|x| self.original_var_list.contains_key(x))
    }

    fn check_no_missing_variables(&self, config_data: &ConfigData<V>) -> bool {
        if !self.check_variables_valid(config_data) {
            return false;
        }

        self.original_var_list
            .iter()
            .all(|(var, var_def)| match config_data.get(var.clone()) {
                Some(v) => var_def.checks_value(v),
                None => false,
            })
    }
}

use collomatique_ilp::DefaultRepr;

#[derive(Debug, Clone)]
pub struct Solution<'a, T: EvalObject, V: EvalVar<T>> {
    config: collomatique_ilp::Config<
        'a,
        ProblemVar<T, V>,
        ConstraintDesc<T>,
        DefaultRepr<ProblemVar<T, V>>,
    >,
}

impl<'a, T: EvalObject, V: EvalVar<T>> Solution<'a, T, V> {
    pub fn get_data(&self) -> ConfigData<V> {
        ConfigData::from(self.config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                ProblemVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemVar<T, V>> {
        ConfigData::from(self.config.get_values())
    }

    pub fn is_feasable(&self) -> bool {
        self.config.is_feasable()
    }

    pub fn into_feasable(self) -> Option<FeasableSolution<'a, T, V>> {
        Some(FeasableSolution {
            feasable_config: self.config.into_feasable()?,
        })
    }

    pub fn blame(&self) -> Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)> {
        self.config.blame().cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub struct FeasableSolution<'a, T: EvalObject, V: EvalVar<T>> {
    feasable_config: collomatique_ilp::FeasableConfig<
        'a,
        ProblemVar<T, V>,
        ConstraintDesc<T>,
        DefaultRepr<ProblemVar<T, V>>,
    >,
}

impl<'a, T: EvalObject, V: EvalVar<T>> FeasableSolution<'a, T, V> {
    pub fn into_solution(self) -> Solution<'a, T, V> {
        Solution {
            config: self.feasable_config.into_inner(),
        }
    }

    pub fn get_data(&self) -> ConfigData<V> {
        ConfigData::from(self.feasable_config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                ProblemVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemVar<T, V>> {
        ConfigData::from(self.feasable_config.get_values())
    }
}
