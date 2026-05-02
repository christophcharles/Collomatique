use crate::native_extras::build_native_extras;
use crate::problem::Problem;
use crate::types::{ConstraintDesc, ReifiedVarName};
use collo_ml::script_feeder::{ScriptError, ScriptFeeder};
use collo_ml::{SemWarning, SqliteDatabaseConnection, SqliteDatabaseDriver};
use collomatique_binding_colloscopes::scripts::SimpleScriptError;
use collomatique_binding_colloscopes::vars::{Var, VarEnv};
use collomatique_ilp::{ObjectiveSense, Variable};
use collomatique_ilp_modeler::Modeler;
use collomatique_ilp_modeler::bundle::ReifyError;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemBuilder {
    feeder: ScriptFeeder<SqliteDatabaseDriver, Var, ReifiedVarName, ConstraintDesc>,
}

pub(crate) type MyModeler<'m> =
    Modeler<'m, Var, ReifiedVarName, ConstraintDesc, VarEnv, ReifyError<Var, ReifiedVarName>>;

impl ProblemBuilder {
    pub fn get_warnings(&self) -> &[SemWarning] {
        self.feeder.get_warnings()
    }

    pub async fn build(
        self,
        db: &sqlx::SqlitePool,
        db_connection: Option<SqliteDatabaseConnection>,
    ) -> Result<Problem, ScriptError<SqliteDatabaseConnection>> {
        let script_bundle = self.feeder.build(db_connection).await?;

        let env = VarEnv::load(db).await;

        let mut modeler: MyModeler<'_> = Modeler::from_described(&env);

        let original_var_list: HashMap<Var, Variable> = modeler
            .base_vars()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, desc) in &original_var_list {
            if !desc.is_integer() {
                return Err(ScriptError::NonIntegerVariable(format!("{:?}", name)));
            }
        }

        modeler
            .apply_bundle(script_bundle.into_general())
            .expect("no duplicate extras from script");

        let native_bundle = build_native_extras(&env);
        modeler
            .apply_bundle(native_bundle.into_general())
            .expect("no duplicate extras from native");

        let students_per_group_bundle = crate::students_per_group::build(&env);
        modeler
            .apply_bundle(students_per_group_bundle.into_general())
            .expect("no duplicate extras from students_per_group");

        let students_have_groups_bundle = crate::students_have_groups::build(&env);
        modeler
            .apply_bundle(students_have_groups_bundle.into_general())
            .expect("no duplicate extras from students_have_groups");

        let students_per_group_for_subject_bundle =
            crate::students_per_group_for_subject::build(&env);
        modeler
            .apply_bundle(students_per_group_for_subject_bundle.into_general())
            .expect("no duplicate extras from students_per_group_for_subject");

        let groups_filled_by_ascending_order_bundle =
            crate::groups_filled_by_ascending_order::build(&env);
        modeler
            .apply_bundle(groups_filled_by_ascending_order_bundle.into_general())
            .expect("no duplicate extras from groups_filled_by_ascending_order");

        let forbidden_groups_bundle = crate::forbidden_groups::build(&env);
        modeler
            .apply_bundle(forbidden_groups_bundle.into_general())
            .expect("no duplicate extras from forbidden_groups");

        let model = modeler
            .build(&env)
            .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e));

        Ok(Problem::from_model(model, original_var_list))
    }
}

pub async fn default_problem_builder(
    main_module: &str,
) -> Result<ProblemBuilder, SimpleScriptError> {
    use collo_ml::eval::CompileError;
    use collo_ml::script_feeder::ScriptError;
    use collomatique_binding_colloscopes::scripts::MODULES;

    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", main_module);

    let mut feeder = ScriptFeeder::new(&modules).await.map_err(|e| match e {
        ScriptError::CompileError(compile_error) => match compile_error {
            CompileError::ParsingError(parse_err) => SimpleScriptError::ParsingError(parse_err),
            CompileError::SemanticsError { errors, warnings } => {
                SimpleScriptError::SemanticErrors { errors, warnings }
            }
            other => SimpleScriptError::UnexpectedError(format!("{}", other)),
        },
        other => SimpleScriptError::UnexpectedError(format!("{}", other)),
    })?;

    let functions = feeder.get_fn_from_module("main");

    for (fn_name, _) in &functions {
        if fn_name == "constraint" || fn_name.starts_with("constraint_") {
            feeder
                .add_constraint("main", fn_name, vec![])
                .map_err(|e| SimpleScriptError::UnexpectedError(format!("{}", e)))?;
        } else if fn_name == "objective" || fn_name.starts_with("objective_") {
            feeder
                .add_objective("main", fn_name, vec![], 1.0, ObjectiveSense::Minimize)
                .map_err(|e| SimpleScriptError::UnexpectedError(format!("{}", e)))?;
        }
    }

    Ok(ProblemBuilder { feeder })
}
