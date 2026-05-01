//! Script feeder: evaluates ColloML scripts and produces an IntConstraintBundle.
//!
//! Unlike [`ProblemBuilder`], which owns a [`Modeler`] and produces a
//! [`Problem`], `ScriptFeeder` is generic over the extra-variable type `E`
//! and constraint-source type `C`, and returns a raw
//! [`IntConstraintBundle`] that the caller feeds to their own Modeler.

use super::types::{ReifiedVar, ScriptError};
use crate::database::{DatabaseConnection, DatabaseDriver};
use crate::eval::{
    CheckedAST, CustomValue, EvalError, ExprValue, ExternVar, HashedIlpVar, IlpVar, Origin,
    ScriptVar,
};
use crate::semantics::ArgsType;
use crate::traits::VarConversionError;
use crate::{EvalVar, ExprType, SemWarning, SimpleType};
use collomatique_ilp::{IntConstraint, IntLinExpr, Objective, ObjectiveSense, UsableData};
use collomatique_ilp_modeler::Var;
use collomatique_ilp_modeler::bundle::{IntConstraintBundle, ReifyError};
use derivative::Derivative;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct ScriptFeeder<
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
    E: UsableData + From<ReifiedVar<D::Connection>>,
    C: UsableData + From<Option<Origin<D::Connection>>>,
> {
    ast: CheckedAST<D>,
    base_vars: HashMap<String, ArgsType>,
    pending_constraints: Vec<(String, String, Vec<ExprValue<D::Connection>>, bool)>,
    pending_objectives: Vec<(
        String,
        String,
        Vec<ExprValue<D::Connection>>,
        bool,
        ordered_float::OrderedFloat<f64>,
        ObjectiveSense,
    )>,
    phantom: PhantomData<(V, E, C)>,
}

impl<
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
    E: UsableData + From<ReifiedVar<D::Connection>>,
    C: UsableData + From<Option<Origin<D::Connection>>>,
