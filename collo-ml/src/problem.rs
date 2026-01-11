use crate::eval::{
    CompileError, EvalError, EvalHistory, ExprValue, ExternVar, IlpVar, Origin, ScriptVar,
};
use crate::semantics::ArgsType;
use crate::traits::{FieldConversionError, VarConversionError};
use crate::{EvalObject, EvalVar, ExprType, SemWarning, SimpleType};
use collomatique_ilp::linexpr::EqSymbol;
use collomatique_ilp::solvers::Solver;
use collomatique_ilp::{ConfigData, Constraint, LinExpr, Objective, ObjectiveSense, Variable};
use std::collections::{BTreeMap, BTreeSet, HashMap};

mod scripts;
pub use scripts::{Script, ScriptRef, StoredScript};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ReifiedPublicVar<T: EvalObject> {
    name: String,
    params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ReifiedPrivateVar<T: EvalObject> {
    script_ref: ScriptRef,
    name: String,
    from_list: Option<usize>,
    params: Vec<ExprValue<T>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ProblemVar<T: EvalObject, V: EvalVar<T>> {
    Base(V),
    ReifiedPublic(ReifiedPublicVar<T>),
    ReifiedPrivate(ReifiedPrivateVar<T>),
    Helper(u64),
}

struct ReifiedVarDesc {
    func: String,
    args: ArgsType,
    script_ref: ScriptRef,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintDesc<T: EvalObject> {
    Reified {
        script_ref: ScriptRef,
        var_name: String,
        origin: Origin<T>,
    },
    InScript {
        script_ref: ScriptRef,
        origin: Origin<T>,
    },
    Objectify {
        script_ref: ScriptRef,
        origin: Origin<T>,
    },
}

pub struct ProblemBuilder<
    'a,
    T: EvalObject,
    V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
> {
    /// Reference to the evaluation environment
    env: &'a T::Env,

    /// Shared cache for evaluation
    cache: Option<T::Cache>,

    /// Variables that define the problem
    /// The set of possible values of these variables is one-to-one with
    /// the set of solutions to our problem
    base_vars: HashMap<String, ArgsType>,

    /// Public reified variables defined from scripts
    reified_vars: HashMap<String, ReifiedVarDesc>,

    /// Scripts stored for future evaluation
    /// These scripts contain the definition of the public reified variables
    /// They must be stored as the precise value of the constraints
    /// depend on the call arguments of the reified variable.
    stored_scripts: Vec<StoredScript<T>>,

    /// List of constraints incrementally built
    constraints: Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>,

    /// List of public reified variable that we have already
    /// evaluated and are already defined in the constraints set.
    ///
    /// If such a variable is needed a second time, we don't need to
    /// reevaluate
    called_public_reified_variables: BTreeSet<ReifiedPublicVar<T>>,

    /// Objective function
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
    /// Then reified variables (public and private) as well as
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
    #[error("Script already used for reified variables")]
    ScriptAlreadyUsed(Script),
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
        'a,
        T: EvalObject,
        V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > ProblemBuilder<'a, T, V>
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

    fn generate_current_vars(&self) -> HashMap<String, ArgsType> {
        self.base_vars
            .iter()
            .map(|(n, a)| (n.clone(), a.clone()))
            .chain(
                self.reified_vars
                    .iter()
                    .map(|(name, desc)| (name.clone(), desc.args.clone())),
            )
            .collect()
    }

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
            ProblemVar::Helper(_)
            | ProblemVar::ReifiedPrivate(_)
            | ProblemVar::ReifiedPublic(_) => Variable::binary(),
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
        &mut self,
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
            return self.objectify_single_constraint(constraints.next().unwrap(), origin, var);
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
                self.objectify_single_constraint(constraint, origin.clone(), var);
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

    fn clean_var(&self, script_ref: &ScriptRef, var: &IlpVar<T>) -> ProblemVar<T, V> {
        match var {
            IlpVar::Base(extern_var) => {
                if self.base_vars.contains_key(&extern_var.name) {
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
                    if !self.reified_vars.contains_key(&extern_var.name) {
                        panic!("Undeclared variable {}: this should have been caught in the semantic analysis", extern_var.name);
                    }

                    ProblemVar::ReifiedPublic(ReifiedPublicVar {
                        name: extern_var.name.clone(),
                        params: extern_var.params.clone(),
                    })
                }
            }
            IlpVar::Script(ScriptVar {
                name,
                from_list,
                params,
                ..
            }) => ProblemVar::ReifiedPrivate(ReifiedPrivateVar {
                script_ref: script_ref.clone(),
                name: name.clone(),
                from_list: from_list.clone(),
                params: params.clone(),
            }),
        }
    }

    fn clean_constraint(
        &self,
        script_ref: &ScriptRef,
        constraint: &Constraint<IlpVar<T>>,
    ) -> Constraint<ProblemVar<T, V>> {
        constraint.transmute(|v| self.clean_var(script_ref, v))
    }

    fn clean_lin_expr(
        &self,
        script_ref: &ScriptRef,
        lin_expr: &LinExpr<IlpVar<T>>,
    ) -> LinExpr<ProblemVar<T, V>> {
        lin_expr.transmute(|v| self.clean_var(script_ref, v))
    }

    fn update_origin(origin: Option<Origin<T>>, script_ref: ScriptRef) -> ConstraintDesc<T> {
        let origin = origin.expect("All constraints should have an origin");
        ConstraintDesc::InScript { script_ref, origin }
    }

    fn eval_constraint_in_history<'b>(
        &self,
        script_ref: &ScriptRef,
        eval_history: &mut EvalHistory<'b, T>,
        fn_name: &str,
        args: &Vec<ExprValue<T>>,
        allow_list: bool,
    ) -> Result<
        (
            Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>,
            Origin<T>,
        ),
        ProblemError<T>,
    > {
        let (constraints_expr, origin) =
            eval_history
                .eval_fn(fn_name, args.clone())
                .map_err(|e| match e {
                    EvalError::ArgumentCountMismatch {
                        identifier,
                        expected,
                        found,
                    } => ProblemError::ArgumentCountMismatch {
                        func: identifier,
                        expected,
                        found,
                    },
                    EvalError::InvalidExprValue { param } => {
                        ProblemError::InvalidExprValue(format!("{:?}", args[param]))
                    }
                    EvalError::UnknownFunction(func) => ProblemError::UnknownFunction(func),
                    EvalError::Panic(value) => ProblemError::Panic(value),
                    _ => panic!("Unexpected error: {:?}", e),
                })?;

        let constraints = match constraints_expr {
            ExprValue::Constraint(constraints) => constraints,
            ExprValue::List(list)
                if list.iter().all(|x| matches!(x, ExprValue::Constraint(_))) && allow_list =>
            {
                list.into_iter()
                    .flat_map(|x| match x {
                        ExprValue::Constraint(constraints) => constraints.into_iter(),
                        _ => panic!(
                            "This should be unreachable, we only have constraints at this point"
                        ),
                    })
                    .collect()
            }
            _ => {
                return Err(ProblemError::UnexpectedReturnValue {
                    func: fn_name.to_string(),
                    returned: constraints_expr,
                    expected: SimpleType::Constraint.into(),
                })
            }
        };

        Ok((
            constraints
                .into_iter()
                .map(|c_with_o| {
                    (
                        self.clean_constraint(script_ref, &c_with_o.constraint),
                        Self::update_origin(c_with_o.origin, script_ref.clone()),
                    )
                })
                .collect(),
            origin,
        ))
    }

    fn eval_obj_in_history<'b>(
        &mut self,
        script_ref: &ScriptRef,
        eval_history: &mut EvalHistory<'b, T>,
        fn_name: &str,
        args: &Vec<ExprValue<T>>,
        obj_sense: &ObjectiveSense,
    ) -> Result<
        (
            Objective<ProblemVar<T, V>>,
            Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)>,
        ),
        ProblemError<T>,
    > {
        let (fn_result, origin) =
            eval_history
                .eval_fn(fn_name, args.clone())
                .map_err(|e| match e {
                    EvalError::ArgumentCountMismatch {
                        identifier,
                        expected,
                        found,
                    } => ProblemError::ArgumentCountMismatch {
                        func: identifier,
                        expected,
                        found,
                    },
                    EvalError::InvalidExprValue { param } => {
                        ProblemError::InvalidExprValue(format!("{:?}", args[param]))
                    }
                    EvalError::UnknownFunction(func) => ProblemError::UnknownFunction(func),
                    EvalError::Panic(value) => ProblemError::Panic(value),
                    _ => panic!("Unexpected error: {:?}", e),
                })?;

        let mut values_list = vec![];
        match fn_result {
            ExprValue::LinExpr(lin_expr) => values_list.push(ExprValue::LinExpr(lin_expr)),
            ExprValue::Constraint(constraint) => {
                values_list.push(ExprValue::Constraint(constraint))
            }
            ExprValue::List(list) => values_list.extend(list),
            _ => {
                return Err(ProblemError::UnexpectedReturnValue {
                    func: fn_name.to_string(),
                    returned: fn_result,
                    expected: SimpleType::LinExpr.into(),
                })
            }
        };

        let mut obj = Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize);
        let mut constraints = vec![];

        for value in values_list {
            match value {
                ExprValue::LinExpr(lin_expr) => {
                    let cleaned_lin_expr = self.clean_lin_expr(script_ref, &lin_expr);
                    obj = obj + Objective::new(cleaned_lin_expr, obj_sense.clone());
                }
                ExprValue::Constraint(c) => {
                    let cleaned_constraints: Vec<_> = c
                        .into_iter()
                        .map(|c_with_o| self.clean_constraint(script_ref, &c_with_o.constraint))
                        .collect();
                    let new_origin = ConstraintDesc::Objectify {
                        script_ref: script_ref.clone(),
                        origin: origin.clone(),
                    };
                    let (new_obj, new_constraints) =
                        self.objectify_constraints(cleaned_constraints.iter(), new_origin);
                    obj = obj + new_obj;
                    constraints.extend(new_constraints);
                }
                _ => {
                    return Err(ProblemError::UnexpectedReturnValue {
                        func: fn_name.to_string(),
                        returned: value,
                        expected: SimpleType::LinExpr.into(),
                    })
                }
            }
        }

        Ok((obj, constraints))
    }

    fn look_for_uncalled_public_reified_var<'b>(
        &self,
        constraints: impl Iterator<Item = &'b Constraint<ProblemVar<T, V>>>,
    ) -> Vec<ReifiedPublicVar<T>>
    where
        T: 'b,
        V: 'b,
    {
        let mut output = vec![];
        for constraint in constraints {
            output.extend(
                self.look_for_uncalled_public_reified_var_in_lin_expr(constraint.get_lhs()),
            );
        }
        output
    }

    fn look_for_uncalled_public_reified_var_in_lin_expr(
        &self,
        lin_expr: &LinExpr<ProblemVar<T, V>>,
    ) -> Vec<ReifiedPublicVar<T>> {
        let mut output = vec![];
        for var in lin_expr.variables() {
            let ProblemVar::ReifiedPublic(reified_pub_var) = var else {
                continue;
            };
            if !self
                .called_public_reified_variables
                .contains(&reified_pub_var)
            {
                output.push(reified_pub_var);
            }
        }
        output
    }

    fn evaluate_recursively(
        &mut self,
        script: StoredScript<T>,
        mut start_constraints: Vec<(String, Vec<ExprValue<T>>)>,
        mut start_obj: Vec<(String, Vec<ExprValue<T>>, f64, ObjectiveSense)>,
    ) -> Result<HashMap<ScriptRef, Vec<SemWarning>>, ProblemError<T>> {
        let mut current_script_opt = Some(script);
        let mut pending_reification = HashMap::<ScriptRef, Vec<PendingReification<T>>>::new();
        let mut warnings = HashMap::new();

        let list_of_lin_expr_and_constraints = ExprType::simple(SimpleType::List(
            ExprType::sum([SimpleType::LinExpr, SimpleType::Constraint]).unwrap(),
        ));

        while let Some(script) = current_script_opt {
            let ast = script.get_ast();
            let ast_funcs = ast.get_functions();
            warnings.insert(script.get_ref().clone(), ast.get_warnings().clone());

            let mut uncalled_vars = vec![];
            let mut constraints_to_reify =
                BTreeMap::<ProblemVar<T, V>, (Vec<Constraint<ProblemVar<T, V>>>, Origin<T>)>::new();

            let mut eval_history = ast
                .start_eval_history_with_cache(&self.env, self.cache.take().unwrap_or_default())
                .expect("Environment should be compatible with AST");
            // We start by evaluating the proper constraints if any
            // (this will happen on the first iteration of the loop only)
            while let Some((fn_name, args)) = start_constraints.pop() {
                let (_params, out_typ) = ast_funcs
                    .get(&("main".to_string(), fn_name.clone()))
                    .ok_or(ProblemError::UnknownFunction(fn_name.clone()))?;
                if !out_typ.is_constraint() && !out_typ.is_list_of_constraints() {
                    return Err(ProblemError::WrongReturnType {
                        func: fn_name.clone(),
                        returned: out_typ.clone(),
                        expected: ExprType::simple(SimpleType::Constraint),
                    });
                }

                let (new_constraints, _origin) = self.eval_constraint_in_history(
                    script.get_ref(),
                    &mut eval_history,
                    &fn_name,
                    &args,
                    true,
                )?;
                uncalled_vars.extend(
                    self.look_for_uncalled_public_reified_var(
                        new_constraints.iter().map(|(c, _o)| c),
                    ),
                );
                self.constraints.extend(new_constraints);
            }

            // We then evaluate the objective if any
            // (this will happen on the first iteration of the loop only)
            while let Some((fn_name, args, coef, obj_sense)) = start_obj.pop() {
                let (_params, out_typ) = ast_funcs
                    .get(&("main".to_string(), fn_name.clone()))
                    .ok_or(ProblemError::UnknownFunction(fn_name.clone()))?;
                if !out_typ.is_lin_expr()
                    && !out_typ.is_constraint()
                    && !out_typ.is_subtype_of(&list_of_lin_expr_and_constraints)
                {
                    return Err(ProblemError::WrongReturnType {
                        func: fn_name.clone(),
                        returned: out_typ.clone(),
                        expected: ExprType::simple(SimpleType::LinExpr),
                    });
                }

                let (new_obj, new_constraints) = self.eval_obj_in_history(
                    script.get_ref(),
                    &mut eval_history,
                    &fn_name,
                    &args,
                    &obj_sense,
                )?;
                uncalled_vars.extend(
                    self.look_for_uncalled_public_reified_var_in_lin_expr(new_obj.get_function()),
                );
                uncalled_vars.extend(
                    self.look_for_uncalled_public_reified_var(
                        new_constraints.iter().map(|(c, _o)| c),
                    ),
                );
                self.constraints.extend(new_constraints);
                self.objective = &self.objective + coef * new_obj;
            }

            // Then we evaluate the missing public reified vars from this scripts
            let reifications_from_current_script = pending_reification.remove(script.get_ref());
            if let Some(reifications) = reifications_from_current_script {
                for reification in reifications {
                    let (_params, out_typ) = ast_funcs
                        .get(&("main".to_string(), reification.func.clone()))
                        .ok_or(ProblemError::UnknownFunction(reification.func.clone()))?;
                    if !out_typ.is_constraint() {
                        return Err(ProblemError::WrongReturnType {
                            func: reification.func.clone(),
                            returned: out_typ.clone(),
                            expected: ExprType::simple(SimpleType::Constraint),
                        });
                    }
                    let (reification_constraints, new_origin) = self.eval_constraint_in_history(
                        script.get_ref(),
                        &mut eval_history,
                        &reification.func,
                        &reification.args,
                        false,
                    )?;
                    let dropped_origin: Vec<_> = reification_constraints
                        .into_iter()
                        .map(|(c, _o)| c)
                        .collect();
                    uncalled_vars
                        .extend(self.look_for_uncalled_public_reified_var(dropped_origin.iter()));

                    let reified_pub_var = ReifiedPublicVar {
                        name: reification.name.clone(),
                        params: reification.args.clone(),
                    };
                    let new_var = ProblemVar::ReifiedPublic(reified_pub_var.clone());

                    self.vars_desc.insert(new_var.clone(), Variable::binary());
                    self.called_public_reified_variables.insert(reified_pub_var);
                    constraints_to_reify.insert(new_var, (dropped_origin, new_origin));
                }
            }

            // We're done evaluating. Let's collect the private reified vars
            let (var_def, new_cache) = eval_history.into_var_def_and_cache();
            self.cache = Some(new_cache);
            for ((_var_module, var_name, var_args), (constraints, new_origin)) in var_def.vars {
                let cleaned_constraints: Vec<_> = constraints
                    .into_iter()
                    .map(|c| self.clean_constraint(script.get_ref(), &c))
                    .collect();

                uncalled_vars
                    .extend(self.look_for_uncalled_public_reified_var(cleaned_constraints.iter()));

                let reified_priv_var = ReifiedPrivateVar {
                    script_ref: script.get_ref().clone(),
                    name: var_name,
                    from_list: None,
                    params: var_args,
                };
                let new_var = ProblemVar::ReifiedPrivate(reified_priv_var);

                self.vars_desc.insert(new_var.clone(), Variable::binary());
                constraints_to_reify.insert(new_var, (cleaned_constraints, new_origin));
            }
            for (
                (_var_list_module, var_list_name, var_list_args),
                (constraints_list, new_origin),
            ) in var_def.var_lists
            {
                for (i, constraints) in constraints_list.into_iter().enumerate() {
                    let cleaned_constraints: Vec<_> = constraints
                        .into_iter()
                        .map(|c| self.clean_constraint(script.get_ref(), &c))
                        .collect();

                    uncalled_vars.extend(
                        self.look_for_uncalled_public_reified_var(cleaned_constraints.iter()),
                    );

                    let reified_priv_var = ReifiedPrivateVar {
                        script_ref: script.get_ref().clone(),
                        name: var_list_name.clone(),
                        from_list: Some(i),
                        params: var_list_args.clone(),
                    };
                    let new_var = ProblemVar::ReifiedPrivate(reified_priv_var);

                    self.vars_desc.insert(new_var.clone(), Variable::binary());
                    constraints_to_reify.insert(new_var, (cleaned_constraints, new_origin.clone()));
                }
            }

            // Ok, we finally reify the constraints
            // Originally this was done as a last path to have variables defined before.
            // This could work for private reification. It does not work for chained public reification.
            // So this is just a final pass, wiht the reason for being last being historic.
            for (var, (constraints, origin)) in constraints_to_reify {
                let var_name = match &var {
                    ProblemVar::ReifiedPrivate(ReifiedPrivateVar {
                        script_ref: _,
                        name,
                        from_list: _,
                        params: _,
                    }) => name.clone(),
                    ProblemVar::ReifiedPublic(ReifiedPublicVar { name, params: _ }) => name.clone(),
                    _ => panic!("Unexpected variable type to reify: {:?}", var),
                };

                let new_origin = ConstraintDesc::Reified {
                    script_ref: script.get_ref().clone(),
                    var_name,
                    origin,
                };

                let reified_constraints =
                    self.reify_constraint(constraints.iter(), new_origin, var);

                self.constraints.extend(reified_constraints);
            }

            // At this point, we've build all the constraints for the current script
            // uncalled_vars contains all the public reified vars that are not yet defined
            // and need to be defined. So let's add them to the pending_reifications
            for var in uncalled_vars {
                let desc = self.reified_vars.get(&var.name).expect("Variable should be defined. This should have been caught in the semantic analysis otherwise");

                let new_pending = PendingReification {
                    name: var.name,
                    func: desc.func.clone(),
                    args: var.params,
                };
                match pending_reification.get_mut(&desc.script_ref) {
                    Some(list) => list.push(new_pending),
                    None => {
                        pending_reification.insert(desc.script_ref.clone(), vec![new_pending]);
                    }
                }
            }

            // pending_reification contains all pending operations at this point
            // Let's walk the scripts backward in the dependancy tree and find the first
            // script with pending operations
            current_script_opt = None;
            for stored_script in self.stored_scripts.iter().rev() {
                if pending_reification.contains_key(stored_script.get_ref()) {
                    current_script_opt = Some(stored_script.clone());
                    break;
                }
            }
        }

        Ok(warnings)
    }
}

