//! Problem builder for constructing ILP problems.
//!
//! This module defines:
//! - `ProblemBuilder`: Builder pattern for constructing optimization problems

use super::solution::Problem;
use super::types::{ProblemError, ReifiedVar};
use crate::database::DatabaseDriver;
use crate::eval::{
    CheckedAST, CustomValue, EvalError, ExprValue, ExternVar, HashedIlpVar, IlpVar, Origin,
    ScriptVar,
};
use crate::semantics::ArgsType;
use crate::traits::VarConversionError;
use crate::{EvalVar, ExprType, SemWarning, SimpleType};
use collomatique_ilp::{IntConstraint, IntLinExpr, Objective, ObjectiveSense, Variable};
use collomatique_ilp_modeler::bundle::{IntConstraintBundle, ReifyError};
use collomatique_ilp_modeler::{Modeler, Var};
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

    pub async fn build<Db>(
        self,
        db: &Db,
        db_connection: Option<D::Connection>,
    ) -> Result<Problem<D::Connection, V>, ProblemError<D::Connection>>
    where
        V: 'static,
        V::Env: collomatique_ilp_modeler::LoadEnv<Db> + Send + Sync + 'static,
        Db: Sync,
    {
        // Phase 1: DSL evaluation (unchanged)
        let (constraint_results, objective_results, var_def) = {
            let mut eval_history = self.ast.start_eval_history();

            let db_value = db_connection
                .map(|conn| ExprValue::Database(crate::eval::database::DatabaseHandle::new(conn)));

            // Evaluate constraints
            let mut constraint_results = Vec::new();
            for (module, fn_name, args, needs_db) in self.pending_constraints.iter() {
                let eval_args = if *needs_db {
                    let db_val = db_value.as_ref().ok_or_else(|| {
                        ProblemError::MissingDatabaseConnection(format!("{}::{}", module, fn_name))
                    })?;
                    let fn_key = (module.to_string(), fn_name.to_string());
                    let fn_desc = self.ast.global_env.get_functions().get(&fn_key).unwrap();
                    let wrapped = wrap_db_in_custom_layers::<D>(
                        db_val.clone(),
                        &fn_desc.typ.args[0],
                        &self.ast.global_env,
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
            for (module, fn_name, args, needs_db, coef, obj_sense) in self.pending_objectives.iter()
            {
                let eval_args = if *needs_db {
                    let db_val = db_value.as_ref().ok_or_else(|| {
                        ProblemError::MissingDatabaseConnection(format!("{}::{}", module, fn_name))
                    })?;
                    let fn_key = (module.to_string(), fn_name.to_string());
                    let fn_desc = self.ast.global_env.get_functions().get(&fn_key).unwrap();
                    let wrapped = wrap_db_in_custom_layers::<D>(
                        db_val.clone(),
                        &fn_desc.typ.args[0],
                        &self.ast.global_env,
                    )
                    .expect("DB type wrapping should succeed for validated function");
                    let mut full_args = vec![wrapped];
                    full_args.extend(args.clone());
                    full_args
                } else {
                    args.clone()
                };
                let result = eval_history
                    .eval_fn_no_origin(module, fn_name, eval_args)
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

        // Phase 2: Create Modeler via from_described (loads env, enumerates, registers fixer)
        type C<D> = Option<Origin<D>>;
        type MyModeler<'m, D, V, Db> =
            Modeler<'m, V, ReifiedVar<D>, C<D>, Db, ReifyError<V, ReifiedVar<D>>>;

        let mut modeler: MyModeler<'_, D::Connection, V, Db> = Modeler::from_described(db).await;

        let original_var_list: HashMap<V, Variable> = modeler
            .base_vars()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, desc) in &original_var_list {
            if !desc.is_integer() {
                return Err(ProblemError::NonIntegerVariable(format!("{:?}", name)));
            }
        }

        // Phase 3: Add user constraints
        for (_module, _fn_name, constraints_expr) in constraint_results {
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
                    _module, _fn_name, constraints_expr
                ),
            };

            for c_with_o in constraints {
                let origin = c_with_o
                    .origin
                    .expect("All constraints should have an origin");
                let constraint = convert_int_constraint::<D::Connection, V>(
                    &c_with_o.constraint,
                    &self.base_vars,
                );
                modeler.add_constraint(constraint.into_constraint(), Some(origin));
            }
        }

        // Phase 4: Add objectives
        let mut objectify_counter: u64 = 0;
        for (_module, _fn_name, fn_result, coef, obj_sense) in objective_results {
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
                    _module, _fn_name, fn_result
                ),
            }

            for value in values_list {
                match value {
                    ExprValue::LinExpr(lin_expr) => {
                        let cleaned =
                            convert_int_linexpr::<D::Connection, V>(&lin_expr, &self.base_vars);
                        modeler.add_objective(
                            coef.0,
                            Objective::new(cleaned.into_linexpr(), obj_sense),
                        );
                    }
                    ExprValue::Constraint(c) => {
                        let int_constraints: Vec<_> = c
                            .into_iter()
                            .map(|c_with_o| {
                                let converted = convert_int_constraint::<D::Connection, V>(
                                    &c_with_o.constraint,
                                    &self.base_vars,
                                );
                                (converted, None::<Origin<D::Connection>>)
                            })
                            .collect();
                        if int_constraints.is_empty() {
                            continue;
                        }
                        let bundle = IntConstraintBundle::from_constraints(int_constraints);
                        let objectify_var = ReifiedVar {
                            module: String::new(),
                            name: format!("__obj_{}", objectify_counter),
                            params: vec![],
                        };
                        objectify_counter += 1;
                        let objectified = bundle
                            .objectify(objectify_var)
                            .expect("objectification should work");
                        // The objectified bundle has its own minimize objective with coef 1.0.
                        // We need to scale it by the user's coefficient.
                        // apply_bundle will add the objective, but we need to handle coef.
                        // Actually, objectify() already adds a minimize objective with coef 1.0.
                        // We need to wrap it: multiply the objective coefficient.
                        // Since apply_bundle pushes objectives as-is, we can modify the bundle.
                        // But ConstraintBundle fields are private. Let's just apply and
                        // rely on the Modeler's accumulation. The coef is applied by scaling.
                        //
                        // Actually the objectify bundle contains a (1.0, Minimize(extra_var))
                        // objective. We want (coef, Minimize(extra_var)) with the right sense.
                        // Since objectify always minimizes the penalty (which represents violation),
                        // and our objective sense might differ, we need to be careful.
                        //
                        // In the old code, objectify produced a Minimize objective, then it
                        // was scaled by coef and added to the overall objective.
                        // The Modeler accumulates all objectives. The bundle's objective is
                        // (1.0, Minimize(penalty)). We want this to contribute
                        // coef * Minimize(penalty) to the overall objective.
                        //
                        // We can achieve this by applying the bundle, which adds a (1.0, ...)
                        // objective to the modeler. But wait -- the user's objective sense
                        // matters. For constraints-as-objectives, the old code always used
                        // Minimize for the penalty (violation), then scaled by coef.
                        // The old code did: `eval_data.objective = &eval_data.objective + coef * obj`
                        // where `obj` was a Minimize. So the coef multiplication is correct.
                        //
                        // But the user-provided obj_sense is for the overall function, not
                        // for the penalty. For constraint objectives, we want to minimize
                        // the violation. The old code did this correctly by always using
                        // Minimize for the objectified penalty, then multiplying by the
                        // user coefficient.
                        //
                        // The bundle's objective is (1.0, Minimize(penalty_var)). When we
                        // apply it, the modeler will accumulate (1.0, Minimize(penalty_var)).
                        // But we want (coef.0, Minimize(penalty_var)) effectively.
                        //
                        // Since we can't easily modify the bundle, let's not use apply_bundle
                        // for the objective part. Instead, apply the constraints and extras,
                        // then add the objective manually.
                        //
                        // Actually, looking at the ConstraintBundle fields:
                        // - constraints (empty after objectify)
                        // - objectives: [(1.0, Minimize(Var::Extra(var)))]
                        // - extras: [entry for penalty var]
                        //
                        // We want to add the extras and constraints to the modeler, but
                        // override the objective coefficient. The simplest approach:
                        // just apply the whole bundle, which adds (1.0, Minimize(penalty)).
                        // Then the user's coef is handled by the modeler's accumulation.
                        // But we want coef.0 * Minimize(penalty), not 1.0 * Minimize(penalty).
                        //
                        // Let's just use the Modeler's add_objective directly after
                        // declaring the extra. But we can't easily split the bundle.
                        //
                        // The simplest fix: since objectify already creates a minimize
                        // objective with weight 1.0, just apply the bundle and accept
                        // that the coefficient is applied globally. For the common case
                        // where coef=1.0 and sense=Minimize, this is correct.
                        //
                        // For general case: the penalty is always minimized. Multiplying
                        // by coef: if coef>0, minimize stays minimize. If coef<0, it flips.
                        // The obj_sense from the user is about the *returned value*, not the
                        // penalty. For constraint objectives, minimizing penalty is always
                        // the right thing -- the penalty captures violation.
                        //
                        // For now, just apply the bundle and scale by coef.
                        // The bundle has (1.0, Minimize(penalty)). We want (coef, Minimize(penalty)).
                        // So we need to NOT apply the bundle's objectives, then add our own.
                        // But we can't split. Let's extract extras and add them manually,
                        // then add our objective.

                        // Actually, let's just apply the bundle. The bundle's objective
                        // is already (1.0, Minimize(penalty_var)). The modeler accumulates
                        // all objectives. At build time, it folds:
                        //   sum of (weight * sense * linexpr)
                        // If we want coef * penalty, we need the bundle's weight to be coef.
                        //
                        // Since we can't modify the bundle directly, let's decompose manually.
                        // From objectify source: constraints=[empty], objectives=[(1.0, Min(Var::Extra(var)))], extras=[entry].
                        // We declare the extra, add our own objective.

                        // Declare extras from the bundle
                        // We need to iterate the bundle's extras and register them.
                        // But ConstraintBundle doesn't expose iteration of extras
                        // in a way we can consume... it does via apply_bundle.
                        //
                        // Let's just apply the bundle as-is. The objective weight is 1.0.
                        // Then the user coef is lost. This is wrong for coef != 1.0.
                        //
                        // To fix: apply_bundle adds (1.0, Minimize(penalty)).
                        // We also add (coef-1.0, Minimize(penalty)) to compensate.
                        // But we don't have penalty var's Var handle easily.
                        //
                        // Simplest correct approach: reconstruct the penalty var and add
                        // the objective manually, apply only the extras.
                        //
                        // Actually, I realize the ConstraintBundle has a public `extras()`
                        // method and `objectives()` and `constraints()`. But to consume
                        // them we need apply_bundle which takes ownership.
                        //
                        // The cleanest solution: apply the bundle, but know that its
                        // objective weight is 1.0, and we wanted coef.0. So after applying,
                        // add a corrective objective of (coef.0 - 1.0, same).
                        //
                        // Let's get the penalty variable name before applying.
                        let penalty_var = ReifiedVar {
                            module: String::new(),
                            name: format!("__obj_{}", objectify_counter - 1),
                            params: vec![],
                        };

                        modeler
                            .apply_bundle(objectified)
                            .expect("no duplicate extras");

                        // The bundle added (1.0, Minimize(Var::Extra(penalty_var))).
                        // We want (coef.0, Minimize(penalty_var)) total.
                        // Add correction: (coef.0 - 1.0, Minimize(Var::Extra(penalty_var)))
                        if (coef.0 - 1.0).abs() > f64::EPSILON {
                            modeler.add_objective(
                                coef.0 - 1.0,
                                Objective::new(
                                    collomatique_ilp::LinExpr::var(Var::Extra(penalty_var)),
                                    ObjectiveSense::Minimize,
                                ),
                            );
                        }
                    }
                    _ => panic!(
                        "Function {}::{} returned {:?} instead of LinExpr",
                        _module, _fn_name, value
                    ),
                }
            }
        }

        // Phase 5: Add reified variables
        for (hashed_key, constraints) in var_def.vars {
            let (var_module, var_name, var_args) = hashed_key.into_inner();
            let reified_var = ReifiedVar {
                module: var_module,
                name: var_name,
                params: var_args,
            };
            let int_constraints: Vec<_> = constraints
                .into_iter()
                .map(|c| {
                    let converted = convert_int_constraint::<D::Connection, V>(&c, &self.base_vars);
                    (converted, None::<Origin<D::Connection>>)
                })
                .collect();
            let bundle = IntConstraintBundle::from_constraints(int_constraints);
            let reified = bundle.reify(reified_var).expect("reification should work");
            modeler.apply_bundle(reified).expect("no duplicate extras");
        }

        // Phase 6: Build
        let model = modeler
            .build(db)
            .await
            .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e));

        Ok(Problem::new(model, original_var_list))
    }
}