> ScriptFeeder<D, V, E, C>
{
    pub async fn new(modules: &BTreeMap<&str, &str>) -> Result<Self, ScriptError<D::Connection>> {
        let base_vars = V::field_schema();
        let ast = CheckedAST::new(modules, base_vars.clone()).await?;

        Ok(ScriptFeeder {
            ast,
            base_vars,
            pending_constraints: vec![],
            pending_objectives: vec![],
            phantom: PhantomData,
        })
    }

    pub fn get_warnings(&self) -> &[SemWarning] {
        self.ast.get_warnings()
    }

    pub fn get_fn_signature(&self, module: &str, fn_name: &str) -> Option<(ArgsType, ExprType)> {
        let functions = self.ast.get_functions();
        let key = (module.to_string(), fn_name.to_string());
        functions.get(&key).cloned()
    }

    pub fn get_fn_from_module(&self, module: &str) -> BTreeMap<String, (ArgsType, ExprType)> {
        self.ast
            .get_functions()
            .into_iter()
            .filter(|((mod_name, _), _)| mod_name == module)
            .map(|((_, fn_name), sig)| (fn_name, sig))
            .collect()
    }

    fn validate_function(
        &self,
        module: &str,
        fn_name: &str,
        args: &[ExprValue<D::Connection>],
        expected_return: &ExprType,
    ) -> Result<bool, ScriptError<D::Connection>> {
        let functions = self.ast.get_functions();
        let key = (module.to_string(), fn_name.to_string());

        let (args_type, output_type) = functions
            .get(&key)
            .ok_or_else(|| ScriptError::UnknownFunction(format!("{}::{}", module, fn_name)))?;

        let needs_db =
            !args_type.is_empty() && self.ast.global_env.is_database_schema_deep(&args_type[0]);
        let expected_args = if needs_db {
            args_type.len() - 1
        } else {
            args_type.len()
        };

        if args.len() != expected_args {
            return Err(ScriptError::ArgumentCountMismatch {
                func: format!("{}::{}", module, fn_name),
                expected: expected_args,
                found: args.len(),
            });
        }

        if !output_type.is_subtype_of(expected_return)
            && !expected_return.is_subtype_of(output_type)
        {
            return Err(ScriptError::WrongReturnType {
                func: format!("{}::{}", module, fn_name),
                returned: output_type.clone(),
                expected: expected_return.clone(),
            });
        }

        Ok(needs_db)
    }

    pub fn add_constraint(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<D::Connection>>,
    ) -> Result<(), ScriptError<D::Connection>> {
        let constraint_type = ExprType::from_variants([
            SimpleType::Constraint,
            SimpleType::List(SimpleType::Constraint.into()),
        ]);

        let needs_db = self.validate_function(module, fn_name, &args, &constraint_type)?;

        self.pending_constraints
            .push((module.to_string(), fn_name.to_string(), args, needs_db));
        Ok(())
    }

    pub fn add_objective(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<D::Connection>>,
        coefficient: f64,
        sense: ObjectiveSense,
    ) -> Result<(), ScriptError<D::Connection>> {
        let obj_types = ExprType::from_variants([
            SimpleType::LinExpr,
            SimpleType::Constraint,
            SimpleType::List(ExprType::from_variants([
                SimpleType::LinExpr,
                SimpleType::Constraint,
            ])),
        ]);

        let needs_db = self.validate_function(module, fn_name, &args, &obj_types)?;

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
        db_connection: Option<D::Connection>,
    ) -> Result<
        IntConstraintBundle<'static, V, E, C, (), ReifyError<V, E>>,
        ScriptError<D::Connection>,
    >
    where
        V: 'static,
        E: 'static,
        C: 'static,
    {
        // Phase 1: DSL evaluation
        let (constraint_results, objective_results, var_def) = {
            let mut eval_history = self.ast.start_eval_history();

            let db_value = db_connection
                .map(|conn| ExprValue::Database(crate::eval::database::DatabaseHandle::new(conn)));

            let mut constraint_results = Vec::new();
            for (module, fn_name, args, needs_db) in self.pending_constraints.iter() {
                let eval_args = if *needs_db {
                    let db_val = db_value.as_ref().ok_or_else(|| {
                        ScriptError::MissingDatabaseConnection(format!("{}::{}", module, fn_name))
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
                        EvalError::Panic(v) => ScriptError::Panic(v),
                        _ => panic!(
                            "Evaluation should succeed (function was validated): {:?}",
                            e
                        ),
                    })?;
                constraint_results.push((module.clone(), fn_name.clone(), result));
            }

            let mut objective_results = Vec::new();
            for (module, fn_name, args, needs_db, coef, obj_sense) in self.pending_objectives.iter()
            {
                let eval_args = if *needs_db {
                    let db_val = db_value.as_ref().ok_or_else(|| {
                        ScriptError::MissingDatabaseConnection(format!("{}::{}", module, fn_name))
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
                        EvalError::Panic(v) => ScriptError::Panic(v),
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
                EvalError::Panic(v) => ScriptError::Panic(v),
                _ => panic!(
                    "Evaluation should succeed (variables were validated): {:?}",
                    e
                ),
            })?;
            (constraint_results, objective_results, var_def)
        };

        // Phase 2: Build the IntConstraintBundle
        let mut bundle: IntConstraintBundle<'static, V, E, C, (), ReifyError<V, E>> =
            IntConstraintBundle::new();

        // Add user constraints
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
                let constraint = convert_int_constraint::<D::Connection, V, E>(
                    &c_with_o.constraint,
                    &self.base_vars,
                );
                bundle = bundle.with_constraint(constraint, C::from(Some(origin)));
            }
        }

        // Add objectives
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
                            convert_int_linexpr::<D::Connection, V, E>(&lin_expr, &self.base_vars);
                        bundle = bundle.with_objective(
                            coef.0,
                            Objective::new(cleaned.into_linexpr(), obj_sense),
                        );
                    }
                    ExprValue::Constraint(c) => {
                        let int_constraints: Vec<_> = c
                            .into_iter()
                            .map(|c_with_o| {
                                let converted = convert_int_constraint::<D::Connection, V, E>(
                                    &c_with_o.constraint,
                                    &self.base_vars,
                                );
                                (converted, C::from(None))
                            })
                            .collect();
                        if int_constraints.is_empty() {
                            continue;
                        }
                        let sub_bundle = IntConstraintBundle::from_constraints(int_constraints);
                        let objectify_var = E::from(ReifiedVar {
                            module: String::new(),
                            name: format!("__obj_{}", objectify_counter),
                            params: vec![],
                        });
                        objectify_counter += 1;
                        let objectified = sub_bundle
                            .objectify_with_coef(objectify_var, coef.0)
                            .expect("objectification should work");
                        bundle = bundle.merge(objectified).expect("no duplicate extras");
                    }
                    _ => panic!(
                        "Function {}::{} returned {:?} instead of LinExpr",
                        _module, _fn_name, value
                    ),
                }
            }
        }

        // Add reified variables
        for (hashed_key, constraints) in var_def.vars {
            let (var_module, var_name, var_args) = hashed_key.into_inner();
            let reified_var = E::from(ReifiedVar {
                module: var_module,
                name: var_name,
                params: var_args,
            });
            let int_constraints: Vec<_> = constraints
                .into_iter()
                .map(|c| {
                    let converted =
                        convert_int_constraint::<D::Connection, V, E>(&c, &self.base_vars);
                    (converted, C::from(None))
                })
                .collect();
            let sub_bundle = IntConstraintBundle::from_constraints(int_constraints);
            let reified = sub_bundle
                .reify(reified_var)
                .expect("reification should work");
            bundle = bundle.merge(reified).expect("no duplicate extras");
        }

        Ok(bundle)
    }
}

// ---------------------------------------------------------------------------
// Conversion functions (generic over E)
// ---------------------------------------------------------------------------

fn convert_ilp_var<
    D: DatabaseConnection,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D>, Error = VarConversionError>,
    E: UsableData + From<ReifiedVar<D>>,
>(
    var: &HashedIlpVar<D>,
    base_vars: &HashMap<String, ArgsType>,
) -> Var<V, E> {
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
        }) => Var::Extra(E::from(ReifiedVar {
            module: module.clone(),
            name: name.clone(),
            params: params.clone(),
        })),
    }
}

fn convert_int_constraint<
    D: DatabaseConnection,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D>, Error = VarConversionError>,
    E: UsableData + From<ReifiedVar<D>>,
>(
    constraint: &IntConstraint<HashedIlpVar<D>>,
    base_vars: &HashMap<String, ArgsType>,
) -> IntConstraint<Var<V, E>> {
    constraint.transmute(|v| convert_ilp_var::<D, V, E>(v, base_vars))
}

fn convert_int_linexpr<
    D: DatabaseConnection,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D>, Error = VarConversionError>,
    E: UsableData + From<ReifiedVar<D>>,
>(
    expr: &IntLinExpr<HashedIlpVar<D>>,
    base_vars: &HashMap<String, ArgsType>,
) -> IntLinExpr<Var<V, E>> {
    expr.transmute(|v| convert_ilp_var::<D, V, E>(v, base_vars))
}

// ---------------------------------------------------------------------------
// DB wrapping helper
// ---------------------------------------------------------------------------

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