struct PendingReification<T: EvalObject> {
    name: String,
    func: String,
    args: Vec<ExprValue<T>>,
}

impl<
        'a,
        T: EvalObject,
        V: EvalVar<T> + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > ProblemBuilder<'a, T, V>
{
    pub fn new(env: &'a T::Env) -> Result<Self, ProblemError<T>> {
        let base_vars = Self::build_vars()?;
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
        Ok(ProblemBuilder {
            env,
            cache: None,
            base_vars,
            reified_vars: HashMap::new(),
            stored_scripts: vec![],
            constraints: vec![],
            called_public_reified_variables: BTreeSet::new(),
            objective: Objective::new(LinExpr::constant(0.), ObjectiveSense::Minimize),
            current_helper_id: 0,
            vars_desc,
            original_var_list,
        })
    }

    pub fn add_reified_variables(
        &mut self,
        script: Script,
        func_and_names: Vec<(String, String)>,
    ) -> Result<Vec<SemWarning>, ProblemError<T>> {
        let stored_script = self.compile_script(script)?;
        let warnings = stored_script.get_ast().get_warnings().clone();
        self.add_reified_variables_with_compiled_script(stored_script, func_and_names)?;
        Ok(warnings)
    }

    pub fn compile_script(&self, script: Script) -> Result<StoredScript<T>, ProblemError<T>> {
        let vars = self.generate_current_vars();
        StoredScript::new(script, vars)
    }

    pub fn add_reified_variables_with_compiled_script(
        &mut self,
        stored_script: StoredScript<T>,
        func_and_names: Vec<(String, String)>,
    ) -> Result<(), ProblemError<T>> {
        for (_func, name) in &func_and_names {
            if self.base_vars.contains_key(name) {
                return Err(ProblemError::VariableAlreadyDefined(name.clone()));
            }
            if self.reified_vars.contains_key(name) {
                return Err(ProblemError::VariableAlreadyDefined(name.clone()));
            }
        }

        for s in &self.stored_scripts {
            if stored_script == *s {
                return Err(ProblemError::ScriptAlreadyUsed(stored_script.script()));
            }
        }
        let script_ref = stored_script.get_ref().clone();
        let ast = stored_script.get_ast();

        let func_map = ast.get_functions();

        for (func, name) in func_and_names {
            let Some((args, expr_type)) = func_map.get(&("main".to_string(), func.clone())) else {
                return Err(ProblemError::UnknownFunction(func));
            };

            if !expr_type.is_constraint() {
                return Err(ProblemError::WrongReturnType {
                    func,
                    returned: expr_type.clone(),
                    expected: SimpleType::Constraint.into(),
                });
            }

            self.reified_vars.insert(
                name.clone(),
                ReifiedVarDesc {
                    func,
                    args: args.clone(),
                    script_ref: script_ref.clone(),
                },
            );
        }
        self.stored_scripts.push(stored_script);

        Ok(())
    }

    pub fn add_constraints(
        &mut self,
        script: Script,
        funcs: Vec<(String, Vec<ExprValue<T>>)>,
    ) -> Result<Vec<SemWarning>, ProblemError<T>> {
        let script = self.compile_script(script)?;
        let script_ref = script.get_ref().clone();
        let start_funcs = funcs;
        let mut warnings = self.evaluate_recursively(script, start_funcs, vec![])?;
        Ok(warnings
            .remove(&script_ref)
            .expect("There should be warnings (maybe empty) for the initial script"))
    }

    pub fn add_to_objective(
        &mut self,
        script: Script,
        objectives: Vec<(String, Vec<ExprValue<T>>, f64, ObjectiveSense)>,
    ) -> Result<Vec<SemWarning>, ProblemError<T>> {
        let script = self.compile_script(script)?;
        let script_ref = script.get_ref().clone();
        let mut warnings = self.evaluate_recursively(script, vec![], objectives)?;
        Ok(warnings
            .remove(&script_ref)
            .expect("There should be warnings (maybe empty) for the initial script"))
    }

    pub fn add_constraints_and_objectives(
        &mut self,
        script: Script,
        funcs: Vec<(String, Vec<ExprValue<T>>)>,
        objectives: Vec<(String, Vec<ExprValue<T>>, f64, ObjectiveSense)>,
    ) -> Result<Vec<SemWarning>, ProblemError<T>> {
        let script = self.compile_script(script)?;
        let script_ref = script.get_ref().clone();
        let mut warnings = self.evaluate_recursively(script, funcs, objectives)?;
        Ok(warnings
            .remove(&script_ref)
            .expect("There should be warnings (maybe empty) for the initial script"))
    }

    pub fn add_constraints_and_objectives_with_compiled_script(
        &mut self,
        stored_script: StoredScript<T>,
        funcs: Vec<(String, Vec<ExprValue<T>>)>,
        objectives: Vec<(String, Vec<ExprValue<T>>, f64, ObjectiveSense)>,
    ) -> Result<(), ProblemError<T>> {
        let _warnings = self.evaluate_recursively(stored_script, funcs, objectives)?;
        Ok(())
    }

    pub fn build(mut self) -> Problem<T, V> {
        for (constraint, _desc) in self.constraints.iter_mut() {
            let mut fixed_variables = BTreeMap::new();
            for var in constraint.variables() {
                if fixed_variables.contains_key(&var) {
                    continue;
                }
                let ProblemVar::Base(v) = var else {
                    continue;
                };
                let Some(value) = v.fix(&self.env) else {
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
        for var in self.objective.get_function().variables() {
            if fixed_variables.contains_key(&var) {
                continue;
            }
            let ProblemVar::Base(v) = var else {
                continue;
            };
            let Some(value) = v.fix(&self.env) else {
                continue;
            };
            fixed_variables.insert(ProblemVar::Base(v), value);
        }
        if !fixed_variables.is_empty() {
            self.objective = self.objective.reduce(&fixed_variables);
        }
        self.constraints = self
            .constraints
            .into_iter()
            .filter(|(c, _d)| !c.is_trivially_true())
            .collect();

        let reification_constraints: Vec<_> = self
            .constraints
            .iter()
            .filter_map(|(c, d)| match d {
                ConstraintDesc::InScript {
                    script_ref: _,
                    origin: _,
                } => None,
                ConstraintDesc::Objectify {
                    script_ref: _,
                    origin: _,
                } => Some((c.clone(), ExtraDesc::Orig(d.clone()))),
                ConstraintDesc::Reified {
                    script_ref: _,
                    var_name: _,
                    origin: _,
                } => Some((c.clone(), ExtraDesc::Orig(d.clone()))),
            })
            .collect();

        let mut problem_builder = collomatique_ilp::ProblemBuilder::new()
            .set_variables(self.vars_desc.clone())
            .add_constraints(self.constraints);
        problem_builder = problem_builder.set_objective(self.objective);

        let reification_problem_builder = collomatique_ilp::ProblemBuilder::new()
            .set_variables(self.vars_desc)
            .add_constraints(reification_constraints);

        Problem {
            problem: problem_builder.build().expect("Problem should be valid"),
            reification_problem_builder,
            original_var_list: self.original_var_list,
        }
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