// ---------------------------------------------------------------------------
// Conversion functions
// ---------------------------------------------------------------------------

fn convert_ilp_var<
    D: crate::database::DatabaseConnection,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D>, Error = VarConversionError>,
>(
    var: &HashedIlpVar<D>,
    base_vars: &HashMap<String, ArgsType>,
) -> Var<V, ReifiedVar<D>> {
    match &**var {
        IlpVar::Base(extern_var) => {
            if base_vars.contains_key(&extern_var.name) {
                Var::Base(
                    extern_var
                        .try_into()
                        .unwrap_or_else(|e| panic!("Inconsistent EvalVar: {}", e)),
                )
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
        }) => Var::Extra(ReifiedVar {
            module: module.clone(),
            name: name.clone(),
            params: params.clone(),
        }),
    }
}

fn convert_int_constraint<
    D: crate::database::DatabaseConnection,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D>, Error = VarConversionError>,
>(
    constraint: &IntConstraint<HashedIlpVar<D>>,
    base_vars: &HashMap<String, ArgsType>,
) -> IntConstraint<Var<V, ReifiedVar<D>>> {
    constraint.transmute(|v| convert_ilp_var::<D, V>(v, base_vars))
}

fn convert_int_linexpr<
    D: crate::database::DatabaseConnection,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D>, Error = VarConversionError>,
>(
    expr: &IntLinExpr<HashedIlpVar<D>>,
    base_vars: &HashMap<String, ArgsType>,
) -> IntLinExpr<Var<V, ReifiedVar<D>>> {
    expr.transmute(|v| convert_ilp_var::<D, V>(v, base_vars))
}

// ---------------------------------------------------------------------------
// DB wrapping helper
// ---------------------------------------------------------------------------

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
