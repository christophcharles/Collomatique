use crate::eval::{
    CompileError, EvalError, EvalHistory, ExprValue, ExternVar, IlpVar, Origin, ScriptVar,
};
use crate::semantics::ArgsType;
use crate::traits::{FieldConversionError, VarConversionError};
use crate::{CheckedAST, EvalObject, EvalVar, ExprType, SemWarning};
use collomatique_ilp::{Constraint, LinExpr, Objective, ObjectiveSense, Variable};
use std::collections::{BTreeMap, BTreeSet, HashMap};

mod scripts;
use scripts::{Script, ScriptRef, StoredScript};

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
pub enum ProblemVar<T: EvalObject, V: EvalVar> {
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
}

pub struct ProblemBuilder<
    'a,
    T: EvalObject,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
> {
    /// Reference to the evaluation environment
    env: &'a T::Env,

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
    stored_scripts: Vec<StoredScript>,

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
    vars_desc: BTreeMap<ProblemVar<T, V>, collomatique_ilp::Variable>,
}

use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum ProblemError {
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
    ScriptAlreadyUsed(StoredScript),
    #[error(transparent)]
    CompileError(#[from] CompileError),
    #[error("Function {func} returns {returned} instead of {expected}")]
    WrongReturnType {
        func: String,
        returned: ExprType,
        expected: ExprType,
    },
}

impl<
        'a,
        T: EvalObject,
        V: EvalVar + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > ProblemBuilder<'a, T, V>
{
    fn build_vars() -> Result<HashMap<String, Vec<ExprType>>, ProblemError> {
        V::field_schema()
            .into_iter()
            .map(|(name, typ)| {
                Ok((
                    name,
                    typ.into_iter()
                        .map(|x| x.convert_to_expr_type::<T>())
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

    fn reify_single_constraint(
        &mut self,
        constraint: &Constraint<ProblemVar<T, V>>,
        origin: ConstraintDesc<T>,
        var: ProblemVar<T, V>,
    ) -> Vec<(Constraint<ProblemVar<T, V>>, ConstraintDesc<T>)> {
        use collomatique_ilp::linexpr::EqSymbol;
        match constraint.get_symbol() {
            EqSymbol::LessThan => {
                let lin_expr = constraint.get_lhs().clone();
                let range = lin_expr.compute_range(&self.vars_desc);
                let min = *range.start();
                let max = *range.end();
                assert!(
                    min.is_finite() && max.is_finite(),
                    "Linear expression from ColloML should always have finite ranges. But this expression is unbounded: {:?} (found range: {:?})",
                    lin_expr,
                    range
                );
                let one = LinExpr::constant(1.);
                let var = LinExpr::var(var);
                let constraints = vec![
                    (lin_expr.leq(&(max * (&one - &var))), origin.clone()),
                    (lin_expr.geq(&((min - 1.) * &var + &one)), origin),
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
        ProblemError,
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
                    _ => panic!("Unexpected error: {:?}", e),
                })?;

        let constraints = match constraints_expr {
            ExprValue::Constraint(constraints) => constraints,
            ExprValue::List(ExprType::Constraint, list) if allow_list => list
                .into_iter()
                .flat_map(|x| match x {
                    ExprValue::Constraint(constraints) => constraints.into_iter(),
                    _ => panic!("Inconsistent ExprValue::List"),
                })
                .collect(),
            _ => {
                return Err(ProblemError::WrongReturnType {
                    func: fn_name.to_string(),
                    returned: constraints_expr.get_type(&self.env),
                    expected: ExprType::Constraint,
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

    fn eval_lin_expr_in_history<'b>(
        &self,
        script_ref: &ScriptRef,
        eval_history: &mut EvalHistory<'b, T>,
        fn_name: &str,
        args: &Vec<ExprValue<T>>,
    ) -> Result<(LinExpr<ProblemVar<T, V>>, Origin<T>), ProblemError> {
        let (lin_expr_expr, origin) =
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
                    _ => panic!("Unexpected error: {:?}", e),
                })?;

        let lin_expr = match lin_expr_expr {
            ExprValue::LinExpr(lin_expr) => lin_expr,
            _ => {
                return Err(ProblemError::WrongReturnType {
                    func: fn_name.to_string(),
                    returned: lin_expr_expr.get_type(&self.env),
                    expected: ExprType::LinExpr,
                })
            }
        };

        Ok((self.clean_lin_expr(script_ref, &lin_expr), origin))
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
        script: StoredScript,
        mut start_constraints: Vec<(String, Vec<ExprValue<T>>)>,
        mut start_obj: Vec<(String, Vec<ExprValue<T>>, f64, ObjectiveSense)>,
    ) -> Result<HashMap<ScriptRef, Vec<SemWarning>>, ProblemError> {
        let vars = self.generate_current_vars();
        let mut current_script_opt = Some(script);
        let mut pending_reification = HashMap::<ScriptRef, Vec<PendingReification<T>>>::new();
        let mut warnings = HashMap::new();

        while let Some(script) = current_script_opt {
            // We got a script to evaluate
            // We evaluate everything we can in this script to avoid multiple parsing of the AST
            let ast = CheckedAST::<T>::new(script.get_content(), vars.clone())?;
            warnings.insert(script.get_ref().clone(), ast.get_warnings().clone());

            let mut uncalled_vars = vec![];
            let mut constraints_to_reify =
                BTreeMap::<ProblemVar<T, V>, (Vec<Constraint<ProblemVar<T, V>>>, Origin<T>)>::new();

            let mut eval_history = ast
                .start_eval_history(&self.env)
                .expect("Environment should be compatible with AST");
            // We start by evaluating the proper constraints if any
            // (this will happen on the first iteration of the loop only)
            while let Some((fn_name, args)) = start_constraints.pop() {
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
                let (lin_expr, _origin) = self.eval_lin_expr_in_history(
                    script.get_ref(),
                    &mut eval_history,
                    &fn_name,
                    &args,
                )?;
                uncalled_vars
                    .extend(self.look_for_uncalled_public_reified_var_in_lin_expr(&lin_expr));
                self.objective = &self.objective + Objective::new(coef * lin_expr, obj_sense);
            }

            // Then we evaluate the missing public reified vars from this scripts
            let reifications_from_current_script = pending_reification.remove(script.get_ref());
            if let Some(reifications) = reifications_from_current_script {
                for reification in reifications {
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
            let var_def = eval_history.into_var_def();
            for ((var_name, var_args), (constraints, new_origin)) in var_def.vars {
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
            for ((var_list_name, var_list_args), (constraints_list, new_origin)) in
                var_def.var_lists
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

            // Last pass: now that all variables are declared in vars_desc,
            // we can compute the range of all lin_expr. So we can finally reify the
            // constraints
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
        V: EvalVar + for<'b> TryFrom<&'b ExternVar<T>, Error = VarConversionError>,
    > ProblemBuilder<'a, T, V>
{
    pub fn new(env: &'a T::Env) -> Result<Self, ProblemError> {
        let base_vars = Self::build_vars()?;
        let vars_desc = V::vars::<T>(env)
            .into_iter()
            .map(|(name, desc)| (ProblemVar::Base(name), desc))
            .collect();
        Ok(ProblemBuilder {
            env,
            base_vars,
            reified_vars: HashMap::new(),
            stored_scripts: vec![],
            constraints: vec![],
            called_public_reified_variables: BTreeSet::new(),
            objective: Objective::new(LinExpr::constant(0.), ObjectiveSense::Maximize),
            current_helper_id: 0,
            vars_desc,
        })
    }

    pub fn add_reified_variables(
        &mut self,
        script: Script,
        func_and_names: Vec<(String, String)>,
    ) -> Result<Vec<SemWarning>, ProblemError> {
        for (_func, name) in &func_and_names {
            if self.base_vars.contains_key(name) {
                return Err(ProblemError::VariableAlreadyDefined(name.clone()));
            }
            if self.reified_vars.contains_key(name) {
                return Err(ProblemError::VariableAlreadyDefined(name.clone()));
            }
        }

        let stored_script = StoredScript::new(script);
        for s in &self.stored_scripts {
            if stored_script == *s {
                return Err(ProblemError::ScriptAlreadyUsed(stored_script));
            }
        }
        let script_ref = stored_script.get_ref().clone();

        let vars = self.generate_current_vars();
        let ast = CheckedAST::<T>::new(stored_script.get_content(), vars)?;

        self.stored_scripts.push(stored_script);

        let func_map = ast.get_functions();

        for (func, name) in func_and_names {
            let Some((args, expr_type)) = func_map.get(&func) else {
                return Err(ProblemError::UnknownFunction(func));
            };

            if *expr_type != ExprType::Constraint {
                return Err(ProblemError::WrongReturnType {
                    func,
                    returned: expr_type.clone(),
                    expected: ExprType::Constraint,
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

        Ok(ast.get_warnings().clone())
    }

    pub fn add_constraints(
        &mut self,
        script: Script,
        funcs: Vec<(String, Vec<ExprValue<T>>)>,
    ) -> Result<Vec<SemWarning>, ProblemError> {
        let script = StoredScript::new(script);
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
    ) -> Result<Vec<SemWarning>, ProblemError> {
        let script = StoredScript::new(script);
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
    ) -> Result<Vec<SemWarning>, ProblemError> {
        let script = StoredScript::new(script);
        let script_ref = script.get_ref().clone();
        let mut warnings = self.evaluate_recursively(script, funcs, objectives)?;
        Ok(warnings
            .remove(&script_ref)
            .expect("There should be warnings (maybe empty) for the initial script"))
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
                let Some(value) = v.fix() else {
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
            let Some(value) = v.fix() else {
                continue;
            };
            fixed_variables.insert(ProblemVar::Base(v), value);
        }
        if !fixed_variables.is_empty() {
            self.objective = self.objective.reduce(&fixed_variables);
        }

        let mut problem_builder = collomatique_ilp::ProblemBuilder::new()
            .set_variables(self.vars_desc)
            .add_constraints(self.constraints);
        problem_builder = problem_builder.set_objective(self.objective);

        Problem {
            problem: problem_builder.build().expect("Problem should be valid"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Problem<T: EvalObject, V: EvalVar> {
    problem: collomatique_ilp::Problem<ProblemVar<T, V>, ConstraintDesc<T>>,
}

impl<T: EvalObject, V: EvalVar> Problem<T, V> {
    pub fn get_inner_problem(
        &self,
    ) -> &collomatique_ilp::Problem<ProblemVar<T, V>, ConstraintDesc<T>> {
        &self.problem
    }
}
